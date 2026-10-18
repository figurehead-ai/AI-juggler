//! AJ-E609 cost-ledger tests: exact pricing maths (cache classes
//! included), the unpriced `—` law, query-time repricing (stored rows
//! byte-identical), event-time rate selection, staleness, `ZCode`
//! credits alongside USD, the dual subscription view, model-switch
//! attribution, and CSV-vs-rollup reconciliation to the cent.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::cost_ledger::{
    self, CostRollup, LedgerPriceRow, PricingCatalog, RollupDimension, STALE_PRICING_MS,
    load_seed_rows, rollup, subscription_view, write_csv,
};
use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};
use juggler_core::storage::Storage;

const NOW: i64 = 1_792_094_400_000;

fn row(
    model: &str,
    effective_from_ms: i64,
    input: f64,
    output: f64,
    cache_r: f64,
    cache_w: f64,
) -> LedgerPriceRow {
    LedgerPriceRow {
        model: model.into(),
        effective_from_ms,
        input_per_1k: input,
        output_per_1k: output,
        cache_read_per_1k: cache_r,
        cache_write_per_1k: cache_w,
        source_url: "https://example.test/pricing".into(),
        captured: "2026-10-16".into(),
    }
}

fn event(
    session: &str,
    model: &str,
    ts: i64,
    tokens: TokenCounts,
    credits: Option<f64>,
) -> UsageEvent {
    UsageEvent {
        account: "acc".into(),
        session: Some(session.into()),
        ts,
        model: Some(model.into()),
        tokens,
        credits,
        usd_est: None,
        source: UsageSource::Sqlite,
        confidence: if credits.is_some() {
            Confidence::Computed
        } else {
            Confidence::Measured
        },
        msg_uuid: Some(format!("{session}-{model}-{ts}")),
    }
}

fn catalog(rows: &[LedgerPriceRow]) -> PricingCatalog {
    PricingCatalog {
        bundled: rows.to_vec(),
        overrides: Vec::new(),
        last_synced_ms: Some(NOW),
    }
}

#[test]
fn pricing_math_is_exact_including_cache_classes() {
    let table = catalog(&[row("m", 0, 3.0, 15.0, 0.3, 3.75)]);
    // 1k input + 2k output + 4k cache-read + 1k cache-write:
    // 3.0 + 30.0 + 1.2 + 3.75 = 37.95 — hand-computed oracle.
    let usd = table
        .price_tokens(
            "m",
            NOW,
            TokenCounts {
                input: 1_000,
                output: 2_000,
                cache_read: 4_000,
                cache_write: 1_000,
                reasoning: 0,
            },
        )
        .unwrap();
    assert!((usd - 37.95).abs() < 1e-9);
}

#[test]
fn unpriced_model_is_none_never_zero() {
    let table = catalog(&[row("m", 0, 3.0, 15.0, 0.3, 3.75)]);
    let events = vec![event(
        "s",
        "mystery-model",
        NOW,
        TokenCounts {
            input: 1_000,
            ..TokenCounts::default()
        },
        None,
    )];
    let rows = rollup(&events, &table, RollupDimension::Session);
    assert_eq!(rows[0].usd.usd, None, "unpriced renders —, never $0.00");
    assert_eq!(
        rows[0].usd.label,
        Confidence::Estimated,
        "USD is always Estimate"
    );
    assert_eq!(rows[0].tokens.input, 1_000, "tokens still counted");
}

#[test]
fn effective_from_selects_event_time_rates() {
    // Rate halves from day 2: an event BEFORE uses the old row, one
    // AFTER uses the new row.
    let day2 = NOW + 86_400_000;
    let table = catalog(&[
        row("m", 0, 3.0, 15.0, 0.3, 3.75),
        row("m", day2, 1.5, 7.5, 0.15, 1.875),
    ]);
    let before = table
        .price_tokens(
            "m",
            day2 - 1,
            TokenCounts {
                input: 1_000,
                ..TokenCounts::default()
            },
        )
        .unwrap();
    let after = table
        .price_tokens(
            "m",
            day2,
            TokenCounts {
                input: 1_000,
                ..TokenCounts::default()
            },
        )
        .unwrap();
    assert!((before - 3.0).abs() < 1e-9);
    assert!((after - 1.5).abs() < 1e-9);
    // A model whose only row starts later is unpriced before it.
    let future = catalog(&[row("late", day2, 1.0, 1.0, 0.0, 0.0)]);
    assert!(
        future
            .price_tokens("late", NOW, TokenCounts::default())
            .is_none()
    );
}

