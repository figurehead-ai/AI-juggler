//! AJ-E607 pipeline integration: a seeded account's managed dir
//! (statusline spool + transcript tree) flows into the AJ-E606 store
//! through ONE `ingest_pass`, idempotently — the second pass (no file
//! changes) inserts zero new rows, and the health map records the
//! sources.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_app::state::AppState;
use juggler_app::usage_pipeline;
use juggler_core::model::{
    Account, AccountId, HarnessId, LaunchRecipe, MachineId, ProfileId, SessionId, SessionStatus,
    Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(
        AppState::init_with_registry(
            dir.join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    )
}

fn seed_claude_account(state: &Arc<AppState>, dir: &Path) {
    let managed = dir.join("managed-claude");
    std::fs::create_dir_all(managed.join("projects")).unwrap();
    // AI Juggler's own copy carries its marker (the status line is only
    // ever installed into a marked folder — ADR-0018).
    std::fs::write(
        managed.join(juggler_adapters::claude_code::MANAGED_MARKER),
        "",
    )
    .unwrap();
    // settings.json WITHOUT a prior statusline (a fresh managed dir).
    std::fs::write(
        managed.join("settings.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"model": "claude-opus-4"})).unwrap(),
    )
    .unwrap();
    // A transcript with two keyed usage lines + one resumed duplicate.
    std::fs::write(
        managed.join("projects").join("t.jsonl"),
        concat!(
            "{\"type\":\"assistant\",\"uuid\":\"p-1\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:01.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n",
            "{\"type\":\"assistant\",\"uuid\":\"p-2\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:02.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":20,\"output_tokens\":2}}}\n",
            "{\"type\":\"assistant\",\"uuid\":\"p-1\",\"sessionId\":\"s\",\"timestamp\":\"2026-10-15T20:00:01.000Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n",
        ),
    )
    .unwrap();
    // A spool with ONE quota-bearing payload.
    std::fs::create_dir_all(managed.join("quota-spool")).unwrap();
    std::fs::write(
        managed.join("quota-spool").join("statusline.jsonl"),
        "1760000000000\t{\"rate_limits\":{\"five_hour\":{\"used_percentage\":33.0,\"resets_at\":1760006000000}}}\n",
    )
    .unwrap();

    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-pipe"),
            harness: HarnessId::new("claude-code"),
            label: "pipe".into(),
            plan: None,
            state_dir: managed,
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        })
        .unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .profiles()
        .insert(&LaunchRecipe {
            id: ProfileId::new("rec-pipe"),
            name: "pipe".into(),
            account: AccountId::new("acc-pipe"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    // A session row so the account is "real" to any read model.
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&juggler_core::model::SessionRecord {
            id: SessionId::new("sess-pipe"),
            conversation: None,
            profile: ProfileId::new("rec-pipe"),
            machine: MachineId::new("this-mac"),
            kind: juggler_core::model::InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Finished,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

#[test]
fn one_pass_ingests_all_sources_and_repeats_insert_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_claude_account(&state, dir.path());

    // Pass 1 (tick 4 = file-scan tick): spool + transcript + install.
    usage_pipeline::ingest_pass(&state, 4);
    let events = {
        let storage = state.storage.lock().unwrap();
        storage.usage().events_for_account("acc-pipe", 0).unwrap()
    };
    // p-1 once, p-2 once (the duplicate uuid absorbed at the store).
    assert_eq!(events.len(), 2);
    let samples = {
        let storage = state.storage.lock().unwrap();
        storage
            .usage()
            .samples_for_account("acc-pipe", juggler_core::quota::WindowKind::FiveHour)
            .unwrap()
    };
    assert_eq!(samples.len(), 1);
    assert!((samples[0].used_pct - 33.0).abs() < 1e-9);

    // The managed wrapper is installed and CHAINS nothing (no prior).
    let managed = dir.path().join("managed-claude");
    assert!(managed.join("juggler-statusline.sh").exists());

    // Pass 2 (tick 8): no file changes ⇒ zero new rows.
    usage_pipeline::ingest_pass(&state, 8);
    let again = {
        let storage = state.storage.lock().unwrap();
        storage.usage().events_for_account("acc-pipe", 0).unwrap()
    };
    assert_eq!(
        again.len(),
        2,
        "pipeline idempotency: no changes, no inserts"
    );

    // Health: both sources observed.
    let health = usage_pipeline::source_health(&state);
    let sources: Vec<&str> = health.iter().map(|info| info.source.as_str()).collect();
    assert!(sources.contains(&"statusline"), "health: {sources:?}");
    assert!(sources.contains(&"transcript"), "health: {sources:?}");
    let transcript = health
        .iter()
        .find(|info| info.source == "transcript")
        .unwrap();
    assert!(transcript.last_event_at.is_some());
    assert_eq!(transcript.parse_errors, 0);
}

/// The 0.0.12 report: Codex's own limit readings (rollout `token_count`
/// `rate_limits`) reach the store as Measured samples, the account's
/// windows read as measured, and its credits show. Fixture values are
/// invented; the shape is the real line's.
#[test]
fn codex_rollout_limits_reach_the_display_as_measured_with_credits() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/21");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../juggler-adapters/tests/fixtures/quota/codex-rollout-rate-limits.jsonl"),
        day.join("rollout-2026-10-21T10-00-00-th-9.jsonl"),
    )
    .unwrap();
    let account = Account {
        id: AccountId::new("acc-codex"),
        harness: HarnessId::new("codex"),
        label: "codex".into(),
        plan: None,
        state_dir: home,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .insert(&account)
        .unwrap();

    usage_pipeline::ingest_pass(&state, 4);

    let storage = state.storage.lock().unwrap();
    let five = storage
        .usage()
        .samples_for_account("acc-codex", juggler_core::quota::WindowKind::FiveHour)
        .unwrap();
    let week = storage
        .usage()
        .samples_for_account(
            "acc-codex",
            juggler_core::quota::WindowKind::Weekly { anchor_ms: None },
        )
        .unwrap();
    assert_eq!(five.len(), 2);
    assert_eq!(week.len(), 1);

    let now = juggler_adapters::quota_ingest::iso_to_epoch_ms("2026-10-21T10:35:00.000Z").unwrap();
    let info = juggler_app::commands::quota_display_inner(&storage, &account, now);
    let five = info.five_hour.as_ref().unwrap();
    assert_eq!(five.state, "tracking");
    assert!(
        five.percentage.is_some_and(|p| (p - 20.0).abs() < 1e-9),
        "{five:?}"
    );
    assert_eq!(five.badge.as_deref(), Some("measured"));
    assert_eq!(five.reset_basis.as_deref(), Some("measured"));
    let week = info.weekly.as_ref().unwrap();
    assert!(
        week.percentage.is_some_and(|p| (p - 40.0).abs() < 1e-9),
        "{week:?}"
    );
    let credits = info.credits.as_ref().unwrap();
    assert_eq!(credits.balance.as_deref(), Some("16.25"));
    assert!(credits.has_credits && !credits.unlimited);
}
