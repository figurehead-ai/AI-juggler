//! AJ-E1004 integration: the morning summary is read from the status
//! journal — every state lands in its severity group, the transitions are
//! timestamped, nothing is lost across an app restart (journal replay),
//! "seen" moves the window, and 50 Instances summarise inside 2 s.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::overnight;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

fn seed(state: &Arc<AppState>, dir: &Path) {
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
}

/// An Instance walked through `path` (journalled on each change).
fn walk(state: &Arc<AppState>, id: &str, path: &[SessionStatus], dir: &Path) {
    let storage = state.storage.lock().unwrap();
    let mut record = InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new("rec-work"),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: dir.to_path_buf(),
        status: SessionStatus::Starting,
        started_at: Timestamp::now(),
        last_activity: None,
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    };
    storage.sessions().insert(&record).unwrap();
    for status in path {
        record.status = *status;
        storage.sessions().update(&record).unwrap();
    }
}

#[test]
fn every_state_lands_in_its_group_and_a_restart_loses_nothing() {
    use SessionStatus::*;
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("a");
    let state = app_state(&data);
    seed(&state, work.path());
    walk(&state, "s-crashed", &[Running, Failed], work.path());
    walk(&state, "s-stuck", &[Running, Stuck], work.path());
    walk(&state, "s-waiting", &[Running, WaitingUser], work.path());
    walk(&state, "s-quota", &[Running, QuotaBlocked], work.path());
    walk(&state, "s-running", &[Running], work.path());
    walk(&state, "s-finished", &[Running, Finished], work.path());
    walk(&state, "s-stopped", &[Running, Stopped], work.path());

    let summary = overnight::overnight_summary_impl(&state, None).unwrap();
    let order: Vec<(&str, &str)> = summary
        .rows
        .iter()
        .map(|row| (row.session_id.as_str(), row.group.as_str()))
        .collect();
    assert_eq!(
        order[0],
        ("s-crashed", "crashed"),
        "crashes first: {order:?}"
    );
    assert!(
        order[1..4].iter().all(|(_, group)| *group == "needs_you"),
        "{order:?}"
    );
    assert_eq!(order[4], ("s-running", "running"));
    assert_eq!(order[5], ("s-finished", "finished"));
    assert_eq!(order[6], ("s-stopped", "ended"));
    assert_eq!(summary.counts["needs_you"], 3);
    let crashed = &summary.rows[0];
    assert_eq!(
        crashed
            .transitions
            .iter()
            .map(|t| t.to.as_str())
            .collect::<Vec<_>>(),
        ["starting", "running", "failed"],
        "the history, timestamped, not reconstructed"
    );
    assert!(crashed.transitions.iter().all(|t| t.at_ms > 0));
    assert_eq!(crashed.tokens, None, "unmeasured stays unknown");

    // The app restarts: the same journal, the same summary.
    let since = summary.since_ms;
    drop(state);
    let state = app_state(&data);
    let again = overnight::overnight_summary_impl(&state, Some(since)).unwrap();
    for before in &summary.rows {
        let after = again
            .rows
            .iter()
            .find(|row| row.session_id == before.session_id)
            .unwrap();
        assert_eq!(
            &after.transitions[..before.transitions.len()],
            &before.transitions[..],
            "no transition lost across the restart ({})",
            before.session_id
        );
    }
    // Crash recovery on start-up is itself journalled: the Instance that
    // was running (its process gone) is now recorded as stopped.
    let running = again
        .rows
        .iter()
        .find(|row| row.session_id == "s-running")
        .unwrap();
    assert_eq!(running.transitions.last().unwrap().to, "stopped");

    // Seen: only what changes from now on (plus what still runs).
    overnight::overnight_summary_seen_impl(&state).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    let fresh = overnight::overnight_summary_impl(&state, None).unwrap();
    assert!(
        fresh.rows.is_empty(),
        "nothing changed since the last look and nothing runs: {:?}",
        fresh.rows
    );
}

#[test]
fn fifty_instances_summarise_inside_two_seconds() {
    use SessionStatus::*;
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("a"));
    seed(&state, work.path());
    for n in 0..50 {
        let path: &[SessionStatus] = match n % 5 {
            0 => &[Running, Failed],
            1 => &[Running, Stuck],
            2 => &[Running],
            3 => &[Running, Finished],
            _ => &[Running, QuotaBlocked],
        };
        walk(&state, &format!("s-{n}"), path, work.path());
    }
    let started = Instant::now();
    let summary = overnight::overnight_summary_impl(&state, None).unwrap();
    let took = started.elapsed();
    eprintln!("[perf] overnight summary over 50 Instances: {took:?}");
    assert_eq!(summary.rows.len(), 50);
    assert!(took < Duration::from_secs(2), "{took:?}");
}
