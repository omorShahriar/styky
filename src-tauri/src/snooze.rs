//! Snooze presets (shared meaning with `src/lib/time.ts`): 10 min, 1 hour,
//! this evening (18:00), tomorrow morning (09:00).

use chrono::{Days, Local, NaiveTime, TimeZone};

use crate::repeat::from_naive;

pub const MIN_MS: i64 = 60_000;

/// Absolute snooze time for a preset id (`10m`, `1h`, `evening`, `tomorrow`), or
/// `None` for anything else (e.g. `custom`, which the UI handles itself).
/// "This evening" is today 18:00, or tomorrow 18:00 once 18:00 has passed.
pub fn preset_until(preset: &str, now_ms: i64) -> Option<i64> {
    match preset {
        "10m" => Some(now_ms + 10 * MIN_MS),
        "1h" => Some(now_ms + 60 * MIN_MS),
        "evening" => {
            let now = Local.timestamp_millis_opt(now_ms).single()?;
            let six = NaiveTime::from_hms_opt(18, 0, 0)?;
            let today = from_naive(now.date_naive().and_time(six))?;
            if today > now_ms {
                Some(today)
            } else {
                from_naive(now.date_naive().checked_add_days(Days::new(1))?.and_time(six))
            }
        }
        "tomorrow" => {
            let now = Local.timestamp_millis_opt(now_ms).single()?;
            let nine = NaiveTime::from_hms_opt(9, 0, 0)?;
            from_naive(now.date_naive().checked_add_days(Days::new(1))?.and_time(nine))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
        Local.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap().timestamp_millis()
    }

    #[test]
    fn relative_presets() {
        let now = ms(2026, 3, 10, 12, 0);
        assert_eq!(preset_until("10m", now), Some(now + 600_000));
        assert_eq!(preset_until("1h", now), Some(now + 3_600_000));
    }

    #[test]
    fn evening_is_today_until_six_then_tomorrow() {
        assert_eq!(preset_until("evening", ms(2026, 3, 10, 12, 0)), Some(ms(2026, 3, 10, 18, 0)));
        assert_eq!(preset_until("evening", ms(2026, 3, 10, 18, 0)), Some(ms(2026, 3, 11, 18, 0)));
        assert_eq!(preset_until("evening", ms(2026, 3, 10, 23, 30)), Some(ms(2026, 3, 11, 18, 0)));
    }

    #[test]
    fn tomorrow_is_nine_am_next_day() {
        assert_eq!(preset_until("tomorrow", ms(2026, 3, 10, 8, 0)), Some(ms(2026, 3, 11, 9, 0)));
        assert_eq!(preset_until("tomorrow", ms(2026, 12, 31, 22, 0)), Some(ms(2027, 1, 1, 9, 0)));
    }

    #[test]
    fn unknown_presets_are_none() {
        assert_eq!(preset_until("custom", 0), None);
    }
}
