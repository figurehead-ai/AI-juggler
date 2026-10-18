//! AJ-E507 evidence: the verify-before-kill sheet's backend — the
//! two guardrails (same-tick re-verification, NO AUDIT ⇒ NO SIGNAL)
//! and the three fixtures (vite-like port holder freed on TERM,
//! TERM-ignorer escalated to KILL with a delivered-signal witness,
//! setsid escapee honestly reported still-holding then re-offered).
//! Run with `--nocapture` to archive the evidence dumps.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands::{self, AuditWriteError, AuditWriter, KILL_REFUSAL_COPY};
use juggler_app::error::AppError;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
use juggler_supervisor::LiveStatsCollector;
use juggler_supervisor::tree::LedgerStore as _;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn sh_spec(script: &str, cwd: &Path) -> LaunchSpec {
    LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn spawn_ctx(state: &Arc<AppState>) -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-kill".into()),
        project_id: Some("proj-kill".into()),
        // AJ-E511: spawn attribution rides the persisted local-machine
        // identity — a hardcoded machine now refuses at the kill gate.
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn kill9(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}

fn wait_for(ms: u64, mut done: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    while std::time::Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    done()
}

/// A port nothing is bound to right now (bind :0, read it, drop).
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn port_open(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// Spawns the session and waits until the ledger row is live.
fn spawn_root(state: &Arc<AppState>, session: &SessionId, script: &str, dir: &Path) -> u32 {
    state
        .supervisor
        .spawn(session, &sh_spec(script, dir), &spawn_ctx(&state.clone()))
        .unwrap();
    let landed = wait_for(5_000, || {
        state
            .ledger
            .session_rows(session)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.role == "root" && row.state == "running")
            })
            .unwrap_or(false)
    });
    assert!(landed, "root row never landed in the ledger");
    state
        .ledger
        .session_rows(session)
        .unwrap()
        .iter()
        .find(|row| row.role == "root")
        .and_then(|row| row.pid)
        .unwrap()
}

fn cancel_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// A writer whose intent ALWAYS fails — the fault-injection gate.
struct FailingAudit;

impl AuditWriter for FailingAudit {
    fn intent(
        &self,
        _record: &juggler_core::storage::CleanupAuditRecord,
    ) -> Result<i64, AuditWriteError> {
        Err(AuditWriteError)
    }

    fn complete(
        &self,
        _id: i64,
        _outcome: &str,
        _ports_after: Option<&str>,
    ) -> Result<(), AuditWriteError> {
        Err(AuditWriteError)
    }
}

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

/// Guardrail (quality gate): a failed audit intent REFUSES the kill
/// with the typed error and ZERO signals — the trap log stays empty
/// and the process keeps running.
#[test]
fn guardrail_no_audit_means_no_signal() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let session = SessionId::new("sess-no-audit");
    let script = format!(
        "trap 'echo term >> {}/term.log' TERM; sleep 300 & wait",
        dir.path().display()
    );
    let pid = spawn_root(&state, &session, &script, dir.path());

    let refused = commands::kill_execute_audited(
        &state,
        &FailingAudit,
        session.as_str(),
        pid,
        "pid",
        &cancel_flag(),
        None,
    );
    match refused {
        Err(AppError::AuditUnavailable) => {}
        other => panic!("expected AuditUnavailable, got {other:?}"),
    }
    assert!(alive(pid), "zero signals — the process must survive");
    assert!(
        !dir.path().join("term.log").exists(),
        "no TERM may be delivered without an audit intent row"
    );
    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
}

