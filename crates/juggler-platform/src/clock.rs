//! A time to come in the user's own words, for the Floor (its OUT reads
//! "OUT · RESETS 9:31 PM" as the app's Out chip does): the system's own
//! formatter, in the user's locale and time zone (OS seam: macOS here;
//! the other systems say nothing yet and the Floor shows just "OUT").
//!
//! The same rule as the app's `whenWords` (`ui/src/lib/time-words.ts`):
//! "3:10 PM" today, "8:00 AM tomorrow", "Thu 8:00 AM" within the week, and
//! with its date from a week on ("Thu, Nov 5 8:00 AM").

/// `ms` (epoch milliseconds) as the app writes a time to come, seen from
/// `now_ms`; `None` for a time that's past or where the system can't say.
#[must_use]
pub fn when_words(ms: i64, now_ms: i64) -> Option<String> {
    if ms <= now_ms {
        return None;
    }
    imp::when_words(ms, now_ms)
}

/// Which words a time `days` calendar days ahead takes, around its clock
/// (`clock`), its weekday (`day`) and its weekday with the date (`date`).
fn words(
    days: i64,
    clock: &str,
    day: impl FnOnce() -> String,
    date: impl FnOnce() -> String,
) -> String {
    match days {
        0 => clock.to_owned(),
        1 => format!("{clock} tomorrow"),
        2..=6 => format!("{} {clock}", day()),
        _ => format!("{} {clock}", date()),
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_foundation::{NSCalendar, NSDate, NSDateFormatter, NSString};

    pub(super) fn when_words(ms: i64, now_ms: i64) -> Option<String> {
        #[allow(clippy::cast_precision_loss)]
        // epoch milliseconds: exact in an f64 for 285,000 years
        let at = |ms: i64| NSDate::dateWithTimeIntervalSince1970(ms as f64 / 1000.0);
        let (date, now) = (at(ms), at(now_ms));
        let calendar = NSCalendar::currentCalendar();
        let span = calendar
            .startOfDayForDate(&date)
            .timeIntervalSinceDate(&calendar.startOfDayForDate(&now));
        #[allow(clippy::cast_possible_truncation)] // whole days, a few at most
        let days = (span / 86_400.0).round() as i64;
        // the formatter's own locale and time zone: the user's
        let format = |template: &str| {
            let formatter = NSDateFormatter::new();
            formatter.setLocalizedDateFormatFromTemplate(&NSString::from_str(template));
            formatter.stringFromDate(&date).to_string()
        };
        let clock = format("jmm");
        Some(super::words(
            days,
            &clock,
            || format("EEE"),
            || format("EEEMMMd"),
        ))
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub(super) fn when_words(_ms: i64, _now_ms: i64) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_time_to_come_reads_as_the_app_writes_it() {
        let day = || "Thu".to_owned();
        let date = || "Thu, Nov 5".to_owned();
        assert_eq!(super::words(0, "3:10 PM", day, date), "3:10 PM");
        assert_eq!(super::words(1, "8:00 AM", day, date), "8:00 AM tomorrow");
        assert_eq!(super::words(3, "8:00 AM", day, date), "Thu 8:00 AM");
        assert_eq!(super::words(9, "8:00 AM", day, date), "Thu, Nov 5 8:00 AM");
        // past: nothing
        assert_eq!(super::when_words(1_000, 2_000), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_system_says_a_clock_time_with_its_minutes() {
        let now = 1_800_000_000_000;
        let words = super::when_words(now + 3 * 60 * 60 * 1000, now).unwrap();
        // the user's locale decides 12 or 24 hours; minutes always show
        assert!(words.contains(':') || words.contains('.'), "{words}");
        let later = super::when_words(now + 10 * 24 * 60 * 60 * 1000, now).unwrap();
        assert!(later.len() > words.len(), "{later} has its date");
    }
}
