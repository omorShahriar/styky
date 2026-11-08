//! Due-time engine. A Tokio task wakes at the next effective fire time
//! (`snoozed_until ?? due_at`), never later than 30 s, and re-plans whenever the
//! item list changes. It reads the wall clock on every pass, so sleep/resume and
//! clock changes just work; `last_fired_at` makes sure nothing fires twice.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Listener, Manager};
use tokio::sync::Notify;

use crate::commands::now_ms;
use crate::db::Db;
use crate::model::{Item, PRIORITY_CRITICAL};
use crate::notify;

const MAX_SLEEP_MS: i64 = 30_000;
/// A wall-clock jump bigger than this between passes means sleep/resume.
const GAP_MS: i64 = 90_000;
/// While catching up, items due longer ago than this are "missed" (summarised).
const MISSED_AFTER_MS: i64 = 60_000;
pub const PAUSE_MS: i64 = 3_600_000;

/// Reminders are held back until this wall-clock time (0 = not paused).
static PAUSED_UNTIL: AtomicI64 = AtomicI64::new(0);

pub struct Scheduler {
    wake: Arc<Notify>,
}

pub fn paused_until() -> Option<i64> {
    let t = PAUSED_UNTIL.load(Ordering::SeqCst);
    (t > now_ms()).then_some(t)
}

/// Pause reminders for an hour, or resume if already paused. Returns the new pause end.
pub fn toggle_pause(app: &AppHandle) -> Option<i64> {
    let until = if paused_until().is_some() { 0 } else { now_ms() + PAUSE_MS };
    PAUSED_UNTIL.store(until, Ordering::SeqCst);
    if let Some(s) = app.try_state::<Scheduler>() {
        s.wake.notify_one();
    }
    (until > 0).then_some(until)
}

/// What to do at `now`: the items to fire, and when the next one is due.
#[derive(Debug, PartialEq, Default)]
pub struct Plan {
    /// Items whose due (or snooze) time has arrived.
    pub due: Vec<Item>,
    /// Pro: items whose pre-alert time has arrived (still before the due time).
    pub pre: Vec<Item>,
    /// Pro: Critical items that were notified and are still open: notify again.
    pub nag: Vec<Item>,
    pub next: Option<i64>,
}

/// Settings that shape escalation (Pro).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Escalation {
    pub pro: bool,
    /// Minutes between Critical re-notifications.
    pub nag_minutes: i64,
}

pub const DEFAULT_NAG_MINUTES: i64 = 10;

/// Decide what fires at `now`. Every kind of fire has its own "already fired" marker
/// (`last_fired_at`, `pre_fired_at`, `last_nag_at`), written *before* the toast is shown, so a pass can
/// be repeated (or two can overlap after sleep/resume) without ever notifying twice.
pub fn plan(items: &[Item], now: i64, esc: Escalation) -> Plan {
    let mut p = Plan::default();
    let mut next: Option<i64> = None;
    let mut soon = |t: i64| next = Some(next.map_or(t, |n: i64| n.min(t)));
    for item in items.iter().filter(|i| !i.is_done()) {
        let Some(at) = item.effective_at() else { continue };
        if at <= now {
            // Fired already for this (possibly snoozed) time?
            if item.last_fired_at.is_none_or(|f| f < at) {
                p.due.push(item.clone());
            } else if esc.pro && item.priority == PRIORITY_CRITICAL {
                // Notified, still open: nag every N minutes since the last notification.
                let since = item.last_fired_at.unwrap_or(0).max(item.last_nag_at.unwrap_or(0));
                let again = since + esc.nag_minutes.max(1) * 60_000;
                if again <= now {
                    p.nag.push(item.clone());
                } else {
                    soon(again);
                }
            }
        } else {
            soon(at);
        }
        // Pre-alert: relative to the due time, only while the item is not snoozed and not yet due.
        if esc.pro
            && item.snoozed_until.is_none()
            && let (Some(due), Some(min)) = (item.due_at, item.prealert_min)
        {
            let at_pre = due - min * 60_000;
            if due > now && item.pre_fired_at.is_none_or(|f| f < at_pre) {
                if at_pre <= now {
                    p.pre.push(item.clone());
                } else {
                    soon(at_pre);
                }
            }
        }
    }
    p.next = next;
    p
}

