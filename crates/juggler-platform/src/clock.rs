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

/// When a session started, in the user's own words, as the app's
/// `startedWords` says it (D-96: an untitled session is told apart by its
/// folder, its App and this): "started 1:14 AM" today, "started yesterday
/// 1:14 AM", "started Thu 1:14 AM" within the week, and with its date from
/// a week back ("started Thu, Oct 23 1:14 AM"); `None` where the system
/// can't say.
#[must_use]
pub fn started_words(ms: i64, now_ms: i64) -> Option<String> {
    imp::started_words(ms, now_ms)
}

/// Which words a start `days` calendar days back takes, around its clock
/// (`clock`), its weekday (`day`) and its weekday with the date (`date`).
#[cfg(any(target_os = "macos", test))]
fn started(
    days: i64,
    clock: &str,
    day: impl FnOnce() -> String,
    date: impl FnOnce() -> String,
) -> String {
    match days {
        ..=0 => format!("started {clock}"),
        1 => format!("started yesterday {clock}"),
        2..=6 => format!("started {} {clock}", day()),
        _ => format!("started {} {clock}", date()),
    }
}

/// Which words a time `days` calendar days ahead takes, around its clock
/// (`clock`), its weekday (`day`) and its weekday with the date (`date`).
#[cfg(any(target_os = "macos", test))]
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
    use std::cell::RefCell;
    use std::collections::HashMap;

    use objc2::rc::Retained;
    use objc2_foundation::{NSCalendar, NSDate, NSDateFormatter, NSString};

    thread_local! {
        /// One formatter per template, made once per thread (making one
        /// costs far more than using it: the Floor names 150 sessions a
        /// frame), in the locale and time zone it was made in: the user's.
        static FORMATTERS: RefCell<HashMap<&'static str, Retained<NSDateFormatter>>> =
            RefCell::new(HashMap::new());
    }

    /// `date` by `template`, in the user's locale and time zone.
    fn format(template: &'static str, date: &NSDate) -> String {
        FORMATTERS.with(|cache| {
            let mut cache = cache.borrow_mut();
            let formatter = cache.entry(template).or_insert_with(|| {
                let formatter = NSDateFormatter::new();
                formatter.setLocalizedDateFormatFromTemplate(&NSString::from_str(template));
                formatter
            });
            formatter.stringFromDate(date).to_string()
        })
    }

    /// `ms` as a date, and the calendar days from `now_ms`'s day to its.
    fn on_calendar(ms: i64, now_ms: i64) -> (Retained<NSDate>, Retained<NSDate>, i64) {
        #[allow(
            clippy::cast_precision_loss,
            reason = "epoch milliseconds: exact in an f64 for 285,000 years"
        )]
        let at = |ms: i64| NSDate::dateWithTimeIntervalSince1970(ms as f64 / 1000.0);
        let (date, now) = (at(ms), at(now_ms));
        let calendar = NSCalendar::currentCalendar();
        let span = calendar
            .startOfDayForDate(&date)
            .timeIntervalSinceDate(&calendar.startOfDayForDate(&now));
        #[allow(clippy::cast_possible_truncation, reason = "whole days, a few at most")]
        let days = (span / 86_400.0).round() as i64;
        (date, now, days)
    }

    pub(super) fn when_words(ms: i64, now_ms: i64) -> Option<String> {
        let (date, _, days) = on_calendar(ms, now_ms);
        let clock = format("jmm", &date);
        // a formatter that says nothing (no locale data) says nothing here
        (!clock.is_empty()).then(|| {
            super::words(
                days,
                &clock,
                || format("EEE", &date),
                || format("EEEMMMd", &date),
            )
        })
    }

    /// What a start minute was said as, and the bounds (epoch seconds) of
    /// the day it was said in.
    type Said = (Option<String>, f64, f64);

    thread_local! {
        /// What each start minute was said as: the words hold until that
        /// day ends.
        static SAID: RefCell<HashMap<i64, Said>> = RefCell::new(HashMap::new());
    }

    pub(super) fn started_words(ms: i64, now_ms: i64) -> Option<String> {
        let minute = ms.div_euclid(60_000);
        #[allow(clippy::cast_precision_loss, reason = "epoch milliseconds")]
        let seconds_now = now_ms as f64 / 1000.0;
        let kept = SAID.with(|said| {
            said.borrow()
                .get(&minute)
                .filter(|(_, from, until)| (*from..*until).contains(&seconds_now))
                .map(|(words, _, _)| words.clone())
        });
        if let Some(words) = kept {
            return words;
        }
        let words = say_started(ms, now_ms);
        let calendar = NSCalendar::currentCalendar();
        let from = calendar
            .startOfDayForDate(&NSDate::dateWithTimeIntervalSince1970(seconds_now))
            .timeIntervalSince1970();
        // Tomorrow's start: a day is 23 to 25 hours (a clock change), so 36
        // hours on is always tomorrow.
        let until = calendar
            .startOfDayForDate(&NSDate::dateWithTimeIntervalSince1970(from + 36.0 * 3600.0))
            .timeIntervalSince1970();
        SAID.with(|said| {
            let mut said = said.borrow_mut();
            if said.len() > 4096 {
                said.clear();
            }
            said.insert(minute, (words.clone(), from, until));
        });
        words
    }

    fn say_started(ms: i64, now_ms: i64) -> Option<String> {
        let (date, now, days) = on_calendar(ms, now_ms);
        let clock = format("jmm", &date);
        (!clock.is_empty()).then(|| {
            super::started(
                -days,
                &clock,
                || format("EEE", &date),
                // its year only when not this one's
                || {
                    let year = format("y", &date) != format("y", &now);
                    format(if year { "EEEMMMdy" } else { "EEEMMMd" }, &date)
                },
            )
        })
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub(super) fn when_words(_ms: i64, _now_ms: i64) -> Option<String> {
        None
    }

    pub(super) fn started_words(_ms: i64, _now_ms: i64) -> Option<String> {
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

    #[test]
    fn a_start_reads_as_the_app_writes_it() {
        let day = || "Thu".to_owned();
        let date = || "Thu, Oct 23".to_owned();
        assert_eq!(super::started(0, "1:14 AM", day, date), "started 1:14 AM");
        assert_eq!(
            super::started(1, "1:14 AM", day, date),
            "started yesterday 1:14 AM"
        );
        assert_eq!(
            super::started(3, "1:14 AM", day, date),
            "started Thu 1:14 AM"
        );
        assert_eq!(
            super::started(9, "1:14 AM", day, date),
            "started Thu, Oct 23 1:14 AM"
        );
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
        let started = super::started_words(now - 60 * 1000, now).unwrap();
        assert!(started.starts_with("started "), "{started}");
        let long_ago = super::started_words(now - 10 * 24 * 60 * 60 * 1000, now).unwrap();
        assert!(long_ago.len() > started.len(), "{long_ago} has its date");
    }
}
