//! AJ-E505 evidence: the port occupancy map against REAL listeners
//! (netstat2, no shell-outs). The fixture: a vite-like helper
//! (`python3 -m http.server :5173`) inside a supervised Instance —
//! root exits → within grace+2 ticks the inventory reads `Managed
//! orphan … holds :5173` with full attribution; an UNSUPERVISED
//! `python3 -m http.server :8000` reads `Not managed` with ZERO kill
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

    // The vite-like helper holds :5173 inside a supervised Instance.
    let session = SessionId::new("sess-e505");
    let spawned = state
        .supervisor
        .spawn(
            &session,
            &sh_spec(
                "python3 -m http.server 5173 --bind 127.0.0.1 & wait",
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();

    // The UNSUPERVISED stranger holds :8000 (the user's own process).
    let mut stranger = std::process::Command::new("python3")
        .args(["-m", "http.server", "8000", "--bind", "127.0.0.1"])
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // (a) While the Instance lives: helper row carries listens_on=5173
    // (the tick join) and the inventory classifies BOTH rows.
    let joined = tick_until(&state, 20_000, || {
        if let Ok(rows) = state.ledger.session_rows(&session) {
            rows.iter().any(|row| {
                row.listens_on
                    .as_deref()
                    .is_some_and(|ports| ports.split(',').any(|p| p == "5173"))
            })
        } else {
            false
        }
    });
    assert!(joined, "the tick join never stored listens_on=5173");

    let live = commands::port_inventory_inner(&state);
    println!("[live] {}", serde_json::to_string_pretty(&live).unwrap());
    assert!(live.note.is_none(), "the real probe works on this platform");
    let row_5173 = live.rows.iter().find(|r| r.port == 5173).unwrap();
    assert_eq!(row_5173.class, "managed-running");
    assert_eq!(row_5173.provider.as_deref(), Some("claude-code"));
    assert_eq!(row_5173.account.as_deref(), Some("acc-e505"));
    assert_eq!(row_5173.project.as_deref(), Some("proj-e505"));
    assert!(row_5173.killable);
    let row_8000 = live.rows.iter().find(|r| r.port == 8000).unwrap();
    assert_eq!(row_8000.class, "not-managed");
    assert!(
        !row_8000.killable,
        "unmanaged rows NEVER carry kill affordances"
    );
    assert!(
        row_8000.session.is_none(),
        "no attribution is claimed for strangers"
    );

    // (b) Root exits WITHOUT the supervisor's ladder — since AJ-E507
    // `stop()` TERMs the whole process group (the :5173 helper
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
    let row_5173 = inventory.rows.iter().find(|r| r.port == 5173).unwrap();
    assert_eq!(row_5173.class, "managed-orphan");
    assert_eq!(row_5173.provider.as_deref(), Some("claude-code"));
    assert_eq!(row_5173.project.as_deref(), Some("proj-e505"));
    assert!(
        row_5173.listener_age_ms.is_some(),
        "kernel-based listener age"
    );
    let row_8000 = inventory.rows.iter().find(|r| r.port == 8000).unwrap();
    assert_eq!(
        row_8000.class, "not-managed",
        "the stranger stays display-only"
    );

    // (c) The pre-launch warning fires with the copy-exact message
    // (the learned expected port of THIS project, held by its orphan).
    let warning = commands::launch_port_warning_inner(&state, Some("proj-e505".into()));
    let warning = warning.expect("the orphan-held learned port must warn");
    assert_eq!(warning.port, 5173);
    assert_eq!(warning.pid, row_5173.pid);
    assert!(
        warning
            .message
            .starts_with(":5173 is already held by an orphan from ")
            && warning.message.ends_with(")."),
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
        .arg(row_5173.pid.to_string())
        .status();
    let _ = stranger.kill();
    let _ = stranger.wait();
}
