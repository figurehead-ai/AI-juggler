//! AJ-E1604 integration: Grok Build is a registered Provider — listed, an
//! Account gets its own home and passes the isolation checklist (its
//! shell line keeps the user's Claude setup out), its limits say they
//! aren't read, its sessions are listed and become projects, and its
//! default home is used where it is, unwritten. Synthetic files only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/grok_fixture/mod.rs"]
mod grok_fixture;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{GROK_BUILD_ID, GrokBuildAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn stub(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("grok");
    std::fs::write(&path, "#!/bin/sh\necho 'grok 1.0.45'\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        GrokBuildAdapter::with_bin(stub(dir)).with_home(dir.join("home")),
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

#[test]
fn grok_build_is_listed_isolated_and_says_its_limits_arent_read() {
    let providers = commands::harnesses_list();
    assert!(
        providers
            .iter()
            .any(|p| p.id == GROK_BUILD_ID && p.name == "Grok Build")
    );
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, GROK_BUILD_ID, "Grok", None).unwrap();
    let rows = commands::isolation_check_impl(&state, &created.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    let expert =
        commands::expert_setup_info_impl(&state, GROK_BUILD_ID, Some(&created.id), None).unwrap();
    let snippet = expert.shell_snippet.unwrap();
    assert!(
        snippet.contains("GROK_CLAUDE_HOOKS_ENABLED='0'"),
        "{snippet}"
    );
    assert!(snippet.contains("-u XAI_API_KEY"), "{snippet}");
    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == created.id)
        .unwrap();
    let display = commands::quota_display_inner(&storage, &account, 1_792_580_400_000);
    assert_eq!(
        display.unavailable_reason.as_deref(),
        Some("Grok Build doesn't report its limits to AI Juggler yet")
    );
}

#[test]
fn its_sessions_become_projects_and_its_default_home_is_never_written() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let source = work.path().join("home/.grok");
    std::fs::create_dir_all(work.path().join("projects/shop")).unwrap();
    let project = work.path().join("projects/shop").canonicalize().unwrap();
    grok_fixture::settings(&source, "made-up");
    grok_fixture::sign_in(&source, "made-up");
    grok_fixture::add_session(&source, "0199aa00-1", &project, "Fix the cart");
    let before = snapshot(&source);
    let result = commands::account_import_mode_impl(
        &state,
        GROK_BUILD_ID,
        source.to_str().unwrap(),
        "Grok",
        commands::ImportRequest {
            in_place: true,
            origin: Some("vendor-default".into()),
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    assert_eq!(result.account.login_state, "signed_in");
    assert!(commands::projects_backfill_impl(&state).unwrap() >= 1);
    let projects = commands::projects_list_impl(&state).unwrap();
    assert!(projects.iter().any(|p| Path::new(&p.root) == project));
    assert_eq!(snapshot(&source), before, "nothing in the home changed");
}
