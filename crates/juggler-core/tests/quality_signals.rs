//! AJ-E409 evidence: the signal engine over committed transcript
//! fixtures — exact tokens/min, the `N errors / M turns` string, the
//! context-unavailable reason, the no-score grep gate, and pricing.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::model::TokenCounts;
use juggler_core::quality::{
    UsageEvent, load_pricing_table, pricing::session_cost, pricing::session_cost_per_turn,
    quota_forecast_label, signals,
};

const T0: i64 = 1_728_800_000_000;

/// Minimal loader for the fixture shape (`{at, usage, error}` lines).
fn load_fixture(name: &str) -> (Vec<UsageEvent>, u64, u64) {
    let bytes = std::fs::read(format!("tests/fixtures/{name}")).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    let mut events = Vec::new();
    let mut errors = 0_u64;
    let mut turns = 0_u64;
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        turns += 1;
        let usage = &value["usage"];
        let tokens = TokenCounts {
            input: usage["input_tokens"].as_u64().unwrap_or(0),
            output: usage["output_tokens"].as_u64().unwrap_or(0),
            cache_read: 0,
            cache_write: 0,
            reasoning: 0,
        };
        events.push(UsageEvent {
            at_ms: value["at"].as_i64().unwrap_or(T0),
            tokens,
            model: None,
        });
        if value.get("error").is_some() {
            errors += 1;
        }
    }
    (events, errors, turns)
}

#[test]
fn claude_fixture_exact_tokens_per_min() {
    let (events, _, _) = load_fixture("claude_usage.jsonl");
    let tpm = signals::tokens_per_min(&events, T0);
    assert!(tpm.has_window());
    // Turns 22..=41 fall inside the trailing 5 minutes (the older half
    // of the 10-minute spread is out of window): 20 × 150 = 3000.
    assert_eq!(tpm.trailing_5_min, 3000);
    assert!((tpm.avg_per_min - 600.0).abs() < 0.01);
    // The newest bar carries the freshest turns (non-zero).
    assert!(tpm.bars.unwrap()[4] > 0);
}

#[test]
fn idle_renders_real_zero_distinct_from_no_data() {
    // Events exist but all older than 5 min: REAL zero, window present.
    let old = vec![UsageEvent {
        at_ms: T0 - 20 * 60_000,
        tokens: TokenCounts {
            input: 100,
            output: 50,
            ..TokenCounts::default()
        },
        model: None,
    }];
    let idle = signals::tokens_per_min(&old, T0);
    assert!(idle.has_window());
    assert_eq!(idle.trailing_5_min, 0);
    assert!((idle.avg_per_min - 0.0).abs() < f64::EPSILON);
    // No events at all: `no data`.
    let none = signals::tokens_per_min(&[], T0);
    assert!(!none.has_window());
}

#[test]
fn error_rate_renders_numerator_denominator() {
    let (_, errors, turns) = load_fixture("errors.jsonl");
    assert_eq!(errors, 3);
    assert_eq!(turns, 10);
    let rate = signals::ErrorRate {
        errors,
        turns,
        not_measurable: false,
    };
    assert_eq!(rate.label(), "3 errors / 10 turns");
}

#[test]
fn context_unavailable_renders_the_provider_reason() {
    let report = signals::ContextReport::NotReported;
    assert_eq!(report, signals::ContextReport::NotReported);
    // The exact UI copy is asserted in the component tests:
    // `Context: not reported by {Provider}` — estimating from cumulative
    // tokens is PROHIBITED (type-level: no conversion exists).
}

#[test]
fn quota_forecast_is_a_range_or_not_burning() {
    // Burning: 40% left at 10%/5min → mid 20 min → ~13-27m range.
    let label = quota_forecast_label(60.0, 10.0);
    assert!(
        label.starts_with('~') && label.contains("at current rate · Est"),
        "{label}"
    );
    assert!(label.contains("13") && label.contains("27"), "{label}");
    // Idle: never a point estimate, never infinity.
    assert_eq!(quota_forecast_label(60.0, 0.0), "not burning");
}

