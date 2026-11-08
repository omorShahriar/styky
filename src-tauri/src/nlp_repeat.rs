//! Repeat phrases of the quick-add grammar (Rust twin of the repeat part of `src/lib/nlp.ts`).
//!
//! Free (legacy values): `every day|weekday|week|month`.
//! Pro (RRULE strings):
//!   every 2 days|weeks|months          every other week
//!   every mon,wed  /  every monday and friday  /  every mon-fri   (weekdays)
//!   every 2 weeks on mon,thu
//!   last friday of the month           first monday of every month   /   every month on the 2nd tuesday
//!   last day of the month              every month on the 15th
//! optionally followed by an end: `for 5 times` / `5 times`, or `until 2026-12-31` / `until dec 31`.

use std::sync::LazyLock;

use chrono::{DateTime, Local, Weekday};
use regex::Regex;

use crate::repeat::{Freq, Rule};

const WD: &str = "mon(?:day)?|tue(?:s(?:day)?)?|wed(?:nesday)?|thu(?:r(?:s(?:day)?)?)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?";
const ORD: &str = "first|second|third|fourth|fifth|last|1st|2nd|3rd|4th|5th";

fn re(p: &str) -> Regex {
    Regex::new(p).expect("static regex")
}

static NTH_OF_MONTH: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)\b(?:every\s+)?({ORD})\s+({WD})\s+(?:of|in)\s+(?:the|every|each)\s+month\b")));
static EVERY_MONTH_ON_NTH: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)\bevery\s+month\s+on\s+the\s+({ORD})\s+({WD})\b")));
static LAST_DAY: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\blast\s+day\s+of\s+(?:the|every|each)\s+month\b"));
static MONTH_ON_DAY: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\bevery\s+month\s+on\s+the\s+(\d{1,2})(?:st|nd|rd|th)\b"));
static INTERVAL: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"(?i)\bevery\s+(\d+|other)\s+(days?|weeks?|months?)(?:\s+on\s+((?:{WD})(?:\s*(?:,|and|&|-|\s)\s*(?:{WD}))*))?\b"
    ))
});
static DAY_LIST: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)\bevery\s+((?:{WD})(?:\s*(?:,|and|&|-|\s)\s*(?:{WD}))*)\b")));
static SIMPLE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bevery\s+(day|weekdays?|week|month)\b"));
static COUNT: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(?:for\s+)?(\d{1,4})\s+(?:times|occurrences)\b"));
static UNTIL: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(?:until|through|till)\s+"));
static WD_TOKEN: LazyLock<Regex> = LazyLock::new(|| re(&format!(r"(?i)\b({WD})\b")));

fn ordinal(s: &str) -> Option<i8> {
    Some(match s.to_lowercase().as_str() {
        "first" | "1st" => 1,
        "second" | "2nd" => 2,
        "third" | "3rd" => 3,
        "fourth" | "4th" => 4,
        "fifth" | "5th" => 5,
        "last" => -1,
        _ => return None,
    })
}

/// "mon,wed", "monday and friday", "mon-fri" -> weekdays (ascending, unique).
fn parse_days(list: &str) -> Vec<Weekday> {
    let range = list.contains('-');
    let toks: Vec<Weekday> = WD_TOKEN.captures_iter(list).filter_map(|c| crate::nlp::weekday_from(&c[1])).collect();
    let mut out: Vec<Weekday> = if range && toks.len() == 2 {
        // mon-fri
        let (a, b) = (toks[0].num_days_from_monday(), toks[1].num_days_from_monday());
        let mut v = vec![];
        let mut d = a;
        loop {
            v.push(Weekday::try_from(d as u8).unwrap_or(Weekday::Mon));
            if d == b {
                break;
            }
            d = (d + 1) % 7;
        }
        v
    } else {
        toks
    };
    out.sort_by_key(|d| d.num_days_from_monday());
    out.dedup();
    out
}

fn rule(freq: Freq, interval: u32) -> Rule {
    Rule { freq, interval, byday: vec![], bymonthday: None, count: None, until: None }
}

/// Cut the first match of `r` from `s`, returning the remainder and the captures.
fn cut(s: &str, r: &Regex) -> Option<(String, Vec<String>)> {
    let c = r.captures(s)?;
    let m = c.get(0)?;
    let groups = c.iter().map(|g| g.map_or(String::new(), |g| g.as_str().to_string())).collect();
    Some((format!("{} {}", &s[..m.start()], &s[m.end()..]), groups))
}

