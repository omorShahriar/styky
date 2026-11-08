//! Next-occurrence maths for repeating items (local time).
//!
//! `item.repeat` holds either a legacy simple value (`none|daily|weekdays|weekly|monthly`, always free)
//! or, with Styky Pro, an RRULE-like string:
//!
//! ```text
//! FREQ=DAILY|WEEKLY|MONTHLY
//! ;INTERVAL=n            every n days / weeks / months (default 1)
//! ;BYDAY=MO,WE           WEEKLY: these weekdays;  MONTHLY: one ordinal weekday, e.g. 2FR, -1FR (last Friday)
//! ;BYMONTHDAY=n          MONTHLY: day of month (1-31, or -1 = last day); short months clamp
//! ;COUNT=n               n occurrences remain, including the current one; completing the last one finishes it
//! ;UNTIL=YYYYMMDD        no occurrence after this local date
//! ```
//!
//! **Drift-safe:** every occurrence is computed from the *start date* of the series (the item's current due date
//! plus, for plain monthly rules, the stored anchor day), never by chaining "previous + 1 month", so the 31st
//! survives short months (Jan 31 -> Feb 28 -> Mar 31) and a catch-up after an ignored reminder lands on the right
//! date. Wall-clock time is preserved across DST changes (`from_naive` resolves gaps and overlaps).

use chrono::{DateTime, Datelike, Days, Timelike, Local, LocalResult, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday};

fn to_local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

/// Local wall-clock time -> ms, resolving DST gaps/overlaps sensibly.
pub fn from_naive(n: NaiveDateTime) -> Option<i64> {
    resolve_in(&Local, n)
}

/// Local day-of-month of a timestamp (the "anchor" a monthly repeat keeps returning to).
pub fn local_day(ms: i64) -> Option<u32> {
    to_local(ms).map(|d| d.day())
}

/// Local time of day of a timestamp, in minutes after midnight.
pub fn local_minutes(ms: i64) -> Option<u32> {
    to_local(ms).map(|d| d.hour() * 60 + d.minute())
}

// ------------------------------------------------------------------ rule

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    /// (ordinal, weekday): ordinal is `None` for WEEKLY and `Some(1..=5 | -1)` for MONTHLY.
    pub byday: Vec<(Option<i8>, Weekday)>,
    pub bymonthday: Option<i8>,
    pub count: Option<u32>,
    pub until: Option<NaiveDate>,
}

const DAYS: [(&str, Weekday); 7] = [
    ("MO", Weekday::Mon),
    ("TU", Weekday::Tue),
    ("WE", Weekday::Wed),
    ("TH", Weekday::Thu),
    ("FR", Weekday::Fri),
    ("SA", Weekday::Sat),
    ("SU", Weekday::Sun),
];

fn day_code(w: Weekday) -> &'static str {
    DAYS.iter().find(|(_, d)| *d == w).map_or("MO", |(c, _)| c)
}

fn day_from_code(c: &str) -> Option<Weekday> {
    DAYS.iter().find(|(k, _)| *k == c).map(|(_, d)| *d)
}

fn simple(freq: Freq, byday: Vec<(Option<i8>, Weekday)>) -> Rule {
    Rule { freq, interval: 1, byday, bymonthday: None, count: None, until: None }
}

