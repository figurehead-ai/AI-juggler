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
    }
}

/// A stub `claude` that answers `--version` with a chosen version string.
fn version_stub(dir: &Path, name: &str, output: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\necho {output:?}\n")).unwrap();
    make_executable(&path);
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

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {
    // Windows discovery fixtures run through the real binary only.
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
