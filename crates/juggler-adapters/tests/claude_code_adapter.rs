//! Claude Code adapter tests (M1-2): fixture-driven discovery, account
//! isolation, metadata-only login detection, launch specs.

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
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter, LoginState};
use juggler_core::model::{AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(unix)]
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn account(state_dir: &Path) -> juggler_core::model::Account {
    juggler_core::model::Account {
        id: AccountId::new("claude-acct"),
        harness: HarnessId::new(CLAUDE_CODE_ID),
        label: "Work Max".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

/// A stub `claude` that answers `--version` with a chosen version string.
fn version_stub(dir: &Path, name: &str, output: &str) -> PathBuf {
    let path = dir.join(name);
    let body = format!("#!/bin/sh\necho {output:?}\n");
    #[cfg(unix)]
    stand_in::write(&path, body);
    #[cfg(not(unix))]
    std::fs::write(&path, body).unwrap();
    path
}

/// Discover with one bounded retry: on loaded CI runners, executing a
/// freshly written script can transiently fail (ETXTBSY-class). The
/// assertion contract is unchanged — the retried report must pass.
/// Flake fix recorded per AJ-E806 (quarantine-or-fix rule); expiry n/a.
fn discover_with_retry(adapter: &ClaudeCodeAdapter) -> DiscoveryReport {
    let first = adapter.discover();
    if first.is_installed() {
        return first;
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    adapter.discover()
}

#[test]
fn discover_verified_version_from_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "claude", "2.1.296 (Claude Code)");
    let report: DiscoveryReport = discover_with_retry(&ClaudeCodeAdapter::with_bin(&stub));
    assert!(report.is_installed());
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "2.1.296".into()
        }
    );
    assert_eq!(report.verification, VerificationStatus::Verified);
    assert!(
        report.notes.is_empty(),
        "no drift note for verified version"
    );
    assert_eq!(report.adapter.as_str(), CLAUDE_CODE_ID);
}

#[test]
fn discover_drift_warns_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "claude", "2.1.301 (Claude Code)");
    let report = discover_with_retry(&ClaudeCodeAdapter::with_bin(&stub));
    assert!(report.is_installed(), "drift must not fail discovery");
    assert_eq!(report.verification, VerificationStatus::Unverified);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("drift") && note.contains("2.1.296")),
        "drift note must name the verified versions: {notes:?}",
        notes = report.notes
    );
}

#[test]
fn discover_missing_binary_is_not_found() {
    let report = ClaudeCodeAdapter::with_bin("/nonexistent/claude").discover();
    assert_eq!(report.install, InstallStatus::NotFound);
}

#[test]
fn prepare_account_creates_private_isolated_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("accounts/claude/work-max");
    let adapter = ClaudeCodeAdapter::new();
    let env = adapter.prepare_account(&state).unwrap();

    assert!(state.is_dir());
    assert!(
        juggler_adapters::claude_code::is_managed_dir(&state),
        "managed marker must exist"
    );
    assert_eq!(
        env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(state.to_str().unwrap())
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&state).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "state dir must be owner-private");
        let marker_mode = std::fs::metadata(state.join(".ai-juggler-managed"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(marker_mode & 0o777, 0o600);
    }

    // Idempotent: preparing again succeeds and keeps the env stable.
    let env_again = adapter.prepare_account(&state).unwrap();
    assert_eq!(env, env_again);
}

#[test]
fn login_state_is_metadata_only() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("acct");
    let adapter = ClaudeCodeAdapter::new();
    adapter.prepare_account(&state).unwrap();

    assert_eq!(adapter.login_state(&state), LoginState::LoggedOut);

    // Presence of a non-empty credentials file reads as logged-in.
    // Content is intentionally never read — size metadata only.
    std::fs::write(state.join(".credentials.json"), b"x").unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::SignedIn);

    // An empty file is PARTIAL — an OAuth abandoned halfway
    // (AJ-E308 Req.1), a distinct state from a clean LoggedOut.
    std::fs::write(state.join(".credentials.json"), b"").unwrap();
    assert_eq!(adapter.login_state(&state), LoginState::Partial);
}

#[test]
fn login_spec_points_at_isolated_dir() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "claude", "2.1.296 (Claude Code)");
    let state = dir.path().join("acct");
    let spec = ClaudeCodeAdapter::with_bin(&stub)
        .login_spec(&state)
        .unwrap();
    assert!(spec.needs_pty, "vendor login flow is interactive");
    assert_eq!(
        spec.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(state.to_str().unwrap())
    );
}