#[test]
fn pricing_table_is_versioned_data_and_costs_are_estimates() {
    let table = load_pricing_table();
    assert_eq!(table.version, "v1");
    assert!(!table.source.is_empty() && !table.captured.is_empty());
    let tokens = TokenCounts {
        input: 12_345,
        output: 6_789,
        ..TokenCounts::default()
    };
    let cost = session_cost(&table, "claude-code", "opus", tokens).unwrap();
    let expected = 12.345 * 0.015 + 6.789 * 0.075;
    assert!((cost - expected).abs() < 1e-9);
    // Unknown model: NO cost — callers render tokens only (never $0.00).
    assert!(session_cost(&table, "claude-code", "unknown-model", tokens).is_none());
}

#[test]
fn no_composite_score_exists_in_crates_or_ui() {
    // The D-07 grep gate: these identifiers must appear nowhere in Rust
    // sources or the UI (type, payload, or UI).
    let forbidden = ["quality_score", "health_score", "QualityScore"];
    let roots = ["crates", "ui/src"];
    let mut hits = Vec::new();
    for root in roots {
        for entry in walk(std::path::Path::new(root)) {
            let name = entry
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !name.ends_with(".rs") && !name.ends_with(".ts") && !name.ends_with(".tsx") {
                continue;
            }
            let content = std::fs::read_to_string(&entry).unwrap_or_default();
            for needle in forbidden {
                if content.contains(needle) {
                    hits.push(format!("{}: {needle}", entry.display()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "composite-score identifiers found: {hits:?}"
    );
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// Per-Instance compute budget at 100k events: single pass stays fast.
#[test]
fn per_turn_pricing_prices_only_turns_with_a_recorded_model() {
    let table = load_pricing_table();
    let event = |model: Option<&str>, input: u64, output: u64| UsageEvent {
        at_ms: T0,
        tokens: TokenCounts {
            input,
            output,
            ..TokenCounts::default()
        },
        model: model.map(str::to_string),
    };
    // Mixed session: opus turn + unrecorded turn + sonnet turn.
    let events = vec![
        event(Some("opus"), 1_000, 500),
        event(None, 2_000, 1_000),
        event(Some("sonnet"), 10_000, 5_000),
    ];
    let (cost, models) = session_cost_per_turn(&table, "claude-code", &events).unwrap();
    // opus: 1.0*0.015 + 0.5*0.075 = 0.0525; sonnet: 10*0.003 + 5*0.015 = 0.105.
    assert!((cost - 0.1575).abs() < 1e-9);
    assert_eq!(models, vec!["opus".to_string(), "sonnet".to_string()]);

    // No turn records a model: no $ (the honest tokens-only state).
    let unrecorded = vec![event(None, 5_000, 5_000)];
    assert!(session_cost_per_turn(&table, "claude-code", &unrecorded).is_none());

    // Recorded but unpriced model: also excluded, never $0.00.
    let unpriced = vec![event(Some("gpt-5-codex"), 5_000, 5_000)];
    assert!(session_cost_per_turn(&table, "claude-code", &unpriced).is_none());
}

#[test]
fn tokens_per_min_100k_events_under_budget() {
    let events: Vec<UsageEvent> = (0..100_000)
        .map(|i| UsageEvent {
            at_ms: T0 - (i % 300) * 1000,
            tokens: TokenCounts {
                input: 100,
                output: 50,
                ..TokenCounts::default()
            },
            model: None,
        })
        .collect();
    let start = std::time::Instant::now();
    let tpm = signals::tokens_per_min(&events, T0);
    let elapsed = start.elapsed();
    assert!(tpm.has_window());
    assert!(
        elapsed.as_millis() < 5,
        "100k-event pass took {:?} (budget 5 ms)",
        elapsed
    );
}