impl Rule {
    /// Parse a legacy simple value or an RRULE-like string. `None` for `none`, empty or anything invalid.
    pub fn parse(s: &str) -> Option<Rule> {
        let s = s.trim();
        match s {
            "daily" => return Some(simple(Freq::Daily, vec![])),
            "weekly" => return Some(simple(Freq::Weekly, vec![])),
            "monthly" => return Some(simple(Freq::Monthly, vec![])),
            "weekdays" => {
                let wd = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];
                return Some(simple(Freq::Weekly, wd.iter().map(|d| (None, *d)).collect()));
            }
            "" | "none" => return None,
            _ => {}
        }
        let mut r = simple(Freq::Daily, vec![]);
        let mut have_freq = false;
        for part in s.split(';') {
            let (k, v) = part.split_once('=')?;
            let v = v.trim();
            match k.trim().to_ascii_uppercase().as_str() {
                "FREQ" => {
                    r.freq = match v.to_ascii_uppercase().as_str() {
                        "DAILY" => Freq::Daily,
                        "WEEKLY" => Freq::Weekly,
                        "MONTHLY" => Freq::Monthly,
                        _ => return None,
                    };
                    have_freq = true;
                }
                "INTERVAL" => r.interval = v.parse().ok().filter(|n| (1..=366).contains(n))?,
                "COUNT" => r.count = Some(v.parse().ok().filter(|n| (1..=9999).contains(n))?),
                "UNTIL" => r.until = Some(NaiveDate::parse_from_str(v, "%Y%m%d").ok()?),
                "BYMONTHDAY" => {
                    let n: i8 = v.parse().ok()?;
                    if !(n == -1 || (1..=31).contains(&n)) {
                        return None;
                    }
                    r.bymonthday = Some(n);
                }
                "BYDAY" => {
                    for tok in v.split(',') {
                        let tok = tok.trim().to_ascii_uppercase();
                        if tok.len() < 2 {
                            return None;
                        }
                        let (ord, code) = tok.split_at(tok.len() - 2);
                        let ord = match ord {
                            "" => None,
                            o => Some(o.parse::<i8>().ok().filter(|n| *n == -1 || (1..=5).contains(n))?),
                        };
                        let d = day_from_code(code)?;
                        if !r.byday.contains(&(ord, d)) {
                            r.byday.push((ord, d));
                        }
                    }
                }
                _ => return None,
            }
        }
        if !have_freq {
            return None;
        }
        // Consistency per frequency.
        match r.freq {
            Freq::Daily => {
                if !r.byday.is_empty() || r.bymonthday.is_some() {
                    return None;
                }
            }
            Freq::Weekly => {
                if r.byday.iter().any(|(o, _)| o.is_some()) || r.bymonthday.is_some() {
                    return None;
                }
                r.byday.sort_by_key(|(_, d)| d.num_days_from_monday());
            }
            Freq::Monthly => {
                if r.byday.len() > 1 || (!r.byday.is_empty() && r.bymonthday.is_some()) || r.byday.iter().any(|(o, _)| o.is_none()) {
                    return None;
                }
            }
        }
        Some(r)
    }

    /// Canonical text (order: FREQ, INTERVAL, BYDAY, BYMONTHDAY, COUNT, UNTIL).
    pub fn to_rrule(&self) -> String {
        let mut p = vec![format!(
            "FREQ={}",
            match self.freq {
                Freq::Daily => "DAILY",
                Freq::Weekly => "WEEKLY",
                Freq::Monthly => "MONTHLY",
            }
        )];
        if self.interval > 1 {
            p.push(format!("INTERVAL={}", self.interval));
        }
        if !self.byday.is_empty() {
            let v: Vec<String> = self
                .byday
                .iter()
                .map(|(o, d)| format!("{}{}", o.map(|n| n.to_string()).unwrap_or_default(), day_code(*d)))
                .collect();
            p.push(format!("BYDAY={}", v.join(",")));
        }
        if let Some(n) = self.bymonthday {
            p.push(format!("BYMONTHDAY={n}"));
        }
        if let Some(n) = self.count {
            p.push(format!("COUNT={n}"));
        }
        if let Some(u) = self.until {
            p.push(format!("UNTIL={}", u.format("%Y%m%d")));
        }
        p.join(";")
    }

    /// Monthly "same day number" repeats need the stored anchor day to survive short months.
    pub fn uses_anchor(&self) -> bool {
        self.freq == Freq::Monthly && self.byday.is_empty() && self.bymonthday.is_none()
    }
}

/// What to store for an incoming `repeat` value: legacy values pass through, valid rules are canonicalised,
/// everything else becomes `none`.
pub fn normalize(s: &str) -> String {
    let t = s.trim();
    if matches!(t, "daily" | "weekdays" | "weekly" | "monthly") {
        return t.to_string();
    }
    Rule::parse(t).map_or_else(|| "none".to_string(), |r| r.to_rrule())
}

/// True for the free-tier values (`none` and the four simple ones).
pub fn is_simple(s: &str) -> bool {
    matches!(s.trim(), "" | "none" | "daily" | "weekdays" | "weekly" | "monthly")
}

pub fn uses_anchor(s: &str) -> bool {
    Rule::parse(s).is_some_and(|r| r.uses_anchor())
}

// ------------------------------------------------------------------ dates

fn last_day_of_month(y: i32, m: u32) -> u32 {
    NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|f| f.checked_add_months(Months::new(1)))
        .and_then(|n| n.pred_opt())
        .map_or(28, |d| d.day())
}

/// The `ord`-th (1..=5) or last (-1) `wd` of the month; `None` when the month has no 5th one.
pub fn nth_weekday(y: i32, m: u32, ord: i8, wd: Weekday) -> Option<NaiveDate> {
    if ord == -1 {
        let last = NaiveDate::from_ymd_opt(y, m, last_day_of_month(y, m))?;
        let back = (last.weekday().num_days_from_monday() + 7 - wd.num_days_from_monday()) % 7;
        return last.checked_sub_days(Days::new(back as u64));
    }
    let first = NaiveDate::from_ymd_opt(y, m, 1)?;
    let fwd = (wd.num_days_from_monday() + 7 - first.weekday().num_days_from_monday()) % 7;
    let d = first.checked_add_days(Days::new(fwd as u64 + 7 * (ord as u64 - 1)))?;
    (d.month() == m).then_some(d)
}

fn resolve_month_day(y: i32, m: u32, day: i8) -> Option<NaiveDate> {
    let last = last_day_of_month(y, m);
    let d = if day == -1 { last } else { (day as u32).min(last) };
    NaiveDate::from_ymd_opt(y, m, d)
}

