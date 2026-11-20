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
        juggler_app::features::sessions::list::instances_shared_snapshot(&state)
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
    let again = juggler_app::features::sessions::list::instances_shared_snapshot(&state)
        .unwrap()
        .into_iter()
        .find(|row| row.id == "s-done")
        .unwrap();
    assert_eq!(again.lifecycle.as_deref(), Some("reviewed"));
}

/// AJ-E1503: Undo on Mark reviewed puts the session back on the finished
/// list, and both steps stay in its history.
#[test]
fn undoing_reviewed_puts_it_back_and_keeps_both_steps() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(&state, work.path(), &[("s-done", SessionStatus::Finished)]);
    let lifecycle_of = |state: &Arc<AppState>| {
        juggler_app::features::sessions::list::instances_shared_snapshot(state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == "s-done")
            .unwrap()
            .lifecycle
    };
    let refused = lifecycle::instance_unmark_reviewed_impl(&state, "s-done").unwrap_err();
    assert!(
        refused.to_string().contains("isn't marked reviewed"),
        "{refused}"
    );

    lifecycle::instance_mark_reviewed_impl(&state, "s-done").unwrap();
    lifecycle::instance_unmark_reviewed_impl(&state, "s-done").unwrap();
    assert_eq!(lifecycle_of(&state).as_deref(), Some("unreviewed"));
    let history = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .lifecycle_events(&SessionId::new("s-done"))
        .unwrap();
    let events: Vec<&str> = history.iter().map(|(event, _)| event.as_str()).collect();
    assert_eq!(events, ["reviewed", "unreviewed"]);
    assert!(lifecycle::instance_unmark_reviewed_impl(&state, "s-nope").is_err());
}

/// AJ-E2910: Home's "Mark all N reviewed": every ended session at once,
/// none when one is live, and one Undo puts them all back.
#[test]
fn mark_all_reviewed_marks_every_ended_one_or_none_and_undoes_in_one() {
    let work = tempfile::tempdir().unwrap();
    let state = app_state(work.path());
    seed(
        &state,
        work.path(),
        &[
            ("s-a", SessionStatus::Finished),
            ("s-b", SessionStatus::Stopped),
            ("s-live", SessionStatus::WaitingUser),
        ],
    );
    let lifecycle_of = |id: &str| {
        juggler_app::features::sessions::list::instances_shared_snapshot(&state)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap()
            .lifecycle
    };
    let ids = |list: &[&str]| list.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    assert!(lifecycle::instances_mark_reviewed_impl(&state, &ids(&["s-a", "s-live"])).is_err());
    assert_ne!(
        lifecycle_of("s-a").as_deref(),
        Some("reviewed"),
        "none when one is live"
    );
    lifecycle::instances_mark_reviewed_impl(&state, &ids(&["s-a", "s-b"])).unwrap();
    assert_eq!(lifecycle_of("s-a").as_deref(), Some("reviewed"));
    assert_eq!(lifecycle_of("s-b").as_deref(), Some("reviewed"));
    lifecycle::instances_unmark_reviewed_impl(&state, &ids(&["s-a", "s-b"])).unwrap();
    assert_ne!(lifecycle_of("s-a").as_deref(), Some("reviewed"));
    assert_ne!(lifecycle_of("s-b").as_deref(), Some("reviewed"));
}

/// The 0.4.6 RC walk K2 (F-K2-7a): Home's "Pick up where you left off"
/// counted only the sessions the app holds (at most 200 ended in 7 days),
/// so "Show all 200" and "Mark all 200 reviewed" said "all" with more to
/// review. Its count, its pages and Mark all are now the backend's, over
/// every one in the window: 300 to review (a failed, a handed-over, an
/// older, a reviewed and an archived one aren't); Mark all marks all 300,
/// and its Undo puts back that batch only.
#[test]
fn every_session_to_review_is_counted_paged_and_marked() {
    use juggler_app::features::sessions::rows::{
        SessionsQuery, sessions_counts_impl, sessions_page_impl,
    };
    use juggler_app::paging::PageRequest;
    const DAY: i64 = 24 * 3_600_000;
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut sessions: Vec<(String, SessionStatus)> = (0..300)
        .map(|n| (format!("s-{n:03}"), SessionStatus::Finished))
        .collect();
    for (n, status) in [SessionStatus::Failed, SessionStatus::HandedOver]
        .into_iter()
        .enumerate()
    {
        sessions.push((format!("x-{n}"), status));
    }
    for name in ["old", "seen", "shelved"] {
        sessions.push((name.to_owned(), SessionStatus::Stopped));
    }
    let named: Vec<(&str, SessionStatus)> =
        sessions.iter().map(|(id, s)| (id.as_str(), *s)).collect();
    seed(&state, dir.path(), &named);
    let now = Timestamp::now().millis();
    {
        let storage = state.storage.lock().unwrap();
        for (n, (id, _)) in sessions.iter().enumerate() {
            let mut record = storage
                .sessions()
                .get(&SessionId::new(id.as_str()))
                .unwrap();
            // Ended over the last six days; "old" eight days ago.
            let last = if id == "old" {
                now - 8 * DAY
            } else {
                now - (n as i64 + 1) * 60_000 * 25
            };
            record.started_at = Timestamp::from_millis(last - 60_000);
            record.last_activity = Some(Timestamp::from_millis(last));
            storage.sessions().update(&record).unwrap();
        }
        storage
            .sessions()
            .record_lifecycle(&SessionId::new("seen"), "reviewed")
            .unwrap();
        storage
            .sessions()
            .record_lifecycle(&SessionId::new("shelved"), "archived")
            .unwrap();
    }
    let query = SessionsQuery {
        which: Some("to_review".into()),
        since_ms: Some(now - 7 * DAY),
        ..SessionsQuery::default()
    };
    // The app holds at most 200 of the week's ended ones: fewer than 300.
    let held = juggler_app::features::sessions::list::instances_shared_snapshot(&state)
        .unwrap()
        .into_iter()
        .filter(|row| row.status == "finished" && row.lifecycle.is_none())
        .count();
    assert!(held < 300, "the working set holds {held}");
    assert_eq!(sessions_counts_impl(&state, &query).unwrap().all, 300);
    // Every one, a page at a time, newest first, each AI Juggler's own.
    let mut rows = Vec::new();
    let mut cursor = None;
    loop {
        let page = sessions_page_impl(
            &state,
            &query,
            &PageRequest {
                cursor,
                limit: Some(50),
            },
        )
        .unwrap();
        assert_eq!(page.total, Some(300));
        rows.extend(page.items);
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let ids: Vec<String> = rows
        .iter()
        .map(|row| row.session.as_ref().unwrap().id.clone())
        .collect();
    let want: Vec<String> = (0..300).map(|n| format!("s-{n:03}")).collect();
    assert_eq!(ids, want, "newest first");
    // Mark all marks all 300 (not the 200 held).
    std::thread::sleep(std::time::Duration::from_millis(5));
    let done = lifecycle::sessions_mark_reviewed_impl(&state, &query).unwrap();
    assert_eq!(done.count, 300);
    assert_eq!(sessions_counts_impl(&state, &query).unwrap().all, 0);
    // One archived since stays archived; Undo puts back the rest of the batch only.
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .record_lifecycle(&SessionId::new("s-007"), "archived")
        .unwrap();
    assert_eq!(
        lifecycle::sessions_unmark_reviewed_impl(&state, done.at_ms).unwrap(),
        299
    );
    assert_eq!(sessions_counts_impl(&state, &query).unwrap().all, 299);
}
