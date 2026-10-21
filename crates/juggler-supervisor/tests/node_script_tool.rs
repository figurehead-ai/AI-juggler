//! The 0.0.11 report: opened from Finder, the app inherits
//! `PATH=/usr/bin:/bin:/usr/sbin:/sbin`. An npm-installed CLI is a
//! `#!/usr/bin/env node` script, so with that `PATH` its `--version`
//! failed ("env: node: No such file or directory"), Codex didn't count
//! as installed, and its sessions and sign-in couldn't start either.
//!
//! The fixture is the Homebrew layout — the tool and `node` in one
//! folder — in a temporary folder, with a stand-in `node`. The checks run
//! in a child of this test with the Finder `PATH` and a throwaway `HOME`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use juggler_adapter::{HarnessAdapter, InstallStatus, LaunchMode};
use juggler_adapters::CodexAdapter;
use juggler_core::model::{AccountId, HarnessId, Profile, ProfileId, SessionId};
use juggler_supervisor::{ExitKind, LifecycleEvent, Supervisor};

/// What an app opened from Finder inherits.
const FINDER_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";
/// Set in the child that runs the checks.
const CHILD: &str = "AJ_NODE_SCRIPT_TOOL_CHILD";
/// The fixture's folder, handed to the child.
const FIXTURE: &str = "AJ_NODE_SCRIPT_TOOL_BIN";

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// `bin/node` (a stand-in: it answers `--version` for the script it runs
/// and otherwise says what it ran) and `bin/codex`, a node script.
fn node_script_fixture(bin: &Path) -> PathBuf {
    std::fs::create_dir_all(bin).unwrap();
    executable(
        &bin.join("node"),
        "#!/bin/sh\nscript=\"$1\"; shift\n\
         if [ \"$1\" = \"--version\" ]; then echo 'codex-cli 0.163.0'; \
         else echo \"fixture-node ran ${script##*/} $*\"; fi\n",
    );
    let codex = bin.join("codex");
    executable(
        &codex,
        "#!/usr/bin/env node\n// a stand-in for @openai/codex\n",
    );
    codex
}

#[test]
fn a_node_script_tool_is_found_and_starts_with_the_finder_path() {
    if std::env::var_os(CHILD).is_some() {
        return checks_with_the_finder_path();
    }
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("homebrew/bin");
    node_script_fixture(&bin);
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_node_script_tool_is_found_and_starts_with_the_finder_path",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("PATH", FINDER_PATH)
        .env("HOME", &home)
        .env(CHILD, "1")
        .env(FIXTURE, &bin)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the checks failed with the Finder PATH:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn checks_with_the_finder_path() {
    assert_eq!(std::env::var("PATH").unwrap(), FINDER_PATH);
    let bin = PathBuf::from(std::env::var_os(FIXTURE).unwrap());
    let codex = bin.join("codex");
    // The fixture really needs a `node` the Finder PATH doesn't have.
    let bare = std::process::Command::new(&codex)
        .arg("--version")
        .output()
        .unwrap();
    assert!(!bare.status.success(), "the fixture ran without its node");

    // Discovery: found, with its version.
    let adapter = CodexAdapter::with_bin(&codex);
    let report = adapter.discover();
    assert_eq!(
        report.install,
        InstallStatus::Installed {
            version: "0.163.0".into()
        },
        "notes: {:?}",
        report.notes
    );

    let work = tempfile::tempdir().unwrap();
    let state_dir = work.path().join("codex-account");
    std::fs::create_dir_all(&state_dir).unwrap();
    let supervisor = Supervisor::new();

    // A session without a terminal (piped).
    let account = juggler_core::model::Account {
        id: AccountId::new("codex-acct"),
        harness: HarnessId::new("codex"),
        label: "Codex".into(),
        plan: None,
        state_dir: state_dir.clone(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let profile = Profile {
        id: ProfileId::new("p1"),
        name: "p1".into(),
        account: account.id.clone(),
        settings_overlay: Default::default(),
        default_cwd: None,
    };
    let headless = adapter
        .launch_spec(
            &account,
            &profile,
            work.path(),
            &LaunchMode::Headless {
                prompt: "hello".into(),
            },
        )
        .unwrap();
    assert!(!headless.needs_pty);
    ran(
        &supervisor,
        "s-headless",
        &headless,
        "fixture-node ran codex exec",
    );

    // The sign-in, in a terminal (PTY).
    let login = adapter.login_spec(&state_dir).unwrap();
    assert!(login.needs_pty);
    ran(
        &supervisor,
        "s-login",
        &login,
        "fixture-node ran codex login",
    );
}

/// Spawns `spec`, then waits for a clean exit that printed `expected`.
fn ran(supervisor: &Supervisor, id: &str, spec: &juggler_adapter::LaunchSpec, expected: &str) {
    let session = SessionId::new(id);
    supervisor
        .spawn(
            &session,
            spec,
            &juggler_supervisor::tree::SpawnContext {
                provider: "codex".into(),
                account_id: Some("codex-acct".into()),
                project_id: None,
                machine: juggler_core::model::MachineId::new("machine-test"),
                role: "root",
            },
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let kind = loop {
        assert!(std::time::Instant::now() < deadline, "{id}: no exit");
        if let Some(LifecycleEvent::Exited {
            session: exited,
            kind,
            ..
        }) = supervisor.recv_timeout(Duration::from_millis(100))
            && exited == session
        {
            break kind;
        }
    };
    let output = String::from_utf8_lossy(&supervisor.output(&session).unwrap()).into_owned();
    assert!(output.contains(expected), "{id}: printed {output:?}");
    assert_eq!(kind, ExitKind::Code(0), "{id}: printed {output:?}");
}
