//! AJ-E608 forecast fixture timelines (deep/09 §Q4, deep/10 §1
//! acceptance): synthetic constant/accelerating/stalled burn with
//! KNOWN expected TTEs — the oracle is the fixture generator's
//! arithmetic (rates chosen so the expected TTE is exact), independent
//! of the implementation. Cold start asserts NO numeric output;
//! confidence inheritance and the reset-bounded TTE law are pinned in
//! types and tests.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::time::Instant;

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{QuotaCap, UsageEvent, UsageSource, WindowKind};
use juggler_core::quota_forecast::{
    self, AdmissionVerdict, CalibrationNote, ForecastState, RateUnits, admission_verdict,
    burn_rate, calibration_note, forecast, median_session_burn,
};

const MIN: i64 = 60_000;
const NOW: i64 = 1_792_094_400_000; // fixed clock

fn event(ts: i64, tokens: u64, confidence: Confidence) -> UsageEvent {
    UsageEvent {
        account: "acc".into(),
        session: Some("sess".into()),
        ts,
        model: Some("m".into()),
        tokens: TokenCounts {
            input: tokens,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence,
        msg_uuid: Some(format!("e-{ts}-{tokens}")),
    }
}

/// Constant burn: `per_min` tokens every minute for `minutes`.
fn constant(minutes: i64, per_min: u64) -> Vec<UsageEvent> {
    (0..minutes)
        .map(|back| event(NOW - back * MIN, per_min, Confidence::Measured))
        .collect()
}

/// Accelerating burn (deep/10 §1 acceptance fixture): the rate doubles
/// every 15 minutes from `start_per_min` — a ramp the EWMA is expected
/// to track within MAPE ≤30% after 20 minutes of burn.
fn accelerating(minutes: i64, start_per_min: u64) -> Vec<UsageEvent> {
    (0..minutes)
        .map(|back| {
            let minute_index = minutes - back; // 1..=minutes, oldest first
            let doublings = (minute_index - 1) / 15;
            let per_min = start_per_min * (1_u64 << doublings.min(10));
            event(NOW - back * MIN, per_min, Confidence::Measured)
        })
        .collect()
}

#[test]
fn cold_start_returns_learning_and_no_number() {
    // 9 minutes of data: one short of the 10-minute cold-start rule.
    let events = constant(9, 100);
    let rate = burn_rate(&events, "acc", RateUnits::Tokens, NOW);
    assert!(
        rate.rate15.is_none(),
        "no number before 10 min, at any confidence"
    );
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(10_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 3 * 60 * MIN),
        NOW,
    );
    assert_eq!(fc.state, ForecastState::Learning);
}

#[test]
fn constant_burn_reproduces_known_tte_within_10pct() {
    // 60 min of 100 tokens/min = 6_000 burned; cap 12_000 ⇒ headroom
    // 6_000 ⇒ expected TTE = 60 min. Reset in 4 h.
    let events = constant(60, 100);
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(12_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 4 * 60 * MIN),
        NOW,
    );
    let ForecastState::Exhausts {
        tte_min,
        range_low_min,
        range_high_min,
    } = fc.state
    else {
        panic!("constant burn must yield Exhausts, got {:?}", fc.state);
    };
    assert!(
        (tte_min - 60.0).abs() <= 6.0,
        "TTE within ±10% of the oracle (60 min): {tte_min}"
    );
    assert!(
        range_low_min <= tte_min && tte_min <= range_high_min,
        "cone brackets the point"
    );
    assert_eq!(fc.confidence, Confidence::Measured);
}

#[test]
fn accelerating_burn_mape_within_30pct_after_20_min() {
    // 45 min ramp from 40/min doubling every 15 (40→80→160). Burned =
    // 15·40 + 15·80 + 15·160 = 4_200. Cap 8_400 ⇒ headroom 4_200.
    // Real TTE from now (integral of the future ramp): the ramp
    // continues 160/min for 15 min (2_400), then 320/min …
    // 4_200 - 2_400 = 1_800 at 320/min ⇒ 5.6 min ⇒ actual ≈ 20.6 min.
    let events = accelerating(45, 40);
    let cap = 8_400_u64;
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(cap),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 4 * 60 * MIN),
        NOW,
    );
    let ForecastState::Exhausts { tte_min, .. } = fc.state else {
        panic!(
            "accelerating fixture must yield Exhausts, got {:?}",
            fc.state
        );
    };
    let actual = 20.6_f64;
    let mape = ((tte_min - actual) / actual * 100.0).abs();
    assert!(
        mape <= 30.0,
        "MAPE ≤30% on the accelerating fixture: forecast {tte_min} vs actual {actual} ({mape:.1}%)"
    );
}

