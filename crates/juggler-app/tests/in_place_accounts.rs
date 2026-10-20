//! ADR-0018: a setup brought in "where it is" — the account uses the
//! user's own folder. AI Juggler writes nothing of its own into it:
//! not at bring-in, not at launch (model and status line go in as flags),
//! not in the usage pipeline. And a copy's settings.json is no longer
//! rewritten for a profile's model.

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

/// A `claude` stub that records its arguments and `CLAUDE_CONFIG_DIR`
/// in a log OUTSIDE the config folder, so the folder stays as it was.
fn claude_stub(dir: &Path) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let log = dir.join("stub.log");
    let path = dir.join("claude-stub");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf 'dir=%s\\n' \"${{CLAUDE_CONFIG_DIR-UNSET}}\" >> '{log}'\nfor a in \"$@\"; do printf 'arg=%s\\n' \"$a\" >> '{log}'; done\n",
            log = log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (path, log)
}

fn app_state(dir: &Path, stub: &Path) -> Arc<AppState> {
    let data = dir.join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin(stub).with_overlays_root(data.join("overlays")),
    ));
    Arc::new(AppState::init_with_registry(data, registry).unwrap())
}

/// The user's own setup: signed in, a session, their own status line.
fn users_setup(root: &Path) -> PathBuf {
    let dir = root.join("ClaudeCodeProfiles/work-a");
    std::fs::create_dir_all(dir.join("projects/-users-dev-app")).unwrap();
    std::fs::write(
        dir.join("settings.json"),
        r#"{"model":"sonnet","statusLine":{"type":"command","command":"my-line"},"hooks":{"Stop":[]}}"#,
    )
    .unwrap();
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    std::fs::write(
        dir.join("projects/-users-dev-app/session-1.jsonl"),
        "{\"type\":\"user\"}\n",
    )
    .unwrap();
    dir
}

/// Every path under `root` with its size and modification time.
fn snapshot(root: &Path) -> Vec<(String, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, u64, std::time::SystemTime)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let meta = std::fs::symlink_metadata(entry.path()).unwrap();
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            out.push((rel, meta.len(), meta.modified().unwrap()));
            if meta.is_dir() {
                walk(&entry.path(), root, out);
            }
        }
    }
    // The folder's own entry too: its mtime changes when anything is
    // created or removed directly inside it.
    let meta = std::fs::metadata(root).unwrap();
    let mut out = vec![(".".to_owned(), meta.len(), meta.modified().unwrap())];
    walk(root, root, &mut out);
    out.sort();
    out
}

fn state_dir(state: &Arc<AppState>, id: &str) -> PathBuf {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(id.to_owned()))
        .unwrap()
        .state_dir
}

fn launch_and_wait(state: &Arc<AppState>, account_id: &str, model: Option<&str>) {
    let storage = state.storage.lock().unwrap();
    let mut main = storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(account_id.to_owned()))
        .unwrap()
        .remove(0);
    main.settings_overlay.model = model.map(str::to_owned);
    storage.profiles().update(&main).unwrap();
    drop(storage);
    commands::launch_profile(state, main.id.as_str(), None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if let Some(juggler_supervisor::LifecycleEvent::Exited { .. }) = state
            .supervisor
            .recv_timeout(std::time::Duration::from_millis(200))
        {
            return;
        }
    }
    panic!("the session never exited");
}

#[test]
fn a_setup_used_where_it_is_is_never_written_by_ai_juggler() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let folder = users_setup(work.path());
    let before = snapshot(&folder);

    let result = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        &format!("{}/", folder.display()),
        "Work A",
        commands::ImportRequest {
            in_place: true,
            origin: Some("manager-tool".into()),
            ..commands::ImportRequest::default()
        },
    )
    .unwrap();
    assert!(result.account.in_place);
    assert_eq!(result.files_copied, 0);
    assert_eq!(result.login, "logged_in", "the sign-in is the folder's own");
    assert_eq!(
        state_dir(&state, &result.account.id),
        folder,
        "the exact folder, no trailing separator"
    );

    // Launch with a model: flags, not edits.
    launch_and_wait(&state, &result.account.id, Some("opus"));
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(
        lines.contains(&format!("dir={}\n", folder.display())),
        "{lines}"
    );
    assert!(lines.contains("arg=--model\narg=opus\n"), "{lines}");
    let overlays = work
        .path()
        .join("app-data/overlays")
        .join(&result.account.id);
    assert!(
        lines.contains(&format!(
            "arg=--settings\narg={}",
            overlays.join("launch-settings.json").display()
        )),
        "{lines}"
    );
    // The capture chains the user's own status line, from AI Juggler's folder.
    let wrapper = std::fs::read_to_string(overlays.join("juggler-statusline.sh")).unwrap();
    assert!(wrapper.contains("CHAIN='my-line'"), "{wrapper}");

    // The usage pipeline reads the folder but installs nothing into it.
    juggler_app::usage_pipeline::ingest_pass(&state, 0);

    // Its checklist passes, honestly worded.
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:?}");
    assert!(
        rows.iter()
            .any(|row| row.label == "Uses your folder where it is")
    );

    // The same folder can't come in twice.
    let again = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        folder.to_str().unwrap(),
        "Work A again",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    );
    assert!(again.is_err());

    assert_eq!(
        snapshot(&folder),
        before,
        "nothing in the user's folder changed"
    );
}

#[test]
fn a_copy_keeps_its_settings_file_when_a_profile_sets_a_model() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let folder = users_setup(work.path());

    let result = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        folder.to_str().unwrap(),
        "Work A copy",
        None,
        None,
        false,
    )
    .unwrap();
    assert!(!result.account.in_place);
    let copy = state_dir(&state, &result.account.id);
    let settings = std::fs::read_to_string(copy.join("settings.json")).unwrap();

    launch_and_wait(&state, &result.account.id, Some("opus"));
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(lines.contains("arg=--model\narg=opus\n"), "{lines}");
    assert!(
        !lines.contains("arg=--settings"),
        "copies keep their own status line install"
    );
    assert_eq!(
        std::fs::read_to_string(copy.join("settings.json")).unwrap(),
        settings,
        "the copy's hooks and status line survive a launch with a model"
    );
}

#[test]
fn ai_juggler_s_own_copy_or_a_non_setup_cannot_be_used_in_place() {
    let work = tempfile::tempdir().unwrap();
    let (stub, _) = claude_stub(work.path());
    let state = app_state(work.path(), &stub);
    let folder = users_setup(work.path());
    let copy = commands::account_import_impl(
        &state,
        CLAUDE_CODE_ID,
        folder.to_str().unwrap(),
        "Copy",
        None,
        None,
        false,
    )
    .unwrap();
    let in_place = |path: &str| {
        commands::account_import_mode_impl(
            &state,
            CLAUDE_CODE_ID,
            path,
            "x",
            commands::ImportRequest {
                in_place: true,
                ..commands::ImportRequest::default()
            },
        )
    };
    let copy_dir = state_dir(&state, &copy.account.id);
    assert!(
        in_place(copy_dir.to_str().unwrap()).is_err(),
        "our own copy"
    );
    let empty = work.path().join("not-claude");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(in_place(empty.to_str().unwrap()).is_err(), "not a setup");
    assert!(
        in_place(work.path().join("gone").to_str().unwrap()).is_err(),
        "missing"
    );
}
