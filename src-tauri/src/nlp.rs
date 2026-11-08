//! Quick-add natural-language parser for the local API / CLI / MCP (Rust port of `src/lib/nlp.ts`).
//!
//! Why a port rather than calling the webview: the API must work with the panel never shown, mid-start-up
//! or with the webview throttled, and from a headless CLI; a deterministic, unit-tested Rust parser has
//! no such failure modes. The in-app quick-add bar keeps using chrono-node (richer dates); this parser
//! implements the same *token* grammar and the common date/time phrases:
//!
//!   #tag   @branch   https://link   !! urgent   !!! critical   ! normal   !low
//!   every day|weekday|week|month   (more repeat forms: see `repeat_rule`)
//!   today | tomorrow | day after tomorrow | [next|this] <weekday> | next week | in 2h / 3 days / 1 week / 2 months
//!   <Mon> 12 | 12 <Mon> | 2026-03-12 | 3/12 | at 5 | 5pm | 5:30pm | 17:30 | noon | midnight | tonight
//!
//! A date without a time is due at 09:00; a repeat without a date starts at the next 09:00.

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, NaiveTime, Timelike, Weekday};
use regex::Regex;
use std::sync::LazyLock;

use crate::repeat::from_naive;

#[derive(Debug, Clone, PartialEq)]
pub struct QuickParse {
    pub title: String,
    pub due_at: Option<i64>,
    pub repeat: String,
    pub priority: i64,
    pub tag: Option<String>,
    pub link: Option<String>,
    pub branch: Option<String>,
}

pub const DEFAULT_HOUR: u32 = 9;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("static regex")
}

static LINK: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)https?://\S+"));
static BRANCH: LazyLock<Regex> = LazyLock::new(|| re(r"(\s)@(\w[\w./-]*)"));
static TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(\s)#(\w[\w-]*)"));
static LOW: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(\s)!low(\s)"));
static BANGS: LazyLock<Regex> = LazyLock::new(|| re(r"(\s)(!{1,3})(\s)"));

/// Cut the first match out of `s` (replaced by one space); returns the capture groups as owned text.
fn take(s: &str, r: &Regex) -> (String, Option<Vec<String>>) {
    match r.captures(s) {
        Some(c) => {
            let m = c.get(0).unwrap();
            let groups = c.iter().map(|g| g.map_or(String::new(), |g| g.as_str().to_string())).collect();
            (format!("{} {}", &s[..m.start()], &s[m.end()..]), Some(groups))
        }
        None => (s.to_string(), None),
    }
}

pub fn parse_quick(input: &str, now: DateTime<Local>) -> QuickParse {
    let mut text = format!(" {input} ");
    let mut link = None;
    let mut branch = None;
    let mut tag = None;
    let mut priority = 1;

    let (rest, m) = take(&text, &LINK);
    if let Some(c) = m {
        link = Some(c[0].trim_end_matches(['.', ',', ';', ')']).to_string());
    }
    text = rest;

    let (rest, rule) = crate::nlp_repeat::extract(&text, now);
    text = rest;
    let mut repeat = rule.unwrap_or_else(|| "none".to_string());

    // Keep the leading whitespace the regex consumed so neighbours stay separated.
    let (rest, m) = take(&text, &BRANCH);
    if let Some(c) = m {
        branch = Some(c[2].trim_end_matches(['.', ',', ';']).to_string());
    }
    text = rest;

    let (rest, m) = take(&text, &TAG);
    if let Some(c) = m {
        tag = Some(c[2].to_string());
    }
    text = rest;

    let (rest, m) = take(&text, &LOW);
    if m.is_some() {
        priority = 0;
        text = rest;
    } else {
        let (rest, m) = take(&text, &BANGS);
        if let Some(c) = m {
            // ! normal, !! urgent, !!! critical
            priority = match c[2].len() {
                3 => 3,
                2 => 2,
                _ => 1,
            };
        }
        text = rest;
    }

    let mut title = text.clone();
    let mut due_at = None;
    if let Some(d) = find_date(&text, now) {
        due_at = Some(d.ms);
        let without = format!("{} {}", &text[..d.start], &text[d.end..]);
        // "tomorrow 5pm" alone: keep the words as the title rather than saving an empty one.
        if !clean(&without).is_empty() {
            title = without;
        }
    }
    let title = clean(&title);
    if due_at.is_none() && repeat != "none" {
        // A rule that names its own days (every mon,wed / last friday of the month) starts on the first such day.
        due_at = crate::repeat::first_match(&repeat, now.timestamp_millis(), DEFAULT_HOUR)
            .or_else(|| Some(next_default_time(now, repeat == "weekdays")));
    }
    if repeat.is_empty() {
        repeat = "none".into();
    }
    QuickParse { title, due_at, repeat, priority, tag, link, branch }
}

