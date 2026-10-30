//! AJ-E410 app-level integration: the Processes command surface, the
//! left-behind detection, the one verified stop path, and the opted-in
//! Tier-1 auto-cleanup — all against real fake-harness processes and
//! real storage.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, Timestamp,
};

fn fake_harness_bin() -> PathBuf {
    common::fake_harness::bin()
}

/// A real AppState (in-repo storage dir) with the launch chain seeded.
fn seeded_state(dir: &Path, cleanup_opt_in: bool) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-e410"),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: "Work Max".into(),
                plan: None,
                state_dir: dir.join("account-state"),
                cleanup_opt_in,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
                in_place: false,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new("rec-e410"),
                name: "fixture".into(),
                account: AccountId::new("acc-e410"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
    }
    state
}

fn seed_instance(state: &Arc<AppState>, dir: &Path, id: &str) -> SessionId {
    let session = SessionId::new(id);
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new("rec-e410"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    session
}

fn harness_spec(extra: &[&str], cwd: &Path) -> LaunchSpec {
    LaunchSpec {
        program: fake_harness_bin(),
        args: extra.iter().map(|arg| (*arg).to_string()).collect(),
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

/// Spawns the orphan fixture under the state's supervisor, ticks the
/// collector until the tree is visible, lets the harness end, waits for
/// the non-clean exit,
/// and runs the exit-snapshot hook. Returns (session, child pid).
fn drive_orphan_fixture(state: &Arc<AppState>, dir: &Path, id: &str) -> (SessionId, u32) {
    let session = seed_instance(state, dir, id);
    let go = dir.join(format!("{id}.go"));
    state
        .supervisor
        .spawn(
            &session,
            &harness_spec(
                &[
                    "run",
                    "--dir",
                    dir.join("transcripts").to_str().unwrap(),
                    "--turns",
                    "3",
                    // The child is in the session's tree only while the
                    // harness lives (then launchd adopts it). A timer's
                    // window was missed under a loaded run (0.3 s turns on
                    // 2026-10-24, then 3 × 1.5 s on 2026-10-30): the
                    // harness now ends only when the test has seen the
                    // child and says so.
                    "--exit-when",
                    go.to_str().unwrap(),
                    "--spawn-child",
                    "/bin/sleep",
                    "--child-args",
                    "120",
                    "--exit",
                    "7",
                ],
                dir,
            ),
            &ctx(state),
        )
        .unwrap();

    // Tick until the tree shows root + child (the snapshot source).
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut child_pid = 0_u32;
    let state_ref = &state;
    while Instant::now() < deadline {
        commands::processes_tick(state_ref);
        let info = commands::instance_processes_arc(state_ref, &session.to_string()).unwrap();
        if let Some(row) = info.rows.iter().find(|row| row.name.contains("sleep")) {
            child_pid = row.pid;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(child_pid != 0, "processes command never saw the child");
    // Seen: the harness may end now.
    std::fs::write(&go, b"").unwrap();

    // Wait for the non-clean exit and run the forwarder's snapshot hook.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut exited = false;
    while Instant::now() < deadline && !exited {
        while let Some(event) = state.supervisor.try_recv() {
            if let juggler_supervisor::LifecycleEvent::Exited { session, pid, kind } = event {
                commands::record_exit_snapshot(state, &session, pid, &kind);
                exited = true;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(exited, "fixture harness never exited");
    (session, child_pid)
}

fn ctx(state: &Arc<AppState>) -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

#[test]
fn processes_command_reports_tree_and_the_sheet_stops_what_was_left() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), false);
    let state_ref = &state;
    let (session, child_pid) = drive_orphan_fixture(state_ref, dir.path(), "sess-proc");

    // The frozen tree rows are still reported post-exit (the Processes
    // tab's final snapshot), with no unavailability note.
    let info = commands::instance_processes_arc(state_ref, &session.to_string()).unwrap();
    assert!(info.rows.iter().any(|row| row.pid == child_pid));
    assert!(info.unavailable_reason.is_none());
    assert!(!info.spark.is_empty(), "the 2 s ring retained samples");

    // AJ-E2407: what was left behind is the ledger's (the reconciler's)
    // and the Processes view's; stopping it is the verified sheet's job:
    // verify, then the audited ladder.
    commands::processes_tick(&state);
    let verify = commands::kill_verify_inner(state_ref, session.as_str(), child_pid).unwrap();
    assert!(
        verify.verified,
        "the left-behind child verifies: {verify:?}"
    );
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = commands::kill_execute_audited(
        state_ref,
        state.audit.as_ref(),
        session.as_str(),
        child_pid,
        "pid",
        &cancel,
        None,
    )
    .unwrap();
    assert!(outcome.refusal.is_none());
    assert!(outcome.all_succeeded, "the child went: {outcome:?}");

    // The audit trail: the intent and its outcome, by hand.
    let rows = state.audit.history(0).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pid, child_pid);
    assert_eq!(rows[0].actor, "user:kill_sheet");
}

#[test]
fn the_sheet_refuses_a_pid_the_ledger_never_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), false);
    let state_ref = &state;
    let (session, _child) = drive_orphan_fixture(state_ref, dir.path(), "sess-refuse");

    // An unrelated live process this app never supervised.
    let mut bystander = std::process::Command::new("/bin/sleep")
        .arg("120")
        .spawn()
        .unwrap();
    let pid = bystander.id();
    assert!(
        commands::kill_verify_inner(state_ref, session.as_str(), pid).is_err(),
        "no ledger row, nothing to verify"
    );
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let refused = commands::kill_execute_audited(
        state_ref,
        state.audit.as_ref(),
        session.as_str(),
        pid,
        "pid",
        &cancel,
        None,
    );
    assert!(refused.is_err(), "no ledger row, no kill");
    assert!(
        juggler_supervisor::process_alive(pid).is_some(),
        "the bystander must survive the refused stop"
    );
    // Refused stops are not audit-logged as kills (nothing was killed).
    assert!(state.audit.history(0).unwrap().is_empty());
    let _ = bystander.kill();
    let _ = bystander.wait();
}

#[test]
fn unknown_instance_is_an_error_not_an_empty_table() {
    let dir = tempfile::tempdir().unwrap();
    let state = seeded_state(dir.path(), false);
    let state_ref = &state;
    let error = commands::instance_processes_arc(state_ref, "no-such-instance").unwrap_err();
    assert!(
        error.to_string().contains("no-such-instance"),
        "the failure names the instance"
    );
}
