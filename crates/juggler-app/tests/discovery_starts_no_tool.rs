//! The 0.2 walk, F19: a bridged Dev build ran each tool's `--version`
//! within its first minute (`switch_targets` warms the tool versions for
//! the fidelity register; Settings and a tool's page discover what's
//! installed). Under the read-only test bridge AI Juggler starts no vendor
//! program: with `tool_programs` stopped (what `dev_bridge` does), every
//! read that discovers a tool reads its files only. Stand-in tools that
//! record each run show it: none of them ran, while the same reads with
//! tools allowed do run them (so the stand-ins would have told).

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp, TranscriptRef,
};

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

/// A stand-in tool that writes each run to `record`, then says a version.
fn recording_tool(dir: &Path, name: &str, version: &str, record: &Path) -> PathBuf {
    let path = dir.join(name);
    stand_in::write(
        &path,
        format!(
            "#!/bin/sh\necho \"{name} $*\" >> '{}'\necho '{version}'\n",
            record.display()
        ),
    );
    path
}

struct Fixture {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    record: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let record = root.join("runs.txt");
    std::fs::write(&record, "").unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(recording_tool(
        &root,
        "claude",
        "1.0.0 (Claude Code)",
        &record,
    ))));
    registry.register(Arc::new(CodexAdapter::with_bin(recording_tool(
        &root,
        "codex",
        "codex-cli 1.0.0",
        &record,
    ))));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let transcript = root.join("conversation.jsonl");
    std::fs::write(
        &transcript,
        "{\"type\":\"user\",\"timestamp\":\"2026-10-28T10:00:00.000Z\",\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        for (id, tool) in [("acc-claude", CLAUDE_CODE_ID), ("acc-codex", CODEX_ID)] {
            let state_dir = root.join("accounts").join(id);
            std::fs::create_dir_all(&state_dir).unwrap();
            storage
                .accounts()
                .insert(&Account {
                    id: AccountId::new(id),
                    harness: HarnessId::new(tool),
                    label: format!("Account {id}"),
                    plan: None,
                    state_dir,
                    cleanup_opt_in: false,
                    renewal_date: None,
                    preview_opt_in: false,
                    import_source_path: None,
                    import_origin: None,
                    imported_at: None,
                    in_place: false,
                })
                .unwrap();
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new(format!("rec-{id}")),
                    name: "Main".into(),
                    account: AccountId::new(id),
                    settings_overlay: SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
        }
        let mut session = InstanceRecord {
            id: SessionId::new("s-1"),
            conversation: None,
            profile: ProfileId::new("rec-acc-claude"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: root.clone(),
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage: juggler_core::model::UsageSnapshot::default(),
            attention: None,
            running: None,
            transcript_ref: Some(TranscriptRef {
                path: transcript,
                native_id: None,
            }),
            window: None,
        };
        storage.sessions().insert(&session).unwrap();
        session.status = SessionStatus::Stopped;
        storage.sessions().update(&session).unwrap();
    }
    Fixture {
        _dir: dir,
        state,
        record,
    }
}

fn runs(record: &Path) -> String {
    std::fs::read_to_string(record).unwrap()
}

#[test]
fn under_the_bridge_discovery_reads_files_and_starts_no_tool() {
    let fx = fixture();
    let state = &fx.state;
    state.tool_programs.stop();

    let rows = juggler_app::commands::providers_discover_impl(state).unwrap();
    for row in &rows {
        assert_eq!(
            row.status, "found",
            "{}: found from its files, its version not asked",
            row.provider
        );
        assert!(row.version.is_none(), "no version was asked");
        assert!(row.binary.is_some(), "where it is, from its files");
    }
    juggler_app::commands::switch_targets_impl(state, "s-1").unwrap();
    for tool in [CLAUDE_CODE_ID, CODEX_ID] {
        let update = juggler_app::features::tool_updates::status_impl(state, tool).unwrap();
        assert!(
            update.installed.is_none(),
            "{tool}: its version isn't known"
        );
    }
    assert_eq!(
        runs(&fx.record),
        "",
        "a read started a tool under the bridge"
    );
}

#[test]
fn with_tools_allowed_the_same_reads_ask_their_version() {
    // The control: the stand-ins record a run when discovery asks, so
    // the test above would have seen one.
    let fx = fixture();
    let rows = juggler_app::commands::providers_discover_impl(&fx.state).unwrap();
    assert!(
        rows.iter()
            .all(|row| row.version.as_deref() == Some("1.0.0")),
        "{rows:?}"
    );
    let record = runs(&fx.record);
    assert!(record.contains("claude --version"), "{record}");
    assert!(record.contains("codex --version"), "{record}");
}