/// Collapse whitespace and drop a dangling connective left by a removed date ("… by", "… at").
fn clean(s: &str) -> String {
    static TAIL: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(^|\s)(at|on|by|due|in|from|until|for)$"));
    let one = s.split_whitespace().collect::<Vec<_>>().join(" ");
    TAIL.replace(&one, "").trim().to_string()
}

fn next_default_time(now: DateTime<Local>, weekdays: bool) -> i64 {
    let mut d = now.date_naive();
    let nine = NaiveTime::from_hms_opt(DEFAULT_HOUR, 0, 0).unwrap();
    let mut at = from_naive(d.and_time(nine)).unwrap_or(0);
    if at <= now.timestamp_millis() {
        d = d.checked_add_days(Days::new(1)).unwrap_or(d);
    }
    while weekdays && matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
        d = d.checked_add_days(Days::new(1)).unwrap_or(d);
    }
    at = from_naive(d.and_time(nine)).unwrap_or(at);
    at
}

// ------------------------------------------------------------------ dates

struct Found {
    ms: i64,
    start: usize,
    end: usize,
}

const MONTHS: &str = "jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?";
const WEEKDAYS: &str = "mon(?:day)?|tue(?:s(?:day)?)?|wed(?:nesday)?|thu(?:r(?:s(?:day)?)?)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?";

fn month_num(s: &str) -> Option<u32> {
    let k = s.to_lowercase();
    ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"]
        .iter()
        .position(|m| k.starts_with(m))
        .map(|i| i as u32 + 1)
}

pub fn weekday_from(s: &str) -> Option<Weekday> {
    let k = s.to_lowercase();
    Some(match &k[..3.min(k.len())] {
        "mon" => Weekday::Mon,
        "tue" => Weekday::Tue,
        "wed" => Weekday::Wed,
        "thu" => Weekday::Thu,
        "fri" => Weekday::Fri,
        "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// A matched piece of the text.
struct Span {
    start: usize,
    end: usize,
}

enum DatePart {
    /// A calendar day.
    Day(NaiveDate),
    /// A weekday name (resolved after the time is known).
    Weekday(Weekday, bool),
}

static REL: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\bin\s+(\d+|an?|half an)\s*(minutes?|mins?|m|hours?|hrs?|h|days?|d|weeks?|w|months?)\b")
});
static ISO: LazyLock<Regex> = LazyLock::new(|| re(r"\b(\d{4})-(\d{1,2})-(\d{1,2})\b"));
static SLASH: LazyLock<Regex> = LazyLock::new(|| re(r"\b(\d{1,2})/(\d{1,2})(?:/(\d{2,4}))?\b"));
static MON_DAY: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(r"(?i)\b({MONTHS})\.?\s+(\d{{1,2}})(?:st|nd|rd|th)?\b(?:,?\s+(\d{{4}}))?"))
});
static DAY_MON: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(r"(?i)\b(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?({MONTHS})\b(?:,?\s+(\d{{4}}))?"))
});
static DAYS_AFTER: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bday after tomorrow\b"));
static WORDS: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(today|tomorrow|tmrw|tmr|tonight)\b"));
static NEXT_WEEK: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bnext week\b"));
static WEEKDAY: LazyLock<Regex> = LazyLock::new(|| re(&format!(r"(?i)\b(?:(next|this|on)\s+)?({WEEKDAYS})\b")));
static T_AMPM: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*([ap])\.?m\b\.?"));
static T_24: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:at\s+)?([01]?\d|2[0-3]):([0-5]\d)\b"));
static T_AT: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bat\s+(\d{1,2})\b"));
static T_WORD: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(?:at\s+)?(noon|midnight)\b"));

