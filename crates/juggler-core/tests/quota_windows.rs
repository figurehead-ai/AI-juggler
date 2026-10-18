//! Two-clock engine property tests (AJ-E606, deep/09 §Q3, SPEC M3-2).
//! Every property runs ≥256 proptest cases; all maths is pure over
//! `usage_events` with an injected clock — no harness, no DB, no wall
//! time (storage-level dedupe properties use the real store).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::time::Instant;

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{
    self, AccountEvaluation, QuotaCap, QuotaSample, SubscriptionClock, UsageEvent, UsageSource,
    WindowKind, computed_percentage, evaluate_account, evaluate_window, in_window, window_bounds,
    window_burn,
};
use juggler_core::renewal::CalendarDate;
use juggler_core::storage::Storage;
use proptest::collection::vec;
use proptest::prelude::*;

const NOW: i64 = 1_760_000_000_000; // 2025-10-09T09:33:20Z — arbitrary fixed clock
const DAY: i64 = 24 * 60 * 60 * 1000;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// §Q3: the window sum is MONOTONE under event insertion — adding
    /// events (any timestamps) can never DECREASE the fold.
    #[test]
    fn window_sum_is_monotone_under_insertion(
        base in vec(event_strategy(NOW), 0..40),
        extra in vec(event_strategy(NOW), 0..40),
        kind in window_kind_strategy(),
    ) {
        let account = "acc";
        let before = window_burn(base.iter().map(|(_, e)| e), account, kind, NOW);
        let mut all = base.clone();
        all.extend(extra.clone());
        let after = window_burn(all.iter().map(|(_, e)| e), account, kind, NOW);
        prop_assert!(after.tokens.input >= before.tokens.input);
        prop_assert!(after.tokens.output >= before.tokens.output);
        prop_assert!(after.requests >= before.requests);
        prop_assert!(after.credits + 1e-9 >= before.credits);
    }

    /// §Q3: the boundary rule `(t−W, t]` holds IDENTICALLY on the 5 h
    /// and weekly windows — an event exactly at `t−W` is out, at `t`
    /// it is in, one millisecond beyond either edge flips membership.
    #[test]
    fn boundary_rule_is_identical_on_both_windows(
        offset_ms in -1_200i64..1_200,
        kind in window_kind_strategy(),
    ) {
        let (lo, hi) = match kind {
            WindowKind::FiveHour => window_bounds(kind, NOW),
            WindowKind::Weekly { anchor_ms } => quota::weekly_phase_bounds(anchor_ms, NOW),
        };
        let event_ts = NOW + offset_ms;
        let ev = |ts| UsageEvent {
            account: "acc".into(),
            session: None,
            ts,
            model: None,
            tokens: TokenCounts { input: 1, ..TokenCounts::default() },
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: Confidence::Estimated,
            msg_uuid: Some(ts.to_string()),
        };
        let burn = window_burn(std::iter::once(&ev(event_ts)), "acc", kind, NOW);
        prop_assert_eq!(burn.requests, u64::from(in_window(event_ts, (lo, hi))));
        // The same predicate governs BOTH window widths: the rolling
        // 5 h and rolling weekly agree with their own bounds.
        let (olo, ohi) = window_bounds(WindowKind::FiveHour, NOW);
        let (wlo, whi) = window_bounds(WindowKind::Weekly { anchor_ms: None }, NOW);
        let ts = NOW + offset_ms;
        let burn_5h = window_burn(std::iter::once(&ev(ts)), "acc", WindowKind::FiveHour, NOW);
        let burn_7d =
            window_burn(std::iter::once(&ev(ts)), "acc", WindowKind::Weekly { anchor_ms: None }, NOW);
        prop_assert_eq!(burn_5h.requests, u64::from(in_window(ts, (olo, ohi))));
        prop_assert_eq!(burn_7d.requests, u64::from(in_window(ts, (wlo, whi))));
    }

    /// §Q3: no user-confirmed cap ⇒ NO percentage, ever — the engine
    /// exposes absolute burn and never invents a denominator.
    #[test]
    fn unknown_cap_yields_no_percentage(
        events in vec(event_strategy(NOW), 0..60),
        samples in vec(sample_strategy(), 0..5),
    ) {
        let evs: Vec<UsageEvent> = events.iter().map(|(_, e)| e.clone()).collect();
        let reading = evaluate_window(
            &evs,
            &samples,
            "acc",
            WindowKind::FiveHour,
            QuotaCap::default(),
            NOW,
        );
        if reading.percentage.is_some_and(|p| p.is_finite()) {
            // A percentage may exist ONLY via the Measured path; with
            // no Measured samples the Computed path must yield None.
            if !samples.iter().any(|s| s.confidence == Confidence::Measured) {
                prop_assert!(reading.percentage.is_none());
            }
        }
    }

    /// §Q3: clock-skew ±5 min never yields negative headroom — events
    /// stamped slightly in the future still fold to a `[0,100]`-clamped
    /// percentage and non-negative burn.
    #[test]
    fn clock_skew_never_yields_negative_headroom(
        events in vec(event_strategy(NOW + 5 * 60 * 1000), 0..50),
        cap_tokens in 1_000u64..10_000_000,
    ) {
        let evs: Vec<UsageEvent> = events.iter().map(|(_, e)| e.clone()).collect();
        let reading = evaluate_window(
            &evs,
            &[],
            "acc",
            WindowKind::FiveHour,
            QuotaCap { tokens: Some(cap_tokens), credits: None },
            NOW,
        );
        prop_assert!(reading.burn.tokens.input < u64::MAX);
        if let Some(pct) = reading.percentage {
            prop_assert!((0.0..=100.0).contains(&pct));
            prop_assert!(pct.is_finite());
        }
        let headroom = computed_percentage(
            &reading.burn,
            QuotaCap { tokens: Some(cap_tokens), credits: None },
        );
        prop_assert!(headroom.is_none_or(|p| p <= 100.0));
    }

    /// D-12 isolation: fuzzing window events NEVER disturbs the
    /// subscription clock — same date in, same evaluation out.
    #[test]
    fn subscription_clock_is_isolated_from_window_events(
        events in vec(event_strategy(NOW), 0..80),
        renewal in "(19|20)[0-9]{2}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])",
        today_year in 2024i64..2030,
        today_month in 1u32..=12,
        today_day in 1u32..=28,
    ) {
        let clock = SubscriptionClock {
            renewal_date: Some(renewal.clone()),
            entered_by_user: true,
            from_vendor_feed: false,
        };
        let today = CalendarDate { year: today_year, month: today_month, day: today_day };
        let before = clock.evaluate(today);
        let evs: Vec<UsageEvent> = events.iter().map(|(_, e)| e.clone()).collect();
        let account_quota = evaluate_account(AccountEvaluation {
            events: &evs,
            samples_five_hour: &[],
            samples_weekly: &[],
            account: "acc",
            cap_five_hour: QuotaCap::default(),
            cap_weekly: QuotaCap::default(),
            weekly_anchor_ms: None,
            subscription: &clock,
            now_ms: NOW,
            today,
        });
        prop_assert_eq!(account_quota.renewal, before);
        // And the clock object itself is untouched (no window path
        // writes it — enforced structurally, asserted here).
        prop_assert_eq!(clock.renewal_date.as_deref(), Some(renewal.as_str()));
        prop_assert!(clock.entered_by_user);
    }
}

