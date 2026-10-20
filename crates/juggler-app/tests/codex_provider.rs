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
    assert!(env_row.detail.contains("CODEX_HOME"), "{}", env_row.detail);
    let probe = rows
        .iter()
        .find(|row| row.id == "probe_independence")
        .unwrap();
    assert!(probe.detail.contains("signed_in"), "{}", probe.detail);
    assert!(
        rows.iter().all(|row| row.pass),
        "a fresh codex account: {rows:#?}"
    );

    // The Expert-setup snippet points the CLI at this home only.
    let expert =
        commands::expert_setup_info_impl(&state, CODEX_ID, Some(&created.id), None).unwrap();
    let snippet = expert.shell_snippet.unwrap();
    assert!(snippet.starts_with("CODEX_HOME='"), "{snippet}");
    assert!(snippet.ends_with("codex --no-daemon"), "{snippet}");
    assert!(
        expert
            .protected_env_keys
            .contains(&"CODEX_SQLITE_HOME".to_owned())
    );
}

#[test]
fn a_codex_shell_alias_becomes_a_codex_candidate() {
    let work = tempfile::tempdir().unwrap();
    let target = work.path().join(".codex-work");
    std::fs::create_dir_all(&target).unwrap();
    let hint = juggler_adapter::environment::AliasHint {
        variable: "CODEX_HOME".into(),
        path: target.clone(),
        source_file: "~/.zshrc".into(),
        line: 3,
        target_exists: true,
    };
    let (rows, _, dangling) = commands::alias_installs_of(&[hint], &mut []);
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
    assert_eq!(imported.account.login_state, "signed_in");
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
    // The preflight is the first thing that needs the file.
    let _ = commands::switch_preflight_impl(&state, &launched.session_id, &profile_b.id);
    let linked = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&session)
        .unwrap()
        .transcript_ref
        .expect("linked on first use");
    assert_eq!(linked.path, rollout);
    assert_eq!(linked.native_id.as_deref(), Some(thread));
}
