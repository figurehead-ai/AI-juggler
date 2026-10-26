//! Which vendor reading describes a window now (QA, 2026-10-21: 30 days
//! of the real Codex account's `rate_limits`, 74,874 readings). Shapes
//! from that analysis; every value is invented.
//!
//! - The newest line is not the newest value: a concurrent session reports
//!   a point lower, and a session resumed days later reports its OLD window.
//! - `resets_at` jitters by up to 75 s inside one window.
//! - After `resets_at` passes with no newer reading, nothing is known about
//!   the new window (D-06).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{
    QuotaCap, QuotaSample, QuotaState, WindowKind, current_reading, evaluate_window,
};

const NOW: i64 = 1_792_000_000_000;
const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;

fn reading(ts: i64, used_pct: f64, resets_at: i64) -> QuotaSample {
    QuotaSample {
        ts,
        used_pct,
        resets_at: Some(resets_at),
        confidence: Confidence::Measured,
    }
}

fn weekly(samples: &[QuotaSample]) -> juggler_core::quota::WindowReading {
    evaluate_window(
        &[],
        samples,
        "acc",
        WindowKind::Weekly { anchor_ms: None },
        QuotaCap::default(),
        NOW,
    )
}

/// Rule 3: inside one window, the highest reading — a concurrent
/// session's later line, a point lower, doesn't take it back.
#[test]
fn a_concurrent_sessions_lower_reading_doesnt_lower_the_window() {
    let reset = NOW + 3 * DAY;
    let samples = [
        reading(NOW - 3 * MIN, 50.0, reset),
        reading(NOW - 2 * MIN, 51.0, reset),
        reading(NOW - MIN, 50.0, reset), // another thread, behind by a point
    ];
    let chosen = current_reading(&samples).unwrap();
    assert_eq!(chosen.used_pct, 51.0);
    let window = weekly(&samples);
    assert_eq!(window.percentage, Some(51.0));
    assert_eq!(
        window.measured_at,
        Some(NOW - 2 * MIN),
        "its as-of is that reading's"
    );
}

/// Rule 3: a session resumed days later reports its old window (seen up
/// to 166 h later). The window with the latest reset wins, whatever line
/// came last: "0%, resets 10-27" over "88%, resets 10-24".
#[test]
fn a_resumed_sessions_old_window_doesnt_replace_the_current_one() {
    let new_reset = NOW + 6 * DAY;
    let old_reset = new_reset - 3 * DAY;
    let samples = [
        reading(NOW - 30 * MIN, 0.0, new_reset),
        reading(NOW - 20 * MIN, 88.0, old_reset), // the resumed thread
        reading(NOW - 10 * MIN, 1.0, new_reset),
        reading(NOW - 5 * MIN, 85.0, old_reset),
    ];
    let window = weekly(&samples);
    assert_eq!(window.state, QuotaState::Tracking);
    assert_eq!(window.percentage, Some(1.0));
    assert_eq!(window.resets_at, Some(new_reset));
}

/// Rule 4: `resets_at` moving by seconds is the same window, not a new one.
#[test]
fn a_jittering_reset_is_one_window() {
    let reset = NOW + 2 * DAY;
    // The newest line names the latest reset (30 s later) and is a point
    // lower: grouped as one window, the higher reading stands.
    let samples = [
        reading(NOW - 3 * MIN, 41.0, reset),
        reading(NOW - 2 * MIN, 40.0, reset + 30_000),
    ];
    let window = weekly(&samples);
    assert_eq!(
        window.percentage,
        Some(41.0),
        "a jittered reset is the same window"
    );
    assert_eq!(window.measured_at, Some(NOW - 3 * MIN));
}

/// A reset an hour or more later is a new window: its reading stands,
/// even when lower.
#[test]
fn a_reset_an_hour_later_is_a_new_window() {
    let reset = NOW + 2 * DAY;
    let samples = [
        reading(NOW - 3 * MIN, 41.0, reset),
        reading(NOW - 2 * MIN, 2.0, reset + HOUR),
    ];
    let window = weekly(&samples);
    assert_eq!(window.percentage, Some(2.0));
    assert_eq!(window.resets_at, Some(reset + HOUR));
}

/// Rule 6: the reading's window reset and nothing newer came: no
/// percentage (never 0%, never the old one) and no reset time.
#[test]
fn after_the_reset_with_no_new_reading_nothing_is_known() {
    let samples = [reading(NOW - 2 * DAY, 64.0, NOW - HOUR)];
    let window = weekly(&samples);
    assert_eq!(window.state, QuotaState::ResetSinceReading);
    assert_eq!(window.percentage, None);
    assert_eq!(window.resets_at, None);
    assert_eq!(window.measured_at, Some(NOW - 2 * DAY));

    // A spent window, too: once its reset passes it isn't "exhausted".
    let spent = [reading(NOW - 2 * DAY, 100.0, NOW - HOUR)];
    assert_eq!(weekly(&spent).state, QuotaState::ResetSinceReading);

    // A reading of the new window brings it back.
    let fresh = [
        reading(NOW - 2 * DAY, 64.0, NOW - HOUR),
        reading(NOW - MIN, 2.0, NOW + 7 * DAY - HOUR),
    ];
    let window = weekly(&fresh);
    assert_eq!(window.state, QuotaState::Tracking);
    assert_eq!(window.percentage, Some(2.0));
}

