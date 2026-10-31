//! 0.3.2: an account carrying on with its credits is watched each pass.
//! The "used up" alert fires once, so credits that ran out mid-window
//! (the balance read as 0) went unsaid: the real "run out of room" need
//! must open then. A window that resets takes it off the watch.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_app::commands::AlertEventInfo;
use juggler_app::features::notifications::sources;
use juggler_app::state::AppState;
use juggler_core::model::{Account, AccountId, Confidence, HarnessId, Timestamp};
use juggler_core::quota::{QuotaSample, WindowKind};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    )
}

fn codex(state: &Arc<AppState>, dir: &Path) {
    let home = dir.join("codex-home");
    std::fs::create_dir_all(&home).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-c"),
            harness: HarnessId::new("codex"),
            label: "Codex".into(),
            plan: None,
            state_dir: home,
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

fn reading(state: &Arc<AppState>, now: i64, used: f64, resets_at: i64) {
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_sample(
            "acc-c",
            WindowKind::Weekly { anchor_ms: None },
            &QuotaSample {
                ts: now,
                used_pct: used,
                resets_at: Some(resets_at),
                confidence: Confidence::Measured,
            },
        )
        .unwrap();
}

fn credits(state: &Arc<AppState>, ts: i64, balance: &str) {
    let snapshot = serde_json::json!({ "ts": ts, "has_credits": true, "unlimited": false, "balance": balance });
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("credits:acc-c", &snapshot.to_string())
        .unwrap();
}

fn needs(state: &Arc<AppState>) -> Vec<String> {
    let storage = state.storage.lock().unwrap();
    juggler_app::features::notifications::centre::open_needs(
        &storage,
        Timestamp::now().millis(),
        20,
    )
    .into_iter()
    .map(|notice| notice.title)
    .collect()
}

fn exhausted() -> AlertEventInfo {
    AlertEventInfo {
        kind: "exhausted".into(),
        account: "acc-c".into(),
        text: String::new(),
        at_ms: Timestamp::now().millis(),
        quota: None,
    }
}

#[test]
fn credits_running_out_mid_window_open_the_out_of_room_need() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    codex(&state, dir.path());
    let now = Timestamp::now().millis();
    reading(&state, now, 100.0, now + 3 * 86_400_000);
    credits(&state, now, "14150.16");
    // The window is used up; the credits carry it on: no need, a notice.
    sources::quota_pass(&state, &[exhausted()]);
    assert!(
        needs(&state).is_empty(),
        "on its credits, nothing needs you: {:?}",
        needs(&state)
    );
    // Later passes (no new alert): still on credits, still nothing.
    sources::quota_pass(&state, &[]);
    assert!(needs(&state).is_empty());
    // The credits run out mid-window, with no new alert: it's out of room now.
    credits(&state, now + 1, "0");
    sources::quota_pass(&state, &[]);
    let told = needs(&state);
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(told[0].contains("run out of room"), "{told:?}");
}

#[test]
fn a_window_that_resets_takes_it_off_the_watch() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    codex(&state, dir.path());
    let now = Timestamp::now().millis();
    reading(&state, now, 100.0, now + 3 * 86_400_000);
    credits(&state, now, "200");
    sources::quota_pass(&state, &[exhausted()]);
    // A new window (a fresh reading at 4%, its own reset), then the credits
    // are spent: nothing to say, it has room.
    reading(&state, now + 1, 4.0, now + 7 * 86_400_000);
    sources::quota_pass(&state, &[]);
    credits(&state, now + 2, "0");
    sources::quota_pass(&state, &[]);
    assert!(needs(&state).is_empty(), "{:?}", needs(&state));
}

#[test]
fn a_balance_read_hours_ago_covers_only_probably() {
    use juggler_app::features::quota::{CREDITS_STALE_MS, CreditsCover, CreditsDisplay};
    let fresh = CreditsDisplay {
        has_credits: true,
        unlimited: false,
        balance: Some("120".into()),
        as_of_ms: 0,
        source: "Codex".into(),
        stale: false,
    };
    assert_eq!(fresh.cover(), Some(CreditsCover::Balance(120.0)));
    let old = CreditsDisplay {
        stale: true,
        ..fresh
    };
    assert_eq!(old.cover(), Some(CreditsCover::Probably));
    assert_eq!(CREDITS_STALE_MS, 6 * 3_600_000);
}
