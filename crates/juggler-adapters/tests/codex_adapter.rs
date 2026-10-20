//! Codex adapter tests (AJ-E1301): fixture-driven discovery, the
//! isolated `CODEX_HOME`, field-shape-only sign-in detection, launch and
//! login specs. Every `auth.json` here is synthetic — no real token.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_adapter::{
    DiscoveryReport, HarnessAdapter, InstallStatus, LaunchMode, VerificationStatus,
};
use juggler_adapters::codex::parse_version;
use juggler_adapters::{CODEX_ID, CodexAdapter, LoginState};
use juggler_core::model::{AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
use std::path::{Path, PathBuf};

fn account(state_dir: &Path) -> juggler_core::model::Account {
    juggler_core::model::Account {
        id: AccountId::new("codex-acct"),
        harness: HarnessId::new(CODEX_ID),
        label: "Codex Plus".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    }
}

fn profile(account: &juggler_core::model::Account, overlay: SettingsOverlay) -> Profile {
    Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: account.id.clone(),
        settings_overlay: overlay,
        default_cwd: None,
    }
}

/// A stub `codex` that answers `--version` like the real 0.163.0 build.
fn version_stub(dir: &Path, output: &str) -> PathBuf {
    let path = dir.join("codex");
    std::fs::write(&path, format!("#!/bin/sh\necho {output:?}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// One bounded retry: executing a freshly written script can fail
/// transiently on loaded runners (same AJ-E806 fix as the Claude tests).
fn discover_with_retry(adapter: &CodexAdapter) -> DiscoveryReport {
    let first = adapter.discover();
    if first.is_installed() {
        return first;
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    adapter.discover()
}

#[test]
fn version_parsing_takes_the_number_after_the_name() {
    assert_eq!(
        parse_version("codex-cli 0.163.0\n").as_deref(),
        Some("0.163.0")
    );
    assert_eq!(parse_version("0.98.0").as_deref(), Some("0.98.0"));
    assert_eq!(parse_version("codex-cli\n"), None);
    assert_eq!(parse_version(""), None);
}

#[cfg(unix)]
#[test]
fn discover_verified_and_drifted_versions() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "codex-cli 0.163.0");
    let report = discover_with_retry(&CodexAdapter::with_bin(&stub));
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "0.163.0".into()
        }
    );
    assert_eq!(report.verification, VerificationStatus::Verified);
    assert_eq!(report.adapter.as_str(), CODEX_ID);

    let other = tempfile::tempdir().unwrap();
    let stub = version_stub(other.path(), "codex-cli 0.170.2");
    let report = discover_with_retry(&CodexAdapter::with_bin(&stub));
    assert!(report.is_installed(), "drift must not fail discovery");
    assert_eq!(report.verification, VerificationStatus::Unverified);
    assert!(report.notes.iter().any(|n| n.contains("0.163.0")));
}

#[test]
fn discover_missing_binary_is_not_found() {
    let report = CodexAdapter::with_bin("/nonexistent/codex").discover();
    assert_eq!(report.install, InstallStatus::NotFound);
}

#[test]
fn prepare_account_creates_the_codex_home_codex_requires() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("accounts/codex/plus");
    let adapter = CodexAdapter::new();
    let env = adapter.prepare_account(&state).unwrap();

    assert!(state.is_dir(), "codex refuses a CODEX_HOME that is missing");
    assert_eq!(
        env.get("CODEX_HOME").map(String::as_str),
        Some(state.to_str().unwrap())
    );
    let marker = std::fs::read_to_string(state.join(".ai-juggler-managed")).unwrap();
    assert!(marker.contains("harness: codex"), "{marker}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&state).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "state dir must be owner-private");
    }
    // Nothing else is written: no config, no auth.
    let names: Vec<String> = std::fs::read_dir(&state)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![".ai-juggler-managed".to_string()]);
    assert_eq!(adapter.prepare_account(&state).unwrap(), env, "idempotent");
}

#[test]
fn two_accounts_never_share_a_home() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = CodexAdapter::new();
    let a = adapter.prepare_account(&dir.path().join("a")).unwrap();
    let b = adapter.prepare_account(&dir.path().join("b")).unwrap();
    assert_ne!(a.get("CODEX_HOME"), b.get("CODEX_HOME"));
}