fn find_time(text: &str) -> Option<(Span, u32, u32)> {
    if let Some(c) = T_AMPM.captures(text) {
        let h: u32 = c[1].parse().ok()?;
        let m: u32 = c.get(2).map_or(Some(0), |m| m.as_str().parse().ok())?;
        if (1..=12).contains(&h) && m < 60 {
            let pm = c[3].eq_ignore_ascii_case("p");
            let h = (h % 12) + if pm { 12 } else { 0 };
            let g = c.get(0)?;
            return Some((Span { start: g.start(), end: g.end() }, h, m));
        }
    }
    if let Some(c) = T_24.captures(text) {
        let g = c.get(0)?;
        return Some((Span { start: g.start(), end: g.end() }, c[1].parse().ok()?, c[2].parse().ok()?));
    }
    if let Some(c) = T_WORD.captures(text) {
        let g = c.get(0)?;
        let h = if c[1].eq_ignore_ascii_case("noon") { 12 } else { 0 };
        return Some((Span { start: g.start(), end: g.end() }, h, 0));
    }
    if let Some(c) = T_AT.captures(text) {
        // "at 5": a bare hour. 1-7 is read as the afternoon (office hours), 8-11 as the morning.
        let h: u32 = c[1].parse().ok()?;
        if h <= 23 {
            let h = if (1..=7).contains(&h) { h + 12 } else { h };
            let g = c.get(0)?;
            return Some((Span { start: g.start(), end: g.end() }, h, 0));
        }
    }
    None
}

fn find_day(text: &str, today: NaiveDate) -> Option<(Span, DatePart)> {
    let year = today.year();
    let mk = |c: &regex::Captures, p: DatePart| {
        let g = c.get(0).unwrap();
        Some((Span { start: g.start(), end: g.end() }, p))
    };
    // Roll a month/day without a year forward to its next occurrence.
    let md = |m: u32, d: u32, y: Option<i32>| -> Option<NaiveDate> {
        match y {
            Some(y) => NaiveDate::from_ymd_opt(y, m, d),
            None => {
                let this = NaiveDate::from_ymd_opt(year, m, d)?;
                if this < today { NaiveDate::from_ymd_opt(year + 1, m, d) } else { Some(this) }
            }
        }
    };
    if let Some(c) = ISO.captures(text) {
        let d = NaiveDate::from_ymd_opt(c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?)?;
        return mk(&c, DatePart::Day(d));
    }
    if let Some(c) = MON_DAY.captures(text) {
        let y = c.get(3).and_then(|y| y.as_str().parse().ok());
        if let Some(d) = md(month_num(&c[1])?, c[2].parse().ok()?, y) {
            return mk(&c, DatePart::Day(d));
        }
    }
    if let Some(c) = DAY_MON.captures(text) {
        let y = c.get(3).and_then(|y| y.as_str().parse().ok());
        if let Some(d) = md(month_num(&c[2])?, c[1].parse().ok()?, y) {
            return mk(&c, DatePart::Day(d));
        }
    }
    if let Some(c) = SLASH.captures(text) {
        // US order, like chrono-node's default: 3/12 = March 12.
        let y = c.get(3).and_then(|y| {
            let n: i32 = y.as_str().parse().ok()?;
            Some(if n < 100 { 2000 + n } else { n })
        });
        if let Some(d) = md(c[1].parse().ok()?, c[2].parse().ok()?, y) {
            return mk(&c, DatePart::Day(d));
        }
    }
    if let Some(c) = DAYS_AFTER.captures(text) {
        return mk(&c, DatePart::Day(today.checked_add_days(Days::new(2))?));
    }
    if let Some(c) = WORDS.captures(text) {
        let w = c[1].to_lowercase();
        let d = if w.starts_with('t') && w != "today" && w != "tonight" { today.checked_add_days(Days::new(1))? } else { today };
        return mk(&c, DatePart::Day(d));
    }
    if let Some(c) = NEXT_WEEK.captures(text) {
        return mk(&c, DatePart::Day(today.checked_add_days(Days::new(7))?));
    }
    if let Some(c) = WEEKDAY.captures(text) {
        let wd = weekday_from(&c[2])?;
        let next = c.get(1).is_some_and(|m| m.as_str().eq_ignore_ascii_case("next"));
        return mk(&c, DatePart::Weekday(wd, next));
    }
    None
}

/// A calendar date at the very start of `tail` (used by `until <date>`): the end of the date text and the date.
pub fn leading_date(tail: &str, now: DateTime<Local>) -> Option<(usize, NaiveDate)> {
    let (span, part) = find_day(tail, now.date_naive())?;
    if !tail[..span.start].trim().is_empty() {
        return None;
    }
    match part {
        DatePart::Day(d) => Some((span.end, d)),
        DatePart::Weekday(..) => None,
    }
}

