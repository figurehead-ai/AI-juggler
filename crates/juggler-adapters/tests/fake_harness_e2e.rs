//! End-to-end test of the adapter contract against the real fake-harness
//! binary: discover → prepare account → launch spec → run → watch → resume
//! (M0-4 acceptance).

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::process::Command;
use std::time::{Duration, Instant};

use juggler_adapter::{
    AdapterRegistry, Capabilities, DiscoveryReport, HarnessAdapter, InstallStatus, LaunchMode,
    SessionEvent, SessionWatcher, VerificationStatus,
};
use juggler_adapters::{FAKE_HARNESS_ID, FakeHarnessAdapter};
use juggler_core::model::{AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};

fn fake_bin() -> String {
    option_env!("CARGO_BIN_EXE_fake-harness")
        .or_else(|| option_env!("CARGO_BIN_EXE_fake_harness"))
        .map(String::from)
        .unwrap_or_else(|| panic!("fake-harness binary not available to tests"))
}

fn adapter() -> FakeHarnessAdapter {
    FakeHarnessAdapter::new(fake_bin())
}

fn test_account(state_dir: &std::path::Path) -> juggler_core::model::Account {
    juggler_core::model::Account {
        id: AccountId::new("fake-account"),
        harness: HarnessId::new(FAKE_HARNESS_ID),
        label: "Fake".into(),
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

fn test_profile() -> Profile {
    Profile {
        id: ProfileId::new("fake-profile"),
        name: "fake profile".into(),
        account: AccountId::new("fake-account"),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    }
}

/// Drains a watcher until `predicate` holds or the deadline expires.
fn wait_for_event(
    watcher: &mut dyn SessionWatcher,
    predicate: impl Fn(&SessionEvent) -> bool,
) -> Option<SessionEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(Some(event)) = watcher.poll(Duration::from_millis(50)) {
            if predicate(&event) {
                return Some(event);
            }
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    None
}

fn run_spec(spec: &juggler_adapter::LaunchSpec, extra_args: &[&str]) -> std::process::Output {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .args(extra_args)
        .env_clear()
        .envs(&spec.env)
        .current_dir(&spec.cwd);
    command.output().unwrap()
}

#[test]
fn discover_reports_installed_and_verified() {
    let report: DiscoveryReport = adapter().discover();
    assert_eq!(report.adapter.as_str(), FAKE_HARNESS_ID);
    let InstallStatus::Installed { version } = &report.install else {
        panic!("expected fake harness installed, got {report:?}");
    };
    assert_eq!(version, "0.1.0");
    assert_eq!(report.verification, VerificationStatus::Verified);
    assert!(report.is_installed());
    assert!(report.binary.is_some());
}

#[test]
fn capabilities_and_versions_are_declared() {
    let adapter = adapter();
    assert!(adapter.capabilities().contains(Capabilities::HEADLESS));
    assert!(adapter.capabilities().contains(Capabilities::RESUME));
    assert_eq!(adapter.verified_against(), &["0.1.0"][..]);
}

#[test]
fn prepare_account_creates_isolated_state() {
    let dir = tempfile::tempdir().unwrap();
    let env = adapter().prepare_account(dir.path()).unwrap();
    assert!(dir.path().join("transcripts").is_dir());
    assert_eq!(
        env.get("FAKE_HARNESS_HOME").map(String::as_str),
        Some(dir.path().to_str().unwrap())
    );
}

#[test]
fn launch_run_watch_and_resume_end_to_end() {
    let workdir = tempfile::tempdir().unwrap();
    let state = workdir.path().join("account");
    let adapter = adapter();
    adapter.prepare_account(&state).unwrap();

    let account = test_account(&state);
    let profile = test_profile();
    let spec = adapter
        .launch_spec(
            &account,
            &profile,
            workdir.path(),
            &LaunchMode::Headless {
                prompt: "do three turns".into(),
            },
        )
        .unwrap();
    assert!(!spec.needs_pty);
    assert!(
        spec.program
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("fake-harness")),
        "program path was {:#?}",
        spec.program
    );

    // Run three turns under a fixed session id.
    let output = run_spec(&spec, &["--turns", "3", "--session-id", "sess-e2e"]);
    assert!(
        output.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    // The watcher discovers the session, sees the messages, then the end.
    let transcripts = state.join("transcripts");
    let mut watcher = adapter.watcher(&transcripts).unwrap();
    let discovered = wait_for_event(&mut *watcher, |event| {
        matches!(event, SessionEvent::Discovered { native_id, .. } if native_id == "sess-e2e")
    })
    .expect("watcher should discover sess-e2e");
    let SessionEvent::Discovered {
        transcript_path, ..
    } = discovered
    else {
        unreachable!()
    };
    assert!(transcript_path.ends_with("sess-e2e.jsonl"));

    let appended = wait_for_event(&mut *watcher, |event| {
        matches!(event, SessionEvent::MessageAppended { .. })
    });
    assert!(appended.is_some(), "watcher should see appended messages");

    let ended = wait_for_event(
        &mut *watcher,
        |event| matches!(event, SessionEvent::Ended { native_id } if native_id == "sess-e2e"),
    );
    assert!(ended.is_some(), "watcher should report session end");

    let transcript = std::fs::read_to_string(transcript_path).unwrap();
    assert!(transcript.contains(r#""type":"session_meta""#));
    assert_eq!(
        transcript
            .lines()
            .filter(|l| l.contains(r#""type":"assistant""#))
            .count(),
        3
    );

    // Resume: append one more turn to the same session id.
    let mut resume_spec = spec.clone();
    resume_spec.args.push("--resume".into());
    resume_spec.args.push("sess-e2e".into());
    let output = run_spec(&resume_spec, &["--turns", "1"]);
    assert!(output.status.success());
    let transcript = std::fs::read_to_string(transcripts.join("sess-e2e.jsonl")).unwrap();
    assert!(transcript.contains(r#""type":"resumed""#));
    assert_eq!(
        transcript
            .lines()
            .filter(|l| l.contains(r#""type":"assistant""#))
            .count(),
        4
    );

    // The watcher reports the resumed growth.
    let resumed_growth = wait_for_event(
        &mut *watcher,
        |event| matches!(event, SessionEvent::MessageAppended { native_id, .. } if native_id == "sess-e2e"),
    );
    assert!(resumed_growth.is_some());
}

#[test]
fn registry_round_trips_the_fake_adapter() {
    let mut registry = AdapterRegistry::new();
    registry.register(std::sync::Arc::new(adapter()));
    let id = HarnessId::new(FAKE_HARNESS_ID);
    assert!(registry.get(&id).is_some());
    let reports = registry.discover_all();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].is_installed());
}

#[test]
fn discover_degrades_gracefully_for_missing_binary() {
    let adapter = FakeHarnessAdapter::new("/nonexistent/fake-harness-binary");
    let report = adapter.discover();
    assert_eq!(report.install, InstallStatus::NotFound);
    assert!(!report.is_installed());
    assert!(report.binary.is_none());
}

#[test]
fn discover_reports_failure_for_non_executable() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_bin = dir.path().join("not-a-binary");
    std::fs::write(&not_a_bin, b"still not a program").unwrap();
    let adapter = FakeHarnessAdapter::new(&not_a_bin);
    let report = adapter.discover();
    assert!(
        matches!(report.install, InstallStatus::Failed { .. }),
        "got {report:?}"
    );
}

#[test]
fn watcher_polls_missing_directory_without_crashing() {
    let mut watcher = adapter()
        .watcher(std::path::Path::new("/nonexistent/transcripts"))
        .unwrap();
    let event = watcher.poll(Duration::from_millis(10)).unwrap();
    assert!(event.is_none());
}