#[test]
fn stalled_burn_is_wont_exhaust_never_a_fake_number() {
    // 60 min of data SPAN, but all burn in the oldest 10 minutes; the
    // last 50 minutes are idle ⇒ the EWMA decays toward zero ⇒ TTE is
    // effectively infinite ⇒ WontExhaust (no "stalled" pill exists —
    // deep/10's four states; a decayed rate is an honest wont-exhaust).
    let events: Vec<UsageEvent> = (50..60)
        .map(|back| event(NOW - back * MIN, 500, Confidence::Measured))
        .collect();
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(1_000_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 60 * MIN),
        NOW,
    );
    assert_eq!(fc.state, ForecastState::WontExhaust);
}

#[test]
fn tte_beyond_the_reset_is_wont_exhaust_never_emitted() {
    // Slow burn, tiny headroom would exhaust in ~300 min — but the
    // reset is 120 min away: the TTE must NOT be emitted.
    let events = constant(60, 100); // headroom 6_000 ⇒ TTE 60? — shrink cap instead:
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(6_060),
            credits: None,
        }, // headroom 60 ⇒ TTE 0.6 min? no: 60/100 = 0.6
        RateUnits::Tokens,
        Some(NOW + 120 * MIN),
        NOW,
    );
    assert!(
        matches!(fc.state, ForecastState::Exhausts { .. }),
        "TTE inside the window is emitted: {:?}",
        fc.state
    );
    // Unknown reset: never an unbounded promise.
    let unknown = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(6_060),
            credits: None,
        },
        RateUnits::Tokens,
        None,
        NOW,
    );
    assert_eq!(
        unknown.state,
        ForecastState::WontExhaust,
        "unknown reset ⇒ no TTE promise"
    );
    // Far reset + big cap ⇒ TTE beyond remaining ⇒ WontExhaust.
    let big = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(1_000_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 30 * MIN),
        NOW,
    );
    assert_eq!(
        big.state,
        ForecastState::WontExhaust,
        "TTE ≥ remaining ⇒ WontExhaust"
    );
}

#[test]
fn one_estimated_input_confidence_inherits_lowest() {
    // Mostly Measured, ONE Estimated event ⇒ the forecast's confidence
    // is Estimated (deep/09 §Q2 Q-2) — and the pill will render the
    // RANGE (AJ-E610), pinned here at the type level: an Estimated
    // forecast with a point-only shape cannot exist (the state always
    // carries the range).
    // Varying rates so the 5↔60 cone genuinely spreads: first half
    // 50/min, second half 150/min.
    let mut events: Vec<UsageEvent> = (0..60)
        .map(|back| {
            let per_min = if back < 30 { 150 } else { 50 };
            event(NOW - back * MIN, per_min, Confidence::Measured)
        })
        .collect();
    events[7] = event(events[7].ts, events[7].tokens.input, Confidence::Estimated);
    let rate = burn_rate(&events, "acc", RateUnits::Tokens, NOW);
    assert_eq!(rate.confidence, Confidence::Estimated);
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(12_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 4 * 60 * MIN),
        NOW,
    );
    assert_eq!(fc.confidence, Confidence::Estimated);
    assert!(matches!(fc.state, ForecastState::Exhausts { .. }));
    let ForecastState::Exhausts {
        range_low_min,
        range_high_min,
        ..
    } = fc.state
    else {
        unreachable!()
    };
    assert!(
        range_high_min > range_low_min,
        "Estimated renders a range, never a bare point"
    );
}

#[test]
fn no_cap_means_no_headroom_forecast() {
    let events = constant(60, 100);
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap::default(),
        RateUnits::Tokens,
        Some(NOW + 4 * 60 * MIN),
        NOW,
    );
    assert_eq!(
        fc.state,
        ForecastState::NoHeadroom,
        "no user cap ⇒ no invented denominator"
    );
}

