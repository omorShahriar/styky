//! Next-occurrence maths for repeating items (local time).

use chrono::{DateTime, Datelike, Days, Local, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone, Weekday};

fn to_local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

/// Local wall-clock time -> ms, resolving DST gaps/overlaps sensibly.
pub fn from_naive(n: NaiveDateTime) -> Option<i64> {
    match Local.from_local_datetime(&n) {
        LocalResult::Single(d) => Some(d.timestamp_millis()),
        LocalResult::Ambiguous(a, _) => Some(a.timestamp_millis()),
        // Spring-forward gap: nudge an hour ahead.
        LocalResult::None => Local
            .from_local_datetime(&(n + chrono::Duration::hours(1)))
            .earliest()
            .map(|d| d.timestamp_millis()),
    }
}

fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

/// One step forward from `date` for the given rule. `anchor_day` keeps monthly
/// repeats from drifting within a single catch-up (31st -> 28th -> 31st).
fn step(date: NaiveDate, repeat: &str, anchor_day: u32, months_from_start: &mut u32, start: NaiveDate) -> Option<NaiveDate> {
    match repeat {
        "daily" => date.checked_add_days(Days::new(1)),
        "weekly" => date.checked_add_days(Days::new(7)),
        "weekdays" => {
            let mut d = date.checked_add_days(Days::new(1))?;
            while is_weekend(d) {
                d = d.checked_add_days(Days::new(1))?;
            }
            Some(d)
        }
        "monthly" => {
            *months_from_start += 1;
            // Add whole months to the first of the start month, then clamp the day.
            let first = start.with_day(1)?.checked_add_months(Months::new(*months_from_start))?;
            let last = first.checked_add_months(Months::new(1))?.pred_opt()?;
            first.with_day(anchor_day.min(last.day()))
        }
        _ => None,
    }
}

/// Local day-of-month of a timestamp (the "anchor" a monthly repeat keeps returning to).
pub fn local_day(ms: i64) -> Option<u32> {
    to_local(ms).map(|d| d.day())
}

/// The next due time after `due_ms` (and strictly after `now_ms`, so a long-overdue
/// repeating item catches up instead of staying in the past). `None` for
/// non-repeating items.
pub fn next_occurrence(due_ms: i64, repeat: &str, now_ms: i64, anchor: Option<u32>) -> Option<i64> {
    if !matches!(repeat, "daily" | "weekdays" | "weekly" | "monthly") {
        return None;
    }
    let due = to_local(due_ms)?;
    let time = due.time();
    let start = due.date_naive();
    let anchor_day = anchor.filter(|d| (1..=31).contains(d)).unwrap_or_else(|| start.day());
    let mut date = start;
    let mut months = 0u32;
    for _ in 0..10_000 {
        date = step(date, repeat, anchor_day, &mut months, start)?;
        let ms = from_naive(date.and_time(time))?;
        if ms > now_ms && ms > due_ms {
            return Some(ms);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
        Local.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap().timestamp_millis()
    }

    #[test]
    fn none_does_not_repeat() {
        assert_eq!(next_occurrence(ms(2026, 1, 5, 9, 0), "none", 0, None), None);
    }

    #[test]
    fn daily_advances_one_day_keeping_time() {
        let due = ms(2026, 3, 10, 17, 30);
        assert_eq!(next_occurrence(due, "daily", due, None), Some(ms(2026, 3, 11, 17, 30)));
    }

    #[test]
    fn weekly_advances_seven_days() {
        let due = ms(2026, 3, 10, 9, 0);
        assert_eq!(next_occurrence(due, "weekly", due, None), Some(ms(2026, 3, 17, 9, 0)));
    }

    #[test]
    fn weekdays_skips_the_weekend() {
        // 2026-03-13 is a Friday.
        let fri = ms(2026, 3, 13, 9, 0);
        assert_eq!(next_occurrence(fri, "weekdays", fri, None), Some(ms(2026, 3, 16, 9, 0)));
        // Thursday -> Friday
        let thu = ms(2026, 3, 12, 9, 0);
        assert_eq!(next_occurrence(thu, "weekdays", thu, None), Some(ms(2026, 3, 13, 9, 0)));
    }

    #[test]
    fn monthly_clamps_to_end_of_month() {
        let jan31 = ms(2026, 1, 31, 8, 0);
        assert_eq!(next_occurrence(jan31, "monthly", jan31, None), Some(ms(2026, 2, 28, 8, 0)));
        // leap year
        let jan31 = ms(2028, 1, 31, 8, 0);
        assert_eq!(next_occurrence(jan31, "monthly", jan31, None), Some(ms(2028, 2, 29, 8, 0)));
        // 30-day month
        let mar31 = ms(2026, 3, 31, 8, 0);
        assert_eq!(next_occurrence(mar31, "monthly", mar31, None), Some(ms(2026, 4, 30, 8, 0)));
    }

    #[test]
    fn monthly_rolls_over_the_year() {
        let dec15 = ms(2026, 12, 15, 8, 0);
        assert_eq!(next_occurrence(dec15, "monthly", dec15, None), Some(ms(2027, 1, 15, 8, 0)));
    }

    #[test]
    fn overdue_items_catch_up_past_now() {
        let due = ms(2026, 3, 1, 9, 0);
        let now = ms(2026, 3, 10, 12, 0);
        assert_eq!(next_occurrence(due, "daily", now, None), Some(ms(2026, 3, 11, 9, 0)));
        assert_eq!(next_occurrence(due, "weekly", now, None), Some(ms(2026, 3, 15, 9, 0)));
    }

    #[test]
    fn monthly_catch_up_does_not_drift() {
        let due = ms(2026, 1, 31, 8, 0);
        let now = ms(2026, 3, 1, 0, 0);
        assert_eq!(next_occurrence(due, "monthly", now, None), Some(ms(2026, 3, 31, 8, 0)));
    }

    #[test]
    fn monthly_anchor_survives_clamped_months() {
        // Jan 31 -> Feb 28 (anchor 31) -> Mar 31 -> Apr 30 -> May 31
        let mut due = ms(2026, 1, 31, 8, 0);
        let expect = [ms(2026, 2, 28, 8, 0), ms(2026, 3, 31, 8, 0), ms(2026, 4, 30, 8, 0), ms(2026, 5, 31, 8, 0)];
        for want in expect {
            due = next_occurrence(due, "monthly", due, Some(31)).unwrap();
            assert_eq!(due, want);
        }
        // Without the anchor the 28th sticks (the old behaviour).
        let feb28 = ms(2026, 2, 28, 8, 0);
        assert_eq!(next_occurrence(feb28, "monthly", feb28, None), Some(ms(2026, 3, 28, 8, 0)));
        assert_eq!(local_day(feb28), Some(28));
    }
}
