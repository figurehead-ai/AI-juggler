//! AJ-E503 evidence: the PID-reuse drill (pass2 §14.3). A PID is a
//! name, not an identity — `verify` is the gate every kill path must
//! pass, and it FAILS CLOSED: any single-field mismatch, any probe
//! error, a dead pid, or a boot change refuses. The drill runs the
//! gate against a REAL spawned process with forged expected
//! identities (real PID reuse is not forceable), asserts the ledger
//! flip is permanent, and proves 0 signals reached the fixture: the
//! trap log stays empty and the process stays alive until the
//! control arm (correct identity) deliberately stops it.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::LaunchSpec;
use juggler_core::model::{MachineId, SessionId};
use juggler_core::storage::ProcessLedgerRepo;
use juggler_supervisor::Supervisor;
use juggler_supervisor::tree::{
    CoreLedgerStore, LedgerStore as _, SpawnContext, Verification, verify,
};

fn ctx() -> SpawnContext {
    SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-identity".into()),
        project_id: Some("proj-identity".into()),
        machine: MachineId::new("machine-identity"),
        role: "root",
    }
}

fn sh_spec(script: &str, cwd: &std::path::Path) -> LaunchSpec {
    LaunchSpec {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        env: Default::default(),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn wait_gone(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        // kill(pid, 0): Ok = alive, ESRCH = gone.
        if nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(i32::try_from(pid).unwrap()),
            None,
        )
        .is_err()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn pid_reuse_drill_fails_closed_and_refuses_to_signal() {
    let dir = tempfile::tempdir().unwrap();
    let trap_log = dir.path().join("trap.log");

    // Evidence runs may pin a persistent DB (AIJ_E503_DB) so the
    // sqlite dump can be taken after the test.
    let db = std::env::var("AIJ_E503_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dir.path().join("identity.sqlite"));
    let repo = ProcessLedgerRepo::open(&db).unwrap();
    let store = Arc::new(CoreLedgerStore::new(repo));
    let supervisor = Supervisor::new().with_ledger(store.clone());
    let session = SessionId::new("sess-identity");

    // The fixture: a TERM-trapping root that writes a log line on
    // EVERY signal it receives — the drill's 0-signal witness.
    let trap = trap_log.to_string_lossy().into_owned();
    let script = format!("trap 'echo signal >> \"{trap}\"' TERM; sleep 600 & wait");
    let spawned = supervisor
        .spawn(&session, &sh_spec(&script, dir.path()), &ctx())
        .unwrap();
    let pid = spawned.pid;

    // Let the identity settle (the journal's authoritative record
    // lands at +250 ms; root_identity reads the refined handle).
    std::thread::sleep(Duration::from_millis(600));
    let recorded = supervisor
        .root_identity(&session)
        .expect("spawn probe landed (retry budget)");
    assert_eq!(recorded.pid, pid);
    println!(
        "[identity] recorded pid={} pgid={:?} start_time={} exe={:?} boot={}",
        recorded.pid, recorded.pgid, recorded.start_time, recorded.exe, recorded.boot_id
    );

    // 1. The true record verifies against the live process.
    assert_eq!(verify(pid, &recorded), Verification::Verified);

    // 2. Single-field forgeries (the spec's simulated stranger):
    //    every one of the three §6.2 facts refuses, alone.
    let mut stranger_start = recorded.clone();
    stranger_start.start_time += 7;
    assert_eq!(
        verify(pid, &stranger_start),
        Verification::Unverifiable,
        "start-time mismatch must refuse"
    );

    let mut stranger_exe = recorded.clone();
    stranger_exe.exe = "/usr/local/bin/definitely-not-ours".into();
    assert_eq!(
        verify(pid, &stranger_exe),
        Verification::Unverifiable,
        "exe mismatch must refuse"
    );

    let mut stranger_boot = recorded.clone();
    stranger_boot.boot_id = "1".into();
    assert_eq!(
        verify(pid, &stranger_boot),
        Verification::Unverifiable,
        "boot mismatch must refuse"
    );

    // 3. The ledger flip is PERMANENT: out of candidates, immune to
    //    the prune (roundtrip-tested in storage; asserted here on a
    //    real ledger row).
    store.mark_unverifiable(pid, &recorded.boot_id).unwrap();
    let flipped = store
        .rows_for_session(&session)
        .unwrap()
        .into_iter()
        .find(|row| row.pid == Some(pid))
        .expect("root row exists");
    assert_eq!(flipped.state, juggler_core::storage::STATE_UNVERIFIABLE);
    assert!(
        !store
            .candidates(&MachineId::new("machine-identity"))
            .unwrap()
            .iter()
            .any(|row| row.pid == Some(pid)),
        "unverifiable rows leave the candidate set"
    );
    store.prune(i64::MAX).unwrap();
    assert!(
        store
            .rows_for_session(&session)
            .unwrap()
            .iter()
            .any(|row| row.pid == Some(pid)),
        "the prune never removes an unverifiable row"
    );

    // 4. 0-signal witness: after every refusal (and the flip), the
    //    fixture has received NOTHING — trap log absent, still alive.
    assert!(
        !trap_log.exists(),
        "no signal may reach a process that failed verification"
    );
    assert!(
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(i32::try_from(pid).unwrap()),
            None
        )
        .is_ok(),
        "the fixture must still be alive — refusals never signal"
    );

    // 5. Dead-pid arm: kill the fixture ourselves, wait for the
    //    reaper, and the SAME recorded identity now fails closed
    //    (probe error is a refusal, never an acquittal).
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(pid).unwrap()),
        nix::sys::signal::Signal::SIGKILL,
    )
    .unwrap();
    assert!(wait_gone(pid), "fixture terminated for the dead-pid arm");
    assert_eq!(
        verify(pid, &recorded),
        Verification::Unverifiable,
        "a dead pid is unverifiable — never Verified on error"
    );
    let _ = supervisor.stop(&session, Duration::from_secs(1));
}

/// The control arm: the SAME fixture shape with its TRUE identity
/// stops cleanly through the normal ladder — the drill's refusals
/// are the gate working, not a broken kill path.
#[test]
fn control_arm_true_identity_stops_normally() {
    let dir = tempfile::tempdir().unwrap();
    let trap_log = dir.path().join("trap.log");

    let supervisor = Supervisor::new();
    let session = SessionId::new("sess-control");
    let trap = trap_log.to_string_lossy().into_owned();
    let script = format!("trap 'echo signal >> \"{trap}\"' TERM; sleep 600 & wait");
    let spawned = supervisor
        .spawn(&session, &sh_spec(&script, dir.path()), &ctx())
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));

    supervisor.stop(&session, Duration::from_secs(5)).unwrap();
    assert!(wait_gone(spawned.pid), "true identity stops via the ladder");
}
