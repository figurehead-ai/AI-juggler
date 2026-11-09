//! 0.4.1: every CLI in AI Juggler's terminal drew in plain white — a PTY
//! child got the app's own environment, and a Mac app opened from the
//! Finder has no `TERM`. Every PTY now starts from one step
//! (`juggler_supervisor::terminal_env`): these hold that a PTY child sees
//! it, that the launch overlay still wins, and that no other place in the
//! workspace opens a PTY (sign-in terminals, the tool's own view, Open in
//! terminal and every session all come through `Supervisor::spawn`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "test crates are pragmatic by policy (QUALITY.md §2: tests excepted)"
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::SessionId;
use juggler_supervisor::{LifecycleEvent, Supervisor};

fn ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-test".into()),
        project_id: None,
        machine: juggler_core::model::MachineId::new("machine-test"),
        role: "login",
    }
}

/// What a PTY child started with `env` as its overlay prints for
/// `$TERM|$COLORTERM|$TERM_PROGRAM` (whether `FORCE_COLOR` is added is
/// the unit tests': this process may have one of its own).
fn pty_sees(name: &str, env: EnvMap) -> String {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new(name);
    let spec = LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec![
            "-c".into(),
            r#"printf 'seen:%s|%s|%s\n' "$TERM" "$COLORTERM" "$TERM_PROGRAM""#.into(),
        ],
        env,
        cwd: workdir.path().to_path_buf(),
        needs_pty: true,
    };
    supervisor.spawn(&session, &spec, &ctx()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(LifecycleEvent::Exited { .. }) =
            supervisor.recv_timeout(Duration::from_millis(100))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "no exit");
    }
    let out = String::from_utf8_lossy(&supervisor.output(&session).unwrap()).into_owned();
    out.lines()
        .find_map(|line| line.trim_end().strip_prefix("seen:").map(str::to_owned))
        .unwrap_or_else(|| panic!("no report in {out:?}"))
}

#[test]
fn a_pty_child_is_told_it_runs_in_a_colour_terminal() {
    assert_eq!(
        pty_sees("s-term-env", EnvMap::new()),
        "xterm-256color|truecolor|AI Juggler"
    );
}

#[test]
fn the_launch_overlay_still_wins() {
    let env = EnvMap::from([
        ("TERM".to_string(), "vt100".to_string()),
        (
            "COLORTERM".to_string(),
            juggler_adapter::launch::ENV_UNSET.to_string(),
        ),
    ]);
    assert_eq!(pty_sees("s-term-overlay", env), "vt100||AI Juggler");
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_pty_starts_through_the_one_terminal_step() {
    let mut files = Vec::new();
    for krate in std::fs::read_dir(workspace()).unwrap().flatten() {
        let src = krate.path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    let opens: Vec<String> = files
        .iter()
        .filter(|path| {
            let text = std::fs::read_to_string(path).unwrap();
            [
                "openpty(",
                "CommandBuilder::new(",
                "forkpty(",
                "posix_openpt(",
            ]
            .iter()
            .any(|needle| text.contains(needle))
        })
        .map(|path| {
            path.strip_prefix(workspace())
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        opens,
        vec!["juggler-supervisor/src/supervisor.rs".to_string()],
        "a PTY opened outside Supervisor::spawn_pty skips terminal_env"
    );
    let supervisor =
        std::fs::read_to_string(workspace().join("juggler-supervisor/src/supervisor.rs")).unwrap();
    let start = supervisor.find("fn spawn_pty(").unwrap();
    let body = &supervisor[start..];
    let body = &body[..body.find("\n    }\n").unwrap()];
    assert_eq!(body.matches("CommandBuilder::new(").count(), 1);
    assert!(
        body.contains("terminal_env::pty_base_env()"),
        "spawn_pty builds its environment through terminal_env"
    );
    let step = std::fs::read_to_string(workspace().join("juggler-supervisor/src/terminal_env.rs"))
        .unwrap();
    assert!(step.contains("terminal_env(std::env::vars_os(), system_utf8_locale())"));
    assert!(
        !body.contains("for (key, value) in std::env::vars_os()"),
        "no raw inheritance beside it"
    );
}