/// Find a repeat phrase (and an optional end clause) in `s`. Returns the text without them and the stored `repeat`
/// value: a legacy name for the free phrases, an RRULE string for the Pro ones.
pub fn extract(s: &str, now: DateTime<Local>) -> (String, Option<String>) {
    let Some((mut rest, found)) = phrase(s) else { return (s.to_string(), None) };
    let mut rule = match found {
        Found::Simple(name) => {
            // A legacy name takes an end clause as well, which makes it an RRULE.
            let (r2, count) = end_clause(&rest, now);
            if count.is_none() {
                return (rest, Some(name.to_string()));
            }
            rest = r2;
            let mut r = Rule::parse(name).unwrap_or_else(|| rule(Freq::Daily, 1));
            apply_end(&mut r, count);
            return (rest, Some(r.to_rrule()));
        }
        Found::Rule(r) => r,
    };
    let (r2, end) = end_clause(&rest, now);
    rest = r2;
    apply_end(&mut rule, end);
    // Mon-Fri chosen explicitly is the free "weekdays" repeat.
    let out = if rule.freq == Freq::Weekly && rule.interval == 1 && rule.count.is_none() && rule.until.is_none() && rule.byday.len() == 5 && rule.byday.iter().all(|(_, d)| d.num_days_from_monday() < 5) {
        "weekdays".to_string()
    } else {
        rule.to_rrule()
    };
    (rest, Some(out))
}

enum Found {
    Simple(&'static str),
    Rule(Rule),
}

enum End {
    Count(u32),
    Until(chrono::NaiveDate),
}

fn phrase(s: &str) -> Option<(String, Found)> {
    if let Some((rest, g)) = cut(s, &NTH_OF_MONTH).or_else(|| cut(s, &EVERY_MONTH_ON_NTH)) {
        let mut r = rule(Freq::Monthly, 1);
        r.byday = vec![(Some(ordinal(&g[1])?), crate::nlp::weekday_from(&g[2])?)];
        return Some((rest, Found::Rule(r)));
    }
    if let Some((rest, _)) = cut(s, &LAST_DAY) {
        let mut r = rule(Freq::Monthly, 1);
        r.bymonthday = Some(-1);
        return Some((rest, Found::Rule(r)));
    }
    if let Some((rest, g)) = cut(s, &MONTH_ON_DAY) {
        let d: i8 = g[1].parse().ok().filter(|d| (1..=31).contains(d))?;
        let mut r = rule(Freq::Monthly, 1);
        r.bymonthday = Some(d);
        return Some((rest, Found::Rule(r)));
    }
    if let Some((rest, g)) = cut(s, &INTERVAL) {
        let n: u32 = if g[1].eq_ignore_ascii_case("other") { 2 } else { g[1].parse().ok().filter(|n| (1..=366).contains(n))? };
        let unit = g[2].to_lowercase();
        let freq = if unit.starts_with("day") { Freq::Daily } else if unit.starts_with("week") { Freq::Weekly } else { Freq::Monthly };
        let days = if g[3].is_empty() { vec![] } else { parse_days(&g[3]) };
        if !days.is_empty() && freq != Freq::Weekly {
            return None;
        }
        // "every 1 week" with nothing else is just the free weekly repeat.
        if n == 1 && days.is_empty() {
            let name = match freq {
                Freq::Daily => "daily",
                Freq::Weekly => "weekly",
                Freq::Monthly => "monthly",
            };
            return Some((rest, Found::Simple(name)));
        }
        let mut r = rule(freq, n);
        r.byday = days.into_iter().map(|d| (None, d)).collect();
        return Some((rest, Found::Rule(r)));
    }
    if let Some((rest, g)) = cut(s, &DAY_LIST) {
        let days = parse_days(&g[1]);
        if !days.is_empty() {
            let mut r = rule(Freq::Weekly, 1);
            r.byday = days.into_iter().map(|d| (None, d)).collect();
            return Some((rest, Found::Rule(r)));
        }
    }
    let (rest, g) = cut(s, &SIMPLE)?;
    let name = match g[1].to_lowercase().as_str() {
        "day" => "daily",
        "week" => "weekly",
        "month" => "monthly",
        _ => "weekdays",
    };
    Some((rest, Found::Simple(name)))
}

/// `for 5 times` or `until <date>`; returns the text without it.
fn end_clause(s: &str, now: DateTime<Local>) -> (String, Option<End>) {
    if let Some((rest, g)) = cut(s, &COUNT)
        && let Ok(n) = g[1].parse::<u32>()
        && n >= 1
    {
        return (rest, Some(End::Count(n)));
    }
    if let Some(m) = UNTIL.find(s) {
        let tail = &s[m.end()..];
        if let Some((span, date)) = crate::nlp::leading_date(tail, now) {
            let rest = format!("{} {}", &s[..m.start()], &tail[span..]);
            return (rest, Some(End::Until(date)));
        }
    }
    (s.to_string(), None)
}

fn apply_end(rule: &mut Rule, end: Option<End>) {
    match end {
        Some(End::Count(n)) => rule.count = Some(n),
        Some(End::Until(d)) => rule.until = Some(d),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 3, 11, 10, 0, 0).unwrap()
    }
    fn x(s: &str) -> (String, Option<String>) {
        let (rest, r) = extract(&format!(" {s} "), now());
        (rest.split_whitespace().collect::<Vec<_>>().join(" "), r)
    }
    fn rr(s: &str) -> String {
        x(s).1.unwrap()
    }