#[test]
fn overrides_supersede_bundled_and_repricing_is_retroactive() {
    let storage = Storage::open_in_memory().unwrap();
    let events = vec![event(
        "s",
        "m",
        NOW,
        TokenCounts {
            input: 10_000,
            ..TokenCounts::default()
        },
        None,
    )];
    storage.usage().insert_events(&events).unwrap();
    let raw_before: String = {
        let conn = storage.raw_conn();
        conn.query_row(
            "SELECT group_concat(msg_uuid) FROM usage_events ORDER BY ts",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };

    let bundled = catalog(&[row("m", 0, 3.0, 15.0, 0.3, 3.75)]);
    let stored = storage.usage().events_for_account("acc", 0).unwrap();
    let before = rollup(&stored, &bundled, RollupDimension::All);
    assert!((before[0].usd.usd.unwrap() - 30.0).abs() < 1e-9);

    // The user reprices: history reprices retroactively…
    let mut repriced = bundled.clone();
    repriced.overrides = vec![row("m", 0, 1.0, 5.0, 0.1, 1.25)];
    let after = rollup(&stored, &repriced, RollupDimension::All);
    assert!(
        (after[0].usd.usd.unwrap() - 10.0).abs() < 1e-9,
        "query-time join: history repriced"
    );

    // …and the stored events are byte-identical.
    let raw_after: String = {
        let conn = storage.raw_conn();
        conn.query_row(
            "SELECT group_concat(msg_uuid) FROM usage_events ORDER BY ts",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        raw_before, raw_after,
        "usage_events rows never change on repricing"
    );

    // The override persists through the repo round-trip.
    storage
        .usage()
        .pricing_override_upsert(&row("m", 0, 1.0, 5.0, 0.1, 1.25))
        .unwrap();
    assert_eq!(storage.usage().pricing_overrides().unwrap().len(), 1);
    storage.usage().pricing_override_delete("m", 0).unwrap();
    assert!(storage.usage().pricing_overrides().unwrap().is_empty());
}

#[test]
fn stale_pricing_flag_trips_after_30_days() {
    let table = catalog(&[]);
    let fresh = PricingCatalog {
        last_synced_ms: Some(NOW),
        ..table.clone()
    };
    let stale = PricingCatalog {
        last_synced_ms: Some(NOW - STALE_PRICING_MS - 1),
        ..table
    };
    assert!(!fresh.is_stale(NOW));
    assert!(stale.is_stale(NOW), ">30d warns inline");
    assert!(
        !PricingCatalog::default().is_stale(NOW),
        "no metadata ⇒ no staleness claim"
    );
}

#[test]
fn zcode_credits_ride_alongside_never_converted() {
    let table = catalog(&[]); // no USD rows at all: credits still shown
    let events = vec![event(
        "s",
        "GLM-5.3",
        NOW,
        TokenCounts {
            input: 1_000,
            ..TokenCounts::default()
        },
        Some(83.28855),
    )];
    let rows = rollup(&events, &table, RollupDimension::Session);
    assert_eq!(rows[0].usd.usd, None);
    assert!(
        (rows[0].credits.unwrap() - 83.28855).abs() < 1e-9,
        "credits are their own column"
    );
    assert_eq!(rows[0].confidence, Confidence::Computed);
}

#[test]
fn subscription_view_ratio_only_with_a_fee() {
    let none = subscription_view(Some(120.0), None);
    assert!(
        none.value_ratio.is_none(),
        "no fee ⇒ no ratio, no guessed fee"
    );
    let both = subscription_view(Some(120.0), Some(200.0));
    assert!((both.value_ratio.unwrap() - 0.6).abs() < 1e-9);
    assert!((both.api_equivalent_usd.unwrap() - 120.0).abs() < 1e-9);
    let zero_fee = subscription_view(Some(120.0), Some(0.0));
    assert!(zero_fee.value_ratio.is_none());
}

#[test]
fn model_switches_split_rows_and_sum_to_the_session_total() {
    let table = catalog(&[
        row("m", 0, 3.0, 15.0, 0.3, 3.75),
        row("m2", 0, 1.0, 5.0, 0.1, 1.25),
    ]);
    let events = vec![
        event(
            "s",
            "m",
            NOW,
            TokenCounts {
                input: 1_000,
                ..TokenCounts::default()
            },
            None,
        ),
        event(
            "s",
            "m2",
            NOW,
            TokenCounts {
                input: 2_000,
                ..TokenCounts::default()
            },
            None,
        ),
    ];
    let rows = rollup(&events, &table, RollupDimension::Session);
    assert_eq!(rows.len(), 2, "one row per (session, model, day)");
    let total: f64 = rows.iter().filter_map(|r| r.usd.usd).sum();
    assert!(
        (total - 5.0).abs() < 1e-9,
        "3.0 + 2.0 — attribution survives switches"
    );
}

#[test]
fn csv_reconciles_to_the_rollup_to_the_cent() {
    let table = catalog(&[row("m", 0, 3.0, 15.0, 0.3, 3.75)]);
    let events: Vec<UsageEvent> = (0..5)
        .map(|index| {
            event(
                "s",
                "m",
                NOW + index * 60_000,
                TokenCounts {
                    input: 1_234,
                    output: 777,
                    cache_read: 12,
                    cache_write: 5,
                    reasoning: 0,
                },
                if index == 4 { Some(1.25) } else { None },
            )
        })
        .collect();
    let rows = rollup(&events, &table, RollupDimension::Session);
    let mut csv = Vec::new();
    write_csv(&rows, &mut csv).unwrap();
    let text = String::from_utf8(csv).unwrap();
    // Parse the CSV back; the USD column's sum equals the rollup total
    // to the cent (2-decimal rendering — reconciliation law).
    let mut csv_sum = 0.0_f64;
    let mut parsed: Vec<f64> = Vec::new();
    for line in text.lines().skip(1) {
        let usd = line.split(',').nth(8).unwrap();
        if usd != "—" {
            csv_sum += usd.parse::<f64>().unwrap();
        }
        parsed.push(
            line.split(',')
                .nth(8)
                .unwrap()
                .parse::<f64>()
                .unwrap_or(0.0),
        );
    }
    let _ = parsed;
    let rollup_total: f64 = rows.iter().filter_map(|row| row.usd.usd).sum();
    assert!(
        (csv_sum - rollup_total).abs() < 5e-3,
        "CSV ${csv_sum:.2} vs rollup ${rollup_total:.2} — to the cent"
    );
    assert!(
        text.contains("usd_estimate"),
        "header carries the label column"
    );
    assert!(text.contains("Estimate"), "every USD row carries the label");
}

#[test]
fn day_buckets_are_utc_and_stable() {
    assert_eq!(cost_ledger::day_of(0), "1970-01-01");
    assert_eq!(cost_ledger::day_of(1_792_094_402_456), "2026-10-15");
    assert_eq!(cost_ledger::day_of(86_399_999), "1970-01-01");
    assert_eq!(cost_ledger::day_of(86_400_000), "1970-01-02");
}

#[test]
fn bundled_seed_loads_with_the_auditable_fields() {
    let rows = load_seed_rows();
    assert!(!rows.is_empty(), "the seed ships rows");
    for row in &rows {
        assert!(!row.source_url.is_empty(), "every row cites its source");
        assert!(
            !row.captured.is_empty(),
            "every row carries its capture date"
        );
    }
    // The AJ-E409 v1 models remain priced (quality/pricing parity).
    assert!(rows.iter().any(|row| row.model == "claude-opus-4"));
}

/// The shared-fixture-corpus cross-check (the ccusage-pattern oracle):
/// the SAME corpus the Rust rollup prices is priced by the independent
/// python oracle (`docs/research/evidence/billing/ccusage-oracle.py`)
/// — both totals are logged in gate-M3.md with delta $0.00. Here the
/// oracle's printed value is embedded as the constant.
#[test]
fn corpus_total_matches_the_independent_oracle() {
    // Corpus: crates/juggler-adapters/tests/fixtures/quota/claude-transcript.jsonl
    // (a-1: in=100,out=20,cr=5,cw=0; a-2: in=200,out=30,cr=10,cw=2).
    // Seed opus rates: 15/75/1.5/18.75 per 1M ⇒ per 1k 0.015/0.075/
    // 0.0015/0.01875. Oracle total: (100+200)*0.015/1000... — computed
    // by the python oracle and embedded below.
    const ORACLE_TOTAL_USD: f64 = 0.0083100;
    let table = catalog(&load_seed_rows());
    let events = vec![
        event(
            "sess-t",
            "claude-opus-4",
            NOW,
            TokenCounts {
                input: 100,
                output: 20,
                cache_read: 5,
                cache_write: 0,
                reasoning: 0,
            },
            None,
        ),
        event(
            "sess-t",
            "claude-opus-4",
            NOW,
            TokenCounts {
                input: 200,
                output: 30,
                cache_read: 10,
                cache_write: 2,
                reasoning: 0,
            },
            None,
        ),
    ];
    let rows = rollup(&events, &table, RollupDimension::All);
    let total = rows
        .first()
        .and_then(|row: &CostRollup| row.usd.usd)
        .unwrap();
    assert!(
        (total - ORACLE_TOTAL_USD).abs() < 5e-8,
        "corpus totals match the independent oracle: rust {total} vs oracle {ORACLE_TOTAL_USD}"
    );
}