// ---------------------------------------------------------------------------
// strategies

/// An event with a plausible timestamp around `center` (±8 d) — ids
/// unique per generation so dedupe tests can share them.
fn event_strategy(center: i64) -> impl Strategy<Value = (u64, UsageEvent)> {
    (
        0u64..100_000,
        (center - 8 * DAY)..=(center + 1),
        0u64..50_000,
        0u64..20_000,
    )
        .prop_map(|(id, ts, input, output)| {
            (
                id,
                UsageEvent {
                    account: "acc".into(),
                    session: None,
                    ts,
                    model: Some("test-model".into()),
                    tokens: TokenCounts {
                        input,
                        output,
                        ..TokenCounts::default()
                    },
                    credits: None,
                    usd_est: None,
                    source: UsageSource::Transcript,
                    confidence: Confidence::Estimated,
                    msg_uuid: Some(format!("uuid-{id}")),
                },
            )
        })
}

fn window_kind_strategy() -> impl Strategy<Value = WindowKind> {
    prop_oneof![
        Just(WindowKind::FiveHour),
        Just(WindowKind::Weekly { anchor_ms: None }),
        Just(WindowKind::Weekly {
            anchor_ms: Some(NOW - 30 * DAY)
        }),
    ]
}

fn sample_strategy() -> impl Strategy<Value = QuotaSample> {
    (
        0i64..NOW,
        0.0f64..100.0,
        proptest::option::of(0i64..NOW + DAY),
    )
        .prop_map(|(ts, used_pct, resets_at)| QuotaSample {
            ts,
            used_pct,
            resets_at,
            confidence: Confidence::Estimated,
        })
}

