//! AJ-E2902's load row, "Usage cost cuts, 2,000 sessions": 10 accounts,
//! 2,000 sessions of usage over 90 days, read as the Usage screen reads
//! them (a day per bucket). The cuts (the tiles and the account table)
//! come from the summaries in ≤ 50 ms (the median of five warm reads;
//! debug build, local SQLite), and the account table's dollars add up to
//! Prices and details' total for the same usage, to the cent. Ignored: the
//! load gate (`just load-gate`) runs it. Which sessions finished or were
//! wasted is `cost_cuts.rs`'s.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::features::usage_history::cuts::cuts_impl;
use juggler_app::features::usage_history::dto::UsageQueryInput;
use juggler_app::features::usage_history::history_impl;
use juggler_app::state::AppState;
use juggler_core::model::{Account, AccountId, Confidence, HarnessId, Timestamp, TokenCounts};
use juggler_core::quota::{UsageDetail, UsageEvent, UsageSource};

const DAY: i64 = 24 * 60 * 60 * 1000;
const SESSIONS: usize = 2_000;
const ACCOUNTS: usize = 10;

#[test]
#[ignore = "a wall-clock budget: the load gate runs it"]
fn usage_cost_cuts_at_two_thousand_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    // The range ends at the start of today's hour less a day: every event
    // is settled, as a 90-day view's are.
    let now = Timestamp::now().millis();
    let end = (now / DAY) * DAY - DAY;
    let start = end - 90 * DAY;
    {
        let storage = state.storage.lock().unwrap();
        for a in 0..ACCOUNTS {
            let id = format!("acc-{a}");
            storage
                .accounts()
                .insert(&Account {
                    id: AccountId::new(id.as_str()),
                    harness: HarnessId::new("claude-code"),
                    label: format!("Account {a:02}"),
                    plan: None,
                    state_dir: PathBuf::from("/fixture/accounts").join(&id),
                    cleanup_opt_in: false,
                    renewal_date: None,
                    preview_opt_in: false,
                    import_source_path: None,
                    import_origin: None,
                    imported_at: None,
                    in_place: false,
                })
                .unwrap();
        }
        let mut events = Vec::new();
        for s in 0..SESSIONS {
            // Five calls each, spread over the 90 days.
            for call in 0..5 {
                let ts = start + ((s * 7919 + call * 104_729) as i64 % (90 * DAY - 1));
                events.push(UsageEvent {
                    account: format!("acc-{}", s % ACCOUNTS),
                    session: Some(format!("c-{s}")),
                    ts,
                    model: Some("claude-opus-4-6".to_owned()),
                    tokens: TokenCounts {
                        input: 1_000 + (s as u64 * 37 + call as u64 * 11) % 9_000,
                        output: 200 + (s as u64 * 13 + call as u64 * 7) % 3_000,
                        ..TokenCounts::default()
                    },
                    credits: None,
                    usd_est: None,
                    source: UsageSource::Transcript,
                    confidence: Confidence::Measured,
                    detail: UsageDetail::default(),
                    msg_uuid: Some(format!("c-{s}-{call}")),
                });
            }
        }
        storage.usage().insert_events(&events).unwrap();
    }
    let input = UsageQueryInput {
        edges_ms: (0..=90).map(|d| start + d * DAY).collect(),
        group_by: "profile".to_owned(),
        profile: None,
        profiles: None,
        project: None,
        top: None,
    };
    let started = Instant::now();
    let cold = cuts_impl(&state, &input).unwrap();
    let first = started.elapsed();
    let mut runs = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let cuts = cuts_impl(&state, &input).unwrap();
        runs.push(started.elapsed());
        assert_eq!(cuts.by_account.len(), cold.by_account.len());
    }
    runs.sort();
    let mid = runs[runs.len() / 2];
    eprintln!(
        "[perf] Usage cost cuts, 2,000 sessions: median {mid:?}, first read {first:?} (runs {runs:?})"
    );
    assert_eq!(cold.by_account.len(), ACCOUNTS, "{:?}", cold.by_account);

    // The account table adds up to Prices and details' total, to the cent.
    let table: f64 = cold.by_account.iter().filter_map(|row| row.usd).sum();
    let total = commands::cost_rollups_impl(&state, None).total_usd.unwrap();
    eprintln!("[perf] reconciled: accounts ${table:.2}, Prices and details ${total:.2}");
    assert_eq!(
        (table * 100.0).round(),
        (total * 100.0).round(),
        "accounts ${table} vs total ${total}"
    );
    assert!(mid <= Duration::from_millis(50), "{mid:?}");

    // The weekly review sheet: the week's summary and its cuts, as it reads them.
    let week = UsageQueryInput {
        edges_ms: (0..=7).map(|d| end - 7 * DAY + d * DAY).collect(),
        ..input
    };
    let mut review = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        history_impl(&state, &week).unwrap();
        cuts_impl(&state, &week).unwrap();
        review.push(started.elapsed());
    }
    review.sort();
    let review_mid = review[review.len() / 2];
    eprintln!(
        "[perf] Usage weekly review, 2,000 sessions: median {review_mid:?} (runs {review:?})"
    );
    assert!(review_mid <= Duration::from_millis(100), "{review_mid:?}");
}
