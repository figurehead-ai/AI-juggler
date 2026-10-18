//! AJ-E611 evidence: hard filters greyed-with-reason, the weighted
//! score with Estimated-discounted headroom (Measured wins ties),
//! explanation completeness (≥2 factors; unknown headroom says so),
//! the §Q5 fixture-day notification sequence, the at-most-once
//! property (random sequences never double-emit one crossing), digest
//! mute/quiet-hours/cadence, and the no-auto-action guardrail shape.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::alerts::{
    AlertEmission, AlertFsm, AlertPhase, AlertSignal, Digest, WindowObservation, digest_add,
    digest_flush, signal, step,
};
use juggler_core::model::Confidence;
use juggler_core::ranking::{self, Candidate, ESTIMATED_HEADROOM_DISCOUNT};
use proptest::collection::vec;
use proptest::prelude::*;

fn candidate(account: &str, headroom: Option<f64>, confidence: Confidence) -> Candidate {
    Candidate {
        account: account.to_owned(),
        logged_out: false,
        wrong_provider: false,
        exhausted_no_reset: false,
        headroom_pct: headroom,
        headroom_confidence: confidence,
        affinity: 0.5,
        fidelity: 0.5,
        projected_usd: Some(1.0),
        active_sessions: 0,
        soft_cap: 4,
        returnable: false,
        source_resets_min: None,
    }
}

#[test]
fn weights_sum_to_one_and_match_the_spec() {
    let sum = ranking::W_HEADROOM
        + ranking::W_AFFINITY
        + ranking::W_FIDELITY
        + ranking::W_COST_FIT
        + ranking::W_CALM;
    assert!((sum - 1.0).abs() < 1e-12, "deep/09 line 454 verbatim");
    assert_eq!(ranking::W_HEADROOM, 0.45);
    assert_eq!(ranking::W_AFFINITY, 0.20);
    assert_eq!(ranking::W_FIDELITY, 0.15);
}

#[test]
fn hard_filters_are_greyed_with_reason_not_dropped() {
    let rows = ranking::rank(&[
        candidate("ok", Some(80.0), Confidence::Measured),
        Candidate {
            logged_out: true,
            ..candidate("logged", None, Confidence::Estimated)
        },
        Candidate {
            wrong_provider: true,
            ..candidate("wrong", Some(90.0), Confidence::Measured)
        },
        Candidate {
            exhausted_no_reset: true,
            ..candidate("spent", Some(0.0), Confidence::Measured)
        },
    ]);
    assert_eq!(rows.len(), 4, "filtered rows stay visible");
    let logged = rows.iter().find(|row| row.account == "logged").unwrap();
    assert_eq!(logged.score, None);
    assert!(
        logged
            .filtered_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("logged out"))
    );
    assert!(
        rows.iter()
            .find(|row| row.account == "wrong")
            .unwrap()
            .filtered_reason
            .is_some()
    );
    assert!(
        rows.iter()
            .find(|row| row.account == "spent")
            .unwrap()
            .filtered_reason
            .as_deref()
            .is_some_and(|r| r.contains("exhausted")),
        "exhausted with no reset before need-by is filtered with its reason"
    );
    // The survivor ranks first; every row explains itself.
    assert_eq!(rows[0].account, "ok");
    assert!(rows[0].top_factors.len() >= 2);
}

#[test]
fn estimated_headroom_discounted_measured_wins_ties() {
    // Identical inputs except confidence: Measured headroom outranks
    // the same number labelled Estimated (deep/09 lines 456-457).
    let rows = ranking::rank(&[
        candidate("estimated", Some(80.0), Confidence::Estimated),
        candidate("measured", Some(80.0), Confidence::Measured),
    ]);
    assert_eq!(rows[0].account, "measured", "Measured truth wins the tie");
    assert_eq!(rows[1].account, "estimated");
    // The discount is the constant, unit-pinned.
    assert_eq!(ESTIMATED_HEADROOM_DISCOUNT, 0.85);
}

#[test]
fn unknown_headroom_ranks_by_remaining_inputs_and_says_so() {
    let rows = ranking::rank(&[
        candidate("unknown-quota", None, Confidence::Estimated),
        candidate("known", Some(10.0), Confidence::Measured),
    ]);
    let unknown = rows
        .iter()
        .find(|row| row.account == "unknown-quota")
        .unwrap();
    assert!(unknown.headroom_unknown);
    assert!(
        unknown
            .top_factors
            .iter()
            .any(|words| words.contains("headroom unknown")),
        "the copy says it ranked without quota"
    );
    assert!(
        unknown.score.is_some(),
        "unknown ≠ filtered: it still ranks"
    );
    // A strong headroom still beats a headroom-less peer.
    let rows = ranking::rank(&[
        candidate("unknown-quota", None, Confidence::Estimated),
        candidate("known", Some(95.0), Confidence::Measured),
    ]);
    assert_eq!(rows[0].account, "known");
}