/// Split due items into (missed -> summarise, fresh -> individual toasts).
pub fn split_missed(due: Vec<Item>, now: i64, catching_up: bool) -> (Vec<Item>, Vec<Item>) {
    if !catching_up {
        return (Vec::new(), due);
    }
    let (missed, fresh): (Vec<_>, Vec<_>) = due
        .into_iter()
        .partition(|i| i.effective_at().is_some_and(|t| t < now - MISSED_AFTER_MS));
    // One missed reminder is just a normal toast.
    if missed.len() < 2 {
        let mut all = missed;
        all.extend(fresh);
        return (Vec::new(), all);
    }
    (missed, fresh)
}

pub fn spawn(app: AppHandle) {
    let wake = Arc::new(Notify::new());
    app.manage(Scheduler { wake: wake.clone() });
    let w = wake.clone();
    app.listen("items-changed", move |_| w.notify_one());
    tauri::async_runtime::spawn(run(app, wake));
}

/// Escalation settings: Pro gate, nag interval and whether a nag raises the panel.
fn escalation(db: &Db) -> (Escalation, bool) {
    let s = db.settings().unwrap_or_default();
    let nag_minutes = s
        .get("escalation_minutes")
        .and_then(|v| v.parse::<i64>().ok())
        .map_or(DEFAULT_NAG_MINUTES, |m| m.clamp(1, 240));
    let bring = s.get("escalation_bring_forward").is_some_and(|v| v == "1");
    (Escalation { pro: crate::license::is_pro(), nag_minutes }, bring)
}

