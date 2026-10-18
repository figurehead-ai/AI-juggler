//! AJ-E208 integration: the redaction policy as the user meets it —
//! the remembered choice (global + per-Provider override) persists
//! across restarts, preflight shows the verbatim policy line and the
//! MASKED detections (values never cross the bridge), unset blocks
//! the first pack (no code path applies a default), and the executed
//! edge records the decision.

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
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
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

/// Seeds a session whose transcript carries a canary key (the
/// preflight's detection input) and returns its id + the B recipe.
fn seeded_session(state: &Arc<AppState>, work: &Path) -> (String, String) {
    let project_dir = work.join("booknyx");
    std::fs::create_dir_all(&project_dir).unwrap();
    let cwd = project_dir.to_string_lossy().into_owned();
    let account_a = commands::create_account(state, CLAUDE_CODE_ID, "Work Max", None).unwrap();
    let account_b = commands::create_account(state, CLAUDE_CODE_ID, "Personal", None).unwrap();
    let profile_a = commands::create_profile(state, &account_a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(state, &account_b.id, "Main", None, None).unwrap();

    let launched = commands::launch_profile(state, &profile_a.id, Some(cwd.clone())).unwrap();
    {
        let setup_args = config_dir_of(state, &account_a.id).join("last-args");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !setup_args.is_file() {
            assert!(std::time::Instant::now() < deadline, "setup stub never ran");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    let transcript = config_dir_of(state, &account_a.id)
        .join("projects")
        .join(cwd.replace('/', "-"))
        .join("native-77.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    std::fs::write(
        &transcript,
        format!(
            "{{\"type\":\"user\",\"uuid\":\"u1\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"deploy with sk-canary12345678 ok\"}}}}\n"
        ),
    )
    .unwrap();
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
            native_id: Some("native-77".to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    (launched.session_id, profile_b.id)
}

#[test]
fn policy_is_remembered_and_persists_across_restart() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());

    // Unset stays unset — nothing applies a default.
    let unset = commands::redaction_policy_get_impl(&state, "claude-code").unwrap();
    assert_eq!(unset.effective, None);
    assert_eq!(unset.source, "unset");
    assert!(unset.line.is_empty());

    // The user chooses a global; the line names it.
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();
    let global = commands::redaction_policy_get_impl(&state, "claude-code").unwrap();
    assert_eq!(global.effective.as_deref(), Some("strip"));
    assert_eq!(global.source, "global");
    assert_eq!(
        global.line,
        "Redaction for claude-code: Strip — replace keys with ••• (your global choice)"
    );

    // A Provider override wins and the line says so.
    commands::redaction_policy_set_override_impl(&state, "codex", Some("keep")).unwrap();
    let overridden = commands::redaction_policy_get_impl(&state, "codex").unwrap();
    assert_eq!(overridden.effective.as_deref(), Some("keep"));
    assert_eq!(overridden.source, "override");
    assert_eq!(
        overridden.line,
        "Redaction for codex: Keep — pack stays on this Mac (override)"
    );
    // Other providers keep the global.
    assert_eq!(
        commands::redaction_policy_get_impl(&state, "claude-code")
            .unwrap()
            .effective
            .as_deref(),
        Some("strip")
    );

    // Unknown wire names are refused, never guessed.
    assert!(commands::redaction_policy_set_global_impl(&state, Some("yolo")).is_err());

    // Clearing the override returns to the global.
    commands::redaction_policy_set_override_impl(&state, "codex", None).unwrap();
    assert_eq!(
        commands::redaction_policy_get_impl(&state, "codex")
            .unwrap()
            .source,
        "global"
    );

    // RESTART (same data dir, fresh state): both choices survive.
    drop(state);
    let reopened = app_state(work.path());
    assert_eq!(
        commands::redaction_policy_get_impl(&reopened, "claude-code")
            .unwrap()
            .effective
            .as_deref(),
        Some("strip"),
        "the remembered choice survives a restart"
    );
}

#[test]
fn preflight_shows_the_line_and_masked_items_only() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let (session_id, target_profile) = seeded_session(&state, work.path());
    commands::redaction_policy_set_global_impl(&state, Some("strip")).unwrap();

    let info = commands::switch_preflight_impl(&state, &session_id, &target_profile).unwrap();
    assert_eq!(
        info.redaction.line,
        "Redaction for claude-code: Strip — replace keys with ••• (your global choice)"
    );
    // The canary is DETECTED and MASKED — the value itself never
    // crosses the bridge.
    assert_eq!(info.redaction_items.len(), 1);
    assert_eq!(info.redaction_items[0].kind, "sk-key");
    assert!(info.redaction_items[0].preview.starts_with('c'));
    assert!(info.redaction_items[0].preview.ends_with("5678"));
    let serialized = serde_json::to_string(&info).unwrap();
    assert!(
        !serialized.contains("sk-canary12345678"),
        "value never serialized"
    );
}

#[test]
fn execute_records_the_redaction_decision_on_the_edge() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let (session_id, target_profile) = seeded_session(&state, work.path());
    commands::redaction_policy_set_global_impl(&state, Some("ask")).unwrap();

    let switched = commands::switch_execute_impl(
        &state,
        &session_id,
        &target_profile,
        "stop-now",
        Some("strip"),
        &[0],
    )
    .unwrap();

    // The edge carries the per-switch decision: policy=strip (the
    // Ask answer), one item un-redacted for this switch only.
    {
        let storage = state.storage.lock().unwrap();
        let rows = storage
            .switch_graph()
            .manifest_for(&juggler_core::model::SwitchEdgeId::new(
                switched.edge_id.clone(),
            ))
            .unwrap();
        let policy_row = rows
            .iter()
            .find(|row| row.kind == "redaction")
            .expect("policy row");
        assert_eq!(
            policy_row.disposition, "strip",
            "the Ask answer is recorded"
        );
        let unredact_row = rows
            .iter()
            .find(|row| row.kind == "redaction-unredact")
            .expect("unredact row");
        assert_eq!(unredact_row.bytes, 1, "one item un-redacted this switch");
    }
}
