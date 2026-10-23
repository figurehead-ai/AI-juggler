//! AJ-E1513: the alert machine re-arms when its window resets, and only
//! then. Before this, a machine that warned once stayed Warned forever:
//! nothing in production sent the reset, so the next window never warned.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::expect_used)]

use juggler_core::alerts::{AlertEmission, AlertFsm, AlertPhase, WindowObservation, rearmed, step};
use juggler_core::model::Confidence;

const HOUR: i64 = 60 * 60 * 1000;
const T0: i64 = 1_792_094_400_000;

fn obs(tte: Option<f64>, pct: Option<f64>, resets_at_ms: Option<i64>) -> WindowObservation {
    WindowObservation {
        tte_min: tte,
        used_pct: pct,
        confidence: Confidence::Measured,
        resets_at_ms,
    }
}

/// Steps the machine through `ticks`, returning the emissions in order.
fn run(fsm: &mut AlertFsm, ticks: &[(i64, WindowObservation)]) -> Vec<AlertEmission> {
    ticks
        .iter()
        .map(|(now, observation)| {
            let (next, emission) = step(fsm, *observation, *now);
            *fsm = next;
            emission
        })
        .collect()
}

fn kinds(emissions: &[AlertEmission]) -> Vec<&'static str> {
    emissions
        .iter()
        .filter_map(|emission| match emission {
            AlertEmission::None => None,
            AlertEmission::TrayAmber { .. } => Some("warn"),
            AlertEmission::Actionable { .. } => Some("critical"),
            AlertEmission::ExhaustedNotification => Some("exhausted"),
            AlertEmission::ResetReached => Some("reset"),
        })
        .collect()
}

#[test]
fn warned_then_critical_in_one_window_never_warns_twice() {
    let resets = Some(T0 + 4 * HOUR);
    let mut fsm = AlertFsm::default();
    let emitted = run(
        &mut fsm,
        &[
            (T0, obs(Some(40.0), Some(70.0), resets)),
            (T0 + HOUR / 4, obs(Some(38.0), Some(75.0), resets)),
            (T0 + HOUR / 2, obs(Some(12.0), Some(91.0), resets)),
            // Back above the critical line, still under the warning one.
            (T0 + HOUR, obs(Some(40.0), Some(80.0), resets)),
            (T0 + 2 * HOUR, obs(Some(10.0), Some(95.0), resets)),
        ],
    );
    assert_eq!(kinds(&emitted), ["warn", "critical"]);
    assert_eq!(fsm.phase, AlertPhase::Critical);
    assert_eq!(fsm.window_resets_at_ms, resets);
}

#[test]
fn the_next_window_warns_again_from_armed() {
    let first = T0 + 4 * HOUR;
    let second = first + 5 * HOUR;
    let mut fsm = AlertFsm::default();
    let emitted = run(
        &mut fsm,
        &[
            (T0, obs(Some(40.0), Some(70.0), Some(first))),
            (T0 + HOUR, obs(Some(10.0), Some(92.0), Some(first))),
            // The window reset: a quiet first reading re-arms silently.
            (first + HOUR, obs(Some(300.0), Some(5.0), Some(second))),
        ],
    );
    assert_eq!(kinds(&emitted), ["warn", "critical"]);
    assert_eq!(fsm, AlertFsm::default(), "re-armed, nothing recorded yet");

    let emitted = run(
        &mut fsm,
        &[
            (first + 3 * HOUR, obs(Some(40.0), Some(70.0), Some(second))),
            (first + 4 * HOUR, obs(Some(10.0), Some(92.0), Some(second))),
        ],
    );
    assert_eq!(kinds(&emitted), ["warn", "critical"], "warned again, once");
    assert_eq!(fsm.window_resets_at_ms, Some(second));
}

#[test]
fn a_later_window_reported_before_the_old_reset_re_arms_but_jitter_does_not() {
    let first = T0 + 4 * HOUR;
    let mut fsm = AlertFsm::default();
    let _ = run(&mut fsm, &[(T0, obs(Some(40.0), Some(70.0), Some(first)))]);
    // Readings of the same window move by seconds: no re-arm.
    let jittered = rearmed(&fsm, Some(first + 90_000), T0 + HOUR);
    assert_eq!(jittered.phase, AlertPhase::Warned);
    // The vendor already reports the next window (the clocks disagree
    // by a little): that is a reset.
    let next = rearmed(&fsm, Some(first + 5 * HOUR), first - 60_000);
    assert_eq!(next, AlertFsm::default());
}

#[test]
fn a_window_with_no_reset_instant_never_re_arms_on_a_guess() {
    let mut fsm = AlertFsm::default();
    let emitted = run(
        &mut fsm,
        &[
            (T0, obs(Some(40.0), None, None)),
            // A day later, a week later: still the same machine.
            (T0 + 24 * HOUR, obs(Some(300.0), None, None)),
            (T0 + 24 * HOUR + 1, obs(Some(40.0), None, None)),
            (T0 + 7 * 24 * HOUR, obs(Some(40.0), None, None)),
        ],
    );
    assert_eq!(kinds(&emitted), ["warn"], "one warning, never a second");
    assert_eq!(fsm.phase, AlertPhase::Warned);
    assert_eq!(fsm.window_resets_at_ms, None);
}

#[test]
fn a_machine_saved_before_the_reset_instant_existed_still_loads() {
    let fsm: AlertFsm =
        serde_json::from_str(r#"{"phase":"Warned","snoozed_until_ms":null}"#).expect("old blob");
    assert_eq!(fsm.phase, AlertPhase::Warned);
    assert_eq!(fsm.window_resets_at_ms, None);
    // It learns the window's reset from the next reading, then re-arms
    // after it.
    let (fsm, _) = step(&fsm, obs(Some(40.0), None, Some(T0 + HOUR)), T0);
    assert_eq!(fsm.window_resets_at_ms, Some(T0 + HOUR));
    let (fsm, emission) = step(
        &fsm,
        obs(Some(40.0), None, Some(T0 + 6 * HOUR)),
        T0 + 2 * HOUR,
    );
    assert!(matches!(emission, AlertEmission::TrayAmber { .. }));
    assert_eq!(fsm.window_resets_at_ms, Some(T0 + 6 * HOUR));
}
