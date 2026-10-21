//! AJ-E505 evidence: the port occupancy map against REAL listeners
//! (netstat2, no shell-outs). The fixture: a vite-like helper
//! (`python3 -m http.server :<free port>`) inside a supervised Instance —
//! root exits → within grace+2 ticks the inventory reads `Managed
//! orphan … holds :<port>` with full attribution; an UNSUPERVISED
//! `python3 -m http.server :<another port>` reads `Not managed` with ZERO kill
//! affordances. API JSON dumps are the evidence (run with
//! `--nocapture`).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::LaunchSpec;
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
use juggler_supervisor::tree::{LedgerStore as _, SpawnContext};

fn ctx(state: &Arc<AppState>) -> SpawnContext {
    SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-e505".into()),
        project_id: Some("proj-e505".into()),
        // The REAL local machine id — the inventory queries
        // `candidates(local_machine_id)`; a mismatched id owns nothing.
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

fn sh_spec(script: &str, cwd: &Path) -> LaunchSpec {
    LaunchSpec {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        env: Default::default(),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Drives the app-state collector ticks (the AJ-E504 cadence) until
/// `done`, printing tick timings for the budget evidence.
fn tick_until<F: FnMut() -> bool>(state: &Arc<AppState>, deadline_ms: u64, mut done: F) -> bool {
    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    while Instant::now() < deadline {
        let started = Instant::now();
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        let tick_ms = started.elapsed().as_millis();
        if tick_ms > 30 {
            println!("[tick] slow: {tick_ms}ms (probe budget ≤30ms p95)");
        }
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

/// Reaps the test's listeners however it ends: a failed assertion used
/// to leave `python3 -m http.server` on :5173 and :8000, and every later
/// run then failed against the leftovers.
struct Reaper {
    /// The supervised root's process group (setsid: pgid = root pid).
    group: u32,
    /// The unsupervised stranger.
    stranger: u32,
}

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = std::process::Command::new("kill")
            .args(["-9", "--", &format!("-{}", self.group)])
            .status();
        let _ = std::process::Command::new("kill")
            .args(["-9", &self.stranger.to_string()])
            .status();
    }
}

/// A port nothing listens on right now (the OS picks it).
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .expect("a free local port")
}

#[test]
fn port_inventory_classifies_orphan_and_unmanaged_listeners() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    // Test-only grace tune (the default stays 30 s — never lowered).
    state.supervisor.set_orphan_grace("claude-code", 3);

    // Two free ports, never fixed ones: a developer's own dev server
    // (vite on :5173) would otherwise answer for the fixture.
    let helper_port = free_port();
    let stranger_port = free_port();
    // The vite-like helper holds its port inside a supervised Instance.
    let session = SessionId::new("sess-e505");
    let spawned = state
        .supervisor
        .spawn(
            &session,
            &sh_spec(
                &format!("python3 -m http.server {helper_port} --bind 127.0.0.1 & wait"),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();

    // The UNSUPERVISED stranger holds its own port (the user's own process).
    let mut stranger = std::process::Command::new("python3")
        .args([
            "-m",
            "http.server",
            &stranger_port.to_string(),
            "--bind",
            "127.0.0.1",
        ])
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _reaper = Reaper {
        group: spawned.pid,
        stranger: stranger.id(),
    };

    // (a) While the Instance lives: helper row carries listens_on=<helper port>
    // (the tick join) and the inventory classifies BOTH rows.
    let joined = tick_until(&state, 20_000, || {
        if let Ok(rows) = state.ledger.session_rows(&session) {
            rows.iter().any(|row| {
                row.listens_on
                    .as_deref()
                    .is_some_and(|ports| ports.split(',').any(|p| p == helper_port.to_string()))
            })
        } else {
            false
        }
    });
    assert!(joined, "the tick join never stored the helper's port");

    let live = commands::port_inventory_inner(&state);
    println!("[live] {}", serde_json::to_string_pretty(&live).unwrap());
    assert!(live.note.is_none(), "the real probe works on this platform");
    let row_helper = live.rows.iter().find(|r| r.port == helper_port).unwrap();
    assert_eq!(row_helper.class, "managed-running");
    assert_eq!(row_helper.provider.as_deref(), Some("claude-code"));
    assert_eq!(row_helper.account.as_deref(), Some("acc-e505"));
    assert_eq!(row_helper.project.as_deref(), Some("proj-e505"));
    assert!(row_helper.killable);
    let row_stranger = live.rows.iter().find(|r| r.port == stranger_port).unwrap();
    assert_eq!(row_stranger.class, "not-managed");
    assert!(
        !row_stranger.killable,
        "unmanaged rows NEVER carry kill affordances"
    );
    assert!(
        row_stranger.session.is_none(),
        "no attribution is claimed for strangers"
    );

    // (b) Root exits WITHOUT the supervisor's ladder — since AJ-E507
    // `stop()` TERMs the whole process group (the helper
    // included), the orphan scenario needs the root to die another
    // way: an external SIGKILL to the root pid only (the crash
    // shape). The helper survives it.
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(spawned.pid.to_string())
        .status();
    let orphaned = tick_until(&state, 20_000, || {
        if let Ok(rows) = state.ledger.session_rows(&session) {
            rows.iter()
                .any(|row| row.state == juggler_core::storage::STATE_ORPHANED)
        } else {
            false
        }
    });
    assert!(orphaned, "the helper never surfaced as orphaned");

    let inventory = commands::port_inventory_inner(&state);
    println!(
        "[orphan] {}",
        serde_json::to_string_pretty(&inventory).unwrap()
    );
    let row_helper = inventory
        .rows
        .iter()
        .find(|r| r.port == helper_port)
        .unwrap();
    assert_eq!(row_helper.class, "managed-orphan");
    assert_eq!(row_helper.provider.as_deref(), Some("claude-code"));
    assert_eq!(row_helper.project.as_deref(), Some("proj-e505"));
    assert!(
        row_helper.listener_age_ms.is_some(),
        "kernel-based listener age"
    );
    let row_stranger = inventory
        .rows
        .iter()
        .find(|r| r.port == stranger_port)
        .unwrap();
    assert_eq!(
        row_stranger.class, "not-managed",
        "the stranger stays display-only"
    );

    // (c) The pre-launch warning fires with the copy-exact message
    // (the learned expected port of THIS project, held by its orphan).
    let warning = commands::launch_port_warning_inner(&state, Some("proj-e505".into()));
    let warning = warning.expect("the orphan-held learned port must warn");
    assert_eq!(warning.port, helper_port);
    assert_eq!(warning.pid, row_helper.pid);
    assert!(
        warning.message.starts_with(&format!(
            ":{helper_port} is already held by an orphan from "
        )) && warning.message.ends_with(")."),
        "copy-exact warning, got: {}",
        warning.message
    );
    println!(
        "[warning] {}",
        serde_json::to_string_pretty(&warning).unwrap()
    );
    // A DIFFERENT project has no learned ports → no warning, no block.
    assert!(commands::launch_port_warning_inner(&state, Some("proj-other".into())).is_none());

    // Cleanup: the orphan is OURS (ledger row) — reap by pid; the
    // stranger we started — kill it; no process survives the test.
    let _ = std::process::Command::new("kill")
        .arg(row_helper.pid.to_string())
        .status();
    let _ = stranger.kill();
    let _ = stranger.wait();
}
