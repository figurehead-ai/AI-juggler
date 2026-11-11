//! AJ-E1512: a window's runway from fixture histories — sessions left
//! from the account's own typical session, the burn-down's pace and
//! exhaustion against the reset, and the confidence carried through.

#![allow(clippy::unwrap_used, clippy::pedantic)]

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{QuotaSample, UsageEvent, UsageSource};
use juggler_core::quota_runway::runway;

const NOW: i64 = 1_792_094_400_000;
const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const FIVE_HOURS: i64 = 5 * HOUR;

fn sample(ago_min: i64, used: f64, resets_at: i64) -> QuotaSample {
    QuotaSample {
        ts: NOW - ago_min * MIN,
        used_pct: used,
        resets_at: Some(resets_at),
        confidence: Confidence::Measured,
        held_until: None,
    }
}

fn event(ts: i64, session: &str, tokens: u64, confidence: Confidence) -> UsageEvent {
    UsageEvent {
        account: "acc".into(),
        session: Some(session.into()),
        ts,
        model: None,
        tokens: TokenCounts {
            input: tokens,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence,
        detail: juggler_core::quota::UsageDetail::default(),
        msg_uuid: None,
    }
}

/// Two hours of a window: 0 % to 40 %, a reading every 10 minutes
/// (20 % an hour), resetting at `resets_at`.
fn steady(resets_at: i64) -> Vec<QuotaSample> {
    (0..=12)
        .map(|i| sample(120 - i * 10, f64::from(i as i32) * 40.0 / 12.0, resets_at))
        .collect()
}

/// Four past sessions of 100k tokens each (yesterday), and 400k tokens
/// spent while the window moved 0 → 40 % (so 1 % per 10k tokens: a
/// typical session costs 10 %).
fn history(confidence: Confidence) -> Vec<UsageEvent> {
    let mut events: Vec<UsageEvent> = (0..4)
        .map(|n| {
            event(
                NOW - 24 * HOUR + n * HOUR,
                &format!("past-{n}"),
                100_000,
                confidence,
            )
        })
        .collect();
    events
        .extend((0..8).map(|n| event(NOW - 110 * MIN + n * 13 * MIN, "today", 50_000, confidence)));
    events
}

#[test]
fn sessions_left_come_from_the_accounts_own_typical_session() {
    let resets_at = NOW + 2 * HOUR;
    let runway = runway(
        &steady(resets_at),
        &history(Confidence::Measured),
        FIVE_HOURS,
        NOW,
    )
    .unwrap();
    assert!((runway.used_pct - 40.0).abs() < 1e-9);
    // Five sessions counted (four past, today's); the median is 100k.
    assert_eq!(runway.sessions_counted, 5);
    let pct = runway.session_pct.unwrap();
    assert!((pct - 10.0).abs() < 0.01, "{pct}");
    assert_eq!(runway.sessions_left, Some(6), "60 % left at 10 % a session");
    // Worked out, never above Computed.
    assert_eq!(runway.confidence, Confidence::Computed);
}

#[test]
fn the_burn_down_runs_out_before_the_reset_or_lasts_past_it() {
    // 20 % an hour with 60 % left: out in three hours.
    let late_reset = NOW + 4 * HOUR;
    let out = runway(
        &steady(late_reset),
        &history(Confidence::Measured),
        FIVE_HOURS,
        NOW,
    )
    .unwrap();
    let rate = out.pct_per_hour.unwrap();
    assert!((rate - 20.0).abs() < 0.01, "{rate}");
    let at = out.exhausts_at_ms.unwrap();
    assert!((at - (NOW + 3 * HOUR)).abs() < MIN, "{}", (at - NOW) / MIN);
    assert_eq!(out.resets_at_ms, late_reset);
    assert_eq!(out.starts_ms, late_reset - FIVE_HOURS);
    assert_eq!(out.series.len(), 13);
    assert_eq!(out.series.first().unwrap().used_pct, 0.0);

    // Resets in two hours: it lasts past the reset at this pace.
    let lasts = runway(
        &steady(NOW + 2 * HOUR),
        &history(Confidence::Measured),
        FIVE_HOURS,
        NOW,
    )
    .unwrap();
    assert_eq!(lasts.exhausts_at_ms, None);
    assert!(lasts.pct_per_hour.is_some());
}

#[test]
fn the_confidence_is_the_lowest_of_what_went_in() {
    let runway = runway(
        &steady(NOW + 2 * HOUR),
        &history(Confidence::Estimated),
        FIVE_HOURS,
        NOW,
    )
    .unwrap();
    assert_eq!(runway.confidence, Confidence::Estimated);
}

#[test]
fn too_little_to_go_on_stays_unknown() {
    let resets_at = NOW + 2 * HOUR;
    // Two past sessions only: no typical session yet.
    let few: Vec<UsageEvent> = history(Confidence::Measured)
        .into_iter()
        .filter(|event| {
            event.session.as_deref() != Some("past-0") && event.session.as_deref() != Some("past-1")
        })
        .collect();
    let short = runway(&steady(resets_at), &few[..few.len()], FIVE_HOURS, NOW).unwrap();
    assert_eq!(short.sessions_counted, 3, "past-2, past-3, today");
    // Three is enough; two is not.
    let fewer: Vec<UsageEvent> = few
        .into_iter()
        .filter(|event| event.session.as_deref() != Some("past-2"))
        .collect();
    let none = runway(&steady(resets_at), &fewer, FIVE_HOURS, NOW).unwrap();
    assert_eq!((none.session_pct, none.sessions_left), (None, None));
    assert!(
        !none.series.is_empty(),
        "the burn-down still shows the readings"
    );

    // The window barely moved: no percent per token.
    let flat: Vec<QuotaSample> = (0..=6)
        .map(|i| sample(60 - i * 10, 12.0 + f64::from(i as i32) * 0.1, resets_at))
        .collect();
    let still = runway(&flat, &history(Confidence::Measured), FIVE_HOURS, NOW).unwrap();
    assert_eq!(still.session_pct, None);

    // A single reading: no pace.
    let one = runway(&[sample(1, 30.0, resets_at)], &[], FIVE_HOURS, NOW).unwrap();
    assert_eq!((one.pct_per_hour, one.exhausts_at_ms), (None, None));
}

#[test]
fn no_measured_reading_of_a_live_window_is_no_runway() {
    assert!(runway(&[], &[], FIVE_HOURS, NOW).is_none());
    // The window reset already.
    assert!(runway(&[sample(30, 80.0, NOW - MIN)], &[], FIVE_HOURS, NOW).is_none());
    // Estimated readings don't count.
    let mut guessed = sample(5, 50.0, NOW + HOUR);
    guessed.confidence = Confidence::Estimated;
    assert!(runway(&[guessed], &[], FIVE_HOURS, NOW).is_none());
}

#[test]
fn only_this_windows_readings_are_drawn_and_long_ones_are_thinned() {
    let resets_at = NOW + HOUR;
    let mut samples = vec![sample(400, 95.0, NOW - 90 * MIN)]; // the last window's
    samples.extend((0..500).map(|i| sample(250 - i / 2, f64::from(i as i32) / 10.0, resets_at)));
    let runway = runway(&samples, &[], FIVE_HOURS, NOW).unwrap();
    assert!(runway.series.len() <= 120, "{}", runway.series.len());
    assert!(
        runway.series.iter().all(|point| point.used_pct < 95.0),
        "the old window's reading is left out"
    );
    assert_eq!(
        runway.series.last().unwrap().at_ms,
        samples.iter().map(|s| s.ts).max().unwrap()
    );
}

/// V50's one pace-start rule (`pace_start`), the same for a stream read
/// every few seconds and for the held readings stored for it.
mod pace_start_rule {
    use super::*;
    use juggler_core::quota_runway::pace_start;

    fn every_12s(from_min: i64, to_min: i64, used: f64, resets_at: i64) -> Vec<QuotaSample> {
        let (from, to) = (NOW - from_min * MIN, NOW - to_min * MIN);
        (0..)
            .map(|n| from + n * 12_000)
            .take_while(|ts| *ts <= to)
            .map(|ts| QuotaSample {
                ts,
                used_pct: used,
                resets_at: Some(resets_at),
                confidence: Confidence::Measured,
                held_until: None,
            })
            .collect()
    }

    fn held(from_min: i64, to_min: i64, used: f64, resets_at: i64) -> Vec<QuotaSample> {
        let first = QuotaSample {
            ts: NOW - from_min * MIN,
            used_pct: used,
            resets_at: Some(resets_at),
            confidence: Confidence::Measured,
            held_until: Some(NOW - to_min * MIN),
        };
        vec![
            first,
            QuotaSample {
                ts: NOW - to_min * MIN,
                held_until: None,
                ..first
            },
        ]
    }

    #[test]
    fn a_reading_held_through_the_hours_start_starts_the_pace_there() {
        let reset = NOW + 3 * HOUR;
        for stream in [
            [
                every_12s(120, 30, 20.0, reset),
                every_12s(29, 0, 35.0, reset),
            ]
            .concat(),
            [held(120, 30, 20.0, reset), held(29, 0, 35.0, reset)].concat(),
        ] {
            let window: Vec<&QuotaSample> = stream.iter().collect();
            let start = pace_start(&window, NOW - HOUR).unwrap();
            assert_eq!((start.ts, start.used_pct), (NOW - HOUR, 20.0));
            let run = runway(&stream, &[], FIVE_HOURS, NOW).unwrap();
            // 15 points in the hour.
            assert_eq!(run.pct_per_hour, Some(15.0));
        }
    }

    #[test]
    fn a_held_reading_that_is_the_latest_paces_at_zero_not_none() {
        let reset = NOW + 3 * HOUR;
        for stream in [every_12s(120, 0, 42.0, reset), held(120, 0, 42.0, reset)] {
            let run = runway(&stream, &[], FIVE_HOURS, NOW).unwrap();
            assert_eq!(run.pct_per_hour, Some(0.0));
            assert_eq!(run.exhausts_at_ms, None);
        }
    }

    #[test]
    fn an_idle_gap_starts_afresh_and_is_never_taken_as_held() {
        let reset = NOW + 3 * HOUR;
        // 40 % read at 3 h ago, then nothing until 20 min ago (an idle
        // spell), then 40 % again and 46 % now.
        for stream in [
            [
                every_12s(180, 179, 40.0, reset),
                every_12s(20, 10, 40.0, reset),
                every_12s(9, 0, 46.0, reset),
            ]
            .concat(),
            [
                held(180, 179, 40.0, reset),
                held(20, 10, 40.0, reset),
                held(9, 0, 46.0, reset),
            ]
            .concat(),
        ] {
            let window: Vec<&QuotaSample> = stream.iter().collect();
            let start = pace_start(&window, NOW - HOUR).unwrap();
            assert_eq!(start.ts, NOW - 20 * MIN, "the first reading after the gap");
        }
    }
}