#[test]
fn login_state_reads_field_shape_only() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("acct");
    let adapter = CodexAdapter::new();
    adapter.prepare_account(&state).unwrap();
    let auth = state.join("auth.json");

    assert_eq!(adapter.login_state(&state), LoginState::LoggedOut);

    std::fs::write(&auth, b"").unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::Partial);

    std::fs::write(
        &auth,
        br#"{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":"fixture.id.token","access_token":"fixture-access","refresh_token":"fixture-refresh","account_id":"acct-fixture-1"},"last_refresh":"2026-10-20T00:00:00Z"}"#,
    )
    .unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::SignedIn);

    std::fs::write(
        &auth,
        br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-fixture"}"#,
    )
    .unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::ApiKey);

    // A ChatGPT sign-in can ALSO hold an exchanged key: auth_mode wins.
    std::fs::write(
        &auth,
        br#"{"auth_mode":"chatgpt","OPENAI_API_KEY":"sk-exchanged","tokens":{"refresh_token":"fixture-refresh"}}"#,
    )
    .unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::SignedIn);

    // Files written before `auth_mode` existed: Codex's fallback order.
    std::fs::write(
        &auth,
        br#"{"OPENAI_API_KEY":null,"tokens":{"access_token":"fixture"}}"#,
    )
    .unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::SignedIn);
    std::fs::write(&auth, br#"{"OPENAI_API_KEY":"sk-fixture"}"#).unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::ApiKey);

    // A mode whose credential is missing is not a sign-in.
    std::fs::write(&auth, br#"{"auth_mode":"chatgpt","tokens":null}"#).unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::LoggedOut);

    // After `codex logout` the file is gone; an emptied object is the
    // same answer.
    std::fs::write(&auth, br#"{"OPENAI_API_KEY":null,"tokens":null}"#).unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::LoggedOut);

    // A damaged file is a probe failure, never "logged out" (AJ-E308).
    std::fs::write(&auth, b"{not json").unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::Unknown);

    assert_eq!(
        adapter.login_state(&dir.path().join("missing")),
        LoginState::Unknown
    );
}

#[cfg(unix)]
#[test]
fn login_spec_runs_the_vendor_flow_in_the_isolated_home() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "codex-cli 0.163.0");
    let state = dir.path().join("acct");
    let spec = CodexAdapter::with_bin(&stub).login_spec(&state).unwrap();
    assert!(spec.needs_pty, "vendor login flow is interactive");
    assert_eq!(spec.args, vec!["login".to_string()]);
    assert_eq!(
        spec.env.get("CODEX_HOME").map(String::as_str),
        Some(state.to_str().unwrap())
    );
}