/// Guardrail (quality gate): the kill path re-verifies in the SAME
/// call — a pid whose recorded identity no longer matches (here: the
/// process died between the sheet and the click) gets ZERO signals
/// and the exact refusal copy, and NO audit row is written.
#[test]
fn guardrail_stale_identity_refuses_with_zero_signals() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let session = SessionId::new("sess-stale");
    let pid = spawn_root(&state, &session, "sleep 300", dir.path());

    // The PID reuse shape: the recorded process is gone by click time.
    kill9(pid);
    assert!(wait_for(3_000, || !alive(pid)));

    let verify = commands::kill_verify_inner(&state, session.as_str(), pid).unwrap();
    assert!(!verify.verified);
    assert_eq!(verify.refusal.as_deref(), Some(KILL_REFUSAL_COPY));

    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        pid,
        "pid",
        &cancel_flag(),
        None,
    )
    .unwrap();
    assert_eq!(outcome.refusal.as_deref(), Some(KILL_REFUSAL_COPY));
    assert_eq!(outcome.members[0].outcome, "refused");
    assert!(!outcome.all_succeeded);
    assert!(
        state.audit.history(0).unwrap().is_empty(),
        "no intent row without a verified identity"
    );
    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
}

/// Fixture (1) — the vite-like helper: `python3 -m http.server` as a
/// helper of a supervised root, killed PID-only through the sheet,
/// exits on TERM and the port is honestly FREE afterwards; the audit
/// row carries the outcome and the before/after port facts.
#[test]
fn vite_like_helper_frees_its_port_on_term() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let session = SessionId::new("sess-vite");
    let port = free_port();
    let pid = spawn_root(
        &state,
        &session,
        &format!("python3 -m http.server {port} --bind 127.0.0.1 & wait"),
        dir.path(),
    );
    let _ = pid;

    // The reconciler tick discovers and records the helper (its row
    // carries the kernel exe — the E506 dogfood requirement).
    let mut collector = LiveStatsCollector::new();
    let landed = wait_for(10_000, || {
        collector.tick(&state.supervisor, now_ms());
        state
            .ledger
            .session_rows(&session)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.role == "helper"
                        && row.state == "running"
                        && row.exe_path.to_ascii_lowercase().contains("python")
                })
            })
            .unwrap_or(false)
    });
    assert!(landed, "helper row never landed");
    assert!(wait_for(5_000, || port_open(port)), "port never opened");
    let helper_pid = state
        .ledger
        .session_rows(&session)
        .unwrap()
        .iter()
        .find(|row| row.role == "helper" && row.exe_path.to_ascii_lowercase().contains("python"))
        .and_then(|row| row.pid)
        .unwrap();

    // Step 1 — the sheet's verify: click-time identity + the port.
    let verify = commands::kill_verify_inner(&state, session.as_str(), helper_pid).unwrap();
    assert!(verify.verified, "live helper must verify: {verify:?}");
    assert!(verify.ports.contains(&port));
    assert_eq!(verify.platform, "TERM → grace → KILL");
    assert!(verify.members.iter().any(|member| member.pid == helper_pid));

    // Step 3 — PID-only ladder over the verified helper.
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        helper_pid,
        "pid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(
        outcome
            .members
            .iter()
            .any(|member| member.pid == helper_pid && member.outcome == "exited on TERM"),
        "per-process outcome missing: {:?}",
        outcome.members
    );
    assert!(outcome.all_succeeded);
    assert!(
        outcome.still_holding.is_empty(),
        "the port must be free, not claimed: {:?}",
        outcome.still_holding
    );
    assert!(
        wait_for(3_000, || !port_open(port)),
        "port {port} still open after the ladder"
    );
    // The audit row: intent + completion with the honest facts.
    let history = state.audit.history(0).unwrap();
    let row = history
        .iter()
        .find(|row| row.pid == helper_pid)
        .unwrap_or_else(|| panic!("no audit row for {helper_pid}: {history:#?}"));
    assert!(row.outcome.contains("exited on TERM"));
    assert_eq!(row.ports_before.as_deref(), Some(port.to_string().as_str()));
    println!("[vite-fixture] port {port} freed; audit: {row:#?}");

    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
}

