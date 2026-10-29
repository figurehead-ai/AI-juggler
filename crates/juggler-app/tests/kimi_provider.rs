//! AJ-E1603 integration: Kimi Code is a registered Provider — listed, an
//! Account gets its own home and passes the isolation checklist, its
//! limits say honestly they aren't read, its sessions are listed and
//! become projects, its default home is used where it is unwritten, and a
//! membership account that crashes is never started again by itself
//! (its plans are for interactive use only). Synthetic files only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-adapters/tests/kimi_fixture/mod.rs"]
mod kimi_fixture;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{KIMI_CODE_ID, KimiCodeAdapter};
use juggler_app::commands;
use juggler_app::features::sessions::restart::{self, Ending};
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

/// A `kimi` that prints its version, and crashes when run otherwise.
fn crashing_kimi(dir: &Path) -> PathBuf {
    let path = dir.join("kimi");
    stand_in::write(
        &path,
        "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 2.3.0; exit 0; fi\necho 'panicked' >&2\nexit 3\n",
    );
    path
}

/// The app with Kimi Code's home in the sandbox.
fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        KimiCodeAdapter::with_bin(crashing_kimi(dir)).with_home(dir.join("home")),
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
fn kimi_code_is_a_listed_provider() {
    let providers = tauri::async_runtime::block_on(commands::harnesses_list());
    let kimi = providers
        .iter()
        .find(|p| p.id == KIMI_CODE_ID)
        .expect("listed");
    assert_eq!(kimi.name, "Kimi Code");
}

#[test]
fn a_kimi_account_has_its_own_home_and_its_limits_say_they_arent_read() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    assert_eq!(created.login_state, "logged_out");
    let rows = commands::isolation_check_impl(&state, &created.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    let expert =
        commands::expert_setup_info_impl(&state, KIMI_CODE_ID, Some(&created.id), None).unwrap();
    let snippet = expert.shell_snippet.unwrap();
    assert!(
        snippet.contains(&format!("KIMI_CODE_HOME='{}'", dir.display())),
        "{snippet}"
    );
    assert!(snippet.ends_with(" kimi"), "{snippet}");

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
        Some("Kimi Code doesn't report its limits to AI Juggler yet")
    );
}

#[test]
fn its_sessions_are_listed_and_their_folders_become_projects() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    let project = work
        .path()
        .join("projects/shop")
        .canonicalize()
        .unwrap_or_else(|_| {
            std::fs::create_dir_all(work.path().join("projects/shop")).unwrap();
            work.path().join("projects/shop").canonicalize().unwrap()
        });
    kimi_fixture::add_session(&dir, "ses-1", &project, "Fix the cart");

    assert!(commands::projects_backfill_impl(&state).unwrap() >= 1);
    let projects = commands::projects_list_impl(&state).unwrap();
    let shop = projects
        .iter()
        .find(|p| Path::new(&p.root) == project)
        .expect("the session's folder is a project");
    assert!(shop.accounts_used.contains(&"Kimi".to_owned()));
    let listed = state.conversations.all().unwrap();
    let conversation = listed
        .iter()
        .find(|c| c.summary.id == "ses-1")
        .expect("the session is listed");
    assert_eq!(conversation.tool, KIMI_CODE_ID);
    assert_eq!(conversation.summary.title.as_deref(), Some("Fix the cart"));
    let line =
        juggler_app::features::conversations::resume_command_impl(&state, &created.id, "ses-1")
            .unwrap();
    assert!(line.ends_with("kimi '--session' 'ses-1'"), "{line}");
}

#[test]
fn the_default_home_is_used_where_it_is_and_never_written() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let source = work.path().join("home/.kimi-code");
    kimi_fixture::settings(&source, "made-up");
    kimi_fixture::sign_in_membership(&source, "made-up");
    kimi_fixture::add_session(&source, "ses-1", Path::new("/w/app"), "old");
    let before = snapshot(&source);
    let result = commands::account_import_mode_impl(
        &state,
        KIMI_CODE_ID,
        source.to_str().unwrap(),
        "Kimi",
        commands::ImportRequest {
            in_place: true,
            origin: Some("vendor-default".into()),
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    assert_eq!(result.account.login_state, "signed_in");
    juggler_app::usage_pipeline::ingest_pass(&state, 0);
    let _ = commands::projects_backfill_impl(&state);
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
    assert_eq!(snapshot(&source), before, "nothing in the home changed");
}

/// Waits for `session`'s exit and stores it as the forwarder would.
fn crashed(state: &Arc<AppState>, session: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "{session} never exited");
        if let Some(event) = state.supervisor.recv_timeout(Duration::from_millis(200)) {
            let exited = matches!(&event, juggler_supervisor::LifecycleEvent::Exited { session: id, .. } if id.as_str() == session);
            let storage = state.storage.lock().unwrap();
            commands::apply_lifecycle_event(&storage, event, false, None);
            if exited {
                break;
            }
        }
    }
    restart::after_exit(state, &SessionId::new(session), Ending::Crashed);
}

#[test]
fn a_membership_account_that_crashes_is_never_started_again_by_itself() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, KIMI_CODE_ID, "Kimi", None).unwrap();
    let dir = state.accounts_dir().join(KIMI_CODE_ID).join(&created.id);
    kimi_fixture::sign_in_membership(&dir, "made-up");
    let profile = commands::create_profile(
        &state,
        &created.id,
        "Daily",
        None,
        Some(work.path().to_string_lossy().into_owned()),
    )
    .unwrap()
    .id;
    let prefs = juggler_app::settings_policy::SessionPrefsInfo {
        restart_on_crash: true,
        ..juggler_app::settings_policy::session_prefs_get_impl(&state).unwrap()
    };
    juggler_app::settings_policy::session_prefs_set_impl(&state, &prefs).unwrap();

    let first = commands::launch_profile(&state, &profile, None).unwrap();
    crashed(&state, &first.session_id);
    std::thread::sleep(Duration::from_secs(4));
    let storage = state.storage.lock().unwrap();
    assert_eq!(
        storage.sessions().list(None).unwrap().len(),
        1,
        "nothing was started again"
    );
    let history: Vec<String> = storage
        .sessions()
        .lifecycle_events(&SessionId::new(first.session_id.as_str()))
        .unwrap()
        .into_iter()
        .map(|(event, _)| event)
        .collect();
    assert!(
        history
            .iter()
            .any(|event| event.contains("not started again after a crash")
                && event.contains("interactive use only")),
        "{history:?}"
    );
}
