//! AJ-E1301 integration: Codex is a registered Provider — it is listed,
//! an Account gets its own `CODEX_HOME`, its sign-in state comes from the
//! Codex probe (not a hard-coded "logged out"), and the isolation check's
//! rows name Codex's own variable. Synthetic files only.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CODEX_ID, ClaudeCodeAdapter, CodexAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

fn stub(dir: &Path, name: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\necho 'codex-cli 0.163.0'\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(stub(dir, "claude"))));
    registry.register(Arc::new(CodexAdapter::with_bin(stub(dir, "codex"))));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

#[test]
fn codex_is_a_listed_provider() {
    let providers = commands::harnesses_list();
    let codex = providers
        .iter()
        .find(|p| p.id == CODEX_ID)
        .expect("codex listed");
    assert_eq!(codex.name, "Codex");
    assert!(providers.iter().any(|p| p.id == "claude-code"));
    assert!(
        juggler_app::state::default_registry()
            .ids()
            .iter()
            .any(|id| id.as_str() == CODEX_ID)
    );
}

#[test]
fn a_codex_account_gets_its_own_home_and_real_sign_in_state() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let created = commands::create_account(&state, CODEX_ID, "Codex Plus", None).unwrap();
    assert_eq!(created.provider, CODEX_ID);
    assert_eq!(created.login_state, "logged_out");

    let dir = state.accounts_dir().join(CODEX_ID).join(&created.id);
    assert!(dir.is_dir(), "codex refuses a missing CODEX_HOME");
    // The vendor login writes auth.json (synthetic here).
    std::fs::write(
        dir.join("auth.json"),
        r#"{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"refresh_token":"fixture"}}"#,
    )
    .unwrap();
    let listed = state.storage.lock().unwrap().accounts().list().unwrap();
    let account = listed.iter().find(|a| a.id.as_str() == created.id).unwrap();
    assert_eq!(
        CodexAdapter::new().login_state(&account.state_dir),
        juggler_adapters::LoginState::SignedIn
    );

    // The isolation check names Codex's own variable and passes.
    let rows = commands::isolation_check_impl(&state, &created.id).unwrap();
    let env_row = rows
        .iter()
        .find(|row| row.id == "launch_env_managed_only")
        .unwrap();
    assert!(env_row.pass, "{rows:#?}");
    assert!(
        env_row.detail.contains("CODEX_HOME") && env_row.detail.contains("CODEX_SQLITE_HOME"),
        "the row checks both pinned variables"
    );
    let probe = rows
        .iter()
        .find(|row| row.id == "probe_independence")
        .unwrap();
    assert!(
        probe.detail.contains("signed_in"),
        "the probe reads signed in"
    );
    assert!(
        rows.iter().all(|row| row.pass),
        "a fresh codex account: {rows:#?}"
    );

    // The Expert-setup snippet points the CLI at this home only.
    let expert =
        commands::expert_setup_info_impl(&state, CODEX_ID, Some(&created.id), None).unwrap();
    let snippet = expert.shell_snippet.unwrap();
    assert!(
        snippet.starts_with("CODEX_HOME='") && snippet.contains(" CODEX_SQLITE_HOME='"),
        "the snippet pins the home and the SQLite home"
    );
    assert!(
        snippet.ends_with("codex --no-daemon"),
        "the snippet runs codex without the daemon"
    );
    assert!(
        expert
            .protected_env_keys
            .contains(&"CODEX_SQLITE_HOME".to_owned())
    );
}

#[test]
fn a_codex_shell_alias_becomes_a_codex_candidate() {
    // Through the adapter's real rc-file scan, not a hand-built hint.
    let work = tempfile::tempdir().unwrap();
    let target = work.path().join(".codex-work");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        work.path().join(".zshrc"),
        format!("export CODEX_HOME=\"{}\"\n", target.display()),
    )
    .unwrap();
    let hints = juggler_adapters::codex::codex_aliases(work.path());
    assert_eq!(hints.len(), 1);
    let (rows, _, dangling) = commands::alias_installs_of(&hints, &mut []);
    assert_eq!(dangling, 0);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].harness, CODEX_ID);
}

#[test]
fn importing_a_codex_home_copies_it_and_leaves_the_original_alone() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let source = work.path().join(".codex-side");
    std::fs::create_dir_all(source.join("sessions/2026/10/20")).unwrap();
    std::fs::write(source.join("config.toml"), "model = \"gpt-5-codex\"\n").unwrap();
    std::fs::write(
        source.join("auth.json"),
        r#"{"auth_mode":"chatgpt","tokens":{"refresh_token":"fixture"}}"#,
    )
    .unwrap();
    std::fs::write(source.join("state_5.sqlite"), "absolute paths inside").unwrap();
    let listing = |dir: &Path| {
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        for entry in walk(dir) {
            out.push((
                entry
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(&entry).unwrap(),
            ));
        }
        out.sort();
        out
    };
    let before = listing(&source);

    let imported = commands::account_import_impl(
        &state,
        CODEX_ID,
        source.to_str().unwrap(),
        "Side",
        None,
        Some("home-pattern".to_string()),
        false,
    )
    .unwrap();

    assert_eq!(listing(&source), before, "the original is byte-identical");
    assert_eq!(imported.account.provider, CODEX_ID);
    // The sign-in stays with the original (single-use refresh tokens):
    // the copy signs in once, and says so.
    assert_eq!(imported.account.login_state, "logged_out");
    assert!(
        !state
            .accounts_dir()
            .join(CODEX_ID)
            .join(&imported.account.id)
            .join("auth.json")
            .exists()
    );
    let dir = state
        .accounts_dir()
        .join(CODEX_ID)
        .join(&imported.account.id);
    assert!(dir.join("config.toml").is_file());
    assert!(
        !dir.join("state_5.sqlite").exists(),
        "a copied index would point at the original's sessions"
    );
    let rows = commands::isolation_check_impl(&state, &imported.account.id).unwrap();
    assert!(rows.iter().all(|row| row.pass), "{rows:#?}");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// AJ-E1303: a Codex session the app launched gets its rollout linked