async fn run(app: AppHandle, wake: Arc<Notify>) {
    let mut last_tick = now_ms();
    let mut catching_up = true; // app start
    let mut was_paused = false;
    loop {
        let now = now_ms();
        if now - last_tick > GAP_MS || now < last_tick {
            catching_up = true; // woke from sleep, or the clock moved
        }
        last_tick = now;

        let mut sleep_ms = MAX_SLEEP_MS;
        if let Some(until) = paused_until() {
            was_paused = true;
            sleep_ms = sleep_ms.min(until - now);
        } else {
            if was_paused {
                was_paused = false;
                catching_up = true;
            }
            let db = app.state::<Db>();
            match db.list_open() {
                Ok(items) => {
                    let (esc, bring_forward) = escalation(&db);
                    let p = plan(&items, now, esc);
                    if !p.pre.is_empty() {
                        let ids: Vec<String> = p.pre.iter().map(|i| i.id.clone()).collect();
                        if let Err(e) = db.mark_pre_fired(&ids, now) {
                            eprintln!("styky: mark_pre_fired failed: {e}");
                        }
                        for item in &p.pre {
                            notify::show_prealert(&app, item);
                        }
                    }
                    if !p.nag.is_empty() {
                        let ids: Vec<String> = p.nag.iter().map(|i| i.id.clone()).collect();
                        if let Err(e) = db.mark_nagged(&ids, now) {
                            eprintln!("styky: mark_nagged failed: {e}");
                        }
                        for item in &p.nag {
                            notify::show_nag(&app, item);
                        }
                        if bring_forward {
                            crate::window::show_panel(&app);
                        }
                    }
                    if !p.due.is_empty() {
                        let ids: Vec<String> = p.due.iter().map(|i| i.id.clone()).collect();
                        // Mark first: a slow toast must never lead to a second one.
                        if let Err(e) = db.mark_fired(&ids, now) {
                            eprintln!("styky: mark_fired failed: {e}");
                        }
                        let (missed, fresh) = split_missed(p.due, now, catching_up);
                        if !missed.is_empty() {
                            notify::show_summary(&app, &missed);
                        }
                        for item in &fresh {
                            notify::show_item(&app, item);
                        }
                    }
                    catching_up = false;
                    if let Some(next) = p.next {
                        sleep_ms = sleep_ms.min(next - now);
                    }
                }
                Err(e) => eprintln!("styky: scheduler could not read items: {e}"),
            }
        }
        let dur = Duration::from_millis(sleep_ms.max(250) as u64);
        let _ = tokio::time::timeout(dur, wake.notified()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FREE: Escalation = Escalation { pro: false, nag_minutes: 10 };
    const PRO: Escalation = Escalation { pro: true, nag_minutes: 10 };
    const MIN: i64 = 60_000;

    fn item(id: &str, due: Option<i64>, snoozed: Option<i64>, fired: Option<i64>) -> Item {
        Item {
            id: id.into(),
            title: id.into(),
            body_md: String::new(),
            due_at: due,
            repeat: "none".into(),
            priority: 1,
            tag: None,
            link: None,
            branch: None,
            status: "open".into(),
            snoozed_until: snoozed,
            last_fired_at: fired,
            completed_at: None,
            sort_order: 0.0,
            created_at: 0,
            updated_at: 0,
            repeat_anchor_day: None,
            prealert_min: None,
            pre_fired_at: None,
            last_nag_at: None,
        }
    }

    #[test]
    fn plans_due_items_and_the_next_wake() {
        let items = [item("a", Some(100), None, None), item("b", Some(500), None, None), item("c", Some(900), None, None)];
        let p = plan(&items, 600, FREE);
        assert_eq!(p.due.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(p.next, Some(900));
    }

    #[test]
    fn effective_time_prefers_snooze_and_never_double_fires() {
        // Snoozed past now: not due, wakes at the snooze time.
        let p = plan(&[item("a", Some(100), Some(800), None)], 600, FREE);
        assert!(p.due.is_empty());
        assert_eq!(p.next, Some(800));
        // Already fired for this effective time.
        assert!(plan(&[item("a", Some(100), None, Some(150))], 600, FREE).due.is_empty());
        // Fired for the due time, then snoozed and the snooze elapsed: fires again.
        assert_eq!(plan(&[item("a", Some(100), Some(500), Some(150))], 600, FREE).due.len(), 1);
    }

    #[test]
    fn done_and_undated_items_are_ignored() {
        let mut done = item("d", Some(1), None, None);
        done.status = "done".into();
        let p = plan(&[done, item("n", None, None, None)], 100, FREE);
        assert_eq!(p, Plan::default());
    }

    #[test]
    fn catching_up_summarises_two_or_more_missed_but_not_one() {
        let now = 1_000_000;
        let old = |id: &str| item(id, Some(now - 600_000), None, None);
        let fresh = item("f", Some(now - 5_000), None, None);
        let (missed, rest) = split_missed(vec![old("a"), old("b"), fresh.clone()], now, true);
        assert_eq!((missed.len(), rest.len()), (2, 1));
        let (missed, rest) = split_missed(vec![old("a"), fresh.clone()], now, true);
        assert_eq!((missed.len(), rest.len()), (0, 2));
        let (missed, rest) = split_missed(vec![old("a"), old("b")], now, false);
        assert_eq!((missed.len(), rest.len()), (0, 2));
    }

    fn pre_item(due: i64, pre_min: i64, pre_fired: Option<i64>) -> Item {
        let mut i = item("p", Some(due), None, None);
        i.prealert_min = Some(pre_min);
        i.pre_fired_at = pre_fired;
        i
    }

    #[test]
    fn prealert_fires_once_inside_its_window() {
        let due = 100 * MIN;
        let it = pre_item(due, 15, None);
        // Before the window: nothing now, wake at due-15min.
        let p = plan(std::slice::from_ref(&it), 80 * MIN, PRO);
        assert!(p.pre.is_empty() && p.due.is_empty());
        assert_eq!(p.next, Some(85 * MIN));
        // Inside the window: pre fires.
        let p = plan(std::slice::from_ref(&it), 86 * MIN, PRO);
        assert_eq!(p.pre.len(), 1);
        assert_eq!(p.next, Some(due));
        // Once marked it never fires again, however often we plan.
        let fired = pre_item(due, 15, Some(86 * MIN));
        for now in [86 * MIN, 90 * MIN, 99 * MIN] {
            assert!(plan(std::slice::from_ref(&fired), now, PRO).pre.is_empty(), "{now}");
        }
        // At the due time only the main toast fires; a stale pre-alert is dropped, not sent late.
        let p = plan(std::slice::from_ref(&it), due, PRO);
        assert!(p.pre.is_empty());
        assert_eq!(p.due.len(), 1);
    }

    #[test]
    fn prealert_is_pro_only_and_skipped_while_snoozed() {
        let it = pre_item(100 * MIN, 30, None);
        assert!(plan(std::slice::from_ref(&it), 80 * MIN, FREE).pre.is_empty());
        let mut snoozed = it.clone();
        snoozed.snoozed_until = Some(120 * MIN);
        assert!(plan(&[snoozed], 80 * MIN, PRO).pre.is_empty());
        // No pre-alert chosen: nothing.
        let mut none = it;
        none.prealert_min = None;
        assert!(plan(&[none], 80 * MIN, PRO).pre.is_empty());
    }

    #[test]
    fn a_new_due_time_re_arms_the_prealert() {
        // pre_fired_at belongs to the old due time (it is before the new pre-alert time), so it fires again.
        let mut it = pre_item(200 * MIN, 15, Some(86 * MIN));
        assert!(plan(std::slice::from_ref(&it), 190 * MIN, PRO).pre.len() == 1);
        it.pre_fired_at = Some(186 * MIN);
        assert!(plan(&[it], 190 * MIN, PRO).pre.is_empty());
    }

    fn critical(due: i64, fired: Option<i64>, nagged: Option<i64>) -> Item {
        let mut i = item("c", Some(due), None, fired);
        i.priority = 3;
        i.last_nag_at = nagged;
        i
    }

    #[test]
    fn critical_items_re_notify_every_n_minutes_after_the_first_toast() {
        let due = 100 * MIN;
        // The first fire is the normal one, not a nag.
        let p = plan(&[critical(due, None, None)], due, PRO);
        assert_eq!((p.due.len(), p.nag.len()), (1, 0));
        // Fired at 100: quiet until 110, then nag; next nag 10 minutes after the last nag.
        let fired = critical(due, Some(100 * MIN), None);
        let p = plan(std::slice::from_ref(&fired), 105 * MIN, PRO);
        assert!(p.nag.is_empty() && p.due.is_empty());
        assert_eq!(p.next, Some(110 * MIN));
        assert_eq!(plan(std::slice::from_ref(&fired), 110 * MIN, PRO).nag.len(), 1);
        let nagged = critical(due, Some(100 * MIN), Some(110 * MIN));
        assert!(plan(std::slice::from_ref(&nagged), 115 * MIN, PRO).nag.is_empty());
        assert_eq!(plan(std::slice::from_ref(&nagged), 115 * MIN, PRO).next, Some(120 * MIN));
        assert_eq!(plan(std::slice::from_ref(&nagged), 120 * MIN, PRO).nag.len(), 1);
        // The interval is a setting.
        let fast = Escalation { pro: true, nag_minutes: 2 };
        assert_eq!(plan(&[fired], 102 * MIN, fast).nag.len(), 1);
    }

    #[test]
    fn nagging_stops_when_done_snoozed_downgraded_or_not_pro() {
        let due = 100 * MIN;
        let base = critical(due, Some(100 * MIN), None);
        let now = 150 * MIN;
        assert_eq!(plan(std::slice::from_ref(&base), now, PRO).nag.len(), 1);
        // Free build: critical behaves like any other item (no nag).
        assert!(plan(std::slice::from_ref(&base), now, FREE).nag.is_empty());
        let mut done = base.clone();
        done.status = "done".into();
        assert_eq!(plan(&[done], now, PRO), Plan::default());
        // Snoozed to the future: silent until then, then the normal fire (not a nag).
        let mut snoozed = base.clone();
        snoozed.snoozed_until = Some(200 * MIN);
        snoozed.last_fired_at = None;
        snoozed.last_nag_at = None;
        let p = plan(std::slice::from_ref(&snoozed), now, PRO);
        assert!(p.nag.is_empty() && p.due.is_empty());
        assert_eq!(p.next, Some(200 * MIN));
        let p = plan(&[snoozed], 200 * MIN, PRO);
        assert_eq!((p.due.len(), p.nag.len()), (1, 0));
        // Urgent (not critical) never nags.
        let mut urgent = base;
        urgent.priority = 2;
        assert!(plan(&[urgent], now, PRO).nag.is_empty());
    }

    #[test]
    fn a_pass_never_double_fires_when_markers_are_written_first() {
        // Simulate the run loop: plan, write markers (as mark_* do), plan again at the same instant.
        let now = 100 * MIN;
        let mut it = critical(now, None, None);
        it.prealert_min = Some(5);
        let p1 = plan(std::slice::from_ref(&it), now, PRO);
        assert_eq!(p1.due.len(), 1);
        it.last_fired_at = Some(now);
        let p2 = plan(std::slice::from_ref(&it), now, PRO);
        assert_eq!((p2.due.len(), p2.pre.len(), p2.nag.len()), (0, 0, 0));
        // Resume from sleep much later: one nag only, then the marker blocks the rest of the same pass.
        let later = now + 500 * MIN;
        let p3 = plan(std::slice::from_ref(&it), later, PRO);
        assert_eq!(p3.nag.len(), 1);
        it.last_nag_at = Some(later);
        assert!(plan(&[it], later, PRO).nag.is_empty());
    }
}
