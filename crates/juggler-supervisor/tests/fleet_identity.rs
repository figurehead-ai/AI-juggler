//! AJ-E511 evidence: fleet-shaped ownership — the `session_id +
//! machine_id` composite key is the ONLY join key (the same session
//! on two machines stays two disjoint row-sets, never merged), the
//! machine-scoped candidate read excludes foreign rows, and the
//! typed `WrongMachine` refusal names both machines. The kill-path
//! end of the law (foreign candidate → refusal copy, ZERO signals)
//! is proven in `juggler-app/tests/guardrails.rs`.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::sync::Arc;

use juggler_core::model::MachineId;
use juggler_core::storage::LedgerRow;
use juggler_supervisor::SupervisorError;
use juggler_supervisor::tree::{CoreLedgerStore, LedgerStore};

const NOW: i64 = 1_728_800_000_000;

fn row(session: &str, machine: &str, pid: u32, role: &str, state: &str) -> LedgerRow {
    LedgerRow {
        pid: Some(pid),
        pgid: None,
        started_at: Some(100),
        exe_path: "/bin/sleep".into(),
        cmdline_hash: format!("h-{pid}"),
        cmdline_trunc: format!("sleep 300 ({session}/{machine})"),
        boot_id: "boot-fleet".into(),
        session_id: session.into(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: Some("acc-fleet".into()),
        project_id: Some("proj-fleet".into()),
        machine_id: machine.into(),
        root_pid: Some(pid),
        parent_pid: Some(pid),
        depth: if role == "root" { 0 } else { 1 },
        role: role.into(),
        state: state.into(),
        listens_on: None,
        cwd: "/tmp".into(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: NOW,
        last_seen: NOW,
    }
}

#[test]
fn composite_key_two_machines_same_session_stay_disjoint() {
    let repo = juggler_core::storage::ProcessLedgerRepo::open_in_memory().unwrap();
    // The SAME session id on TWO machines — the fleet shape.
    repo.upsert_helpers(
        "sess-fleet",
        &[
            row("sess-fleet", "machine-a", 1001, "root", "running"),
            row("sess-fleet", "machine-a", 1002, "helper", "running"),
        ],
    )
    .unwrap();
    repo.upsert_helpers(
        "sess-fleet",
        &[
            row("sess-fleet", "machine-b", 2001, "root", "running"),
            row("sess-fleet", "machine-b", 2002, "helper", "orphaned"),
        ],
    )
    .unwrap();

    let a = repo
        .rows_for_session_on_machine("sess-fleet", "machine-a")
        .unwrap();
    let b = repo
        .rows_for_session_on_machine("sess-fleet", "machine-b")
        .unwrap();
    assert_eq!(a.len(), 2, "machine-a's rows only");
    assert_eq!(b.len(), 2, "machine-b's rows only");
    assert!(
        a.iter().all(|row| row.machine_id == "machine-a"),
        "session-only reads would MERGE the machines — the false-kill bug at fleet scale"
    );
    assert!(b.iter().all(|row| row.machine_id == "machine-b"));
    // The composite-key walk sees BOTH keys (never merged, never lost).
    let mut keys = repo.live_session_machines().unwrap();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            ("sess-fleet".to_string(), "machine-a".to_string()),
            ("sess-fleet".to_string(), "machine-b".to_string()),
        ]
    );
}

#[test]
fn candidates_are_machine_scoped() {
    let repo = juggler_core::storage::ProcessLedgerRepo::open_in_memory().unwrap();
    repo.upsert_helpers(
        "sess-a",
        &[
            row("sess-a", "machine-a", 1001, "root", "running"),
            row("sess-a", "machine-a", 1002, "helper", "orphaned"),
        ],
    )
    .unwrap();
    repo.upsert_helpers(
        "sess-b",
        &[row("sess-b", "machine-b", 2002, "helper", "orphaned")],
    )
    .unwrap();

    let store: Arc<dyn LedgerStore> = Arc::new(CoreLedgerStore::new(repo));
    let local = store.candidates(&MachineId::new("machine-a")).unwrap();
    assert!(
        local.iter().all(|row| row.machine_id == "machine-a"),
        "this node's sweep NEVER sees machine-b's rows: {local:?}"
    );
    assert!(local.iter().any(|row| row.pid == Some(1002)));
}

#[test]
fn wrong_machine_is_typed_and_names_both_machines() {
    let err = SupervisorError::WrongMachine {
        recorded: MachineId::new("machine-b"),
        local: MachineId::new("machine-a"),
    };
    let text = err.to_string();
    assert!(
        text.contains("machine-b") && text.contains("machine-a"),
        "the typed refusal names both machines: {text}"
    );
    assert!(
        text.contains("will not touch it"),
        "the refusal copy is the no-signal law: {text}"
    );
}