#[test]
fn launch_specs_merge_overlay_env() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "claude", "2.1.296 (Claude Code)");
    let adapter = ClaudeCodeAdapter::with_bin(&stub);
    let state = dir.path().join("acct");
    let acct = account(&state);
    let mut overlay = SettingsOverlay::default();
    overlay.env.insert("ANTHROPIC_MODEL".into(), "opus".into());
    let profile = Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: acct.id.clone(),
        settings_overlay: overlay,
        default_cwd: None,
    };

    let interactive = adapter
        .launch_spec(&acct, &profile, dir.path(), &LaunchMode::Interactive)
        .unwrap();
    assert!(interactive.needs_pty);
    assert_eq!(
        interactive.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        Some(state.to_str().unwrap())
    );
    assert_eq!(
        interactive.env.get("ANTHROPIC_MODEL").map(String::as_str),
        Some("opus")
    );

    let headless = adapter
        .launch_spec(
            &acct,
            &profile,
            dir.path(),
            &LaunchMode::Headless {
                prompt: "fix the flaky test".into(),
            },
        )
        .unwrap();
    assert!(!headless.needs_pty);
    assert!(headless.args.contains(&"-p".to_string()));
    assert!(headless.args.contains(&"fix the flaky test".to_string()));
    assert!(headless.args.contains(&"stream-json".to_string()));
}

#[test]
fn unmanaged_dirs_are_recognizable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!juggler_adapters::claude_code::is_managed_dir(dir.path()));
    let _guard: BTreeMap<(), ()> = BTreeMap::new(); // keep import used on all targets
}

#[cfg(unix)]
#[test]
fn a_resume_id_that_looks_like_an_option_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let stub = version_stub(dir.path(), "claude", "2.1.296 (Claude Code)");
    let adapter = ClaudeCodeAdapter::with_bin(&stub);
    let acct = account(&dir.path().join("acct"));
    let profile = Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: acct.id.clone(),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    for id in ["--continue", "-x"] {
        let mode = LaunchMode::Resume {
            session_id: id.into(),
            delta_prompt: None,
        };
        assert!(
            adapter
                .launch_spec(&acct, &profile, dir.path(), &mode)
                .is_err()
        );
    }
}

/// The one rule for a `claude` session (QA on #126): Claude Code's own
/// executable run as its search helper isn't one; `claude` run by name, by
/// a path with spaces, or as the npm package is.
#[test]
fn a_claude_run_as_its_search_helper_is_not_a_session() {
    use juggler_adapters::claude_code::is_cli_process;
    let copy = Path::new(
        "/Users/you/Library/Application Support/ClaudeProfiles/b/claude-code/2.1/abc/claude.app/Contents/MacOS/claude",
    );
    assert!(!is_cli_process(
        "claude",
        Some(copy),
        "ugrep -r --json needle ."
    ));
    assert!(!is_cli_process("claude", Some(copy), "rg needle"));
    assert!(is_cli_process(
        "claude",
        Some(copy),
        &format!("{} --resume abc", copy.display())
    ));
    assert!(is_cli_process(
        "claude",
        Some(Path::new("/Users/you/.local/bin/claude")),
        "claude"
    ));
    assert!(is_cli_process(
        "node",
        Some(Path::new("/usr/local/bin/node")),
        "node /usr/local/lib/node_modules/@anthropic-ai/claude-code/cli.js"
    ));
    assert!(!is_cli_process(
        "rg",
        Some(Path::new("/opt/homebrew/bin/rg")),
        "rg needle"
    ));
    assert!(
        is_cli_process("claude", Some(copy), ""),
        "an unreadable command line"
    );
}

/// The docs fact-check (fix batch 4): AI Juggler never opens Claude's
/// `.credentials.json`. A file whose contents claim it expired long ago
/// still reads as signed in: only its presence and size count. An API key
/// in settings.json is still seen (its field filled), never its value.
#[test]
fn the_credentials_file_is_never_opened_only_seen() {
    use juggler_adapters::claude_code::{ClaudeCodeAdapter, LoginState};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"sk-secret","expiresAt":1},"expires_at":1,"expiry":1}"#,
    )
    .unwrap();
    let adapter = ClaudeCodeAdapter::new();
    assert_eq!(adapter.login_state(dir.path()), LoginState::SignedIn);
    let keyed = tempfile::tempdir().unwrap();
    std::fs::write(
        keyed.path().join("settings.json"),
        r#"{"apiKey":"sk-secret"}"#,
    )
    .unwrap();
    assert_eq!(adapter.login_state(keyed.path()), LoginState::ApiKey);
}
