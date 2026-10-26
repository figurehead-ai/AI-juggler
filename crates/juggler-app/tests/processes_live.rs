//! AJ-E2401/E2404/E2405 end to end, against a real fake-harness session
//! and real storage: the graph is built on the 2 s tick's own data (no
//! scan of its own), shows the session and its subprocess, says so when
//! the session ends and leaves it behind (the reconciler's
//! `HelperOrphaned`, no longer dropped), agrees with This Mac's count,
//! honours Keep, pages, and keeps what ended for the Ended view.
//!
//! Every process here is the test's own fixture, stopped through the
//! ledger's verified path at the end.

#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::features::processes::{
    self, commands as pc, dto::ProcessQueryInput, model::ProcState,
};
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, Timestamp,
};

/// Ends the fixture child however the test ends (its own pid only).
struct Reap(u32);

impl Drop for Reap {
    fn drop(&mut self) {
        if self.0 != 0 {
            let _ = std::process::Command::new("/bin/kill")
                .args(["-9", &self.0.to_string()])
                .status();
        }
    }
}

fn seeded_state(dir: &Path) -> Arc<AppState> {
    let state = Arc::new(
        AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new("acc-e24"),
                harness: juggler_core::model::HarnessId::new("fake-harness"),
                label: "Work Max".into(),
                plan: None,
                state_dir: dir.join("account-state"),
                cleanup_opt_in: false,
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
                id: ProfileId::new("rec-e24"),
                name: "fixture".into(),
                account: AccountId::new("acc-e24"),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new("sess-e24"),
                conversation: None,
                profile: ProfileId::new("rec-e24"),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: dir.join("web-app"),
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
    }
    state
}

/// The 2 s tick as the app runs it: the collector's, then the graph's;
/// the forwarder's part for orphans and exits. Returns whether the graph
/// said it changed.
fn tick(state: &Arc<AppState>) -> bool {
    commands::processes_tick(state);
    while let Some(event) = state.supervisor.try_recv() {
        match event {
            juggler_supervisor::LifecycleEvent::HelperOrphaned { session, pid } => {
                processes::helper_orphaned(state, &session, pid);
            }
            juggler_supervisor::LifecycleEvent::Exited { session, pid, kind } => {
                commands::record_exit_snapshot(state, &session, pid, &kind);
            }
            juggler_supervisor::LifecycleEvent::Started { .. } => {}
        }
    }
    processes::tick(state).is_some()
}

fn child_state(state: &Arc<AppState>, pid: u32) -> Option<ProcState> {
    state
        .processes
        .graph()
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|session| session.processes.iter())
        .find(|node| node.pid == pid)
        .map(|node| node.state)
}

fn wait_for(what: &str, secs: u64, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!("never: {what}");
}

#[test]
fn a_session_and_what_it_leaves_behind_on_the_one_model() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("web-app")).unwrap();
    let state = seeded_state(dir.path());
    state.supervisor.set_orphan_grace("fake-harness", 1);
    let session = SessionId::new("sess-e24");
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-e24".into()),
        project_id: Some("web-app".into()),
        machine: commands::local_machine_id(&state),
        role: "root",
    };
    state
        .supervisor
        .spawn(
            &session,
            &LaunchSpec {
                program: common::fake_harness::bin(),
                args: [
                    "run",
                    "--dir",
                    dir.path().join("transcripts").to_str().unwrap(),
                    "--turns",
                    "3",
                    "--delay-ms",
                    "1500",
                    "--spawn-child",
                    "/bin/sleep",
                    "--child-args",
                    "120",
                    "--exit",
                    "0",
                ]
                .iter()
                .map(|arg| (*arg).to_string())
                .collect(),
                env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
                cwd: dir.path().join("web-app"),
                needs_pty: false,
            },
            &ctx,
        )
        .unwrap();
    // 1. The session and its subprocess, AI Juggler's, under AI Juggler.
    let mut reap = Reap(0);
    let mut changed = false;
    wait_for("the child in the graph", 40, || {
        changed |= tick(&state);
        let graph = state.processes.graph();
        let found = graph
            .apps
            .iter()
            .filter(|app| app.ours)
            .flat_map(|app| app.sessions.iter())
            .flat_map(|s| s.processes.iter())
            .find(|node| node.name.contains("sleep"));
        if let Some(node) = found {
            reap = Reap(node.pid);
            return true;
        }
        false
    });
    assert!(changed, "the graph said it changed");
    let child = reap.0;
    assert!(matches!(
        child_state(&state, child),
        Some(ProcState::Working | ProcState::Idle)
    ));
    // 2. The session ends; its child lives on: Left behind, told once.
    wait_for("the child left behind", 40, || {
        tick(&state);
        child_state(&state, child) == Some(ProcState::LeftBehind)
    });
    let graph = state.processes.graph();
    assert!(graph.counts.to_clean >= 1);
    assert_eq!(
        state.processes.leftovers().map(|(n, _)| n),
        Some(graph.counts.to_clean)
    );
    let page = pc::page_impl(
        &state,
        &ProcessQueryInput {
            view: "attention".into(),
            ..ProcessQueryInput::default()
        },
    )
    .unwrap();
    assert!(
        page.items
            .iter()
            .any(|row| row.process.as_ref().is_some_and(|p| p.pid == child))
    );
    assert_eq!(page.counts.to_clean, graph.counts.to_clean);
    // 3. Keep takes it out of what to clean, and the sweep honours it.
    let start = state
        .processes
        .graph()
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|s| s.processes.iter())
        .find(|node| node.pid == child)
        .unwrap()
        .start_time;
    pc::mark_impl(&state, child, start, Some("keep"), None).unwrap();
    tick(&state);
    assert_eq!(
        state.processes.graph().counts.to_clean,
        graph.counts.to_clean - 1
    );
    assert!(state.processes.is_kept(child, start));
    pc::mark_impl(&state, child, start, None, None).unwrap();
    // 4. Stopped through the ledger's verified path: then Ended (7 days).
    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        child,
        "pid",
        &flag,
        None,
    )
    .unwrap();
    assert!(outcome.all_succeeded, "{outcome:?}");
    reap.0 = 0;
    wait_for("the child in Ended", 30, || {
        tick(&state);
        let ended = pc::page_impl(
            &state,
            &ProcessQueryInput {
                view: "ended".into(),
                ..ProcessQueryInput::default()
            },
        )
        .unwrap();
        ended
            .items
            .iter()
            .any(|row| row.process.as_ref().is_some_and(|p| p.pid == child))
    });
    // 5. The history names it, for this session.
    let history = pc::history_impl(&state, Some(session.as_str()), None, None, None).unwrap();
    assert!(history.items.iter().any(|event| event.pid == child));
}
