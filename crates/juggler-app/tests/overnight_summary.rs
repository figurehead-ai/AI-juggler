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
    eprintln!(
        "[summary] {}",
        serde_json::to_string_pretty(&summary).unwrap()
    );
    let order: Vec<(&str, &str)> = summary
        .rows
        .iter()
        .map(|row| (row.session_id.as_str(), row.group.as_str()))
        .collect();
    // D-96 C: a non-zero exit is an error, never "crashed" (no signal
    // said so); the failures first.
    assert_eq!(
        order[0],
        ("s-crashed", "error"),
        "failures first: {order:?}"
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
    eprintln!(
        "[replay] after restart: {}",
        serde_json::to_string_pretty(&again).unwrap()
    );
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

    // Seen: only what changes from now on (plus what still runs). The
    // user looks after the events (the journal is millisecond-grained:
    // a look in the very millisecond of a change still counts it).
    std::thread::sleep(Duration::from_millis(5));
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

/// AJ-E3307 (D2): one inserted straight into `status`, its last activity
/// `ago_ms` back, waiting since `waited_ms` back when given.
fn ended_or_waiting(
    state: &Arc<AppState>,
    id: &str,
    status: SessionStatus,
    ago_ms: i64,
    waited_ms: Option<i64>,
    dir: &Path,
) {
    let now = Timestamp::now().millis();
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: SessionId::new(id),
            conversation: None,
            profile: ProfileId::new("rec-work"),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status,
            started_at: Timestamp::from_millis(now - ago_ms - 20 * 60_000),
            last_activity: Some(Timestamp::from_millis(now - ago_ms)),
            usage: Default::default(),
            attention: waited_ms.map(|waited| juggler_core::model::AttentionReason {
                rule: "tool_asks".into(),
                inputs: vec!["Claude needs your permission to use Bash".into()],
                since_ms: now - waited,
            }),
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
}

#[test]
fn an_ended_one_counts_when_it_ended_and_the_longest_wait_comes_first() {
    use SessionStatus::*;
    const H: i64 = 3_600_000;
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("a"));
    seed(&state, work.path());
    // Written now (as a seeded lab, or an import, writes them), ended a
    // day and a half ago: not "since you were away" 12 h ago.
    ended_or_waiting(
        &state,
        "s-ended-long-ago",
        Finished,
        36 * H,
        None,
        work.path(),
    );
    std::thread::sleep(Duration::from_millis(5));
    ended_or_waiting(
        &state,
        "s-ended-overnight",
        Finished,
        3 * H,
        None,
        work.path(),
    );
    // Touched a minute ago by its tool's report; asked 7 h ago.
    // Written in the order they asked: the journal's newest-first put
    // the later ask on top.
    ended_or_waiting(
        &state,
        "w-asked-early",
        WaitingUser,
        60_000,
        Some(7 * H),
        work.path(),
    );
    std::thread::sleep(Duration::from_millis(5));
    ended_or_waiting(
        &state,
        "w-asked-late",
        WaitingUser,
        60_000,
        Some(H),
        work.path(),
    );
    let since = Timestamp::now().millis() - 12 * H;
    let summary = overnight::overnight_summary_impl(&state, Some(since)).unwrap();
    let order: Vec<&str> = summary
        .rows
        .iter()
        .map(|row| row.session_id.as_str())
        .collect();
    assert_eq!(
        order,
        ["w-asked-early", "w-asked-late", "s-ended-overnight"],
        "oldest wait first; the long-ago one left out"
    );
    assert_eq!(
        summary.counts.get("finished"),
        Some(&1),
        "{:?}",
        summary.counts
    );
    let early = &summary.rows[0];
    let waited = summary.generated_ms - early.at_ms.unwrap();
    assert!(
        (7 * H - 5_000..=7 * H + 5_000).contains(&waited),
        "waiting since its ask, not its last report: {waited} ms"
    );
    let ended = &summary.rows[2];
    assert!(
        ended.at_ms.unwrap() <= summary.generated_ms - 3 * H + 5_000,
        "ended when its last activity says"
    );
}

/// D-96 C: a running session that exits `kind` (the user's Stop when
/// `stopped`), through the supervisor's own path.
fn exits(
    state: &Arc<AppState>,
    id: &str,
    kind: juggler_supervisor::ExitKind,
    stopped: bool,
    dir: &Path,
) {
    walk(state, id, &[SessionStatus::Running], dir);
    let storage = state.storage.lock().unwrap();
    juggler_app::features::sessions::attention::apply_lifecycle_event(
        &storage,
        juggler_supervisor::LifecycleEvent::Exited {
            session: SessionId::new(id),
            pid: 1,
            kind,
        },
        stopped,
        None,
    )
    .unwrap();
}

#[test]
fn an_ended_run_says_how_it_ended_from_its_exit() {
    use juggler_supervisor::ExitKind::{Code, Signaled};
    let work = tempfile::tempdir().unwrap();
    let state = app_state(&work.path().join("a"));
    seed(&state, work.path());
    exits(&state, "s-done", Code(0), false, work.path());
    exits(&state, "s-error", Code(1), false, work.path());
    exits(&state, "s-crash", Signaled(Some(11)), false, work.path());
    exits(&state, "s-yours", Signaled(Some(15)), true, work.path());
    // Stopped with nothing said (AI Juggler closed while it ran, or an
    // older run): only "Ended".
    walk(
        &state,
        "s-unsaid",
        &[SessionStatus::Running, SessionStatus::Stopped],
        work.path(),
    );
    let summary = overnight::overnight_summary_impl(&state, None).unwrap();
    let by = |id: &str| {
        summary
            .rows
            .iter()
            .find(|row| row.session_id == id)
            .unwrap()
    };
    let said: Vec<(&str, &str, Option<&str>)> =
        ["s-crash", "s-error", "s-done", "s-yours", "s-unsaid"]
            .iter()
            .map(|id| {
                (
                    by(id).group.as_str(),
                    by(id).words.as_str(),
                    by(id).detail.as_deref(),
                )
            })
            .collect();
    assert_eq!(
        said,
        [
            ("crashed", "Crashed", Some("signal 11")),
            ("error", "Error", Some("exit code 1")),
            ("finished", "Finished", None),
            ("stopped", "Stopped by you", None),
            ("ended", "Ended", None),
        ]
    );
    let order: Vec<&str> = summary
        .rows
        .iter()
        .map(|row| row.session_id.as_str())
        .collect();
    assert_eq!(
        order,
        ["s-crash", "s-error", "s-done", "s-yours", "s-unsaid"],
        "crashes, then errors first"
    );
    // Its history says it too, in words.
    let events = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .lifecycle_events(&SessionId::new("s-error"))
        .unwrap();
    assert_eq!(
        events.last().map(|(what, _)| what.as_str()),
        Some("ended: error (exit code 1)")
    );
}
