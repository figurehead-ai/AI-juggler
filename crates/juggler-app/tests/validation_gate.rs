//! AJ-E210 app integration: the validation matrix read side
//! (Settings → Beta), the human-run dogfood-cell record path
//! (persisted, restart-pinned), and the badge flow — the ranked
//! switch targets derive their badge + validated state from the SAME
//! matrix (one source, no drift), a failing cell lowers the
//! advertised ceiling into the UI, and unknown pairs are refused
//! (cells are recorded, never invented).

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter, FakeHarnessAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn claude_stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_CONFIG_DIR/last-args\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = claude_stub(dir);
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    registry.register(Arc::new(FakeHarnessAdapter::new(dir.join("fake-harness"))));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// A launched claude source session with a seeded transcript (the
/// ranked-targets read side needs a real session).
fn launched_source(state: &Arc<AppState>, work: &Path) -> String {
    let project_dir = work.join("proj");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();
    let account = commands::create_account(state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let profile = commands::create_profile(state, &account.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(state, &profile.id, Some(cwd.clone())).unwrap();
    let config = state
        .accounts_dir()
        .join(CLAUDE_CODE_ID)
        .join(account.id.as_str());
    let setup_args = config.join("last-args");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !setup_args.is_file() {
        assert!(std::time::Instant::now() < deadline, "setup stub never ran");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let transcript = config
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join("native-9.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    std::fs::write(&transcript, String::new()).unwrap();
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: transcript,
            native_id: Some("native-9".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    launched.session_id
}

#[test]
fn matrix_reads_honestly_and_cells_persist_across_restart() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());

    // The read side: 11 rows, one real-CI row, everything honestly
    // unvalidated with open beta debt.
    let rows = commands::validation_matrix_impl(&state).unwrap();
    assert_eq!(rows.len(), 11);
    let claude = rows
        .iter()
        .find(|row| row.from == "claude-code" && row.to == "claude-code")
        .unwrap();
    assert!(claude.ci_green && claude.mechanism_green);
    assert!(!claude.validated);
    assert_eq!(claude.badge, "F1 · Beta");
    assert!(claude.status.contains("open beta debt"));
    for row in &rows {
        assert!(!row.badge.to_lowercase().contains("lossless"));
        if !row.ci_green {
            assert!(!row.notes.is_empty(), "unproven rows say why");
        }
    }

    // The record path: unknown pairs are refused, never invented.
    let refused = commands::validation_dogfood_record_impl(
        &state,
        "claude-code",
        "made-up-cli",
        "pass",
        "2026-10-16",
        "tiny",
        "aa",
        "bb",
        "note",
    );
    assert!(refused.is_err(), "unknown pair refused");
    // Unknown verdict too.
    assert!(
        commands::validation_dogfood_record_impl(
            &state,
            "claude-code",
            "claude-code",
            "maybe",
            "2026-10-16",
            "tiny",
            "aa",
            "bb",
            "n",
        )
        .is_err()
    );

    // A FAILING cell on the real-CI row: the advertised ceiling drops
    // one level, everywhere the badge is derived.
    commands::validation_dogfood_record_impl(
        &state,
        "claude-code",
        "claude-code",
        "fail",
        "2026-10-16",
        "monorepo",
        "sha-src",
        "sha-dst",
        "S-F4: rules ring lost",
    )
    .unwrap();
    let rows = commands::validation_matrix_impl(&state).unwrap();
    let claude = rows
        .iter()
        .find(|row| row.from == "claude-code" && row.to == "claude-code")
        .unwrap();
    assert_eq!(claude.advertised, "F0", "F1 lowered one level");
    assert_eq!(claude.badge, "F0 · Beta");
    assert_eq!(claude.dogfood.as_ref().unwrap().verdict, "fail");
    assert!(claude.status.contains("ceiling lowered"));

    // Persistence: the cell is restart-pinned (same data dir, fresh
    // process state).
    let data_dir = work.path().join("app-data");
    drop(state);
    let registry = {
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(ClaudeCodeAdapter::new()));
        registry
    };
    let reopened = Arc::new(AppState::init_with_registry(data_dir, registry).unwrap());
    let rows = commands::validation_matrix_impl(&reopened).unwrap();
    let claude = rows
        .iter()
        .find(|row| row.from == "claude-code" && row.to == "claude-code")
        .unwrap();
    assert_eq!(claude.advertised, "F0", "the fail survives the restart");
}

#[test]
fn switch_target_badges_derive_from_the_recorded_cells() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let source = launched_source(&state, work.path());
    let account_b = commands::create_account(&state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    std::fs::write(
        state
            .accounts_dir()
            .join(CLAUDE_CODE_ID)
            .join(account_b.id.as_str())
            .join(".credentials.json"),
        r#"{"stub":true}"#,
    )
    .unwrap();
    commands::create_profile(&state, &account_b.id, "Main", None, None).unwrap();

    // Baseline: the same-provider target reads the researched badge.
    let targets = commands::switch_targets_impl(&state, &source).unwrap();
    let target = targets
        .iter()
        .find(|t| t.account_label == "Personal")
        .unwrap();
    assert_eq!(target.badge, "F1 · Beta");
    assert!(!target.validated);

    // A PASSING cell on the real-CI row graduates the badge — the
    // ranked rows read the same matrix (one source, no drift).
    commands::validation_dogfood_record_impl(
        &state,
        "claude-code",
        "claude-code",
        "pass",
        "2026-10-16",
        "tiny",
        "sha-src",
        "sha-dst",
        "target answered the pre-switch question",
    )
    .unwrap();
    let targets = commands::switch_targets_impl(&state, &source).unwrap();
    let target = targets
        .iter()
        .find(|t| t.account_label == "Personal")
        .unwrap();
    assert_eq!(target.badge, "F1 · Validated");
    assert!(target.validated, "the live cell flipped the row green");

    // A later FAIL replaces it: the ceiling lowers INTO the ranked
    // rows (the honest recovery — badges follow evidence).
    commands::validation_dogfood_record_impl(
        &state,
        "claude-code",
        "claude-code",
        "fail",
        "2026-10-17",
        "tiny",
        "sha-src",
        "sha-dst",
        "S-F6: target launch failed",
    )
    .unwrap();
    let targets = commands::switch_targets_impl(&state, &source).unwrap();
    let target = targets
        .iter()
        .find(|t| t.account_label == "Personal")
        .unwrap();
    assert_eq!(target.badge, "F0 · Beta");
    assert!(!target.validated);
}
