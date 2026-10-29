//! AJ-E2306: a terminal session's exact state from its tool's own hooks,
//! end to end on fixtures: the fake tool (launched by the app's own launch
//! path) runs the **real** `juggler-agent hook <event>` from its per-launch
//! hooks file, the program reports over the app's socket with the session's
//! report key, and the session is Asks you (with what it asks), then ended.
//!
//! Also: the per-launch file is in AI Juggler's own overlays (`0700`,
//! `0600`), nothing lands in the account's folder, a report with an
//! unknown key is refused, and the hook never prints or fails.
// Unix only: its fixtures are /bin/sh scripts, Unix sockets, modes or signals.
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_agentbridge::wire::{self, HookReport, Refusal, Report};
use juggler_app::features::agent_bridge::calls;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

const BRIDGE: &str = env!("CARGO_BIN_EXE_juggler-agent");

fn fake_harness() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let bin = target.join("debug").join("fake-harness");
    assert!(
        bin.exists(),
        "build it first: cargo build -p juggler-adapters --bins"
    );
    bin
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn the_tools_own_hooks_make_the_state_exact() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::FakeHarnessAdapter::new(
        fake_harness(),
    )));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    state.agent_bridge.set_program(Some(PathBuf::from(BRIDGE)));
    let project = root.join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let account = juggler_app::commands::create_account(&state, "fake", "Work", None).unwrap();
    let profile = juggler_app::commands::create_profile(
        &state,
        &account.id,
        "Main",
        None,
        Some(project.to_string_lossy().into_owned()),
    )
    .unwrap();
    // The fake asks a permission after its turns (through its hooks file).
    {
        let storage = state.storage.lock().unwrap();
        let mut recipe = storage
            .profiles()
            .get(&juggler_core::model::ProfileId::new(profile.id.as_str()))
            .unwrap();
        recipe
            .settings_overlay
            .env
            .insert("FAKE_HARNESS_WAIT_PROMPT".into(), "rm -rf build".into());
        storage.profiles().update(&recipe).unwrap();
    }
    let session = juggler_app::commands::launch_profile(&state, &profile.id, None)
        .unwrap()
        .session_id;
    let runtime = || {
        juggler_app::commands::instances_shared_snapshot(&state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == session)
            .and_then(|row| row.runtime)
    };
    wait_for("Asks you", || {
        runtime().is_some_and(|r| r.state.as_deref() == Some("asks_you"))
    });
    let info = runtime().unwrap();
    assert_eq!(info.kind, "terminal");
    assert_eq!(info.asks[0].summary, "Bash: rm -rf build");
    assert!(info.asks[0].options.is_empty(), "answered in its terminal");
    let record = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get(&SessionId::new(session.as_str()))
        .unwrap();
    assert_eq!(record.status.as_str(), "waiting_user");
    assert_eq!(record.attention.as_ref().unwrap().rule, "tool_asks");
    // Linked to its conversation by the tool's own word.
    assert!(record.conversation.is_some());
    assert!(record.transcript_ref.is_some_and(|t| t.path.is_file()));

    // The per-launch file is AI Juggler's, closed; the account's folder has
    // only what the tool itself writes.
    let launch_dir = state.overlays_dir().join("runtime").join(session.as_str());
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&launch_dir), 0o700);
    assert_eq!(mode(&launch_dir.join("fake-hooks.json")), 0o600);
    let mut in_account = Vec::new();
    let state_dir = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&juggler_core::model::AccountId::new(account.id.as_str()))
        .unwrap()
        .state_dir;
    for entry in walk(&state_dir) {
        in_account.push(entry);
    }
    assert!(
        in_account
            .iter()
            .all(|p| p.to_string_lossy().contains("/transcripts/")),
        "{in_account:?}"
    );

    // Stopped: the state follows the process, never a stale ask.
    state
        .supervisor
        .stop(&SessionId::new(session.as_str()), Duration::from_secs(2))
        .unwrap();
    wait_for("the end", || {
        state
            .supervisor
            .is_finished(&SessionId::new(session.as_str()))
    });
}

fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn a_report_with_a_stranger_key_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let report = Report {
        v: wire::VERSION,
        token: "not-a-key".into(),
        hook: HookReport {
            event: "PermissionRequest".into(),
            ..HookReport::default()
        },
    };
    let line = serde_json::to_string(&report).unwrap();
    let answer = calls::handle_line(&state, &line);
    assert_eq!(answer.refused, Some(Refusal::Unknown));
    // An agent's call can't come in a report's shape, nor a report in a
    // call's: one key, one scope.
    let mixed = r#"{"v":1,"token":"t","call":"whoami","hook":{"event":"Stop"}}"#;
    assert_eq!(
        calls::handle_line(&state, mixed).refused,
        Some(Refusal::Bad)
    );
}

#[test]
fn the_hook_command_is_silent_and_never_fails() {
    for env in [
        vec![],
        vec![
            (wire::REPORT_TOKEN_ENV, "k"),
            (wire::SOCKET_ENV, "/nowhere/s.sock"),
        ],
    ] {
        let mut child = Command::new(BRIDGE)
            .args(["hook", "PermissionRequest"])
            .env_remove(wire::REPORT_TOKEN_ENV)
            .env_remove(wire::SOCKET_ENV)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(br#"{"session_id":"s","tool_name":"Bash","tool_input":{"command":"ls"}}"#)
            .unwrap();
        let started = Instant::now();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        assert!(
            out.stdout.is_empty(),
            "a hook that prints nothing decides nothing"
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