fn find_date(text: &str, now: DateTime<Local>) -> Option<Found> {
    let today = now.date_naive();
    let now_ms = now.timestamp_millis();

    // "in 2h" / "in 3 days": an exact offset (the time of day is certain).
    if let Some(c) = REL.captures(text) {
        let n: f64 = match c[1].to_lowercase().as_str() {
            "a" | "an" => 1.0,
            "half an" => 0.5,
            v => v.parse().ok()?,
        };
        let unit = c[2].to_lowercase();
        let g = c.get(0)?;
        let ms = match unit.chars().next()? {
            'm' if unit.starts_with("month") => {
                let d = now.checked_add_months(Months::new(n as u32))?;
                d.timestamp_millis()
            }
            'm' => now_ms.checked_add((n * 60_000.0) as i64)?,
            'h' => now_ms.checked_add((n * 3_600_000.0) as i64)?,
            'd' => from_naive(today.checked_add_days(Days::new(n as u64))?.and_time(now.time().with_nanosecond(0)?))?,
            'w' => from_naive(today.checked_add_days(Days::new((n as u64).saturating_mul(7)))?.and_time(now.time().with_nanosecond(0)?))?,
            _ => return None,
        };
        return Some(Found { ms, start: g.start(), end: g.end() });
    }

    let day = find_day(text, today);
    let time = find_time(text);
    if day.is_none() && time.is_none() {
        return None;
    }
    // "tonight" implies the evening when no time is spoken.
    let tonight = day.as_ref().is_some_and(|(s, _)| text[s.start..s.end].eq_ignore_ascii_case("tonight"));
    let (h, m) = match &time {
        Some((_, h, m)) => (*h, *m),
        None if tonight => (20, 0),
        None => (DEFAULT_HOUR, 0),
    };
    let at = |d: NaiveDate| from_naive(d.and_time(NaiveTime::from_hms_opt(h, m, 0)?));
    let date_ms = match &day {
        Some((_, DatePart::Day(d))) => at(*d)?,
        Some((_, DatePart::Weekday(wd, next))) => {
            let mut d = today;
            let delta = (wd.num_days_from_monday() as i64 - today.weekday().num_days_from_monday() as i64).rem_euclid(7) as u64;
            d = d.checked_add_days(Days::new(delta))?;
            // The same weekday as today, or "next <weekday>": a week on if it already passed / was asked for.
            if (delta == 0 && at(d)? <= now_ms) || (*next && delta == 0) || (*next && delta < 7 && today.weekday() == *wd) {
                d = d.checked_add_days(Days::new(7))?;
            }
            at(d)?
        }
        None => {
            // A time alone: today if still ahead, otherwise tomorrow.
            let t = at(today)?;
            if t > now_ms { t } else { at(today.checked_add_days(Days::new(1))?)? }
        }
    };
    // Merge the spans: the day and the time (when adjacent words) are cut out together.
    let spans: Vec<&Span> = day.iter().map(|(s, _)| s).chain(time.iter().map(|(s, _, _)| s)).collect();
    let start = spans.iter().map(|s| s.start).min()?;
    let end = spans.iter().map(|s| s.end).max()?;
    // Only swallow the words between the two spans when they are a connector ("fri at 5pm", "5pm on friday").
    let (start, end) = if spans.len() == 2 {
        let (a, b) = if spans[0].start <= spans[1].start { (spans[0], spans[1]) } else { (spans[1], spans[0]) };
        let gap = text[a.end.min(b.start)..b.start].trim().to_lowercase();
        if matches!(gap.as_str(), "" | "at" | "on" | "@" | "by" | ",") {
            (start, end)
        } else {
            // Not adjacent: cut only the day; the time words stay in the title.
            (day.as_ref().map(|(s, _)| s.start)?, day.as_ref().map(|(s, _)| s.end)?)
        }
    } else {
        (start, end)
    };
    Some(Found { ms: date_ms, start, end })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Wed 2026-03-11 10:00 local.
    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 3, 11, 10, 0, 0).unwrap()
    }
    fn ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        Local.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp_millis()
    }
    fn p(s: &str) -> QuickParse {
        parse_quick(s, now())
    }

    #[test]
    fn plain_title() {
        let q = p("Buy milk");
        assert_eq!((q.title.as_str(), q.due_at, q.repeat.as_str(), q.priority), ("Buy milk", None, "none", 1));
    }

    #[test]
    fn the_documented_example_matches_the_ts_parser() {
        // TS: "Rotate keys fri 10am #infra every month !!"
        let q = p("Rotate keys fri 10am #infra every month !!");
        assert_eq!(q.title, "Rotate keys");
        assert_eq!(q.due_at, Some(ms(2026, 3, 13, 10, 0)));
        assert_eq!((q.repeat.as_str(), q.priority, q.tag.as_deref()), ("monthly", 2, Some("infra")));
    }

    #[test]
    fn tokens_priority_branch_link() {
        let q = p("Release v2.3 @release/2.3 https://example.com/x. !!!");
        assert_eq!((q.priority, q.branch.as_deref(), q.link.as_deref()), (3, Some("release/2.3"), Some("https://example.com/x")));
        assert_eq!(q.title, "Release v2.3");
        assert_eq!(p("sweep !low").priority, 0);
        assert_eq!(p("a ! b").priority, 1);
        // e-mail addresses and C# are left alone.
        let q = p("mail bob@example.com about C#");
        assert_eq!((q.branch, q.tag), (None, None));
    }

    #[test]
    fn dates_and_times() {
        assert_eq!(p("call mom tomorrow 5pm").due_at, Some(ms(2026, 3, 12, 17, 0)));
        assert_eq!(p("call mom tomorrow").due_at, Some(ms(2026, 3, 12, 9, 0)));
        assert_eq!(p("standup at 17:30").due_at, Some(ms(2026, 3, 11, 17, 30)));
        // A time that already passed today means tomorrow.
        assert_eq!(p("gym 7am").due_at, Some(ms(2026, 3, 12, 7, 0)));
        assert_eq!(p("lunch noon").due_at, Some(ms(2026, 3, 11, 12, 0)));
        assert_eq!(p("pay rent on the 1st").due_at, None);
        assert_eq!(p("pay rent apr 1").due_at, Some(ms(2026, 4, 1, 9, 0)));
        assert_eq!(p("renew 5 march").due_at, Some(ms(2027, 3, 5, 9, 0)));
        assert_eq!(p("x 2026-12-25 8pm").due_at, Some(ms(2026, 12, 25, 20, 0)));
        assert_eq!(p("x 3/20").due_at, Some(ms(2026, 3, 20, 9, 0)));
        assert_eq!(p("day after tomorrow dentist").due_at, Some(ms(2026, 3, 13, 9, 0)));
    }

    #[test]
    fn weekdays_resolve_forward() {
        assert_eq!(p("review friday").due_at, Some(ms(2026, 3, 13, 9, 0)));
        assert_eq!(p("review monday 2pm").due_at, Some(ms(2026, 3, 16, 14, 0)));
        // Today is Wednesday: the 09:00 default has passed, so the next Wednesday.
        assert_eq!(p("review wed").due_at, Some(ms(2026, 3, 18, 9, 0)));
        assert_eq!(p("review wed 5pm").due_at, Some(ms(2026, 3, 11, 17, 0)));
        assert_eq!(p("review next wed 5pm").due_at, Some(ms(2026, 3, 18, 17, 0)));
        assert_eq!(p("review next week").due_at, Some(ms(2026, 3, 18, 9, 0)));
    }

    #[test]
    fn relative_offsets() {
        let n = now().timestamp_millis();
        assert_eq!(p("stretch in 2h").due_at, Some(n + 2 * 3_600_000));
        assert_eq!(p("tea in 15 min").due_at, Some(n + 15 * 60_000));
        assert_eq!(p("check in 3 days").due_at, Some(ms(2026, 3, 14, 10, 0)));
        assert_eq!(p("check in 1 week").due_at, Some(ms(2026, 3, 18, 10, 0)));
        assert_eq!(p("report in 2 months").due_at, Some(ms(2026, 5, 11, 10, 0)));
    }

    #[test]
    fn repeat_without_date_starts_at_the_next_nine() {
        let q = p("water plants every day");
        assert_eq!((q.repeat.as_str(), q.due_at), ("daily", Some(ms(2026, 3, 12, 9, 0))));
        // Friday evening-ish: weekday repeat skips the weekend.
        let fri = Local.with_ymd_and_hms(2026, 3, 13, 12, 0, 0).unwrap();
        assert_eq!(parse_quick("standup every weekday", fri).due_at, Some(ms(2026, 3, 16, 9, 0)));
    }

    #[test]
    fn advanced_repeats_parse_and_start_on_the_right_day() {
        // "every mon,wed" with no date starts on the next listed day at 09:00 (today is Wed 10:00 -> next Mon).
        let q = p("gym every mon,wed");
        assert_eq!((q.title.as_str(), q.repeat.as_str()), ("gym", "FREQ=WEEKLY;BYDAY=MO,WE"));
        assert_eq!(q.due_at, Some(ms(2026, 3, 16, 9, 0)));
        // "last friday of the month" -> Mar 27.
        let q = p("send report last friday of the month");
        assert_eq!((q.title.as_str(), q.repeat.as_str()), ("send report", "FREQ=MONTHLY;BYDAY=-1FR"));
        assert_eq!(q.due_at, Some(ms(2026, 3, 27, 9, 0)));
        // "every 2 weeks" without a date: next 09:00; with a date the date wins.
        let q = p("sync every 2 weeks");
        assert_eq!((q.repeat.as_str(), q.due_at), ("FREQ=WEEKLY;INTERVAL=2", Some(ms(2026, 3, 12, 9, 0))));
        let q = p("sync every 2 weeks fri 3pm for 4 times");
        assert_eq!((q.title.as_str(), q.repeat.as_str()), ("sync", "FREQ=WEEKLY;INTERVAL=2;COUNT=4"));
        assert_eq!(q.due_at, Some(ms(2026, 3, 13, 15, 0)));
    }

    /// Arbitrary text arrives from the CLI, MCP clients and any program on the pipe: nothing may panic.
    #[test]
    fn odd_input_never_panics() {
        let fixed = [
            "", " ", "\n\t", "#", "@", "!", "!!", "!!!", "!!!!", "#@!", "@#", "# #", "@ @ @", "! ! !", "!low", "!LOW!", "every", "every ", "every every",
            "every 0 days", "every 99999999999999999999 days", "every -3 weeks", "every 367 days", "every mon,", "every ,", "every -", "every mon-", "every mon-mon",
            "every sun-mon", "last", "last friday", "last friday of", "of the month", "first of the month", "5th", "99th friday of the month", "0th monday of the month",
            "for 0 times", "for 99999999999999 times", "until", "until until", "until 99999-99-99", "until 2026-13-45", "until \u{0}",
            "in", "in 5", "in 99999999999999999999999 hours", "in 1e999 days", "in -5 minutes", "in 18446744073709551615 weeks", "in 4294967296 months",
            "at", "at 99", "at 25:99", "99:99", "13pm", "0am", "12:60pm", "2026-02-30", "0000-00-00", "13/45/99", "3/0", "feb 30", "dec 0", "32 jan",
            "٣ pm", "at ٥", "２０２６-０３-１１", "every ٢ weeks", "٥ times every day", "٣٣th friday of the month", "https://", "http://\u{0}", "https://a.b/\u{1F600}",
            "😀", "😀 tomorrow 😀", "#😀", "@😀", "é #é @é !!", "ﬁ ﬂ", "tomorrow\u{200B}5pm", "Ａ", "\u{202E}tomorrow",
        ];
        let now = Local.with_ymd_and_hms(2026, 3, 11, 10, 0, 0).unwrap();
        for s in fixed {
            let _ = parse_quick(s, now);
        }
        // Very long inputs.
        for unit in ["a", "#", "@", "!", "every ", "tomorrow ", "😀", "1", "in 5 ", "mon,"] {
            let _ = parse_quick(&unit.repeat(20_000), now);
        }
        // Deterministic pseudo-random soup of tokens, digits and multibyte characters.
        let pieces = [
            "every", "day", "days", "week", "weeks", "month", "mon", "tue", "wed", "fri", "sun", "last", "first", "2nd", "of", "the", "until", "for", "times", "in", "at",
            "tomorrow", "today", "next", "noon", "pm", "am", ":", ",", "-", "/", "#", "@", "!", "!!", "!!!", "0", "5", "99", "2026", "٣", "😀", "é", " ", "  ", "\u{200B}",
            "https://x.y", "1e9", "-1",
        ];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..4000 {
            let mut s = String::new();
            for _ in 0..(1 + seed % 12) {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                s.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
                if !(seed >> 20).is_multiple_of(3) {
                    s.push(' ');
                }
            }
            let _ = parse_quick(&s, now);
        }
    }

    #[test]
    fn title_survives_when_only_a_date_is_given() {
        assert_eq!(p("tomorrow 5pm").title, "tomorrow 5pm");
        assert_eq!(p("send invoice by friday").title, "send invoice");
    }
}
