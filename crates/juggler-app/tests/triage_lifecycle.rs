//! AJ-E1005 integration: an ended Instance leaves the triage's finished
//! list only through an explicit "reviewed" step, recorded with its time
//! in the lifecycle journal; a live Instance cannot be marked reviewed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_app::lifecycle;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn seed(state: &Arc<AppState>, dir: &Path, sessions: &[(&str, SessionStatus)]) {
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&Account {
            id: AccountId::new("acc-work"),
            harness: HarnessId::new("claude-code"),
            label: "Work Max".into(),
            plan: None,
            state_dir: dir.join("state-work"),
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
            id: ProfileId::new("rec-work"),
            name: "Main".into(),
            account: AccountId::new("acc-work"),
            settings_overlay: Default::default(),
            default_cwd: None,
        })
        .unwrap();
    for (id, status) in sessions {
        storage
            .sessions()
            .insert(&InstanceRecord {
                id: SessionId::new(*id),
                conversation: None,
                profile: ProfileId::new("rec-work"),
                machine: MachineId::new("this-mac"),
                kind: InstanceKind::Terminal,
                project: None,
                cwd: dir.to_path_buf(),
                status: *status,
                started_at: Timestamp::now(),
                last_activity: None,
                usage: Default::default(),
                attention: None,
                running: None,
                transcript_ref: None,
                window: None,
            })
            .unwrap();
    }
}

#[test]
fn reviewed_is_an_explicit_recorded_step_and_only_for_an_ended_instance() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(
        &state,
        work.path(),
        &[
            ("s-done", SessionStatus::Finished),
            ("s-live", SessionStatus::WaitingUser),
        ],
    );
    let row = |id: &str| {
        juggler_app::commands::instances_shared_snapshot(&state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap()
    };
    assert_eq!(
        row("s-done").lifecycle,
        None,
        "unreviewed until the user says so"
    );

    // This thread's CPU time (other programs' turns on a busy machine
    // don't count against it); wall time is printed beside it.
    let wall = std::time::Instant::now();
    let started = cpu_time::ThreadTime::now();
    let at = lifecycle::instance_mark_reviewed_impl(&state, "s-done").unwrap();
    let done = row("s-done");
    let (took, wall) = (started.elapsed(), wall.elapsed());
    eprintln!("[perf] mark reviewed → visible in the snapshot: {took:?} CPU, {wall:?} wall");
    assert!(
        took < std::time::Duration::from_millis(200),
        "{took:?} CPU (wall {wall:?})"
    );
    assert_eq!(done.lifecycle.as_deref(), Some("reviewed"));
    assert_eq!(done.lifecycle_at_ms, Some(at));

    let refused = lifecycle::instance_mark_reviewed_impl(&state, "s-live").unwrap_err();
    assert!(refused.to_string().contains("ended"), "{refused}");
    assert!(lifecycle::instance_mark_reviewed_impl(&state, "s-nope").is_err());

    // Kept across a restart: it is history, not view state.
    drop(state);
    let state = app_state(work.path());
    let again = juggler_app::commands::instances_shared_snapshot(&state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == "s-done")
        .unwrap();
    assert_eq!(again.lifecycle.as_deref(), Some("reviewed"));
}