/// All dates of the series strictly after `start`, ascending. `anchor` is the plain-monthly day number.
fn occurrences(rule: &Rule, start: NaiveDate, anchor: u32) -> Box<dyn Iterator<Item = NaiveDate> + '_> {
    let n = rule.interval.max(1) as u64;
    let limit = start.year() + 300;
    match rule.freq {
        Freq::Daily => Box::new(
            (1u64..)
                .map_while(move |k| start.checked_add_days(Days::new(k * n)))
                .take_while(move |d| d.year() <= limit),
        ),
        Freq::Weekly => {
            let monday0 = start.checked_sub_days(Days::new(start.weekday().num_days_from_monday() as u64));
            let days: Vec<u32> = if rule.byday.is_empty() {
                vec![start.weekday().num_days_from_monday()]
            } else {
                rule.byday.iter().map(|(_, d)| d.num_days_from_monday()).collect()
            };
            Box::new(
                (0u64..)
                    .map_while(move |k| monday0.and_then(|m| m.checked_add_days(Days::new(7 * n * k))))
                    .take_while(move |m| m.year() <= limit)
                    .flat_map(move |monday| days.clone().into_iter().filter_map(move |o| monday.checked_add_days(Days::new(o as u64))))
                    .filter(move |d| *d > start),
            )
        }
        Freq::Monthly => {
            let first = start.with_day(1).unwrap_or(start);
            Box::new(
                (0u64..)
                    .map_while(move |k| first.checked_add_months(Months::new((k * n) as u32)))
                    .take_while(move |f| f.year() <= limit)
                    .filter_map(move |f| {
                        let (y, m) = (f.year(), f.month());
                        if let Some((Some(ord), wd)) = rule.byday.first() {
                            nth_weekday(y, m, *ord, *wd)
                        } else if let Some(d) = rule.bymonthday {
                            resolve_month_day(y, m, d)
                        } else {
                            resolve_month_day(y, m, anchor.clamp(1, 31) as i8)
                        }
                    })
                    .filter(move |d| *d > start),
            )
        }
    }
}

/// The next due time of a repeating item after completing it: strictly after `due_ms` and after `now_ms`
/// (so a long-overdue item catches up instead of staying in the past), plus the rule text to store
/// (`COUNT` counts down). `None` when the item does not repeat or the series is over (COUNT used up, UNTIL passed).
pub fn advance(due_ms: i64, repeat: &str, now_ms: i64, anchor: Option<u32>) -> Option<(i64, String)> {
    advance_in(&Local, due_ms, repeat, now_ms, anchor, None)
}

/// `advance` with the stored time-of-day anchor (minutes after local midnight): the series keeps its real time
/// even after a DST gap pushed one occurrence an hour later.
pub fn advance_full(due_ms: i64, repeat: &str, now_ms: i64, anchor: Option<u32>, anchor_min: Option<u32>) -> Option<(i64, String)> {
    advance_in(&Local, due_ms, repeat, now_ms, anchor, anchor_min)
}

/// Resolve a wall-clock time in `tz`: gap -> an hour later, overlap -> the first instance.
fn resolve_in<Tz: TimeZone>(tz: &Tz, n: NaiveDateTime) -> Option<i64> {
    match tz.from_local_datetime(&n) {
        LocalResult::Single(d) => Some(d.timestamp_millis()),
        LocalResult::Ambiguous(a, _) => Some(a.timestamp_millis()),
        LocalResult::None => tz.from_local_datetime(&(n + chrono::Duration::hours(1))).earliest().map(|d| d.timestamp_millis()),
    }
}

/// The engine, generic over the time zone so DST behaviour can be tested in a zone that has it.
pub fn advance_in<Tz: TimeZone>(
    tz: &Tz,
    due_ms: i64,
    repeat: &str,
    now_ms: i64,
    anchor: Option<u32>,
    anchor_min: Option<u32>,
) -> Option<(i64, String)> {
    let rule = Rule::parse(repeat)?;
    if rule.count == Some(1) {
        return None;
    }
    let due = tz.timestamp_millis_opt(due_ms).single()?;
    let time = anchor_min
        .filter(|m| *m < 24 * 60)
        .and_then(|m| NaiveTime::from_hms_opt(m / 60, m % 60, 0))
        .unwrap_or_else(|| due.time());
    let start = due.date_naive();
    let anchor_day = anchor.filter(|d| (1..=31).contains(d)).unwrap_or_else(|| start.day());
    for date in occurrences(&rule, start, anchor_day).take(20_000) {
        if rule.until.is_some_and(|u| date > u) {
            return None;
        }
        let ms = resolve_in(tz, date.and_time(time))?;
        if ms > now_ms && ms > due_ms {
            let stored = match rule.count {
                Some(c) => Rule { count: Some(c - 1), ..rule.clone() }.to_rrule(),
                None => repeat.to_string(),
            };
            return Some((ms, stored));
        }
    }
    None
}

/// Back-compat helper (legacy callers/tests): the next due time only.
pub fn next_occurrence(due_ms: i64, repeat: &str, now_ms: i64, anchor: Option<u32>) -> Option<i64> {
    advance(due_ms, repeat, now_ms, anchor).map(|(ms, _)| ms)
}

