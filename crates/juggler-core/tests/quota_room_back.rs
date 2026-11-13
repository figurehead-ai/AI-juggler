//! 0.4.4: when an Account out of room has room again — the latest reset
//! among its used-up windows, the one answer every surface gives (a
//! persona lab's Account, out on both windows, read "resets 7:06 PM" on
//! its tiles and "resets Sun 5:06 PM" in Needs you). Every value is
//! invented.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step is the failure it reports"
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{
    QuotaCap, QuotaSample, RoomBack, WindowKind, WindowRoom, evaluate_window, room_back, used_up,
};

const NOW: i64 = 1_792_000_000_000;
const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;
const WEEK: WindowKind = WindowKind::Weekly { anchor_ms: None };

fn room(used_up: bool, resets_at: Option<i64>, ends_by: Option<i64>) -> WindowRoom {
    WindowRoom {
        used_up,
        resets_at,
        ends_by,
    }
}

/// The window's room as the engine reads one Measured reading of it,
/// taken a minute ago.
fn read(kind: WindowKind, used_pct: f64, resets_at: Option<i64>) -> WindowRoom {
    let sample = QuotaSample {
        ts: NOW - 60_000,
        used_pct,
        resets_at,
        confidence: Confidence::Measured,
        held_until: None,
    };
    evaluate_window(&[], &[sample], "acct", kind, QuotaCap::default(), NOW).room(kind)
}

#[test]
fn out_on_both_windows_it_has_room_again_when_the_later_resets() {
    // The lab's shape: both used up, the five hours resetting in two
    // hours, the week in three days.
    let both = [
        read(WindowKind::FiveHour, 100.0, Some(NOW + 2 * HOUR)),
        read(WEEK, 100.0, Some(NOW + 3 * DAY)),
    ];
    assert_eq!(
        room_back(&both),
        Some(RoomBack {
            window: 1,
            at: Some(NOW + 3 * DAY)
        })
    );
    // The order they're given in doesn't matter.
    let flipped = [both[1], both[0]];
    assert_eq!(room_back(&flipped).unwrap().at, Some(NOW + 3 * DAY));
    // A reading that rounds to 0% left is used up too ("0% left").
    assert!(used_up(99.6) && !used_up(99.4));
    let rounded = [
        read(WindowKind::FiveHour, 100.0, Some(NOW + 2 * HOUR)),
        read(WEEK, 99.6, Some(NOW + 3 * DAY)),
    ];
    assert_eq!(room_back(&rounded).unwrap().window, 1);
}

#[test]
fn out_on_one_window_it_is_that_windows_reset() {
    let five_only = [
        read(WindowKind::FiveHour, 100.0, Some(NOW + 2 * HOUR)),
        read(WEEK, 90.0, Some(NOW + 3 * DAY)),
    ];
    assert_eq!(
        room_back(&five_only),
        Some(RoomBack {
            window: 0,
            at: Some(NOW + 2 * HOUR)
        })
    );
    // With room on every window, it isn't out.
    let room_left = [
        read(WindowKind::FiveHour, 40.0, Some(NOW + 2 * HOUR)),
        read(WEEK, 90.0, Some(NOW + 3 * DAY)),
    ];
    assert_eq!(room_back(&room_left), None);
    assert_eq!(room_back(&[]), None);
}

#[test]
fn not_known_only_when_a_needed_reset_truly_isnt() {
    // A used-up week whose reset isn't known: when it has room again isn't.
    let week_unknown = [
        read(WindowKind::FiveHour, 100.0, Some(NOW + 2 * HOUR)),
        read(WEEK, 100.0, None),
    ];
    assert_eq!(
        room_back(&week_unknown),
        Some(RoomBack {
            window: 1,
            at: None
        })
    );
    // A used-up five hours whose reset isn't known ends within five hours
    // of its reading: the week's reset, three days away, is later anyway.
    let five_unknown = [
        read(WindowKind::FiveHour, 100.0, None),
        read(WEEK, 100.0, Some(NOW + 3 * DAY)),
    ];
    assert_eq!(
        room_back(&five_unknown),
        Some(RoomBack {
            window: 1,
            at: Some(NOW + 3 * DAY)
        })
    );
    // But not when the known reset could come first.
    let soon = [
        room(true, None, Some(NOW + 4 * HOUR)),
        room(true, Some(NOW + HOUR), None),
    ];
    assert_eq!(room_back(&soon).unwrap().at, None);
    // A used-up window never read (no reset, no reading time): not known.
    assert_eq!(room_back(&[room(true, None, None)]).unwrap().at, None);
}