#[test]
fn affinity_words_carry_the_wrong_billing_guard() {
    let rows = ranking::rank(&[Candidate {
        affinity: 0.9,
        ..candidate("usual", Some(50.0), Confidence::Measured)
    }]);
    let usual = &rows[0];
    assert!(
        usual
            .top_factors
            .iter()
            .any(|words| words.contains("usual account")),
        "the project-history share appears in words (Claude #87447 guard)"
    );
    // The full inputs feed the Why-this-order expander — §Q6's five
    // plus AJ-E209's return-likelihood factor.
    assert_eq!(usual.factors.len(), 6);
    assert!(
        usual
            .factors
            .iter()
            .any(|(words, _)| words.contains("return target")),
        "the E209 return factor is explained in words"
    );
}

/// The §Q5 fixture day: burn → 90% → exhausted → reset produces
/// EXACTLY the notification sequence — nothing more, ever.
#[test]
fn fixture_day_produces_the_exact_q5_sequence() {
    let mut fsm = AlertFsm::default();
    let mut emissions = Vec::new();
    let now = 1_792_094_400_000_i64;
    let obs = |tte: Option<f64>, pct: Option<f64>, c: Confidence| WindowObservation {
        tte_min: tte,
        used_pct: pct,
        confidence: c,
    };

    // Healthy stretches emit nothing.
    for minute in 0..30 {
        let (next, emission) = step(
            &fsm,
            obs(Some(200.0), Some(20.0), Confidence::Measured),
            now + minute * 60_000,
        );
        assert_eq!(emission, AlertEmission::None);
        fsm = next;
    }
    // TTE crosses under 45 ⇒ ONE tray amber.
    let (next, emission) = step(&fsm, obs(Some(40.0), Some(60.0), Confidence::Measured), now);
    fsm = next;
    emissions.push(emission);
    // Stays under 45 ⇒ nothing new.
    let (next, emission) = step(
        &fsm,
        obs(Some(30.0), Some(70.0), Confidence::Measured),
        now + 60_000,
    );
    fsm = next;
    emissions.push(emission);
    // Crosses critical (≥90%) ⇒ ONE actionable.
    let (next, emission) = step(
        &fsm,
        obs(Some(12.0), Some(91.0), Confidence::Measured),
        now + 120_000,
    );
    fsm = next;
    emissions.push(emission);
    // Keeps crossing ⇒ nothing (at-most-once).
    let (next, emission) = step(
        &fsm,
        obs(Some(5.0), Some(95.0), Confidence::Measured),
        now + 180_000,
    );
    fsm = next;
    emissions.push(emission);
    // Exhausted ⇒ ONE exhaustion notification; repeats nothing.
    let (next, emission) = signal(&fsm, AlertSignal::Exhausted, now + 240_000);
    fsm = next;
    emissions.push(emission);
    let (next, emission) = signal(&fsm, AlertSignal::Exhausted, now + 300_000);
    fsm = next;
    emissions.push(emission);
    // Reset ⇒ re-arm + ONE reset notification.
    let (next, emission) = signal(&fsm, AlertSignal::Reset, now + 360_000);
    fsm = next;
    emissions.push(emission);
    assert_eq!(fsm.phase, AlertPhase::Armed);

    assert_eq!(
        emissions,
        vec![
            AlertEmission::TrayAmber {
                confidence: Confidence::Measured
            },
            AlertEmission::None,
            AlertEmission::Actionable {
                confidence: Confidence::Measured
            },
            AlertEmission::None,
            AlertEmission::ExhaustedNotification,
            AlertEmission::None,
            AlertEmission::ResetReached,
        ],
        "the §Q5 sequence, exactly"
    );
}

