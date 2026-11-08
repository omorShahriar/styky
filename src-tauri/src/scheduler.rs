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
use crate::model::Item;
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
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub due: Vec<Item>,
    pub next: Option<i64>,
}

pub fn plan(items: &[Item], now: i64) -> Plan {
    let mut due = Vec::new();
    let mut next: Option<i64> = None;
    for item in items.iter().filter(|i| !i.is_done()) {
        let Some(at) = item.effective_at() else { continue };
        if at <= now {
            // Fired already for this (possibly snoozed) time?
            if item.last_fired_at.is_none_or(|f| f < at) {
                due.push(item.clone());
            }
        } else {
            next = Some(next.map_or(at, |n| n.min(at)));
        }
    }
    Plan { due, next }
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
                    let p = plan(&items, now);
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
        }
    }

    #[test]
    fn plans_due_items_and_the_next_wake() {
        let items = [item("a", Some(100), None, None), item("b", Some(500), None, None), item("c", Some(900), None, None)];
        let p = plan(&items, 600);
        assert_eq!(p.due.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(p.next, Some(900));
    }

    #[test]
    fn effective_time_prefers_snooze_and_never_double_fires() {
        // Snoozed past now: not due, wakes at the snooze time.
        let p = plan(&[item("a", Some(100), Some(800), None)], 600);
        assert!(p.due.is_empty());
        assert_eq!(p.next, Some(800));
        // Already fired for this effective time.
        assert!(plan(&[item("a", Some(100), None, Some(150))], 600).due.is_empty());
        // Fired for the due time, then snoozed and the snooze elapsed: fires again.
        assert_eq!(plan(&[item("a", Some(100), Some(500), Some(150))], 600).due.len(), 1);
    }

    #[test]
    fn done_and_undated_items_are_ignored() {
        let mut done = item("d", Some(1), None, None);
        done.status = "done".into();
        let p = plan(&[done, item("n", None, None, None)], 100);
        assert_eq!(p, Plan { due: vec![], next: None });
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
}
