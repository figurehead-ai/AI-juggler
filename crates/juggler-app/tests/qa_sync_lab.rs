//! QA of AJ-E1507 (rules and servers in sync) beside a walk on a real
//! setup. Both tests are ignored, and do nothing unless their variables are
//! set, so they never run in CI or on a plain `cargo test`.
//!
//! - `sync_lab_register`: after `qa_real_readonly` brought the real setups
//!   in where they are, adds throwaway lab folders as projects in that same
//!   AI Juggler Dev data folder (`AIJ_QA_REAL_DATA`, `AIJ_QA_SYNC_LABS`: the
//!   folders, comma separated), with the first account seen working in each.
//!   It writes only into the Dev data folder, so the read-only bridge can
//!   show the projects' rules and servers.
//! - `sync_lab_writes`: the writes the bridge refuses, on a scratch copy of
//!   a lab (`AIJ_QA_SYNC_COPY`, holding a CLAUDE.md and an AGENTS.md that
//!   differ). Fixture accounts and a stub tool, never a real account: a
//!   session starts there, CLAUDE.md is applied to AGENTS.md and undone in
//!   the sheet, then applied again and undone from Settings, checked byte
//!   for byte. `AIJ_QA_SYNC_OUT` gets what was seen, as JSON.

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
use juggler_adapters::{CLAUDE_CODE_ID, CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::features::project_sync::{apply_impl, diff_impl, rules_impl};
use juggler_app::features::switch_setup::{list_impl, undo_impl};
use juggler_app::state::AppState;

#[test]
#[ignore = "QA beside a real-setup walk; needs AIJ_QA_REAL_DATA and AIJ_QA_SYNC_LABS"]
fn sync_lab_register() {
    let (Some(data), Some(labs)) = (
        std::env::var_os("AIJ_QA_REAL_DATA").map(PathBuf::from),
        std::env::var("AIJ_QA_SYNC_LABS").ok(),
    ) else {
        eprintln!("AIJ_QA_REAL_DATA / AIJ_QA_SYNC_LABS not set: nothing to do");
        return;
    };
    let state = AppState::init(data).unwrap();
    let storage = state.storage.lock().unwrap();
    // The first Claude Code account, as if a session had run there.
    let first = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .filter(|account| account.harness.as_str() == CLAUDE_CODE_ID)
        .map(|account| account.label)
        .min()
        .expect("a Claude Code account brought in");
    for lab in labs.split(',').map(str::trim).filter(|lab| !lab.is_empty()) {
        let root = Path::new(lab).canonicalize().unwrap();
        let name = root.file_name().unwrap().to_string_lossy().into_owned();
        let record = storage.projects().touch(&root, Some(&name)).unwrap();
        storage
            .projects()
            .merge_history(
                &record.id,
                &juggler_core::model::ProjectHistory {
                    accounts: vec![first.clone()],
                    native_sessions: 1,
                    last_used_ms: Some(juggler_core::model::Timestamp::now().millis()),
                },
            )
            .unwrap();
        eprintln!("registered {name}");
    }
}

/// A `claude` and `codex` that do nothing: the session starts, and nothing
/// real runs.
fn stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("tool-stub");
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
#[ignore = "QA beside a real-setup walk; needs AIJ_QA_SYNC_COPY and AIJ_QA_SYNC_OUT"]
fn sync_lab_writes() {
    let (Some(root), Some(out)) = (
        std::env::var_os("AIJ_QA_SYNC_COPY").map(PathBuf::from),
        std::env::var_os("AIJ_QA_SYNC_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_SYNC_COPY / AIJ_QA_SYNC_OUT not set: nothing to do");
        return;
    };
    let root = root.canonicalize().unwrap();
    let work = tempfile::tempdir().unwrap();
    let bin = stub(work.path());
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&bin)));
    registry.register(Arc::new(CodexAdapter::with_bin(&bin)));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());
    let make = |tool: &str, label: &str| {
        let account = commands::create_account(&state, tool, label, None).unwrap();
        let profile = commands::create_profile(&state, &account.id, "Main", None, None).unwrap();
        std::fs::create_dir_all(state.accounts_dir().join(tool).join(&account.id)).unwrap();
        profile.id
    };
    let claude = make(CLAUDE_CODE_ID, "Lab Claude");
    make(CODEX_ID, "Lab Codex");

    // One Claude Code session in the lab: the project exists, and Claude
    // Code has worked in it.
    let launched =
        commands::launch_profile(&state, &claude, Some(root.to_string_lossy().into_owned()))
            .unwrap();
    let project = {
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.as_str(),
            ))
            .unwrap()
            .project
            .expect("the session is in the lab")
            .to_string()
    };
    let name = root.file_name().unwrap().to_string_lossy().into_owned();
    let agents = || std::fs::read(root.join("AGENTS.md")).unwrap();
    let claude_md = std::fs::read(root.join("CLAUDE.md")).unwrap();
    let before = agents();
    assert_ne!(before, claude_md, "the lab's two rules files must differ");

    let sha = |file: &str| {
        let rules = rules_impl(&state, &project, Some("CLAUDE.md")).unwrap();
        rules
            .files
            .iter()
            .find(|f| f.name == file)
            .and_then(|f| f.sha256.clone())
    };
    let apply = || {
        apply_impl(
            &state,
            &project,
            "CLAUDE.md",
            "AGENTS.md",
            "copy",
            sha("AGENTS.md").as_deref(),
            &sha("CLAUDE.md").unwrap(),
        )
        .unwrap()
    };
    let summary = || {
        rules_impl(&state, &project, Some("CLAUDE.md"))
            .unwrap()
            .summary
    };

    let at_start = summary();
    let diff = diff_impl(&state, &project, "CLAUDE.md", "AGENTS.md", "copy").unwrap();

    // Applied, then undone in the sheet (its done line's Undo).
    let first = apply();
    let applied_equal = agents() == claude_md;
    let after_apply = summary();
    undo_impl(&state, first.record.as_deref().unwrap()).unwrap();
    let sheet_undo_equal = agents() == before;
    let after_sheet_undo = summary();

    // Applied again, then undone from Settings › Context packs & secrets.
    let second = apply();
    let again_equal = agents() == claude_md;
    let listed = list_impl(&state).unwrap();
    assert_eq!(listed.len(), 1, "Settings lists the one change");
    let place = listed[0].place.clone();
    let what = listed[0].what.clone();
    undo_impl(&state, &listed[0].record).unwrap();
    let settings_undo_equal = agents() == before;
    let listed_after = list_impl(&state).unwrap().len();
    let backups = std::fs::read_dir(state.data_dir.join("sync-backups"))
        .map(|entries| entries.count())
        .unwrap_or(0);

    let report = serde_json::json!({
        "summary_at_start": at_start,
        "diff": {"added": diff.added, "removed": diff.removed},
        "apply_message": first.message,
        "applied_agents_equals_claude": applied_equal,
        "summary_after_apply": after_apply,
        "sheet_undo_restores_bytes": sheet_undo_equal,
        "summary_after_sheet_undo": after_sheet_undo,
        "second_apply_message": second.message,
        "second_apply_agents_equals_claude": again_equal,
        "settings_row": {"place": place, "what": what, "is_the_lab": place == name},
        "settings_undo_restores_bytes": settings_undo_equal,
        "settings_rows_after_undo": listed_after,
        "backups_left": backups,
    });
    std::fs::write(&out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert!(applied_equal && sheet_undo_equal && again_equal && settings_undo_equal);
    assert_eq!((listed_after, backups), (0, 0));
}
