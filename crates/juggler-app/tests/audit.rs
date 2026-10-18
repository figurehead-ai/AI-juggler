//! AJ-E509 evidence: the append-only cleanup audit — intent BEFORE
//! signal, completion with honest outcomes, the fault-injection gate
//! (no audit ⇒ NO SIGNAL), the crash-pending record, and the JSONL
//! export. Run with `--nocapture` to archive the evidence dumps.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::AdapterRegistry;
use juggler_app::commands::{self, AuditWriteError, AuditWriter};
use juggler_app::error::AppError;
use juggler_app::state::AppState;
use juggler_core::storage::{CleanupAuditRecord, CleanupAuditRepo};
use juggler_supervisor::ExitSnapshot;
use juggler_supervisor::OrphanTracker;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn record(pid: u32, actor: &str, ports_before: Option<&str>) -> CleanupAuditRecord {
    CleanupAuditRecord {
        id: 0,
        at_ms: now_ms(),
        actor: actor.to_string(),
        pid,
        pgid: Some(pid),
        exe_path: "/usr/bin/python3".into(),
        cmdline_hash: "hash-1".into(),
        cmdline_trunc: "python3 -m http.server 5173".into(),
        session_id: "sess-audit".into(),
        provider: "claude-code".into(),
        account_id: Some("acc-audit".into()),
        project_id: Some("proj-audit".into()),
        machine_id: "machine-audit".into(),
        identity: "{\"start_time\":\"verified\",\"exe\":\"verified\"}".into(),
        scope: "pid".into(),
        outcome: String::new(),
        ports_before: ports_before.map(str::to_string),
        ports_after: None,
    }
}

/// A writer whose intent ALWAYS fails — the fault-injection gate.
struct FailingAudit;

impl AuditWriter for FailingAudit {
    fn intent(&self, _record: &CleanupAuditRecord) -> Result<i64, AuditWriteError> {
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

#[test]
fn audit_intent_completes_and_history_reads_the_outcome_vocabulary() {
    let repo = CleanupAuditRepo::open_in_memory().unwrap();
    // Intent BEFORE the signal (pending), then the honest completion.
    let id = repo
        .intent(&record(8123, "user:orphan_stop", Some("5173")))
        .unwrap();
    let pending = repo.history(0).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].outcome, "TERM'd, outcome unknown");
    assert!(!pending[0].automatic);
    repo.complete(id, "exited on TERM", Some(""), now_ms())
        .unwrap();
    let done = repo.history(0).unwrap();
    assert_eq!(done[0].outcome, "exited on TERM");
    assert_eq!(done[0].ports_before.as_deref(), Some("5173"));
    assert_eq!(done[0].ports_after.as_deref(), Some(""));
    // The automatic actor reads automatic.
    let id2 = repo.intent(&record(4400, "auto:tier-1", None)).unwrap();
    repo.complete(id2, "needed KILL", None, now_ms()).unwrap();
    let rows = repo.history(0).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.automatic && row.outcome == "needed KILL")
    );
    println!("[audit-history] {rows:#?}");
}

#[test]
fn crash_mid_ladder_leaves_the_truthful_pending_record() {
    let repo = CleanupAuditRepo::open_in_memory().unwrap();
    let _id = repo
        .intent(&record(9999, "auto:tier-1", Some("8000")))
        .unwrap();
    // Crash: no completion ever happens.
    let rows = repo.history(0).unwrap();
    assert_eq!(rows[0].outcome, "TERM'd, outcome unknown");
}

#[test]
fn retention_prunes_seven_day_old_records_only() {
    let repo = CleanupAuditRepo::open_in_memory().unwrap();
    let now = now_ms();
    let mut old = record(1111, "user:orphan_stop", None);
    old.at_ms = now - 8 * 24 * 3_600_000;
    repo.intent(&old).unwrap();
    repo.intent(&record(2222, "user:orphan_stop", None))
        .unwrap();
    let removed = repo.prune(now - 7 * 24 * 3_600_000).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(repo.history(0).unwrap().len(), 1);
}

#[test]
fn fault_injection_no_audit_means_no_signal() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );

    // A REAL trapped process the tracker has recorded (the allow-list
    // guard must pass so the audit gate is the ONLY thing refusing).
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("trap 'echo term >> \"$0\"' TERM; sleep 300 & wait")
        .arg(dir.path().join("trap.log"))
        .current_dir(dir.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let session = juggler_core::model::SessionId::new("sess-fault");
    state.orphans.record_exit(ExitSnapshot {
        session: session.clone(),
        root_pid: 1,
        root_program: "fake-harness".into(),
        clean: false,
        taken_ms: now_ms(),
        rows: vec![juggler_supervisor::DescendantRow {
            pid,
            name: "sh".into(),
            rss_bytes: 1024,
        }],
    });
    // Poll past grace so the tracker confirms the orphan (allow-list ✔).
    state.orphans.poll(now_ms() + 11_000, |pid| {
        (pid == pid).then(|| ("sh".to_string(), 1024))
    });

    // The fault: intent fails ⇒ typed refusal, ZERO signals.
    let refused = commands::orphan_stop_audited(&state, &FailingAudit, &session, pid);
    match refused {
        Err(AppError::AuditUnavailable) => {}
        other => panic!("expected AuditUnavailable, got {other:?}"),
    }
    assert!(
        !dir.path().join("trap.log").exists(),
        "NO SIGNAL may reach the process when the audit intent failed"
    );
    let alive = std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|code| code.success())
        .unwrap_or(false);
    assert!(alive, "the process must still be alive (0 signals)");
    // And no intent row was written (the kill never happened).
    assert!(state.audit.history(0).unwrap().is_empty());

    // Recovery: with the REAL audit the same kill proceeds and the
    // audit row completes with the honest outcome.
    let outcome = commands::orphan_stop_inner(&state, &session, pid).unwrap();
    println!("[fault-recovered] {}", outcome.state);
    let rows = state.audit.history(0).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].actor, "user:orphan_stop");
    assert_ne!(rows[0].outcome, "TERM'd, outcome unknown");
    let _ = child.kill();
    let _ = child.wait();
    let _ = OrphanTracker::new();
    let _ = Duration::from_secs(0);
    let _ = Path::new("/");
}

#[test]
fn export_is_valid_jsonl_with_the_redaction_note() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let id = state
        .audit
        .intent(&record(8123, "user:orphan_stop", Some("5173")))
        .unwrap();
    state
        .audit
        .complete(id, "exited on TERM", Some(""), now_ms())
        .unwrap();
    let export = commands::cleanup_audit_export_inner(&state);
    println!("[audit-export]\n{export}");
    let mut lines = export.lines();
    let first = lines.next().unwrap();
    assert!(first.contains("hashed"), "the redaction note leads");
    for line in export.lines() {
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "invalid JSONL: {line}"
        );
    }
    // The full cmdline never persists — only hash + trunc.
    assert!(!export.contains("\"cmdline\":"));
}