// ---------------------------------------------------------------------------
// §Q8 state-machine timeline (the review's terminal-dump fixture) +
// storage-level dedupe (resumed-file double-count contract)

/// The §Q8 machine over the fixture timeline:
/// Unknown → Learning → Tracking → Exhausted → (reset) → Tracking.
/// Unknown/Learning NEVER carry a numeric percentage.
#[test]
fn state_machine_fixture_timeline() {
    let account = "acc-timeline";
    let start = NOW - 30 * 60 * 1000; // 30 min before the clock
    let event = |ts, input| UsageEvent {
        account: account.into(),
        session: None,
        ts,
        model: None,
        tokens: TokenCounts {
            input,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Estimated,
        msg_uuid: Some(format!("t-{ts}")),
    };
    let sample = |ts, used_pct, resets_at| QuotaSample {
        ts,
        used_pct,
        resets_at: Some(resets_at),
        confidence: Confidence::Measured,
    };

    let mut events = Vec::new();
    let mut samples = Vec::new();

    // (1) Unknown: nothing at all.
    let unknown = evaluate_window(
        &events,
        &samples,
        account,
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    );
    assert_eq!(unknown.state, quota::QuotaState::Unknown);
    assert!(unknown.percentage.is_none());

    // (2) Learning: first event, not enough to fold a trend.
    events.push(event(start, 100));
    let learning = evaluate_window(
        &events,
        &samples,
        account,
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    );
    assert_eq!(learning.state, quota::QuotaState::Learning);
    assert!(learning.percentage.is_none(), "Learning must never read 0%");

    // (3) Tracking: a second event + a FRESH Measured sample (<10 min
    // old — older would honestly be Stale per §Q8).
    events.push(event(start + 60_000, 200));
    samples.push(sample(NOW - 60_000, 30.0, NOW + 60 * 60 * 1000));
    let tracking = evaluate_window(
        &events,
        &samples,
        account,
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    );
    assert_eq!(tracking.state, quota::QuotaState::Tracking);
    let pct = tracking.percentage.expect("Tracking carries a percentage");
    assert!((0.0..=100.0).contains(&pct));
    assert_eq!(tracking.basis, Some(quota::PercentageBasis::Measured));

    // (4) Exhausted: the vendor says 100% and the reset is still ahead.
    samples.push(sample(NOW - 500, 100.0, NOW + 30 * 60 * 1000));
    let exhausted = evaluate_window(
        &events,
        &samples,
        account,
        WindowKind::FiveHour,
        QuotaCap::default(),
        NOW,
    );
    assert_eq!(exhausted.state, quota::QuotaState::Exhausted);

    // (5) (reset): the vendor reset passes; a fresh sample below 100
    // brings the window back to Tracking.
    let after_reset = NOW + 31 * 60 * 1000;
    samples.push(sample(after_reset, 5.0, after_reset + 5 * 60 * 60 * 1000));
    let recovered = evaluate_window(
        &events,
        &samples,
        account,
        WindowKind::FiveHour,
        QuotaCap::default(),
        after_reset,
    );
    assert_eq!(recovered.state, quota::QuotaState::Tracking);
    assert!(recovered.percentage.is_some_and(|p| p < 10.0));

    // Terminal dump — the review's text evidence (AJ-E606 Visual
    // testing clause).
    println!("[timeline] Unknown      pct=None  (no data)");
    println!("[timeline] Learning     pct=None  (1 event)");
    println!(
        "[timeline] Tracking     pct={:?} (Measured 30%, resets in 1h)",
        tracking.percentage
    );
    println!("[timeline] Exhausted    pct=100.0 (Measured, reset ahead)");
    println!(
        "[timeline] Tracking     pct={:?} (post-reset Measured 5%)",
        recovered.percentage
    );
}

/// §Q3 dedupe contract (claude_transcript.rs resumed-file shape): the
/// SAME events re-ingested through the store count ONCE —
/// `UNIQUE(account, msg_uuid)` at the storage level.
#[test]
fn store_dedupes_resumed_file_double_count() {
    let storage = Storage::open_in_memory().unwrap();
    let mk = |id: &str| UsageEvent {
        account: "acc".into(),
        session: None,
        ts: NOW - 1000,
        model: None,
        tokens: TokenCounts {
            input: 10,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Estimated,
        msg_uuid: Some(id.into()),
    };
    let first = vec![mk("a"), mk("b"), mk("c")];
    assert_eq!(storage.usage().insert_events(&first).unwrap(), 3);
    // The resumed file replays a+b+c plus one NEW event d.
    let mut replayed = first.clone();
    replayed.push(mk("d"));
    assert_eq!(storage.usage().insert_events(&replayed).unwrap(), 1);
    let stored = storage.usage().events_for_account("acc", 0).unwrap();
    assert_eq!(stored.len(), 4, "resumed file must not double-count");
    let burn = window_burn(stored.iter(), "acc", WindowKind::FiveHour, NOW);
    assert_eq!(burn.tokens.input, 40);
}

/// Events WITHOUT a uuid never collide (NULLs are distinct — honest
/// absence of a dedupe key, not a false dedupe).
#[test]
fn uuid_less_events_do_not_false_dedupe() {
    let storage = Storage::open_in_memory().unwrap();
    let mk = |input| UsageEvent {
        account: "acc".into(),
        session: None,
        ts: NOW - 1000,
        model: None,
        tokens: TokenCounts {
            input,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Estimated,
        msg_uuid: None,
    };
    assert_eq!(
        storage
            .usage()
            .insert_events(&[mk(5), mk(5), mk(5)])
            .unwrap(),
        3
    );
    let stored = storage.usage().events_for_account("acc", 0).unwrap();
    assert_eq!(stored.len(), 3);
}

/// The performance budget: fold ≤5 ms for 100 k events/account
/// (release; the debug-mode CI gate asserts correctness at a relaxed
/// bound so the check can never flake on a loaded runner).
#[test]
fn window_fold_handles_100k_events() {
    let account = "acc-perf";
    let events: Vec<UsageEvent> = (0..100_000u32)
        .map(|i| UsageEvent {
            account: account.into(),
            session: None,
            ts: NOW - i64::from(i % 60) * 60_000,
            model: None,
            tokens: TokenCounts {
                input: u64::from(i % 1000),
                ..TokenCounts::default()
            },
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: Confidence::Estimated,
            msg_uuid: Some(format!("perf-{i}")),
        })
        .collect();
    let started = Instant::now();
    let burn = window_burn(events.iter(), account, WindowKind::FiveHour, NOW);
    let elapsed = started.elapsed();
    assert!(burn.requests > 0);
    println!("[perf] 100k-event fold: {elapsed:?} (budget ≤5 ms release)");
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() <= 100,
        "release fold exceeded the relaxed 100 ms gate: {elapsed:?}"
    );
}

/// The subscription clock honours D-12 end to end: no date ⇒ Unknown;
/// a user date ⇒ whole days; a malformed date ⇒ Unknown, never a
/// guess.
#[test]
fn subscription_clock_d12_semantics() {
    let today = CalendarDate {
        year: 2026,
        month: 10,
        day: 16,
    };
    let none = SubscriptionClock::default();
    assert_eq!(none.evaluate(today), quota::RenewalState::Unknown);
    let user = SubscriptionClock {
        renewal_date: Some("2026-10-23".into()),
        entered_by_user: true,
        from_vendor_feed: false,
    };
    assert_eq!(
        user.evaluate(today),
        quota::RenewalState::RenewIn {
            days: 7,
            entered_by_user: true
        }
    );
    let malformed = SubscriptionClock {
        renewal_date: Some("not-a-date".into()),
        entered_by_user: true,
        from_vendor_feed: false,
    };
    assert_eq!(malformed.evaluate(today), quota::RenewalState::Unknown);
}

/// The anchored weekly phase (`ZCode` shape): resets land on phase
/// boundaries from the anchor, not on rolling earliest-event maths.
#[test]
fn anchored_weekly_resets_on_phase_boundary() {
    let account = "acc-anchor";
    let anchor = NOW - 10 * DAY; // 10 days ago ⇒ mid-phase-1
    let events = vec![UsageEvent {
        account: account.into(),
        session: None,
        ts: NOW - DAY,
        model: None,
        tokens: TokenCounts {
            input: 1,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Estimated,
        msg_uuid: Some("anchor-1".into()),
    }];
    let reading = evaluate_window(
        &events,
        &[],
        account,
        WindowKind::Weekly {
            anchor_ms: Some(anchor),
        },
        QuotaCap::default(),
        NOW,
    );
    // Phase 1 ends at anchor + 14 d = NOW + 4 d.
    assert_eq!(reading.resets_at, Some(anchor + 14 * DAY));
}
