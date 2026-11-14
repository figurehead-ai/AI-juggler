//! AJ-E1605 integration: ZCode is a registered Provider — listed with its
//! spec, an App gets its own tree and passes the isolation checklist, it
//! offers no copyable terminal line (its app ships no terminal UI), and
//! its sessions become projects with the store left unchanged. Synthetic
//! files and a stand-in runtime only; no `ZCode` runs and no real
//! `~/.zcode` is opened.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test fails loudly on a broken fixture"
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/zcode_fixture/mod.rs"]
mod zcode_fixture;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{ZCODE_ID, ZCodeAdapter};
use juggler_app::state::AppState;

fn runtime(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("bin/ZCode");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "#!/bin/sh\necho '0.16.9'\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let engine = zcode_fixture::app(&dir.join("Applications"), "dev.zcode.app");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ZCodeAdapter::with_engine(engine)
            .with_runtime(runtime(dir))
            .with_home(dir.join("home")),
    ));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn state_dir_of(state: &AppState, id: &str) -> PathBuf {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == id)
        .unwrap()
        .state_dir
}

#[test]
fn zcode_is_listed_isolated_and_offers_no_terminal_line() {
    let registry = juggler_app::state::default_registry();
    let spec = registry
        .get(&juggler_core::model::HarnessId::new(ZCODE_ID))
        .expect("ZCode is registered")
        .spec();
    assert_eq!(spec.display_name, "ZCode");
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, ZCODE_ID, "ZCode", None).unwrap();
    let rows = juggler_app::features::isolation::isolation_check_impl(&state, &created.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    let dir = state_dir_of(&state, &created.id);
    assert!(dir.join(".zcode").is_dir());
    let expert = juggler_app::features::expert::expert_setup_info_impl(
        &state,
        ZCODE_ID,
        Some(&created.id),
        None,
    )
    .unwrap();
    // No terminal UI: no line to type (round 3).
    assert_eq!(expert.shell_snippet, None);
}

#[test]
fn its_sessions_become_projects_and_the_store_is_left_unchanged() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created =
        juggler_app::features::account::create_account(&state, ZCODE_ID, "ZCode", None).unwrap();
    let dir = state_dir_of(&state, &created.id);
    std::fs::create_dir_all(work.path().join("projects/shop")).unwrap();
    let project = work.path().join("projects/shop").canonicalize().unwrap();
    zcode_fixture::add_session(&dir, "sess_0a1b", &project);
    let before = files(&dir.join(".zcode"));
    assert!(
        juggler_app::features::projects::backfill::projects_backfill_impl(&state).unwrap() >= 1
    );
    let projects = juggler_app::features::projects::list::projects_list_impl(&state).unwrap();
    assert!(projects.iter().any(|p| Path::new(&p.root) == project));
    assert_eq!(
        files(&dir.join(".zcode")),
        before,
        "nothing in the tree changed"
    );
}

/// AJ-E1605 Req.4: ZCode's limits are its plan's caps, which the user
/// enters; what it used is computed from its own store. Every surface
/// says the same thing: the provider list shows its limit rows, and the
/// Usage screen shows the 5-hour and weekly windows, never "doesn't
/// report its limits", and never a Measured badge.
#[test]
fn its_limits_are_the_plans_caps_and_never_claimed_read() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let caps = juggler_app::features::providers::providers_capabilities_impl(&state);
    let zcode = caps.iter().find(|c| c.provider == ZCODE_ID).unwrap();
    assert!(zcode.reports_limits, "its limit rows are shown");
    let created =
        juggler_app::features::account::create_account(&state, ZCODE_ID, "ZCode", None).unwrap();
    let dir = state_dir_of(&state, &created.id);
    let conn = zcode_fixture::create(&dir);
    let at = 1_792_580_400_000; // 2026-10-21
    zcode_fixture::usage(&conn, "mu_1", "GLM-5.3", "completed", at + 10, 1000);
    drop(conn);
    let storage = state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .find(|a| a.id.as_str() == created.id)
        .unwrap();
    let display =
        juggler_app::features::quota::quota_display_inner(&storage, &account, at + 60_000);
    assert_eq!(display.unavailable_reason, None);
    let five = display.five_hour.expect("the 5-hour window is shown");
    let week = display.weekly.expect("the weekly window is shown");
    for window in [&five, &week] {
        assert_ne!(window.badge.as_deref(), Some("Measured"), "{window:?}");
    }
}
