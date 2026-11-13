//! AJ-E1605 integration: ZCode is a registered Provider — listed with its
//! spec, an App gets its own tree and passes the isolation checklist, its
//! copyable line pins the tree, and its sessions become projects with the
//! store left unchanged. Synthetic files and a stand-in `node` only; no
//! `ZCode` runs and no real `~/.zcode` is opened.

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

fn node(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("bin/node");
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
            .with_node(node(dir))
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
fn zcode_is_listed_isolated_and_its_shell_line_pins_its_tree() {
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
    let snippet = expert.shell_snippet.unwrap();
    for key in [
        "ZCODE_STORAGE_DIR",
        "ZCODE_DATA_BASE_DIR",
        "ZCODE_SESSION_DB_PATH",
    ] {
        assert!(snippet.contains(&format!("{key}=")), "{snippet}");
    }
    assert!(snippet.contains("-u ZCODE_SESSION_DB "), "{snippet}");
    // No `zcode` is on a PATH: where the user's PATH is known, the line
    // names the engine by its path instead.
    let line = juggler_adapter::adapter::snippet_program(
        "zcode",
        Some(std::ffi::OsStr::new("/usr/bin:/bin")),
        || {
            Some(PathBuf::from(
                "/Apps/ZCode.app/Contents/Resources/glm/zcode.cjs",
            ))
        },
    );
    assert_eq!(line, "'/Apps/ZCode.app/Contents/Resources/glm/zcode.cjs'");
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