#[test]
fn zcode_forecasts_run_on_credits() {
    // The shared cone maths on the credits unit (deep/09 handover
    // note): 30 min of 10 credits/min, cap 600 ⇒ headroom 300 ⇒ TTE 30.
    let events: Vec<UsageEvent> = (0..30)
        .map(|back| {
            let mut event = event(NOW - back * MIN, 0, Confidence::Computed);
            event.credits = Some(10.0);
            event
        })
        .collect();
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: None,
            credits: Some(600.0),
        },
        RateUnits::Credits,
        Some(NOW + 4 * 60 * MIN),
        NOW,
    );
    let ForecastState::Exhausts { tte_min, .. } = fc.state else {
        panic!("credits forecast must yield Exhausts, got {:?}", fc.state);
    };
    assert!(
        (tte_min - 30.0).abs() <= 3.0,
        "credits TTE ≈ 30 min: {tte_min}"
    );
}

#[test]
fn admission_gate_verdicts_and_never_blocks() {
    assert_eq!(
        admission_verdict(Some(100), Some(1_000.0)),
        Some(AdmissionVerdict::Ok)
    );
    assert_eq!(
        admission_verdict(Some(600), Some(1_000.0)),
        Some(AdmissionVerdict::Tight)
    );
    assert_eq!(
        admission_verdict(Some(1_001), Some(1_000.0)),
        Some(AdmissionVerdict::No)
    );
    // Unknown inputs ⇒ None (never a guessed median or headroom).
    assert_eq!(admission_verdict(None, Some(1_000.0)), None);
    assert_eq!(admission_verdict(Some(100), None), None);
}

#[test]
fn median_session_burn_ignores_empty_sessions() {
    let mut events: Vec<UsageEvent> = Vec::new();
    for (session, tokens, count) in [("s1", 100_u64, 3_usize), ("s2", 500, 1), ("s3", 0, 1)] {
        for index in 0..count {
            let mut event = event(NOW - index as i64 * MIN, tokens, Confidence::Measured);
            event.session = Some(session.into());
            events.push(event);
        }
    }
    // Totals: s1=300, s2=500, s3 excluded (zero) ⇒ median of [300,500] = 400.
    assert_eq!(
        median_session_burn(&events, &["s1", "s2", "s3", "s4"]),
        Some(400),
        "even count averages the middle two; empty sessions mediate nothing"
    );
}

#[test]
fn calibration_notes_trip_only_beyond_2x() {
    assert!(
        calibration_note("acc", 30.0, 75.0, NOW).is_some(),
        "2.5× trips"
    );
    let CalibrationNote { ratio, .. } = calibration_note("acc", 30.0, 75.0, NOW).unwrap();
    assert!((ratio - 2.5).abs() < 1e-9);
    assert!(
        calibration_note("acc", 30.0, 12.0, NOW).is_some(),
        "0.4× trips (forecast 2.5× late)"
    );
    assert!(
        calibration_note("acc", 30.0, 45.0, NOW).is_none(),
        "1.5× holds"
    );
    assert!(
        calibration_note("acc", 0.0, 45.0, NOW).is_none(),
        "degenerate forecast: no note"
    );
}

/// The performance budget: 30 days of events evaluates ≤500 ms (the
/// per-account recompute budget is ≤2 ms on ingest batches; this
/// end-to-end bound is the CI-safe ceiling).
#[test]
fn thirty_day_fixture_timeline_evaluates_fast() {
    let events: Vec<UsageEvent> =
        (0..30 * 24 * 6) // every 10 minutes for 30 days
            .map(|index| event(NOW - index * 10 * MIN, 250, Confidence::Measured))
            .collect();
    let started = Instant::now();
    let fc = forecast(
        &events,
        "acc",
        WindowKind::FiveHour,
        QuotaCap {
            tokens: Some(1_000_000),
            credits: None,
        },
        RateUnits::Tokens,
        Some(NOW + 60 * MIN),
        NOW,
    );
    let elapsed = started.elapsed();
    assert!(
        matches!(
            fc.state,
            ForecastState::Learning | ForecastState::Exhausts { .. } | ForecastState::WontExhaust
        ),
        "a valid typed state: {:?}",
        fc.state
    );
    println!("[perf] 30-day forecast: {elapsed:?} (budget ≤500 ms)");
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() <= 500,
        "30-day evaluation exceeded 500 ms: {elapsed:?}"
    );
}

/// The headline/cone shape is exported for AJ-E610's pill states.
#[test]
fn rate_cone_shape_is_stable() {
    let events = constant(60, 100);
    let rate = burn_rate(&events, "acc", RateUnits::Tokens, NOW);
    assert_eq!(rate.units, RateUnits::Tokens);
    let _ = quota_forecast::COLD_START_MIN_MS;
    assert!(rate.rate15.is_some_and(|value| (value - 100.0).abs() < 5.0));
}
