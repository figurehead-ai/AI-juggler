//! 0.4.9 (AJ-E2312 item 1): when a session's limit has reset, by its
//! Account's readings. The PM5 walk: Work read 100% used to a reset three
//! minutes after the lab opened; past it, the session that waited still
//! said "Limit reached". Every value is invented.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step is the failure it reports"
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind, limit_reset_at};

const T0: i64 = 1_792_000_000_000;
const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const FIVE: WindowKind = WindowKind::FiveHour;
const WEEK: WindowKind = WindowKind::Weekly { anchor_ms: None };

fn read(ts: i64, used_pct: f64, resets_at: Option<i64>) -> QuotaSample {
    QuotaSample {
        ts,
        used_pct,
        resets_at,
        confidence: Confidence::Measured,
        held_until: None,
    }
}

#[test]
fn a_limit_resets_when_the_window_read_used_up_ends() {
    // PM5: read used up a minute before the session waited, resetting 3 min on.
    let five = [read(T0 - MIN, 100.0, Some(T0 + 3 * MIN))];
    let windows = [(FIVE, &five[..])];
    assert_eq!(
        limit_reset_at(&windows, T0, T0 + 2 * MIN),
        None,
        "before its reset"
    );
    assert_eq!(
        limit_reset_at(&windows, T0, T0 + 4 * MIN),
        Some(T0 + 3 * MIN),
        "past its reset: it reset then"
    );
    // A newer reading of the new window, with room, changes nothing.
    let five = [
        read(T0 - MIN, 100.0, Some(T0 + 3 * MIN)),
        read(T0 + 10 * MIN, 4.0, Some(T0 + 5 * HOUR)),
    ];
    assert_eq!(
        limit_reset_at(&[(FIVE, &five[..])], T0, T0 + 11 * MIN),
        Some(T0 + 3 * MIN)
    );
}

#[test]
fn out_on_another_window_or_again_it_still_holds() {
    let five = [read(T0 - MIN, 100.0, Some(T0 + 3 * MIN))];
    let week = [read(T0 - MIN, 100.0, Some(T0 + 2 * 24 * HOUR))];
    assert_eq!(
        limit_reset_at(&[(FIVE, &five[..]), (WEEK, &week[..])], T0, T0 + 4 * MIN),
        None,
        "the week is still used up"
    );
    // Used up again in the new window.
    let five = [
        read(T0 - MIN, 100.0, Some(T0 + 3 * MIN)),
        read(T0 + 2 * HOUR, 100.0, Some(T0 + 5 * HOUR)),
    ];
    assert_eq!(
        limit_reset_at(&[(FIVE, &five[..])], T0, T0 + 3 * HOUR),
        None
    );
}

#[test]
fn a_limit_no_reading_saw_never_reads_as_reset() {
    // Nothing read: unknown, however long ago (D-06).
    assert_eq!(
        limit_reset_at(&[(FIVE, &[][..])], T0, T0 + 30 * 24 * HOUR),
        None
    );
    // Read with room only: the limit was on something no reading covers.
    let five = [read(T0 - MIN, 60.0, Some(T0 + 3 * MIN))];
    assert_eq!(limit_reset_at(&[(FIVE, &five[..])], T0, T0 + 4 * MIN), None);
    // A used-up window that ended before the limit was reached says
    // nothing about it.
    let five = [read(T0 - 6 * HOUR, 100.0, Some(T0 - HOUR))];
    assert_eq!(limit_reset_at(&[(FIVE, &five[..])], T0, T0 + 4 * MIN), None);
    // An estimate is no reading.
    let mut guess = read(T0 - MIN, 100.0, Some(T0 + 3 * MIN));
    guess.confidence = Confidence::Estimated;
    assert_eq!(
        limit_reset_at(&[(FIVE, &[guess][..])], T0, T0 + 4 * MIN),
        None
    );
}

#[test]
fn a_reading_with_no_reset_ends_a_window_length_after_it_was_taken() {
    let five = [read(T0 - MIN, 100.0, None)];
    let windows = [(FIVE, &five[..])];
    assert_eq!(limit_reset_at(&windows, T0, T0 + 4 * HOUR), None);
    assert_eq!(
        limit_reset_at(&windows, T0, T0 + 5 * HOUR),
        Some(T0 - MIN + 5 * HOUR)
    );
}
