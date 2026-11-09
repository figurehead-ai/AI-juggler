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

/// Set in the child that runs the Claude Code lab.
const CLAUDE_LAB_CHILD: &str = "AJ_TERMINAL_ENV_CLAUDE_LAB_CHILD";
/// The `claude` binary the lab runs, handed to the child.
const CLAUDE_LAB_BIN: &str = "AJ_TERMINAL_ENV_CLAUDE_BIN";

/// Claude Code's brand orange as it draws it in truecolor (2.1.310).
const CLAUDE_ORANGE: &[u8] = b"\x1b[38;2;215;119;87m";

/// rc21: OpenCode drew in colour in AI Juggler's terminal, Claude Code in
/// white and grey. AI Juggler opened from a Claude Code session's shell
/// (`open` passes the caller's environment on) carries the `NO_COLOR=1`
/// and `CLAUDECODE=1` that shell sets, and Claude Code honours
/// `NO_COLOR`. A lab: the real `claude`, signed out, in a throwaway
/// `HOME` and config folder, started by the Claude Code adapter through
/// `Supervisor::spawn` from a process with exactly that inheritance. Its
/// onboarding draws the brand orange. Never signs in: the session is
/// stopped after a few seconds of output.
///
/// Needs `claude`: `cargo test -p juggler-supervisor --test terminal_env
/// -- --ignored claude_draws_in_colour` (`AJ_TERMINAL_ENV_CLAUDE_BIN`
/// names another binary).
#[test]
#[ignore = "lab: needs the claude binary (opt-in)"]
fn claude_draws_in_colour_when_ai_juggler_was_opened_from_a_claude_shell() {
    if std::env::var_os(CLAUDE_LAB_CHILD).is_some() {
        return claude_lab_child();
    }
    let bin = std::env::var_os(CLAUDE_LAB_BIN)
        .map(PathBuf::from)
        .or_else(|| {
            let home = PathBuf::from(std::env::var_os("HOME")?);
            Some(home.join(".local/bin/claude")).filter(|path| path.exists())
        })
        .expect("no claude binary: set AJ_TERMINAL_ENV_CLAUDE_BIN");
    let lab = tempfile::tempdir().unwrap();
    let home = lab.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    // Exactly what a Claude Code shell tool hands an app it opens (the
    // names that matter here), and nothing of this machine's own.
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "claude_draws_in_colour_when_ai_juggler_was_opened_from_a_claude_shell",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("HOME", &home)
        .env("TMPDIR", lab.path())
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("NO_COLOR", "1")
        .env("CLAUDECODE", "1")
        .env(CLAUDE_LAB_CHILD, "1")
        .env(CLAUDE_LAB_BIN, &bin)
        .status()
        .unwrap();
    assert!(status.success(), "the lab child failed: {status}");
}

fn claude_lab_child() {
    use juggler_adapter::{LaunchMode, ToolLaunch};
    use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId};

    let bin = PathBuf::from(std::env::var_os(CLAUDE_LAB_BIN).unwrap());
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let state_dir = home.join("claude-config");
    std::fs::create_dir_all(&state_dir).unwrap();
    let account = Account {
        id: AccountId::new("lab-colour"),
        harness: HarnessId::new("claude-code"),
        label: "Lab".into(),
        plan: None,
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let profile = Profile {
        id: ProfileId::new("lab-colour"),
        name: "lab".into(),
        account: account.id.clone(),
        settings_overlay: Default::default(),
        default_cwd: None,
    };
    let spec = juggler_adapters::ClaudeCodeAdapter::with_bin(&bin)
        .launch_spec(&account, &profile, &home, &LaunchMode::Interactive)
        .unwrap();
    assert!(spec.needs_pty);

    let supervisor = Supervisor::new();
    let session = SessionId::new("s-claude-colour");
    supervisor.spawn(&session, &spec, &ctx()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut seen = Vec::new();
    while std::time::Instant::now() < deadline {
        let _ = supervisor.recv_timeout(Duration::from_millis(200));
        seen = supervisor.output(&session).unwrap_or_default();
        if seen
            .windows(CLAUDE_ORANGE.len())
            .any(|w| w == CLAUDE_ORANGE)
        {
            break;
        }
    }
    let _ = supervisor.stop(&session, Duration::from_secs(2));
    let truecolor = seen.windows(7).filter(|w| w == b"\x1b[38;2;").count();
    assert!(
        seen.windows(CLAUDE_ORANGE.len())
            .any(|w| w == CLAUDE_ORANGE),
        "Claude Code drew no colour ({truecolor} truecolor foregrounds in {} bytes)",
        seen.len()
    );
}