/// Fixture (2) — the TERM-ignoring helper: a trapped root ignores
/// SIGTERM (logged — the delivered-signal witness), the ladder KILLs
/// it after grace and reports `needed KILL`.
#[test]
fn term_ignoring_helper_is_killed_after_grace() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let session = SessionId::new("sess-ignorer");
    let term_log = dir.path().join("term.log");
    let script = format!(
        "trap 'echo term >> {}' TERM; while :; do sleep 300 & wait; done",
        term_log.display()
    );
    let pid = spawn_root(&state, &session, &script, dir.path());

    let verify = commands::kill_verify_inner(&state, session.as_str(), pid).unwrap();
    assert!(verify.verified, "the trapped root must verify: {verify:?}");

    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        pid,
        "pid",
        &cancel_flag(),
        None,
    )
    .unwrap();
    assert_eq!(outcome.members[0].outcome, "needed KILL");
    assert!(outcome.all_succeeded, "`needed KILL` is success");
    assert!(!alive(pid), "the ignorer must be dead after KILL");
    // The witness: TERM was DELIVERED (and survived) before the KILL.
    let logged = std::fs::read_to_string(&term_log).unwrap_or_default();
    assert!(
        logged.contains("term"),
        "TERM was never delivered — witness log: {logged:?}"
    );
    let history = state.audit.history(0).unwrap();
    assert!(
        history
            .iter()
            .any(|row| row.pid == pid && row.outcome.contains("needed KILL"))
    );
    println!(
        "[ignorer-fixture] witness log: {logged:?}; members: {:?}",
        outcome.members
    );

    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
}

/// Fixture (3) — the `setsid` escapee: the helper leaves the group,
/// keeps the port after the root's PID-only kill — reported as
/// STILL HELD with the escapee named (never a success claim), then
/// the re-offer kills it PID-only after fresh verification and the
/// port is honestly free.
#[test]
fn setsid_escapee_is_reported_still_holding_then_reoffered() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let session = SessionId::new("sess-escapee");
    let port = free_port();
    let escaper = format!(
        "python3 -c 'import os; os.setsid(); import http.server; \
         s = http.server.HTTPServer((\"127.0.0.1\", {port}), \
         http.server.BaseHTTPRequestHandler); s.serve_forever()'"
    );
    let script = format!("{escaper} & sleep 300");
    let root_pid = spawn_root(&state, &session, &script, dir.path());

    // The reconciler records the escapee helper (its live PGID left
    // the root's group — the E502 escaped flag).
    let mut collector = LiveStatsCollector::new();
    let landed = wait_for(10_000, || {
        collector.tick(&state.supervisor, now_ms());
        state
            .ledger
            .session_rows(&session)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.role == "helper"
                        && row.state == "running"
                        && row.exe_path.to_ascii_lowercase().contains("python")
                })
            })
            .unwrap_or(false)
    });
    assert!(landed, "escapee helper row never landed");
    assert!(wait_for(5_000, || port_open(port)), "port never opened");
    let escapee_pid = state
        .ledger
        .session_rows(&session)
        .unwrap()
        .iter()
        .find(|row| row.role == "helper" && row.exe_path.to_ascii_lowercase().contains("python"))
        .and_then(|row| row.pid)
        .unwrap();

    // Kill the ROOT PID-only: it dies on TERM, the port does NOT.
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        root_pid,
        "pid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(outcome.all_succeeded);
    assert!(
        port_open(port),
        "the escapee keeps the port — the fixture's premise"
    );
    assert_eq!(
        outcome.still_holding.len(),
        1,
        "{:?}",
        outcome.still_holding
    );
    assert_eq!(outcome.still_holding[0].port, port);
    assert_eq!(outcome.still_holding[0].pid, escapee_pid);
    assert_eq!(outcome.reoffer_pid, Some(escapee_pid));
    println!("[escapee-fixture] root gone, :{port} still held by pid {escapee_pid}");

    // The re-offer: fresh verification of the ESCAPEE, PID-only kill,
    // and the port is honestly free.
    let verify = commands::kill_verify_inner(&state, session.as_str(), escapee_pid).unwrap();
    assert!(verify.verified, "escapee must verify: {verify:?}");
    let reoffer = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        escapee_pid,
        "pid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(reoffer.all_succeeded, "{:?}", reoffer.members);
    assert!(wait_for(3_000, || !port_open(port)));
    assert!(!alive(escapee_pid));
    println!("[escapee-fixture] re-offer done, :{port} free");

    state
        .supervisor
        .stop(&session, Duration::from_secs(5))
        .unwrap();
}
