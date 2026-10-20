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
