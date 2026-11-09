//! The "game day" that today's totals belong to.
//!
//! INFINITAS goes down for maintenance every day at 05:00 JST (earlier on
//! some days, but never later), so a session can never straddle 05:00 JST.
//! Using that as the day boundary keeps a late-night session in one day,
//! whatever time zone the PC is set to.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, Utc};

/// Japan Standard Time. Japan has no daylight saving, so a fixed offset is
/// exact.
pub fn jst() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).expect("valid offset")
}

/// Default day boundary: the start of the daily maintenance.
pub fn default_day_start() -> NaiveTime {
    NaiveTime::from_hms_opt(5, 0, 0).expect("valid time")
}

/// The game day `now` belongs to, when days start at `day_start` (JST).
pub fn game_day(now: DateTime<Utc>, day_start: NaiveTime) -> NaiveDate {
    let since_midnight = day_start - NaiveTime::MIN;
    (now.with_timezone(&jst()) - since_midnight).date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().into()
    }

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn day_changes_at_five_jst() {
        let start = default_day_start();
        // 2026-11-08 04:59 JST is still the 7th's session.
        assert_eq!(
            game_day(utc("2026-11-07T19:59:59Z"), start),
            date("2026-11-07")
        );
        // 05:00 JST starts the 8th.
        assert_eq!(
            game_day(utc("2026-11-07T20:00:00Z"), start),
            date("2026-11-08")
        );
        // Late evening JST is the same day.
        assert_eq!(
            game_day(utc("2026-11-08T14:30:00Z"), start),
            date("2026-11-08")
        );
    }

    #[test]
    fn independent_of_the_pc_time_zone() {
        // 03:00 in Taipei (UTC+8) is 04:00 JST: still the previous day.
        let taipei = DateTime::parse_from_rfc3339("2026-11-08T03:00:00+08:00").unwrap();
        assert_eq!(
            game_day(taipei.into(), default_day_start()),
            date("2026-11-07")
        );
    }

    #[test]
    fn midnight_boundary_is_the_calendar_date() {
        assert_eq!(
            game_day(utc("2026-11-07T15:00:00Z"), NaiveTime::MIN),
            date("2026-11-08")
        );
    }
}