/// the first time a switch looks for it — no file picking, no watcher
/// wiring in the test (the production path).
#[test]
fn a_launched_codex_session_is_linked_to_its_rollout_on_first_use() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, CODEX_ID, "Plus", None).unwrap();
    let b = commands::create_account(&state, CODEX_ID, "Pro", None).unwrap();
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();

    // What Codex writes when the session starts (synthetic).
    let thread = "019a0000-0000-7000-8000-0000000000e1";
    let day = state
        .accounts_dir()
        .join(CODEX_ID)
        .join(&a.id)
        .join("sessions/2026/10/20");
    std::fs::create_dir_all(&day).unwrap();
    let rollout = day.join(format!("rollout-2026-10-20T08-00-00-{thread}.jsonl"));
    std::fs::write(
        &rollout,
        format!(
            "{{\"timestamp\":\"2026-10-20T08:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread}\",\"cwd\":\"{cwd}\"}}}}\n\
             {{\"timestamp\":\"2026-10-20T08:00:01.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"hello\"}}]}}}}\n"
        ),
    )
    .unwrap();

    let session = juggler_core::model::SessionId::new(launched.session_id.clone());
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&session)
            .unwrap()
            .transcript_ref
            .is_none()
    );
    // The preflight reads the file but never writes the link down …
    let preflight = commands::switch_preflight_impl(&state, &launched.session_id, &profile_b.id);
    assert!(
        !matches!(&preflight, Err(error) if error.to_string().contains("Couldn't tell which file")),
        "the preflight found the session's own file"
    );
    let stored = |state: &Arc<AppState>| {
        state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&session)
            .unwrap()
            .transcript_ref
    };
    assert!(
        stored(&state).is_none(),
        "a preflight never persists a link"
    );
    // … the switch does (it may stop later — the link is the session's).
    let _ = commands::switch_execute_impl(
        &state,
        &launched.session_id,
        &profile_b.id,
        "finish-turn",
        None,
        &[],
    );
    let linked = stored(&state).expect("linked by the switch");
    assert_eq!(linked.path, rollout);
    assert_eq!(linked.native_id.as_deref(), Some(thread));
}

/// Two sessions on one account in one folder: Codex writes a rollout only
/// on the first turn, so the only file may be the OTHER session's.
/// Linking must never guess — the switch asks instead.
fn two_sessions_one_file(provider: &str, write: impl Fn(&Path, &str)) {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    let project = work.path().join("storefront");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = commands::create_account(&state, provider, "A", None).unwrap();
    let b = commands::create_account(&state, provider, "B", None).unwrap();
    let recipe_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let recipe_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let s1 = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();
    let s2 = commands::launch_profile(&state, &recipe_a.id, Some(cwd.clone())).unwrap();
    // S1's file appears after both launches.
    write(&state.accounts_dir().join(provider).join(&a.id), &cwd);

    for session in [&s1.session_id, &s2.session_id] {
        let error = commands::switch_preflight_impl(&state, session, &recipe_b.id).unwrap_err();
        assert!(
            error.to_string().contains("Couldn't tell which file"),
            "ambiguous: the switch asks instead of guessing"
        );
        let _ =
            commands::switch_execute_impl(&state, session, &recipe_b.id, "finish-turn", None, &[]);
        let stored = state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&juggler_core::model::SessionId::new(session.clone()))
            .unwrap()
            .transcript_ref;
        assert!(stored.is_none(), "nothing persisted for an ambiguous match");
    }
}

#[test]
fn a_codex_switch_never_takes_another_sessions_rollout() {
    two_sessions_one_file(CODEX_ID, |home, cwd| {
        let day = home.join("sessions/2026/10/20");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(
            day.join("rollout-2026-10-20T08-00-00-019a0000-0000-7000-8000-0000000000e2.jsonl"),
            format!(
                "{{\"timestamp\":\"t\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"019a0000-0000-7000-8000-0000000000e2\",\"cwd\":\"{cwd}\"}}}}\n"
            ),
        )
        .unwrap();
    });
}

#[test]
fn a_claude_switch_never_takes_another_sessions_transcript() {
    two_sessions_one_file(juggler_adapters::CLAUDE_CODE_ID, |home, cwd| {
        let dir = home.join("projects").join(cwd.replace('/', "-"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("claude-s1.jsonl"),
            format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":{{\"role\":\"user\",\"content\":\"hi\"}}}}\n"),
        )
        .unwrap();
    });
}
