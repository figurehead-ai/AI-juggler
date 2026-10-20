//! AJ-E1402 Req.3 (D-08): an AI tool AI Juggler did not start is
//! measured, never managed. Every stop and cleanup entry point refuses
//! its pid, and the process is still running afterwards.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;

fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn every_stop_path_refuses_a_tool_we_did_not_start() {
    let dir = tempfile::tempdir().unwrap();
    let state =
        Arc::new(AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap());
    // Stands in for the Claude app or a terminal `claude`: started
    // outside AI Juggler, so there is no ledger row and no session.
    let mut tool = std::process::Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let pid = tool.id();

    // It is measured (observed) … (after the first tick: that tick's own
    // scan sets the real running tools; the next scan is 30 s away).
    commands::processes_tick(&state);
    state
        .resources
        .lock()
        .unwrap()
        .set_observed_roots(vec![pid]);
    commands::processes_tick(&state);
    commands::processes_tick(&state);
    assert!(
        state
            .resources
            .lock()
            .unwrap()
            .observed_snapshot()
            .contains_key(&pid),
        "the tool is measured"
    );

    // … and every way to stop something refuses it, whatever session the
    // caller claims.
    for session in ["", "sess-any"] {
        let verify = commands::kill_verify_inner(&state, session, pid);
        assert!(
            verify.as_ref().map_or(true, |info| !info.verified),
            "verify-before-kill never vouches for it"
        );
        let cancel = Arc::new(AtomicBool::new(false));
        let execute = commands::kill_execute_audited(
            &state,
            state.audit.as_ref(),
            session,
            pid,
            "pid",
            &cancel,
            None,
        );
        assert!(
            execute.as_ref().map_or(true, |info| info.refusal.is_some()),
            "the kill sheet refuses it"
        );
        let stop = commands::orphan_stop_inner(&state, &SessionId::new(session.to_string()), pid)
            .unwrap();
        assert_ne!(stop.state, "stopped", "the leftover stop refuses it");
    }
    // The automatic tiers never see it either (no ledger row).
    commands::auto_cleanup_sweep(&state, i64::MAX / 2);

    assert!(alive(pid), "never signalled");
    tool.kill().unwrap();
    tool.wait().unwrap();
}