/// First due time for a rule that names its own days (`every mon,wed`, `last friday of the month`) when the user
/// gave no date: the first matching local date from today at `hour`:00 that is still in the future.
/// `None` for rules without such days (the caller then starts at the next default time).
pub fn first_match(repeat: &str, now_ms: i64, hour: u32) -> Option<i64> {
    let rule = Rule::parse(repeat)?;
    if rule.byday.is_empty() && rule.bymonthday.is_none() {
        return None;
    }
    let today = to_local(now_ms)?.date_naive();
    let time = NaiveTime::from_hms_opt(hour, 0, 0)?;
    for k in 0..800u64 {
        let d = today.checked_add_days(Days::new(k))?;
        let hit = match rule.freq {
            Freq::Weekly => rule.byday.iter().any(|(_, w)| *w == d.weekday()),
            Freq::Monthly => {
                if let Some((Some(ord), wd)) = rule.byday.first() {
                    nth_weekday(d.year(), d.month(), *ord, *wd) == Some(d)
                } else {
                    rule.bymonthday.and_then(|m| resolve_month_day(d.year(), d.month(), m)) == Some(d)
                }
            }
            Freq::Daily => false,
        };
        if hit {
            let ms = from_naive(d.and_time(time))?;
            if ms > now_ms {
                return Some(ms);
            }
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

    // ---- legacy values (unchanged behaviour) ----

    #[test]
    fn none_does_not_repeat() {
        assert_eq!(next_occurrence(ms(2026, 1, 5, 9, 0), "none", 0, None), None);
        assert_eq!(next_occurrence(ms(2026, 1, 5, 9, 0), "garbage", 0, None), None);
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
        let thu = ms(2026, 3, 12, 9, 0);
        assert_eq!(next_occurrence(thu, "weekdays", thu, None), Some(ms(2026, 3, 13, 9, 0)));
        // A weekend due date still lands on Monday.
        let sat = ms(2026, 3, 14, 9, 0);
        assert_eq!(next_occurrence(sat, "weekdays", sat, None), Some(ms(2026, 3, 16, 9, 0)));
    }

    #[test]
    fn monthly_clamps_to_end_of_month() {
        let jan31 = ms(2026, 1, 31, 8, 0);
        assert_eq!(next_occurrence(jan31, "monthly", jan31, None), Some(ms(2026, 2, 28, 8, 0)));
        let jan31 = ms(2028, 1, 31, 8, 0);
        assert_eq!(next_occurrence(jan31, "monthly", jan31, None), Some(ms(2028, 2, 29, 8, 0)));
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
        let mut due = ms(2026, 1, 31, 8, 0);
        let expect = [ms(2026, 2, 28, 8, 0), ms(2026, 3, 31, 8, 0), ms(2026, 4, 30, 8, 0), ms(2026, 5, 31, 8, 0)];
        for want in expect {
            due = next_occurrence(due, "monthly", due, Some(31)).unwrap();
            assert_eq!(due, want);
        }
        let feb28 = ms(2026, 2, 28, 8, 0);
        assert_eq!(next_occurrence(feb28, "monthly", feb28, None), Some(ms(2026, 3, 28, 8, 0)));
        assert_eq!(local_day(feb28), Some(28));
    }

    // ---- RRULE parsing ----

    #[test]
    fn rules_round_trip_in_canonical_form() {
        for s in [
            "FREQ=DAILY",
            "FREQ=DAILY;INTERVAL=3",
            "FREQ=WEEKLY;BYDAY=MO,WE,FR",
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU;COUNT=5",
            "FREQ=MONTHLY;BYDAY=-1FR",
            "FREQ=MONTHLY;INTERVAL=2;BYDAY=2TH;UNTIL=20271231",
            "FREQ=MONTHLY;BYMONTHDAY=-1",
            "FREQ=MONTHLY;BYMONTHDAY=15;COUNT=12",
        ] {
            assert_eq!(Rule::parse(s).unwrap().to_rrule(), s, "{s}");
        }
        // Order and case are normalised; duplicate days dropped; weekdays sorted.
        assert_eq!(normalize("freq=weekly;byday=FR,mo,MO;interval=1"), "FREQ=WEEKLY;BYDAY=MO,FR");
        assert_eq!(normalize("weekdays"), "weekdays");
        assert_eq!(normalize("daily"), "daily");
    }

    #[test]
    fn invalid_rules_become_none() {
        for s in [
            "FREQ=YEARLY",
            "INTERVAL=2",
            "FREQ=DAILY;BYDAY=MO",
            "FREQ=WEEKLY;BYDAY=2MO",
            "FREQ=MONTHLY;BYDAY=MO",
            "FREQ=MONTHLY;BYDAY=2MO,3MO",
            "FREQ=MONTHLY;BYDAY=2MO;BYMONTHDAY=3",
            "FREQ=MONTHLY;BYDAY=6MO",
            "FREQ=DAILY;INTERVAL=0",
            "FREQ=DAILY;COUNT=0",
            "FREQ=MONTHLY;BYMONTHDAY=32",
            "FREQ=DAILY;UNTIL=soon",
            "FREQ=DAILY;X=1",
            "banana",
        ] {
            assert!(Rule::parse(s).is_none(), "{s}");
            assert_eq!(normalize(s), "none", "{s}");
        }
        assert!(is_simple("weekly") && is_simple("none") && !is_simple("FREQ=DAILY"));
    }

    // ---- intervals ----

    #[test]
    fn every_n_days_weeks_months() {
        let due = ms(2026, 3, 10, 9, 0);
        assert_eq!(next_occurrence(due, "FREQ=DAILY;INTERVAL=3", due, None), Some(ms(2026, 3, 13, 9, 0)));
        assert_eq!(next_occurrence(due, "FREQ=WEEKLY;INTERVAL=2", due, None), Some(ms(2026, 3, 24, 9, 0)));
        assert_eq!(next_occurrence(due, "FREQ=MONTHLY;INTERVAL=3", due, None), Some(ms(2026, 6, 10, 9, 0)));
        // Catch-up lands on the series, not on now + interval.
        let now = ms(2026, 3, 20, 12, 0);
        assert_eq!(next_occurrence(due, "FREQ=DAILY;INTERVAL=3", now, None), Some(ms(2026, 3, 22, 9, 0)));
        assert_eq!(next_occurrence(due, "FREQ=WEEKLY;INTERVAL=2", now, None), Some(ms(2026, 3, 24, 9, 0)));
    }

    #[test]
    fn every_n_months_keeps_the_anchor_day_without_drift() {
        // Jan 31 every 2 months: Mar 31, May 31, Jul 31 ... never 28/30 sticking.
        let mut due = ms(2026, 1, 31, 8, 0);
        for want in [ms(2026, 3, 31, 8, 0), ms(2026, 5, 31, 8, 0), ms(2026, 7, 31, 8, 0)] {
            due = next_occurrence(due, "FREQ=MONTHLY;INTERVAL=2", due, Some(31)).unwrap();
            assert_eq!(due, want);
        }
        // Every month from the 31st through February, with the anchor.
        let mut due = ms(2027, 1, 31, 8, 0);
        let want = [ms(2027, 2, 28, 8, 0), ms(2027, 3, 31, 8, 0), ms(2027, 4, 30, 8, 0)];
        for w in want {
            due = next_occurrence(due, "FREQ=MONTHLY", due, Some(31)).unwrap();
            assert_eq!(due, w);
        }
        // Every 12 months from Feb 29 (leap day) clamps to Feb 28 in common years and comes back to the 29th.
        let leap = ms(2028, 2, 29, 8, 0);
        assert_eq!(next_occurrence(leap, "FREQ=MONTHLY;INTERVAL=12", leap, Some(29)), Some(ms(2029, 2, 28, 8, 0)));
        let y29 = ms(2029, 2, 28, 8, 0);
        let y30 = next_occurrence(y29, "FREQ=MONTHLY;INTERVAL=12", y29, Some(29)).unwrap();
        assert_eq!(y30, ms(2030, 2, 28, 8, 0));
        let y32 = ms(2031, 2, 28, 8, 0);
        assert_eq!(next_occurrence(y32, "FREQ=MONTHLY;INTERVAL=12", y32, Some(29)), Some(ms(2032, 2, 29, 8, 0)));
    }

    // ---- weekdays ----

    #[test]
    fn chosen_weekdays_cycle_through_the_week() {
        // Mon/Wed/Fri. 2026-03-09 is a Monday.
        let mon = ms(2026, 3, 9, 9, 0);
        let r = "FREQ=WEEKLY;BYDAY=MO,WE,FR";
        let wed = next_occurrence(mon, r, mon, None).unwrap();
        assert_eq!(wed, ms(2026, 3, 11, 9, 0));
        let fri = next_occurrence(wed, r, wed, None).unwrap();
        assert_eq!(fri, ms(2026, 3, 13, 9, 0));
        assert_eq!(next_occurrence(fri, r, fri, None), Some(ms(2026, 3, 16, 9, 0)));
    }

    #[test]
    fn every_other_week_on_chosen_days() {
        // Every 2 weeks on Mon+Thu: Mon 3/9 -> Thu 3/12 (same week) -> Mon 3/23 (skips 3/16 week).
        let r = "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH";
        let mon = ms(2026, 3, 9, 9, 0);
        let thu = next_occurrence(mon, r, mon, None).unwrap();
        assert_eq!(thu, ms(2026, 3, 12, 9, 0));
        let next = next_occurrence(thu, r, thu, None).unwrap();
        assert_eq!(next, ms(2026, 3, 23, 9, 0));
        assert_eq!(next_occurrence(next, r, next, None), Some(ms(2026, 3, 26, 9, 0)));
        // Sunday-ending week boundary: from Sun 3/15 (week of 3/9) the next is Mon 3/23.
        let sun = ms(2026, 3, 15, 9, 0);
        assert_eq!(next_occurrence(sun, r, sun, None), Some(ms(2026, 3, 23, 9, 0)));
    }

    #[test]
    fn a_weekday_rule_with_a_due_date_on_another_weekday_snaps_to_the_next_listed_day() {
        // Due Wed, rule Mon/Fri -> Fri.
        let wed = ms(2026, 3, 11, 9, 0);
        assert_eq!(next_occurrence(wed, "FREQ=WEEKLY;BYDAY=MO,FR", wed, None), Some(ms(2026, 3, 13, 9, 0)));
    }

    // ---- nth / last weekday ----

    #[test]
    fn last_friday_of_the_month() {
        let r = "FREQ=MONTHLY;BYDAY=-1FR";
        let mut due = ms(2026, 1, 30, 9, 0); // last Friday of Jan 2026
        for want in [ms(2026, 2, 27, 9, 0), ms(2026, 3, 27, 9, 0), ms(2026, 4, 24, 9, 0), ms(2026, 5, 29, 9, 0)] {
            due = next_occurrence(due, r, due, None).unwrap();
            assert_eq!(due, want);
        }
    }

    #[test]
    fn nth_weekday_cases() {
        assert_eq!(nth_weekday(2026, 3, 1, Weekday::Mon), NaiveDate::from_ymd_opt(2026, 3, 2));
        assert_eq!(nth_weekday(2026, 3, 2, Weekday::Tue), NaiveDate::from_ymd_opt(2026, 3, 10));
        assert_eq!(nth_weekday(2026, 3, -1, Weekday::Sun), NaiveDate::from_ymd_opt(2026, 3, 29));
        assert_eq!(nth_weekday(2026, 2, -1, Weekday::Sat), NaiveDate::from_ymd_opt(2026, 2, 28));
        // The 5th Monday of March 2026 exists (30th); of Feb 2026 it does not.
        assert_eq!(nth_weekday(2026, 3, 5, Weekday::Mon), NaiveDate::from_ymd_opt(2026, 3, 30));
        assert_eq!(nth_weekday(2026, 2, 5, Weekday::Mon), None);
    }

    #[test]
    fn fifth_weekday_skips_months_that_do_not_have_one() {
        // 5th Monday: Mar 30 2026 -> next month with a 5th Monday is Jun 29 2026 (Apr/May have only 4... May has 5: 4,11,18,25 = 4).
        let due = ms(2026, 3, 30, 9, 0);
        let next = next_occurrence(due, "FREQ=MONTHLY;BYDAY=5MO", due, None).unwrap();
        assert_eq!(next, ms(2026, 6, 29, 9, 0));
    }

    #[test]
    fn second_tuesday_every_two_months() {
        let r = "FREQ=MONTHLY;INTERVAL=2;BYDAY=2TU";
        let due = ms(2026, 1, 13, 9, 0);
        let a = next_occurrence(due, r, due, None).unwrap();
        assert_eq!(a, ms(2026, 3, 10, 9, 0));
        assert_eq!(next_occurrence(a, r, a, None), Some(ms(2026, 5, 12, 9, 0)));
    }

    #[test]
    fn last_day_of_the_month_and_fixed_month_days() {
        let r = "FREQ=MONTHLY;BYMONTHDAY=-1";
        let mut due = ms(2026, 1, 31, 18, 0);
        for want in [ms(2026, 2, 28, 18, 0), ms(2026, 3, 31, 18, 0), ms(2026, 4, 30, 18, 0)] {
            due = next_occurrence(due, r, due, None).unwrap();
            assert_eq!(due, want);
        }
        // The 30th clamps in February and comes back.
        let r = "FREQ=MONTHLY;BYMONTHDAY=30";
        let jan30 = ms(2026, 1, 30, 9, 0);
        let feb = next_occurrence(jan30, r, jan30, None).unwrap();
        assert_eq!(feb, ms(2026, 2, 28, 9, 0));
        assert_eq!(next_occurrence(feb, r, feb, None), Some(ms(2026, 3, 30, 9, 0)));
    }

    // ---- COUNT / UNTIL ----

    #[test]
    fn count_counts_down_and_ends_the_series() {
        let due = ms(2026, 3, 10, 9, 0);
        let (d2, r2) = advance(due, "FREQ=DAILY;COUNT=3", due, None).unwrap();
        assert_eq!((d2, r2.as_str()), (ms(2026, 3, 11, 9, 0), "FREQ=DAILY;COUNT=2"));
        let (d3, r3) = advance(d2, &r2, d2, None).unwrap();
        assert_eq!((d3, r3.as_str()), (ms(2026, 3, 12, 9, 0), "FREQ=DAILY;COUNT=1"));
        // The last occurrence: completing it finishes the item.
        assert_eq!(advance(d3, &r3, d3, None), None);
        // COUNT=1 from the start means a single (non-repeating) occurrence.
        assert_eq!(advance(due, "FREQ=DAILY;COUNT=1", due, None), None);
    }

    #[test]
    fn skipped_occurrences_do_not_use_up_the_count() {
        let due = ms(2026, 3, 1, 9, 0);
        let now = ms(2026, 3, 10, 12, 0);
        let (next, rule) = advance(due, "FREQ=DAILY;COUNT=5", now, None).unwrap();
        assert_eq!((next, rule.as_str()), (ms(2026, 3, 11, 9, 0), "FREQ=DAILY;COUNT=4"));
    }

    #[test]
    fn until_is_inclusive_and_ends_the_series() {
        let r = "FREQ=DAILY;UNTIL=20260312";
        let due = ms(2026, 3, 10, 9, 0);
        let d2 = next_occurrence(due, r, due, None).unwrap();
        assert_eq!(d2, ms(2026, 3, 11, 9, 0));
        let d3 = next_occurrence(d2, r, d2, None).unwrap();
        assert_eq!(d3, ms(2026, 3, 12, 9, 0)); // the until date itself still happens
        assert_eq!(next_occurrence(d3, r, d3, None), None);
        // Overdue past the until date: done, not stuck.
        assert_eq!(next_occurrence(due, r, ms(2026, 3, 20, 0, 0), None), None);
    }

    #[test]
    fn count_and_until_together_stop_at_whichever_comes_first() {
        let due = ms(2026, 3, 10, 9, 0);
        let r = "FREQ=DAILY;COUNT=10;UNTIL=20260311";
        let (d2, r2) = advance(due, r, due, None).unwrap();
        assert_eq!(d2, ms(2026, 3, 11, 9, 0));
        assert_eq!(advance(d2, &r2, d2, None), None);
    }

    // ---- DST ----

    /// Run the DST checks in a zone with transitions regardless of this machine's zone is not possible with
    /// chrono's `Local`, so these assert the invariants that hold in any zone: the wall-clock time is preserved
    /// and a gap/overlap never loses or repeats a day.
    #[test]
    fn wall_clock_time_is_preserved_across_the_year_including_dst_changes() {
        let mut due = ms(2026, 1, 1, 7, 30);
        let end = ms(2027, 1, 1, 0, 0);
        let mut days = 0;
        while due < end {
            let next = next_occurrence(due, "daily", due, None).unwrap();
            let d = to_local(next).unwrap();
            assert_eq!((d.hour(), d.minute()), (7, 30), "{d}");
            let prev = to_local(due).unwrap();
            assert_eq!((d.date_naive() - prev.date_naive()).num_days(), 1);
            due = next;
            days += 1;
        }
        assert_eq!(days, 365);
        // Weekly chosen days across the same range land on the right weekdays with the same wall time.
        let mut due = ms(2026, 1, 5, 20, 15);
        for _ in 0..120 {
            let next = next_occurrence(due, "FREQ=WEEKLY;BYDAY=MO,TH", due, None).unwrap();
            let d = to_local(next).unwrap();
            assert!(matches!(d.weekday(), Weekday::Mon | Weekday::Thu));
            assert_eq!((d.hour(), d.minute()), (20, 15));
            due = next;
        }
    }

    #[test]
    fn a_time_inside_a_spring_forward_gap_is_nudged_not_lost() {
        // Construct a naive time in a gap if this zone has one in 2026; otherwise the call is just a plain conversion.
        let mut found = false;
        let mut d = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        while d.year() == 2026 {
            let n = d.and_hms_opt(2, 30, 0).unwrap();
            if Local.from_local_datetime(&n).single().is_none() {
                let got = from_naive(n).expect("resolved");
                let back = to_local(got).unwrap();
                // Within an hour of the requested wall time, never skipping the day.
                assert_eq!(back.date_naive(), d, "{n}");
                found = true;
            }
            d = d.succ_opt().unwrap();
        }
        // (No assertion that a gap exists: UTC and non-DST zones have none.)
        let _ = found;
    }

    // ---- first_match ----

    #[test]
    fn first_match_finds_the_first_listed_day_for_rules_without_a_date() {
        let now = ms(2026, 3, 11, 10, 0); // Wednesday
        assert_eq!(first_match("FREQ=WEEKLY;BYDAY=MO", now, 9), Some(ms(2026, 3, 16, 9, 0)));
        assert_eq!(first_match("FREQ=WEEKLY;BYDAY=MO,FR", now, 9), Some(ms(2026, 3, 13, 9, 0)));
        // Today counts only if 09:00 is still ahead.
        assert_eq!(first_match("FREQ=WEEKLY;BYDAY=WE", now, 9), Some(ms(2026, 3, 18, 9, 0)));
        assert_eq!(first_match("FREQ=WEEKLY;BYDAY=WE", ms(2026, 3, 11, 8, 0), 9), Some(ms(2026, 3, 11, 9, 0)));
        assert_eq!(first_match("FREQ=MONTHLY;BYDAY=-1FR", now, 9), Some(ms(2026, 3, 27, 9, 0)));
        assert_eq!(first_match("FREQ=MONTHLY;BYMONTHDAY=-1", now, 9), Some(ms(2026, 3, 31, 9, 0)));
        // No named days: the caller picks the default time.
        assert_eq!(first_match("FREQ=DAILY;INTERVAL=2", now, 9), None);
        assert_eq!(first_match("weekly", now, 9), None);
    }

    // ---- DST, in a zone that really has it (America/New_York) ----

    mod dst {
        use super::*;
        use chrono_tz::America::New_York as NY;

        fn at(y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
            // Unambiguous local times only.
            NY.with_ymd_and_hms(y, m, d, h, mi, 0).single().unwrap().timestamp_millis()
        }
        fn wall(ms: i64) -> (NaiveDate, u32, u32) {
            let d = NY.timestamp_millis_opt(ms).single().unwrap();
            (d.date_naive(), d.hour(), d.minute())
        }
        fn adv(due: i64, rule: &str, min: Option<u32>) -> i64 {
            advance_in(&NY, due, rule, due, Some(31), min).unwrap().0
        }
        fn date(y: i32, m: u32, d: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(y, m, d).unwrap()
        }

        #[test]
        fn the_zone_has_the_transitions_this_test_assumes() {
            // 2026-03-08 02:30 does not exist; 2026-11-01 01:30 happens twice.
            let gap = date(2026, 3, 8).and_hms_opt(2, 30, 0).unwrap();
            assert!(NY.from_local_datetime(&gap).single().is_none());
            let overlap = date(2026, 11, 1).and_hms_opt(1, 30, 0).unwrap();
            assert!(matches!(NY.from_local_datetime(&overlap), LocalResult::Ambiguous(..)));
        }

        #[test]
        fn daily_keeps_wall_time_across_spring_forward_and_fall_back() {
            // 09:00 every day over both transitions: always 09:00 local, never a 23/25 hour drift.
            let mut due = at(2026, 3, 6, 9, 0);
            for day in 7..=12 {
                due = adv(due, "daily", None);
                assert_eq!(wall(due), (date(2026, 3, day), 9, 0), "day {day}");
            }
            let mut due = at(2026, 10, 30, 9, 0);
            for (m, day) in [(10, 31), (11, 1), (11, 2), (11, 3)] {
                due = adv(due, "daily", None);
                assert_eq!(wall(due), (date(2026, m, day), 9, 0));
            }
        }

        #[test]
        fn the_real_gap_and_the_hour_after_it() {
            // Daily at 02:30. On Mar 8 02:30 does not exist: it is nudged to 03:30 that day only.
            let mar7 = at(2026, 3, 7, 2, 30);
            let mar8 = adv(mar7, "daily", Some(2 * 60 + 30));
            assert_eq!(wall(mar8), (date(2026, 3, 8), 3, 30));
            // With the stored time-of-day anchor the series goes back to 02:30 on Mar 9 (no permanent drift).
            let mar9 = adv(mar8, "daily", Some(2 * 60 + 30));
            assert_eq!(wall(mar9), (date(2026, 3, 9), 2, 30));
            // Without the anchor (rows from before the anchor existed) it keeps the nudged hour.
            let drifted = adv(mar8, "daily", None);
            assert_eq!(wall(drifted), (date(2026, 3, 9), 3, 30));
        }

        #[test]
        fn the_repeated_hour_in_fall_back_fires_once_per_day() {
            // 01:30 on Nov 1 happens twice; the series resolves to the first and moves on a calendar day.
            let oct31 = at(2026, 10, 31, 1, 30);
            let nov1 = adv(oct31, "daily", Some(90));
            assert_eq!(wall(nov1), (date(2026, 11, 1), 1, 30));
            // The first 01:30 is still EDT (UTC-4): 05:30 UTC.
            assert_eq!(nov1, chrono::Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap().timestamp_millis());
            let nov2 = adv(nov1, "daily", Some(90));
            assert_eq!(wall(nov2), (date(2026, 11, 2), 1, 30));
            // Nov 1 was 25 hours long, so the same wall time is 25 absolute hours later.
            assert_eq!(nov2 - nov1, 25 * 3_600_000);
        }

        #[test]
        fn weekly_and_nth_weekday_across_dst() {
            // Every Sunday 02:30 (a nonexistent time on Mar 8) -> 03:30 on the 8th, back to 02:30 on the 15th.
            let r = "FREQ=WEEKLY;BYDAY=SU";
            let mar1 = at(2026, 3, 1, 2, 30);
            let mar8 = adv(mar1, r, Some(150));
            assert_eq!(wall(mar8), (date(2026, 3, 8), 3, 30));
            assert_eq!(wall(adv(mar8, r, Some(150))), (date(2026, 3, 15), 2, 30));
            // Last Sunday of the month at 18:00 across the DST change: Mar 29 (EDT) then Apr 26.
            let r = "FREQ=MONTHLY;BYDAY=-1SU";
            let feb = at(2026, 2, 22, 18, 0);
            let mar = adv(feb, r, None);
            assert_eq!(wall(mar), (date(2026, 3, 29), 18, 0));
            assert_eq!(wall(adv(mar, r, None)), (date(2026, 4, 26), 18, 0));
            // First Sunday of November (the fall-back day itself).
            let r = "FREQ=MONTHLY;BYDAY=1SU";
            let oct = at(2026, 10, 4, 9, 0);
            assert_eq!(wall(adv(oct, r, None)), (date(2026, 11, 1), 9, 0));
        }

        #[test]
        fn month_end_and_dst_together() {
            // 31st at 02:30 monthly: Feb 28 -> Mar 31 keeps the anchor day (and 02:30 exists on Mar 31).
            let feb = at(2026, 2, 28, 2, 30);
            let got = advance_in(&NY, feb, "monthly", feb, Some(31), Some(150)).unwrap().0;
            assert_eq!(wall(got), (date(2026, 3, 31), 2, 30));
            // BYMONTHDAY=8 at 02:30 in March lands in the gap: nudged to 03:30, April back to 02:30.
            let jan = at(2026, 1, 8, 2, 30);
            let r = "FREQ=MONTHLY;BYMONTHDAY=8";
            let feb8 = adv(jan, r, Some(150));
            let mar8 = adv(feb8, r, Some(150));
            assert_eq!(wall(mar8), (date(2026, 3, 8), 3, 30));
            assert_eq!(wall(adv(mar8, r, Some(150))), (date(2026, 4, 8), 2, 30));
        }

        #[test]
        fn count_and_until_work_in_a_dst_zone() {
            let due = at(2026, 3, 6, 9, 0);
            let (d, rule) = advance_in(&NY, due, "FREQ=DAILY;COUNT=4;UNTIL=20260309", due, None, None).unwrap();
            assert_eq!(wall(d).0, date(2026, 3, 7));
            let (d, rule) = advance_in(&NY, d, &rule, d, None, None).unwrap();
            let (d, rule) = advance_in(&NY, d, &rule, d, None, None).unwrap();
            assert_eq!(wall(d).0, date(2026, 3, 9));
            assert_eq!(advance_in(&NY, d, &rule, d, None, None), None);
        }
    }
}
