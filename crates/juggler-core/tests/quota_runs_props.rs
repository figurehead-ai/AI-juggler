//! V50, as a property: any stream of readings (dense runs, a reading
//! changing, idle spells longer than the held gap), stored as held runs and
//! read back, gives the runway, its pace, the current reading and a chart's
//! step the same as the raw stream did.
#![allow(
    clippy::unwrap_used,
    clippy::pedantic,
    reason = "a property test (QUALITY.md §2)"
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, UsageSource, WindowKind, current_reading};
use juggler_core::quota_runway::runway;
use juggler_core::storage::Storage;
use proptest::prelude::*;

const FIVE_HOURS: i64 = 5 * 3_600_000;
const START: i64 = 1_800_000_000_000;

/// One stretch of the stream: an idle spell first (minutes, 0 = none),
/// then `reads` readings 12 s apart at a value `rise` points above the last.
fn stretch() -> impl Strategy<Value = (u8, u8, u8)> {
    (prop_oneof![Just(0u8), 3u8..40], 1u8..40, 0u8..4)
}

fn stream(stretches: &[(u8, u8, u8)], reset: i64) -> Vec<QuotaSample> {
    let (mut at, mut used) = (START, 10.0_f64);
    let mut out = Vec::new();
    for (idle_min, reads, rise) in stretches {
        at += i64::from(*idle_min) * 60_000;
        used = (used + f64::from(*rise)).min(100.0);
        for _ in 0..*reads {
            at += 12_000;
            out.push(QuotaSample {
                ts: at,
                used_pct: used,
                resets_at: Some(reset),
                confidence: Confidence::Measured,
                held_until: None,
            });
        }
    }
    out
}

/// The value a chart steps to at `at`: the newest reading at or before it.
fn step(samples: &[QuotaSample], at: i64) -> Option<f64> {
    samples
        .iter()
        .filter(|sample| sample.ts <= at)
        .max_by_key(|sample| sample.ts)
        .map(|sample| sample.used_pct)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn a_stream_stored_as_held_runs_reads_back_the_same(stretches in prop::collection::vec(stretch(), 1..24)) {
        let probe = stream(&stretches, 0);
        let last = probe.last().unwrap().ts;
        let reset = last + 2 * 3_600_000;
        let raw = stream(&stretches, reset);
        let storage = Storage::open_in_memory().unwrap();
        let usage = storage.usage();
        for sample in &raw {
            usage
                .insert_sourced_samples(&[("acc".to_owned(), WindowKind::FiveHour, *sample, UsageSource::Rollout)])
                .unwrap();
        }
        let stored = usage.samples_since("acc", WindowKind::FiveHour, i64::MIN).unwrap();
        prop_assert!(stored.len() <= raw.len());

        for now in [last, last + 60_000, last + 30 * 60_000] {
            let (a, b) = (runway(&raw, &[], FIVE_HOURS, now), runway(&stored, &[], FIVE_HOURS, now));
            prop_assert_eq!(a.is_some(), b.is_some());
            if let (Some(a), Some(b)) = (a, b) {
                prop_assert_eq!(a.used_pct, b.used_pct);
                prop_assert_eq!(a.resets_at_ms, b.resets_at_ms);
                prop_assert_eq!(a.starts_ms, b.starts_ms);
                prop_assert_eq!(a.pct_per_hour, b.pct_per_hour, "the pace");
                prop_assert_eq!(a.exhausts_at_ms, b.exhausts_at_ms);
            }
        }
        let (now_raw, now_stored) = (current_reading(&raw).unwrap(), current_reading(&stored).unwrap());
        prop_assert_eq!((now_raw.ts, now_raw.used_pct), (now_stored.ts, now_stored.used_pct));
        for sample in &raw {
            prop_assert_eq!(step(&raw, sample.ts), step(&stored, sample.ts), "the chart at {}", sample.ts);
        }
    }
}
