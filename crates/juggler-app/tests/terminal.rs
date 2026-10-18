//! M1-4 integration: the embedded-terminal command cores — input, resize,
//! snapshot replay, and the stop ladder — against a real PTY (issue #20
//! acceptance: resize/echo correctness; the dock's close/reopen replays
//! from the snapshot).

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
use std::time::Duration;

use base64::Engine as _;
use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_adapters::ClaudeCodeAdapter;
use juggler_app::state::AppState;
use juggler_app::terminal;
use juggler_core::model::SessionId;
use juggler_supervisor::Supervisor;

fn ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: juggler_core::model::MachineId::new("machine-test"),
        role: "root",
    }
}

#[test]
fn terminal_cores_roundtrip_input_snapshot_and_stop() {
    let work = tempfile::tempdir().unwrap();
    // `cat` in a PTY through the supervisor: exactly what the dock drives.
    let state = Arc::new(
        AppState::init_with_registry(work.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let id = SessionId::new("s-term");
    let spec = LaunchSpec {
        program: std::path::PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), "cat".into()],
        env: EnvMap::new(),
        cwd: work.path().to_path_buf(),
        needs_pty: true,
    };
    state.supervisor.spawn(&id, &spec, &ctx()).unwrap();
    let id_str = id.as_str().to_string();

    // Keyboard input flows (cat echoes it back).
    terminal::terminal_write_impl(&state, &id_str, "hello-dock\n").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut echoed = false;
    while std::time::Instant::now() < deadline {
        let snapshot = terminal::terminal_snapshot_impl(&state, &id_str).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&snapshot)
            .unwrap();
        if bytes.windows(10).any(|w| w == b"hello-dock") {
            echoed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(echoed, "input must echo into the snapshot replay");

    // Resize accepted on the live PTY; unknown ids error cleanly.
    terminal::terminal_resize_impl(&state, &id_str, 100, 30).unwrap();
    let err = terminal::terminal_write_impl(&state, "ghost", "x").unwrap_err();
    assert!(err.to_string().contains("not supervised"), "{err}");

    // Stop ladder ends the session; snapshot survives for replay.
    terminal::terminal_kill_impl(&state, &id_str).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !state.supervisor.is_finished(&id) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(state.supervisor.is_finished(&id));
    let snapshot = terminal::terminal_snapshot_impl(&state, &id_str).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&snapshot)
        .unwrap();
    assert!(bytes.windows(10).any(|w| w == b"hello-dock"));
}

#[test]
fn login_terminal_spawn_uses_the_account_scoped_env() {
    // The login terminal must be env-scoped to the account's isolated dir
    // (ADR-0003/0006): prove it by pointing the "claude" binary at a stub
    // that records CLAUDE_CONFIG_DIR.
    let work = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    let stub = work.path().join("claude-stub");
    std::fs::write(
        &stub,
        "#!/bin/sh\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > \"$CLAUDE_CONFIG_DIR/login-ran\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(&stub)));
    let state =
        Arc::new(AppState::init_with_registry(work.path().join("app-data"), registry).unwrap());

    let account = juggler_app::commands::create_account(
        &state,
        juggler_adapters::CLAUDE_CODE_ID,
        "Login flow",
        None,
    )
    .unwrap();

    // Drive the login spec the same way the terminal command does (the
    // command wrapper itself needs a Tauri runtime).
    let account_record = juggler_app::commands::find_account_arc(&state, &account.id).unwrap();
    let adapter = state.registry.get(&account_record.harness).unwrap();
    let spec = adapter.login_spec(&account_record.state_dir).unwrap();
    assert!(spec.needs_pty, "login must run interactive");
    assert_eq!(
        spec.env.get("CLAUDE_CONFIG_DIR"),
        Some(&account_record.state_dir.to_string_lossy().into_owned()),
        "login terminal is scoped to the isolated dir"
    );
    let id = SessionId::new("s-login");
    state.supervisor.spawn(&id, &spec, &ctx()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline && !state.supervisor.is_finished(&id) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        account_record.state_dir.join("login-ran").is_file(),
        "vendor CLI ran under the isolated config dir"
    );
    let _ = Path::new("/bin/sh"); // keep import when cfg flips
    let _ = Supervisor::new(); // linkage sanity
}
