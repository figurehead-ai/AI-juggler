//! ADR-0018 for Codex: a Codex home brought in "where it is". AI Juggler
//! writes nothing of its own into it — not at bring-in, not at launch,
//! not in the usage pass, not for the checklist — and launches point at
//! the folder exactly (Codex's default `~/.codex` runs with its
//! variables unset, as a plain `codex` does). All files are synthetic.

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
use juggler_adapters::CodexAdapter;
use juggler_app::commands;
use juggler_app::error::AppResult;
use juggler_app::state::AppState;

const CODEX: &str = "codex";

/// A `codex` stub that records its arguments and the variables that
/// matter in a log OUTSIDE the home, so the home stays as it was.
fn codex_stub(dir: &Path) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let log = dir.join("stub.log");
    let path = dir.join("codex-stub");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n{{\nprintf 'home=%s\\n' \"${{CODEX_HOME-UNSET}}\"\nprintf 'sqlite=%s\\n' \"${{CODEX_SQLITE_HOME-UNSET}}\"\nprintf 'key=%s\\n' \"${{OPENAI_API_KEY+SET}}\"\nfor a in \"$@\"; do printf 'arg=%s\\n' \"$a\"; done\n}} >> '{log}'\n",
            log = log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (path, log)
}

/// The app with the Codex adapter; `home` stands in for the user's home
/// (where `~/.codex` lives).
fn app_state(dir: &Path, stub: &Path, home: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(CodexAdapter::with_bin(stub).with_home(home)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// The user's own Codex home: signed in (synthetic, no secret), a
/// config, a rollout.
fn users_home(dir: &Path) -> PathBuf {
    let day = dir.join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::write(dir.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
    std::fs::write(
        dir.join("auth.json"),
        r#"{"auth_mode":"chatgpt","tokens":{"refresh_token":"fixture"}}"#,
    )
    .unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../juggler-adapters/tests/fixtures/codex/rollout-paginated.jsonl"),
        day.join("rollout-2026-10-20T08-00-00-0199a213-81c0-7800-8aa1-bbab2a035a53.jsonl"),
    )
    .unwrap();
    dir.to_path_buf()
}

/// Every path under `root` with its size and modification time, the
/// root's own entry too (its mtime moves when anything is created or
/// removed directly inside it).
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

/// Launches the account's Main settings (a model, and an API key in its
/// environment that must never reach Codex) and waits for the stub.
fn launch_and_wait(state: &Arc<AppState>, account_id: &str) {
    let storage = state.storage.lock().unwrap();
    let mut main = storage
        .profiles()
        .list_by_account(&juggler_core::model::AccountId::new(account_id.to_owned()))
        .unwrap()
        .remove(0);
    main.settings_overlay.model = Some("gpt-5-codex".into());
    main.settings_overlay
        .env
        .insert("OPENAI_API_KEY".into(), "placeholder".into());
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

fn in_place(state: &Arc<AppState>, path: &str) -> AppResult<commands::ImportResult> {
    commands::account_import_mode_impl(
        state,
        CODEX,
        path,
        "Codex work",
        commands::ImportRequest {
            in_place: true,
            origin: Some("home-pattern".into()),
            ..commands::ImportRequest::default()
        },
    )
}

fn copy_of(state: &Arc<AppState>, folder: &Path) -> commands::ImportResult {
    commands::account_import_impl(
        state,
        CODEX,
        folder.to_str().unwrap(),
        "Codex work",
        None,
        None,
        false,
    )
    .unwrap()
}

#[test]
fn a_codex_home_used_where_it_is_is_never_written_by_ai_juggler() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = codex_stub(work.path());
    let home = work.path().join("home");
    let folder = users_home(&home.join(".codex-work"));
    let state = app_state(work.path(), &stub, &home);
    let before = snapshot(&folder);

    let result = in_place(&state, &format!("{}/", folder.display())).unwrap();
    assert!(result.account.in_place);
    assert_eq!(result.files_copied, 0);
    assert_eq!(result.login, "logged_in", "the sign-in is the home's own");
    assert_eq!(
        state_dir(&state, &result.account.id),
        folder,
        "the exact folder, no trailing separator"
    );

    // Both variables point at the folder exactly; the key never reaches
    // Codex; the model goes in as a flag, not into config.toml.
    launch_and_wait(&state, &result.account.id);
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(
        lines.contains(&format!("home={}\n", folder.display())),
        "CODEX_HOME is the folder"
    );
    assert!(
        lines.contains(&format!("sqlite={}\n", folder.display())),
        "CODEX_SQLITE_HOME is the folder"
    );
    assert!(lines.contains("key=\n"), "the API key is unset");
    assert!(lines.contains("arg=--model\narg=gpt-5-codex\n"));
    assert!(lines.contains("arg=--no-daemon\n"));

    // The usage pass reads the rollouts; the checklist reads the folder.
    juggler_app::usage_pipeline::ingest_pass(&state, 0);
    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(
        rows.iter().all(|row| row.pass),
        "every checklist row passes"
    );
    let env_row = rows
        .iter()
        .find(|row| row.id == "launch_env_managed_only")
        .unwrap();
    assert_eq!(
        env_row.detail,
        "CODEX_HOME and CODEX_SQLITE_HOME are this folder, exactly"
    );

    // The same home can't come in twice.
    assert!(in_place(&state, folder.to_str().unwrap()).is_err());

    assert_eq!(
        snapshot(&folder),
        before,
        "nothing in the user's Codex home changed"
    );
}

#[test]
fn codex_s_default_home_runs_with_its_variables_unset() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = codex_stub(work.path());
    let home = work.path().join("home");
    let folder = users_home(&home.join(".codex"));
    let state = app_state(work.path(), &stub, &home);
    let before = snapshot(&folder);

    let result = in_place(&state, folder.to_str().unwrap()).unwrap();
    launch_and_wait(&state, &result.account.id);
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(lines.contains("home=UNSET\n"), "CODEX_HOME unset");
    assert!(lines.contains("sqlite=UNSET\n"), "CODEX_SQLITE_HOME unset");
    assert!(lines.contains("key=\n"), "the API key is unset");

    let rows = commands::isolation_check_impl(&state, &result.account.id).unwrap();
    assert!(
        rows.iter().all(|row| row.pass),
        "every checklist row passes"
    );
    let env_row = rows
        .iter()
        .find(|row| row.id == "launch_env_managed_only")
        .unwrap();
    assert_eq!(
        env_row.detail,
        "the default folder runs as Codex runs it (CODEX_HOME and CODEX_SQLITE_HOME unset)"
    );
    assert_eq!(snapshot(&folder), before);
}

#[test]
fn ai_juggler_s_own_codex_copy_or_a_non_home_cannot_be_used_in_place() {
    let work = tempfile::tempdir().unwrap();
    let (stub, _) = codex_stub(work.path());
    let home = work.path().join("home");
    let folder = users_home(&home.join(".codex-work"));
    let state = app_state(work.path(), &stub, &home);
    let copy = copy_of(&state, &folder);
    let copy_dir = state_dir(&state, &copy.account.id);
    let refused = in_place(&state, copy_dir.to_str().unwrap()).unwrap_err();
    assert!(
        refused
            .wire()
            .user_message
            .contains("one of AI Juggler's own copies")
    );
    let empty = work.path().join("not-codex");
    std::fs::create_dir_all(&empty).unwrap();
    let refused = in_place(&state, empty.to_str().unwrap()).unwrap_err();
    assert!(refused.wire().user_message.contains("Codex home"));
    assert!(in_place(&state, work.path().join("gone").to_str().unwrap()).is_err());
}

/// A Codex copy can switch to its original home — the same command as
/// Claude's: the copy stays on disk, launches point at the original,
/// and the original is untouched.
#[test]
fn a_codex_copy_can_switch_to_its_original_home() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = codex_stub(work.path());
    let home = work.path().join("home");
    let folder = users_home(&home.join(".codex-work"));
    let state = app_state(work.path(), &stub, &home);
    let before = snapshot(&folder);
    let copy = copy_of(&state, &folder);
    let copy_dir = state_dir(&state, &copy.account.id);

    let switched = commands::account_use_original_impl(&state, &copy.account.id).unwrap();
    assert!(switched.in_place);
    assert_eq!(
        state_dir(&state, &copy.account.id).canonicalize().unwrap(),
        folder.canonicalize().unwrap()
    );
    assert!(copy_dir.join("config.toml").exists(), "the copy is kept");

    launch_and_wait(&state, &copy.account.id);
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(
        lines.contains(&format!(
            "home={}\n",
            folder.canonicalize().unwrap().display()
        )),
        "launches point at the original"
    );
    assert_eq!(snapshot(&folder), before, "the original is untouched");
}

/// A copy of Codex's default `~/.codex` switches back to `~/.codex`
/// itself — found through the adapter's own home (the same one its launch
/// rule uses), so it then runs with both variables unset. The temp home
/// sits under a link on macOS (`/var` → `/private/var`): the canonical
/// provenance must still map back to the exact default path.
#[test]
fn a_copy_of_the_default_home_switches_back_to_it_exactly() {
    let work = tempfile::tempdir().unwrap();
    let (stub, log) = codex_stub(work.path());
    let home = work.path().join("home");
    let folder = users_home(&home.join(".codex"));
    let state = app_state(work.path(), &stub, &home);
    let before = snapshot(&folder);
    let copy = copy_of(&state, &folder);

    let switched = commands::account_use_original_impl(&state, &copy.account.id).unwrap();
    assert!(switched.in_place);
    assert_eq!(
        state_dir(&state, &copy.account.id),
        folder,
        "the exact default path, as the adapter names it"
    );
    launch_and_wait(&state, &copy.account.id);
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(lines.contains("home=UNSET\n"), "CODEX_HOME unset");
    assert!(lines.contains("sqlite=UNSET\n"), "CODEX_SQLITE_HOME unset");
    assert_eq!(snapshot(&folder), before, "the original is untouched");
}

/// The UI's "use it where it is" offer comes from `can_use_in_place` on
/// each provider row; it must agree with what each registered adapter's
/// `adopt_account` actually supports (a refusal other than Unsupported
/// means adopt is implemented).
#[test]
fn the_in_place_flag_matches_what_each_adapter_supports() {
    let registry = juggler_app::state::default_registry();
    let missing = tempfile::tempdir().unwrap().path().join("missing");
    for id in registry.ids() {
        let adapter = registry.get(&id).unwrap();
        let supported = !matches!(
            adapter.adopt_account(&missing),
            Err(juggler_adapter::AdapterError::Unsupported(_))
        );
        assert_eq!(
            adapter.can_use_in_place(),
            supported,
            "{id:?}: the flag and adopt_account disagree"
        );
    }
    let ids: Vec<String> = registry.ids().iter().map(ToString::to_string).collect();
    assert!(ids.contains(&"codex".to_owned()) && ids.contains(&"claude-code".to_owned()));
}

/// The UI reads the flag from `providers_capabilities`, which answers
/// from the registry alone (no install probe), one row per Provider.
#[test]
fn providers_capabilities_reports_the_flag_without_probing() {
    let work = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            work.path().join("app-data"),
            juggler_app::state::default_registry(),
        )
        .unwrap(),
    );
    let rows = commands::providers_capabilities_impl(&state);
    for row in &rows {
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(row.provider.as_str()))
            .unwrap();
        assert_eq!(row.can_use_in_place, adapter.can_use_in_place());
    }
    let in_place: Vec<&str> = rows
        .iter()
        .filter(|row| row.can_use_in_place)
        .map(|row| row.provider.as_str())
        .collect();
    assert!(in_place.contains(&"claude-code") && in_place.contains(&"codex"));
}

/// 0.0.11 walk (Q3): which tools report their own limits comes from the
/// adapter's quota feed — Claude Code does, Codex doesn't (yet).
#[test]
fn providers_capabilities_says_which_tools_report_their_limits() {
    let work = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            work.path().join("app-data"),
            juggler_app::state::default_registry(),
        )
        .unwrap(),
    );
    let rows = commands::providers_capabilities_impl(&state);
    for row in &rows {
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(row.provider.as_str()))
            .unwrap();
        assert_eq!(
            row.reports_limits,
            adapter
                .capabilities()
                .contains(juggler_adapter::Capabilities::USAGE_QUOTA_FEED),
            "{}",
            row.provider
        );
    }
    let of = |id: &str| {
        rows.iter()
            .find(|row| row.provider == id)
            .map(|row| row.reports_limits)
    };
    assert_eq!(of("claude-code"), Some(true));
    // Codex's rollouts carry its limit readings (0.0.12 report).
    assert_eq!(of("codex"), Some(true));
}