/// Readings without a reset time fall back to the newest.
#[test]
fn readings_without_a_reset_time_take_the_newest() {
    let samples = [
        QuotaSample {
            ts: NOW - 2 * MIN,
            used_pct: 30.0,
            resets_at: None,
            confidence: Confidence::Measured,
        },
        QuotaSample {
            ts: NOW - MIN,
            used_pct: 29.0,
            resets_at: None,
            confidence: Confidence::Measured,
        },
    ];
    assert_eq!(current_reading(&samples).unwrap().used_pct, 29.0);
}

// ---------------------------------------------------------------------
// The maintainer's third report: Claude's own records (the Claude app's
// meter, with no reset time; a limit hit, used up until its reset; the
// last `/usage`). The newest source wins; an old reading is history.

fn meter(ts: i64, used_pct: f64) -> QuotaSample {
    QuotaSample {
        ts,
        used_pct,
        resets_at: None,
        confidence: Confidence::Measured,
    }
}

fn five_hour(samples: &[QuotaSample]) -> juggler_core::quota::WindowReading {
    evaluate_window(
        &[],
        samples,
        "acc",
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    )
}

#[test]
fn a_meter_reading_inside_a_known_window_keeps_its_reset() {
    let reset = NOW + 2 * HOUR;
    // A limit hit named the reset; the app's meter read later.
    let samples = [
        reading(NOW - 2 * HOUR, 100.0, reset),
        meter(NOW - 30 * MIN, 100.0),
    ];
    let window = five_hour(&samples);
    assert_eq!(window.state, QuotaState::Exhausted);
    assert_eq!(window.resets_at, Some(reset), "the hit's reset stays");
    assert_eq!(
        window.measured_at,
        Some(NOW - 30 * MIN),
        "the newest reading's age"
    );

    // A lower meter reading in the same window: use only rises.
    let samples = [
        reading(NOW - 2 * HOUR, 60.0, reset),
        meter(NOW - 30 * MIN, 58.0),
    ];
    let window = five_hour(&samples);
    assert_eq!(window.percentage, Some(60.0));
    assert_eq!(window.resets_at, Some(reset));
}

#[test]
fn a_meter_reading_after_the_known_reset_is_the_new_window_on_its_own() {
    // A hit three days ago, long reset; the meter read 20 min ago.
    let old_reset = NOW - 3 * DAY + 3 * HOUR;
    let samples = [
        reading(NOW - 3 * DAY, 100.0, old_reset),
        meter(NOW - 20 * MIN, 12.0),
    ];
    let window = five_hour(&samples);
    assert_eq!(window.state, QuotaState::Stale);
    assert_eq!(window.percentage, Some(12.0));
    assert_eq!(
        window.resets_at, None,
        "no reset is known for the new window"
    );
}

#[test]
fn an_older_meter_reading_never_beats_a_newer_one_with_a_reset() {
    let reset = NOW + 4 * HOUR;
    let samples = [
        meter(NOW - 40 * MIN, 30.0),
        reading(NOW - 5 * MIN, 35.0, reset),
    ];
    let window = five_hour(&samples);
    assert_eq!(window.percentage, Some(35.0));
    assert_eq!(window.measured_at, Some(NOW - 5 * MIN));
}

#[test]
fn a_five_hour_reading_older_than_five_hours_is_history() {
    let window = five_hour(&[meter(NOW - 13 * HOUR, 40.0)]);
    assert_eq!(window.state, QuotaState::ResetSinceReading);
    assert_eq!(window.percentage, None, "never the state now");
    assert_eq!(window.last_used_pct, Some(40.0), "kept as history");
    assert_eq!(window.measured_at, Some(NOW - 13 * HOUR));
    // A used-up reading too: it isn't "used up" any more.
    assert_eq!(
        five_hour(&[meter(NOW - 6 * HOUR, 100.0)]).state,
        QuotaState::ResetSinceReading
    );
    // Under five hours it stands, with its age.
    let window = five_hour(&[meter(NOW - 55 * MIN, 40.0)]);
    assert_eq!(window.state, QuotaState::Stale);
    assert_eq!(window.percentage, Some(40.0));
}

#[test]
fn a_weekly_reading_stands_until_its_week_could_have_ended() {
    let window = weekly(&[meter(NOW - 3 * DAY, 61.0)]);
    assert_eq!(window.percentage, Some(61.0));
    let window = weekly(&[meter(NOW - 8 * DAY, 61.0)]);
    assert_eq!(window.state, QuotaState::ResetSinceReading);
    assert_eq!(window.last_used_pct, Some(61.0));
}

#[test]
fn a_falling_pair_of_meter_readings_never_interpolates_downwards() {
    use juggler_core::model::TokenCounts;
    use juggler_core::quota::{UsageEvent, UsageSource};
    let event = |ts: i64| UsageEvent {
        account: "acc".into(),
        session: Some("s".into()),
        ts,
        model: None,
        tokens: TokenCounts {
            input: 10,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: None,
    };
    // 80% then 5% (a reset between them), then more use.
    let events = [
        event(NOW - 50 * MIN),
        event(NOW - 10 * MIN),
        event(NOW - 5 * MIN),
    ];
    let samples = [meter(NOW - 60 * MIN, 80.0), meter(NOW - 20 * MIN, 5.0)];
    let window = evaluate_window(
        &events,
        &samples,
        "acc",
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    );
    assert_eq!(window.percentage, Some(5.0), "the vendor number stands");
}