    #[test]
    fn legacy_phrases_stay_legacy_values() {
        assert_eq!(rr("water every day"), "daily");
        assert_eq!(rr("standup every weekday"), "weekdays");
        assert_eq!(rr("standup every weekdays"), "weekdays");
        assert_eq!(rr("review every week"), "weekly");
        assert_eq!(rr("rent every month"), "monthly");
        assert_eq!(rr("x every 1 week"), "weekly");
        assert_eq!(x("plain title"), ("plain title".into(), None));
    }

    #[test]
    fn every_n_units() {
        assert_eq!(x("sync every 2 weeks"), ("sync".into(), Some("FREQ=WEEKLY;INTERVAL=2".into())));
        assert_eq!(rr("x every 3 days"), "FREQ=DAILY;INTERVAL=3");
        assert_eq!(rr("x every 6 months"), "FREQ=MONTHLY;INTERVAL=6");
        assert_eq!(rr("x every other week"), "FREQ=WEEKLY;INTERVAL=2");
        assert_eq!(rr("x every other day"), "FREQ=DAILY;INTERVAL=2");
    }

    #[test]
    fn chosen_weekdays() {
        assert_eq!(x("gym every mon,wed"), ("gym".into(), Some("FREQ=WEEKLY;BYDAY=MO,WE".into())));
        assert_eq!(rr("gym every mon, wed and fri"), "FREQ=WEEKLY;BYDAY=MO,WE,FR");
        assert_eq!(rr("gym every monday and friday"), "FREQ=WEEKLY;BYDAY=MO,FR");
        assert_eq!(rr("gym every mon wed"), "FREQ=WEEKLY;BYDAY=MO,WE");
        assert_eq!(rr("gym every friday"), "FREQ=WEEKLY;BYDAY=FR");
        assert_eq!(rr("gym every sat-sun"), "FREQ=WEEKLY;BYDAY=SA,SU");
        assert_eq!(rr("gym every mon-fri"), "weekdays");
        assert_eq!(rr("gym every 2 weeks on mon,thu"), "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH");
        // "every month" is not a weekday list.
        assert_eq!(rr("rent every month"), "monthly");
    }

    #[test]
    fn nth_and_last_weekday_of_the_month() {
        assert_eq!(x("report last friday of the month"), ("report".into(), Some("FREQ=MONTHLY;BYDAY=-1FR".into())));
        assert_eq!(rr("x first monday of every month"), "FREQ=MONTHLY;BYDAY=1MO");
        assert_eq!(rr("x 2nd tuesday of the month"), "FREQ=MONTHLY;BYDAY=2TU");
        assert_eq!(rr("x every month on the third thursday"), "FREQ=MONTHLY;BYDAY=3TH");
        assert_eq!(rr("x last day of the month"), "FREQ=MONTHLY;BYMONTHDAY=-1");
        assert_eq!(rr("x every month on the 15th"), "FREQ=MONTHLY;BYMONTHDAY=15");
    }

    #[test]
    fn end_after_n_times_or_on_a_date() {
        assert_eq!(x("standup every day for 5 times"), ("standup".into(), Some("FREQ=DAILY;COUNT=5".into())));
        assert_eq!(rr("x every 2 weeks 6 times"), "FREQ=WEEKLY;INTERVAL=2;COUNT=6");
        assert_eq!(rr("x every mon,wed until 2026-12-31"), "FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261231");
        assert_eq!(x("x every week until dec 24").1.as_deref(), Some("FREQ=WEEKLY;UNTIL=20261224"));
        // The end clause is stripped from the title.
        assert_eq!(x("pay rent every month until 2027-01-31 please").0, "pay rent please");
        // Without a repeat phrase "5 times" is left alone.
        assert_eq!(x("ring 5 times"), ("ring 5 times".into(), None));
    }
}
