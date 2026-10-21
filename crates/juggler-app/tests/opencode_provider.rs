//! AJ-E1602 integration: OpenCode is a registered Provider — listed, an
//! Account gets its own XDG folders and passes the isolation checklist
//! (its pinned variables are four different folders), its usage comes
//! from `opencode.db` through the pipeline, its quota says honestly
//! that there is no limit to read, its sessions' folders become
//! projects, and its default folder can be used where it is, unwritten.
//! Synthetic files only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/opencode_fixture/mod.rs"]
mod opencode_fixture;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{ClaudeCodeAdapter, OPENCODE_ID, OpenCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;
use opencode_fixture::{Assistant, Line};

fn stub(dir: &Path, name: &str, version: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\necho '{version}'\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The app with OpenCode's home in the sandbox (its default folder).
fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub(
        dir, "claude", "2.1.80",
    ))));
    registry.register(Arc::new(
        OpenCodeAdapter::with_bin(stub(dir, "opencode", "1.19.3")).with_home(dir.join("home")),
    ));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let meta = entry.metadata().unwrap();
            out.push((entry.path(), meta.len(), meta.modified().unwrap()));
            if meta.is_dir() {
                walk(&entry.path(), out);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Two finished assistant messages (and a user one) in `project`.
fn seed_usage(db: &Path, project: &Path) {
    let conn = opencode_fixture::create(db, Line::V1);
    let at = opencode_fixture::now_ms() - 60_000;
    opencode_fixture::add_session(&conn, Line::V1, "ses_1", project, None, at);
    opencode_fixture::add_session(&conn, Line::V1, "ses_1_sub", project, Some("ses_1"), at);
    opencode_fixture::add_user(&conn, Line::V1, "msg_u", "ses_1", at);
    for (id, input) in [("msg_1", 900), ("msg_2", 300)] {
        opencode_fixture::add_assistant(
            &conn,
            Line::V1,
            &Assistant {
                id,
                session: "ses_1",
                created_ms: at,
                provider: "anthropic",
                model: "claude-sonnet-5-5",
                input,
                output: 100,
                cache_read: 0,
                cost: 0.01,
                completed: true,
            },
        );
    }
}

#[test]
fn opencode_is_a_listed_provider() {
    let providers = commands::harnesses_list();
    let opencode = providers
        .iter()
        .find(|p| p.id == OPENCODE_ID)
        .expect("listed");
    assert_eq!(opencode.name, "OpenCode");
    assert!(
        juggler_app::state::default_registry()
            .ids()
            .iter()
            .any(|id| id.as_str() == OPENCODE_ID)
    );
}

#[test]
fn an_opencode_account_is_isolated_in_four_folders_and_the_checklist_says_so() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, OPENCODE_ID, "OpenCode Go", None).unwrap();
    assert_eq!(created.provider, OPENCODE_ID);
    assert_eq!(created.login_state, "logged_out");
    let dir = state.accounts_dir().join(OPENCODE_ID).join(&created.id);
    for folder in ["data/opencode", "config/opencode", "state/opencode"] {
        assert!(dir.join(folder).is_dir(), "{folder}");
    }

    // The checklist compares each pinned variable with its own folder —
    // before AJ-E1521's `pinned_env` it demanded every one equal the
    // account folder, which no XDG layout can.
    let rows = commands::isolation_check_impl(&state, &created.id).unwrap();
    let env_row = rows
        .iter()
        .find(|row| row.id == "launch_env_managed_only")
        .unwrap();
    assert!(env_row.pass, "{rows:#?}");
    assert!(
        env_row.detail.contains("XDG_DATA_HOME"),
        "{}",
        env_row.detail
    );
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");

    let expert =
        commands::expert_setup_info_impl(&state, OPENCODE_ID, Some(&created.id), None).unwrap();
    let snippet = expert.shell_snippet.unwrap();
    let data = dir.join("data");
    assert!(
        snippet.contains(&format!("XDG_DATA_HOME='{}'", data.display())),
        "{snippet}"
    );
    assert!(snippet.contains("-u OPENCODE_AUTH_CONTENT"), "{snippet}");
    assert!(snippet.ends_with(" opencode"), "{snippet}");
    assert!(
        expert
            .protected_env_keys
            .contains(&"OPENCODE_CONFIG_DIR".to_owned())
    );
}