#[cfg(unix)]
#[test]
fn launch_specs_per_mode() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "codex-cli 0.163.0");
    let adapter = CodexAdapter::with_bin(&stub);
    let state = dir.path().join("acct");
    let acct = account(&state);
    let mut overlay = SettingsOverlay::default();
    overlay.env.insert("RUST_LOG".into(), "warn".into());
    // An override can never repoint the home (AJ-E309 backstop).
    overlay.env.insert("CODEX_HOME".into(), "/elsewhere".into());
    overlay
        .env
        .insert("CODEX_SQLITE_HOME".into(), "/shared".into());
    let recipe = profile(&acct, overlay);
    let spec = |mode: LaunchMode| {
        adapter
            .launch_spec(&acct, &recipe, dir.path(), &mode)
            .unwrap()
    };

    let interactive = spec(LaunchMode::Interactive);
    assert!(interactive.needs_pty);
    assert_eq!(interactive.args, vec!["--no-daemon".to_string()]);
    assert_eq!(
        interactive.env.get("CODEX_HOME").map(String::as_str),
        Some(state.to_str().unwrap())
    );
    assert_eq!(
        interactive.env.get("CODEX_SQLITE_HOME").map(String::as_str),
        Some(state.to_str().unwrap()),
        "an inherited SQLite home must never join two accounts"
    );
    assert_eq!(
        interactive.env.get("RUST_LOG").map(String::as_str),
        Some("warn")
    );

    let primed = spec(LaunchMode::Primed {
        prompt: "continue".into(),
    });
    assert_eq!(primed.args, vec!["--no-daemon", "--", "continue"]);

    let headless = spec(LaunchMode::Headless {
        prompt: "fix it".into(),
    });
    assert!(!headless.needs_pty);
    assert_eq!(headless.args, vec!["exec", "--json", "--", "fix it"]);

    let resume = spec(LaunchMode::Resume {
        session_id: "019a0000-0000-7000-8000-000000000001".into(),
        delta_prompt: Some("while you were away".into()),
    });
    let here = dir.path().to_string_lossy().into_owned();
    assert_eq!(
        resume.args,
        vec![
            "resume",
            "019a0000-0000-7000-8000-000000000001",
            "-C",
            here.as_str(),
            "--no-daemon",
            "--",
            "while you were away"
        ]
    );

    // User text that looks like an option stays text (after `--`).
    let bullet = spec(LaunchMode::Primed {
        prompt: "- fix X".into(),
    });
    assert_eq!(bullet.args, vec!["--no-daemon", "--", "- fix X"]);
    // A session id that looks like an option is refused, never passed.
    for id in ["--last", "-x", ""] {
        assert!(
            adapter
                .launch_spec(
                    &acct,
                    &recipe,
                    dir.path(),
                    &LaunchMode::Resume {
                        session_id: id.into(),
                        delta_prompt: None,
                    },
                )
                .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn model_override_rides_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "codex-cli 0.163.0");
    let adapter = CodexAdapter::with_bin(&stub);
    let acct = account(&dir.path().join("acct"));
    let overlay = SettingsOverlay {
        model: Some("gpt-5-codex".into()),
        ..SettingsOverlay::default()
    };
    let spec = adapter
        .launch_spec(
            &acct,
            &profile(&acct, overlay),
            dir.path(),
            &LaunchMode::Primed {
                prompt: "go".into(),
            },
        )
        .unwrap();
    assert_eq!(
        spec.args,
        vec!["--no-daemon", "--model", "gpt-5-codex", "--", "go"]
    );
}

#[test]
fn isolation_keys_and_snippet() {
    let adapter = CodexAdapter::new();
    assert_eq!(adapter.isolation_env_keys()[0], "CODEX_HOME");
    let snippet = juggler_adapter::adapter::shell_snippet(
        &adapter,
        Path::new("/tmp/acct one"),
        &std::collections::BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(
        snippet,
        "CODEX_HOME='/tmp/acct one' CODEX_SQLITE_HOME='/tmp/acct one' codex --no-daemon"
    );
    assert_eq!(
        adapter.pinned_env_keys(),
        ["CODEX_HOME", "CODEX_SQLITE_HOME"]
    );
}

#[test]
fn keychain_account_matches_codex_naming() {
    let dir = tempfile::tempdir().unwrap();
    let account = juggler_adapters::codex::keychain_account(dir.path()).unwrap();
    assert!(account.starts_with("cli|"), "the account is `cli|<hash>`");
    assert_eq!(account.len(), "cli|".len() + 16);
    assert!(juggler_adapters::codex::keychain_account(&dir.path().join("missing")).is_none());
}

#[test]
fn codex_home_aliases_come_from_the_shell_files() {
    let home = tempfile::tempdir().unwrap();
    let target = home.path().join(".codex-work");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        home.path().join(".zshrc"),
        format!(
            "export PATH=/usr/bin\nexport CODEX_HOME=\"{}\"\nexport CLAUDE_CONFIG_DIR=~/.claude-x\n",
            target.display()
        ),
    )
    .unwrap();
    let hints = juggler_adapters::codex::codex_aliases(home.path());
    assert_eq!(hints.len(), 1, "only CODEX_HOME belongs to Codex");
    assert_eq!(hints[0].variable, "CODEX_HOME");
    assert_eq!(hints[0].path, target);
    assert!(hints[0].target_exists);
    assert_eq!(hints[0].line, 2);
}
