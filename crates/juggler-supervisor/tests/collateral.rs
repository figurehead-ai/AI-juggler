//! AJ-E512 collateral suite (supervisor level): the ladder's SIGNAL
//! GEOMETRY, proved on adversarial fixtures with trap logs — never on
//! UI state. `stop()` TERMs exactly the session's OWN process group
//! (the E505 group guard: `pgid == the child's own pid`), KILLs only
//! survivors after grace, and nothing outside that group — not the
//! `setsid` escapee (own group, ppid chain intact), not another
//! session's tree, not an unmanaged user server — receives a single
//! signal. D-01: zero false cleanup kills.
//!
//! The role/identity gates (MCP exclusion, PID-reuse stranger,
//! Tiers 1-2, crash recovery) are app-level surfaces — their
//! zero-signal matrix lives in `juggler-app/tests/guardrails.rs`.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

mod fixtures;

use std::sync::Arc;
use std::time::Duration;

use fixtures::{FixtureSet, alive, port_free, spawn_unmanaged, unmanaged_fixture};
use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::SessionId;
use juggler_supervisor::tree::{CoreLedgerStore, LedgerStore};
use juggler_supervisor::{LiveStatsCollector, Supervisor};

fn sh_spec(script: &str, cwd: &std::path::Path) -> LaunchSpec {
    LaunchSpec {
        program: std::path::PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-collateral".into()),
        project_id: Some("proj-collateral".into()),
        machine: juggler_core::model::MachineId::new("machine-collateral"),
        role: "root",
    }
}

/// The helper row of one session whose cmdline contains `needle`.
fn helper_pid_by(ledger: &Arc<dyn LedgerStore>, session: &SessionId, needle: &str) -> Option<u32> {
    let rows = ledger.session_rows(session).ok()?;
    rows.iter()
        .find(|row| row.role == "helper" && row.cmdline_trunc.contains(needle))
        .and_then(|row| row.pid)
}

#[test]
fn stop_signals_only_the_own_group_and_kills_survivors_after_grace() {
    let dir = tempfile::tempdir().unwrap();
    let kit = FixtureSet::new(dir.path());

    let ledger: Arc<dyn LedgerStore> = Arc::new(CoreLedgerStore::new(
        juggler_core::storage::ProcessLedgerRepo::open_in_memory().unwrap(),
    ));
    let supervisor = Supervisor::new().with_ledger(Arc::clone(&ledger));
    let mut collector = LiveStatsCollector::new();

    // Session A: every in-tree fixture under one root.
    let a = SessionId::new("sess-a");
    supervisor
        .spawn(
            &a,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(),
        )
        .unwrap();
    // Session B: one mcp fixture — a NEIGHBOUR tree that must never
    // hear session A's stop.
    let b = SessionId::new("sess-b");
    let b_dir = dir.path().join("b");
    std::fs::create_dir_all(&b_dir).unwrap();
    let kit_b = FixtureSet::new(&b_dir);
    let _spawned_b = supervisor
        .spawn(
            &b,
            &sh_spec(&format!("{}\nwait", kit_b.root_script()), &b_dir),
            &ctx(),
        )
        .unwrap();
    // The unmanaged user server: ours to observe, never ours to signal.
    let unmanaged_pid = spawn_unmanaged(dir.path());

    // Discover the fixture pids through the ledger (ticks land the
    // helper rows).
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let (vite, ignorer, _mcp, shell) = loop {
        collector.tick(&supervisor, now_ms());
        let vite = helper_pid_by(&ledger, &a, "--port");
        let ignorer = helper_pid_by(&ledger, &a, "--ignore-term");
        let mcp = helper_pid_by(&ledger, &a, "mcp-server");
        let shell = helper_pid_by(&ledger, &a, "touch");
        if let (Some(vite), Some(ignorer), Some(mcp), Some(shell)) = (vite, ignorer, mcp, shell) {
            break (vite, ignorer, mcp, shell);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fixtures never landed in the ledger"
        );
        std::thread::sleep(Duration::from_millis(250));
    };
    assert!(kit.wait_ready(), "in-tree fixtures ready");
    let port = kit.vite_port();

    // Diagnostics (CI debugging): the root's captured output (a
    // fixture crashing at startup prints here), per-fixture readiness
    // and liveness, and the ledger rows before the stop.
    {
        let out = supervisor.output(&a).unwrap_or_default();
        println!(
            "[collateral-diag] root output ({} bytes): {:?}",
            out.len(),
            String::from_utf8_lossy(&out)
        );
        for kind in ["vite", "term-ignorer", "escapee", "mcp"] {
            let ready = kit.get(kind).ready.exists();
            println!("[collateral-diag] {kind}: ready={ready}");
        }
        for row in ledger.session_rows(&a).unwrap_or_default() {
            println!(
                "[collateral-diag] row pid={:?} role={} state={} alive={}",
                row.pid,
                row.role,
                row.state,
                row.pid.map(alive).unwrap_or(false)
            );
        }
    }

    // THE STOP: 2 s grace — the TERM-ignorer deliberately survives it.
    supervisor.stop(&a, Duration::from_secs(2)).unwrap();

    // Intended, in-group: TERM'd. The vite fixture is cooperative
    // (dies on TERM); the shell traps+logs TERM then dies on the
    // post-grace KILL; the TERM-ignorer logs TERM, survives grace,
    // dies by KILL — exactly the ladder's contract.
    assert!(
        fixtures::wait_for(6_000, || !alive(vite)),
        "cooperative fixture died on TERM"
    );
    assert!(fixtures::wait_for(6_000, || !alive(shell)));
    assert!(
        fixtures::wait_for(6_000, || !alive(ignorer)),
        "TERM-ignorer taken by the post-grace KILL"
    );
    if kit.get("vite").signals() != vec!["TERM".to_string()] {
        println!(
            "[collateral-diag] vite signals={:?} (root output above)",
            kit.get("vite").signals()
        );
    }
    assert_eq!(kit.get("vite").signals(), vec!["TERM".to_string()]);
    assert_eq!(kit.get("shell").signals(), vec!["TERM".to_string()]);
    assert_eq!(kit.get("term-ignorer").signals(), vec!["TERM".to_string()]);

    // COLLATERAL — zero signals, still alive:
    kit.assert_untouched(&["escapee"], "supervisor stop");
    kit_b.assert_untouched(&["mcp"], "supervisor stop");
    assert!(
        unmanaged_fixture(dir.path()).untouched(),
        "the unmanaged user server was signalled"
    );
    assert!(alive(
        helper_pid_by(&ledger, &a, "--setsid").unwrap_or_default()
    ));
    assert!(alive(
        helper_pid_by(&ledger, &b, "mcp-server").unwrap_or_default()
    ));
    assert!(alive(unmanaged_pid));
    // The port-holder's port is honestly free after the intended kill.
    assert!(
        port_free(port),
        "the vite fixture's port is free after TERM"
    );

    // Cleanup: B's tree, the escapee, the unmanaged server.
    supervisor.stop(&b, Duration::from_secs(2)).unwrap();
    if let Some(escapee) = helper_pid_by(&ledger, &a, "--setsid") {
        fixtures::kill9(escapee);
    }
    fixtures::kill9(unmanaged_pid);
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