/// The at-most-once property: RANDOM trigger sequences never emit two
/// notifications for one crossing (per phase entry).
#[derive(Clone, Copy, Debug)]
enum Obs {
    /// A TTE observation.
    Tte(f64),
    /// A used-percentage observation.
    Pct(f64),
    /// The exhaustion signal.
    Exhausted,
    /// The reset signal.
    Reset,
    /// The snooze signal.
    Snooze,
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn random_sequences_never_double_emit(
        sequence in vec(
            prop_oneof![
                3 => (30.0f64..300.0).prop_map(Obs::Tte),
                3 => (0.0f64..100.0).prop_map(Obs::Pct),
                1 => Just(Obs::Exhausted),
                1 => Just(Obs::Reset),
                1 => Just(Obs::Snooze),
            ],
            0..60
        ),
    ) {
        let mut fsm = AlertFsm::default();
        let mut actionable = 0;
        let mut exhausted = 0;
        let mut resets = 0;
        let mut now = 0_i64;
        for obs in sequence {
            now += 60_000;
            let (next, emission) = match obs {
                Obs::Tte(tte) => step(&fsm, WindowObservation { tte_min: Some(tte), used_pct: None, confidence: Confidence::Estimated }, now),
                Obs::Pct(pct) => step(&fsm, WindowObservation { tte_min: None, used_pct: Some(pct), confidence: Confidence::Estimated }, now),
                Obs::Exhausted => signal(&fsm, AlertSignal::Exhausted, now),
                Obs::Reset => signal(&fsm, AlertSignal::Reset, now),
                Obs::Snooze => signal(&fsm, AlertSignal::Snooze, now),
            };
            fsm = next;
            match emission {
                AlertEmission::None => {}
                AlertEmission::TrayAmber { .. } => {} // per-window once — covered by the fixture-day test
                AlertEmission::Actionable { .. } => actionable += 1,
                AlertEmission::ExhaustedNotification => exhausted += 1,
                AlertEmission::ResetReached => resets += 1,
            }
            // Between resets, at most ONE actionable, ONE exhaustion, and
            // (by construction of the cycle close) one reset emission.
            if matches!(emission, AlertEmission::ResetReached) {
                prop_assert!(actionable <= 1 && exhausted <= 1 && resets <= 1, "per cycle: {actionable} actionables, {exhausted} exhaustions, {resets} resets");
                actionable = 0;
                exhausted = 0;
                resets = 0;
            }
        }
        prop_assert!(actionable <= 1 && exhausted <= 1 && resets <= 1);
    }
}

#[test]
fn snooze_holds_the_actionable_thirty_minutes() {
    let mut fsm = AlertFsm {
        phase: AlertPhase::Critical,
        snoozed_until_ms: None,
    };
    let now = 1_000_000_i64;
    let (next, _) = signal(&fsm, AlertSignal::Snooze, now);
    fsm = next;
    // Still crossing while snoozed ⇒ nothing.
    let (next, emission) = step(
        &fsm,
        WindowObservation {
            tte_min: Some(5.0),
            used_pct: None,
            confidence: Confidence::Estimated,
        },
        now + 20 * 60_000,
    );
    fsm = next;
    assert_eq!(emission, AlertEmission::None);
    let _ = fsm;
}

#[test]
fn forecast_misses_never_interrupt() {
    let fsm = AlertFsm::default();
    let (_, emission) = signal(&fsm, AlertSignal::ForecastMissed, 0);
    assert_eq!(
        emission,
        AlertEmission::None,
        ">2× errors are silent calibration notes only"
    );
}

#[test]
fn digest_mutes_defers_and_flushes_hourly() {
    let mut digest = Digest::default();
    digest = digest_add(&digest, "proj-a", "crossed 90% (estimate)");
    digest = digest_add(&digest, "proj-b", "crossed 85% (estimate)");
    // Quiet hours (22:00 local) ⇒ deferred, not dropped.
    let (deferred, out) = digest_flush(&digest, 1_000_000, &[], 23);
    assert!(out.is_empty());
    assert_eq!(deferred.entries.len(), 2);
    // Muted keys drop at flush; the cadence gates the rest.
    let (flushed, out) = digest_flush(&deferred, 1_000_000, &["proj-a".to_owned()], 12);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].key, "proj-b");
    assert!(flushed.entries.is_empty());
    // Before the hour passes again ⇒ nothing.
    let (_, again) = digest_flush(&flushed, 1_000_000 + 3_000_000, &[], 12);
    assert!(again.is_empty());
}

/// The no-auto-action guardrail, structural leg: the ranking and the
/// alert machine are PURE — neither takes a spawn/launch capability,
/// so no code path through them can execute a switch, launch, or
/// resume. (The runtime leg — the reset-queue tick never launches —
/// lives in the app crate's guardrails suite.)
#[test]
fn ranking_and_alerts_are_pure_no_launch_capability() {
    // rank() over storage-derived candidates cannot mutate sessions:
    // it receives only values.
    let rows = ranking::rank(&[candidate("a", Some(50.0), Confidence::Measured)]);
    assert!(rows[0].score.is_some());
    let (_, emission) = step(
        &AlertFsm::default(),
        WindowObservation {
            tte_min: Some(10.0),
            used_pct: None,
            confidence: Confidence::Measured,
        },
        0,
    );
    assert!(matches!(emission, AlertEmission::Actionable { .. }));
}
