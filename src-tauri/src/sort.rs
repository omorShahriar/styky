//! List ordering (PLAN §3): overdue first, then urgent, then due soonest, then
//! no-due items by `sort_order`.

use crate::model::{Item, PRIORITY_URGENT};

/// Open items: ordered per the plan. Completed items: most recently completed first.
pub fn sort_items(items: &mut [Item], now: i64) {
    items.sort_by(|a, b| {
        a.is_done()
            .cmp(&b.is_done())
            .then_with(|| {
                if a.is_done() {
                    b.completed_at.cmp(&a.completed_at)
                } else {
                    open_key(a, now)
                        .partial_cmp(&open_key(b, now))
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            })
            .then_with(|| a.created_at.cmp(&b.created_at))
    });
}

/// (not overdue, not urgent, no due date, effective time, sort_order)
fn open_key(i: &Item, now: i64) -> (u8, u8, u8, i64, f64) {
    let eff = i.effective_at();
    let overdue = eff.is_some_and(|t| t < now);
    (
        u8::from(!overdue),
        u8::from(i.priority < PRIORITY_URGENT),
        u8::from(eff.is_none()),
        eff.unwrap_or(0),
        i.sort_order,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, due: Option<i64>, prio: i64, order: f64) -> Item {
        Item {
            id: id.into(),
            title: id.into(),
            body_md: String::new(),
            due_at: due,
            repeat: "none".into(),
            priority: prio,
            tag: None,
            link: None,
            branch: None,
            status: "open".into(),
            snoozed_until: None,
            last_fired_at: None,
            completed_at: None,
            sort_order: order,
            created_at: 0,
            repeat_anchor_day: None,
            prealert_min: None,
            pre_fired_at: None,
            last_nag_at: None,
            updated_at: 0,
        }
    }

    fn ids(v: &[Item]) -> Vec<&str> {
        v.iter().map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn overdue_then_urgent_then_due_then_manual() {
        let now = 1_000;
        let mut v = vec![
            item("plain-b", None, 1, 2.0),
            item("later", Some(5_000), 1, 9.0),
            item("urgent-nodue", None, 2, 5.0),
            item("plain-a", None, 1, 1.0),
            item("soon", Some(2_000), 1, 9.0),
            item("overdue", Some(500), 1, 9.0),
        ];
        sort_items(&mut v, now);
        assert_eq!(
            ids(&v),
            ["overdue", "urgent-nodue", "soon", "later", "plain-a", "plain-b"]
        );
    }

    #[test]
    fn overdue_urgent_leads_overdue_normal() {
        let now = 1_000;
        let mut v = vec![item("n", Some(100), 1, 0.0), item("u", Some(900), 2, 0.0)];
        sort_items(&mut v, now);
        assert_eq!(ids(&v), ["u", "n"]);
    }

    #[test]
    fn snooze_moves_effective_time() {
        let now = 1_000;
        let mut snoozed = item("snoozed", Some(500), 1, 0.0);
        snoozed.snoozed_until = Some(9_000); // no longer overdue
        let mut v = vec![snoozed, item("due", Some(2_000), 1, 0.0)];
        sort_items(&mut v, now);
        assert_eq!(ids(&v), ["due", "snoozed"]);
    }

    #[test]
    fn completed_items_follow_open_newest_first() {
        let mut a = item("a", None, 1, 0.0);
        a.status = "done".into();
        a.completed_at = Some(10);
        let mut b = item("b", None, 1, 0.0);
        b.status = "done".into();
        b.completed_at = Some(20);
        let open = item("open", None, 1, 0.0);
        let mut v = vec![a, b, open];
        sort_items(&mut v, 0);
        assert_eq!(ids(&v), ["open", "b", "a"]);
    }
}