#[test]
fn usage_comes_from_the_database_once_and_quota_says_there_is_no_limit_to_read() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, OPENCODE_ID, "OpenCode", None).unwrap();
    let dir = state.accounts_dir().join(OPENCODE_ID).join(&created.id);
    seed_usage(&dir.join("data/opencode/opencode.db"), Path::new("/w/app"));

    juggler_app::usage_pipeline::ingest_pass(&state, 0);
    juggler_app::usage_pipeline::ingest_pass(&state, 4);
    let storage = state.storage.lock().unwrap();
    let events = storage.usage().events_for_account(&created.id, 0).unwrap();
    assert_eq!(
        events.len(),
        2,
        "two messages, read twice, stored once each"
    );
    assert!(
        events
            .iter()
            .all(|e| e.source == juggler_core::quota::UsageSource::Sqlite)
    );
    assert_eq!(events.iter().map(|e| e.tokens.input).sum::<u64>(), 1200);
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == created.id)
        .unwrap();
    let display = commands::quota_display_inner(&storage, &account, opencode_fixture::now_ms());
    assert_eq!(
        display.unavailable_reason.as_deref(),
        Some("OpenCode has no limits to read — tokens and estimated cost only")
    );
}

#[test]
fn an_opencode_sessions_folder_becomes_a_project() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, OPENCODE_ID, "OpenCode", None).unwrap();
    let dir = state.accounts_dir().join(OPENCODE_ID).join(&created.id);
    let project = work.path().join("projects/shop");
    std::fs::create_dir_all(&project).unwrap();
    seed_usage(&dir.join("data/opencode/opencode.db"), &project);

    assert!(commands::projects_backfill_impl(&state).unwrap() >= 1);
    let projects = commands::projects_list_impl(&state).unwrap();
    let shop = projects
        .iter()
        .find(|p| Path::new(&p.root) == project)
        .expect("the session's folder is a project");
    assert_eq!(
        shop.history_sessions, 1,
        "the sub-agent's session isn't counted"
    );
    assert!(
        shop.accounts_used.contains(&"OpenCode".to_owned()),
        "{:?}",
        shop.accounts_used
    );
}

#[test]
fn the_default_folder_is_used_where_it_is_and_never_written() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let data = work.path().join("home/.local/share/opencode");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("auth.json"),
        r#"{"openai":{"type":"oauth","refresh":"made-up","access":"made-up","expires":1}}"#,
    )
    .unwrap();
    seed_usage(&data.join("opencode.db"), Path::new("/w/app"));
    let before = snapshot(&data);

    let result = commands::account_import_mode_impl(
        &state,
        OPENCODE_ID,
        data.to_str().unwrap(),
        "OpenCode",
        commands::ImportRequest {
            in_place: true,
            origin: Some("vendor-default".into()),
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    assert_eq!(result.account.login_state, "signed_in");

    juggler_app::usage_pipeline::ingest_pass(&state, 0);
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    let env_row = rows
        .iter()
        .find(|row| row.id == "launch_env_managed_only")
        .unwrap();
    assert!(
        env_row.detail.contains("as OpenCode runs it"),
        "{}",
        env_row.detail
    );
    let events = state
        .storage
        .lock()
        .unwrap()
        .usage()
        .events_for_account(&result.account.id, 0)
        .unwrap();
    assert_eq!(events.len(), 2, "usage is read where the folder is");

    assert_eq!(snapshot(&data), before, "nothing in the folder changed");

    // Any other OpenCode folder comes in as a copy only.
    let other = work.path().join("xdg/opencode");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("auth.json"), "{}").unwrap();
    let refused = commands::account_import_mode_impl(
        &state,
        OPENCODE_ID,
        other.to_str().unwrap(),
        "Other",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    );
    assert!(refused.is_err());
}
