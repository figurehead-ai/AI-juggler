//! 0.1.5 review: History, a project's Sessions and Usage's cost table
//! read every row at once (hundreds, here 10 000), and stayed blank for
//! seconds. On 10 000 sessions over 20 projects and 5 accounts, and
//! 30 000 usage events:
//! - a page is at most its limit, pages walk the whole list once in
//!   order, and the last one says so;
//! - a session added between two reads never shifts or repeats a row;
//! - scope, kind and search narrow it server-side;
//! - the first page arrives well within the 1 s budget (each read prints
//!   a `[perf]` line);
//! - the pools' answer holds counts and the working set's places, never
//!   a row per session; a pool's members come a page at a time.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::commands;
use juggler_app::features::pools;
use juggler_app::features::sessions::rows::{
    SessionsQuery, sessions_counts_impl, sessions_page_impl,
};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, Confidence, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe,
    MachineId, ProfileId, ProjectId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
    TokenCounts,
};
use juggler_core::quota::{UsageEvent, UsageSource};

const SESSIONS: usize = 10_000;
const DAY: i64 = 86_400_000;
/// The fixture's first day: 60 days ago, at UTC midnight (every session
/// ends in the past, about 42 days of them).
fn start() -> i64 {
    let now = Timestamp::now().millis();
    (now - 60 * DAY).div_euclid(DAY) * DAY
}

/// The fixture's second day in words (UTC).
fn second_day() -> String {
    juggler_core::cost_ledger::day_of(start() + DAY)
}

struct Fixture {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    projects: Vec<String>,
}

fn record(n: usize, project: &str, at: i64, status: SessionStatus) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(format!("s-{n:05}")),
        conversation: None,
        profile: ProfileId::new(format!("rec-{}", n % 5)),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: Some(juggler_core::model::ProjectId::new(project)),
        cwd: PathBuf::from(format!("/nowhere/project-{:02}", n % 20)),
        status,
        started_at: Timestamp::from_millis(at),
        last_activity: Some(Timestamp::from_millis(at + 60_000)),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let started = Instant::now();
    let projects: Vec<String>;
    {
        let storage = state.storage.lock().unwrap();
        projects = (0..20)
            .map(|i| {
                storage
                    .projects()
                    .touch(
                        std::path::Path::new(&format!("/nowhere/project-{i:02}")),
                        None,
                    )
                    .unwrap()
                    .id
                    .to_string()
            })
            .collect();
        for i in 0..5 {
            let id = format!("acc-{i}");
            storage
                .accounts()
                .insert(&Account {
                    id: AccountId::new(id.as_str()),
                    harness: HarnessId::new("claude-code"),
                    label: ["Alpha", "Bravo", "Charlie", "Delta", "Echo"][i].to_owned(),
                    plan: None,
                    state_dir: dir.path().join(&id),
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
                    id: ProfileId::new(format!("rec-{i}")),
                    name: "Main".into(),
                    account: AccountId::new(id.as_str()),
                    settings_overlay: SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
        }
        for n in 0..SESSIONS {
            // One in a hundred still open; the rest ended, an hour apart.
            let status = if n % 100 == 0 {
                SessionStatus::Running
            } else {
                SessionStatus::Finished
            };
            let at = start() + i64::try_from(n).unwrap() * 3_600_000 / 10;
            storage
                .sessions()
                .insert(&record(n, &projects[n % 20], at, status))
                .unwrap();
        }
        for n in (1..SESSIONS).step_by(50) {
            storage
                .sessions()
                .record_lifecycle(&SessionId::new(format!("s-{n:05}")), "archived")
                .unwrap();
        }
        let events: Vec<UsageEvent> = (0..30_000)
            .map(|n: usize| UsageEvent {
                account: format!("acc-{}", n % 5),
                session: Some(format!("s-{:05}", n % SESSIONS)),
                ts: start() + i64::try_from(n).unwrap() * (DAY / 1000),
                model: Some(["claude-sonnet-4-5", "claude-opus-4-1", "gpt-5"][n % 3].to_owned()),
                tokens: TokenCounts {
                    input: 100,
                    output: 10,
                    ..TokenCounts::default()
                },
                credits: None,
                usd_est: None,
                source: UsageSource::Transcript,
                confidence: Confidence::Measured,
                msg_uuid: Some(format!("m-{n}")),
            })
            .collect();
        storage.usage().insert_events(&events).unwrap();
    }
    eprintln!(
        "[perf] fixture: 10 000 sessions, 30 000 usage events in {:?}",
        started.elapsed()
    );
    Fixture {
        _dir: dir,
        state,
        projects,
    }
}

fn timed<T>(what: &str, run: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let out = run();
    let took = started.elapsed();
    eprintln!("[perf] {what}: {took:?}");
    (out, took)
}

#[test]
fn sessions_come_a_page_at_a_time_in_a_stable_order() {
    let fx = fixture();
    let state = &fx.state;
    let all = SessionsQuery::default();

    // Before: every row at once (what History and the project tab read).
    let (rows, _) = timed("before: every session at once (sessions_list)", || {
        commands::session_infos_snapshot(&state.storage.lock().unwrap()).unwrap()
    });
    assert_eq!(rows.len(), SESSIONS);

    // Now: the first page.
    let (first, took) = timed("now: History's first page (50 rows)", || {
        sessions_page_impl(state, &all, &PageRequest::default()).unwrap()
    });
    assert_eq!(first.items.len(), 50);
    assert_eq!(first.total, Some(SESSIONS as u64));
    assert!(took < Duration::from_secs(1), "first page took {took:?}");
    // Open first, then newest.
    let open = first
        .items
        .iter()
        .take_while(|row| row.session.as_ref().is_some_and(|s| s.status == "running"))
        .count();
    assert_eq!(
        open, 50,
        "100 open sessions: the whole first page is open ones"
    );

    // Walk it all, 200 at a time: every row once, in order.
    let mut seen = HashSet::new();
    let mut cursor: Option<String> = None;
    let mut last: Option<(bool, i64)> = None;
    let mut pages = 0;
    loop {
        let page = sessions_page_impl(
            state,
            &all,
            &PageRequest {
                cursor: cursor.clone(),
                limit: Some(200),
            },
        )
        .unwrap();
        assert!(page.items.len() <= 200);
        pages += 1;
        for row in &page.items {
            assert!(seen.insert(row.key.clone()), "a row came twice");
            let session = row.session.as_ref().unwrap();
            let here = (
                session.status != "running",
                -session.last_activity_ms.unwrap(),
            );
            if let Some(before) = last {
                assert!(before <= here, "rows out of order");
            }
            last = Some(here);
        }
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(seen.len(), SESSIONS);
    assert_eq!(pages, SESSIONS / 200);

    // A limit over the ceiling is the ceiling.
    let big = sessions_page_impl(state, &all, &PageRequest::first(5000)).unwrap();
    assert_eq!(big.items.len(), juggler_app::paging::MAX_LIMIT);
}

#[test]
fn a_session_added_between_two_reads_never_shifts_or_repeats_a_row() {
    let fx = fixture();
    let state = &fx.state;
    let ended = SessionsQuery {
        which: Some("ended".to_owned()),
        ..SessionsQuery::default()
    };
    let first = sessions_page_impl(state, &ended, &PageRequest::first(50)).unwrap();
    let last_shown = first.items.last().unwrap().session.clone().unwrap();
    {
        let storage = state.storage.lock().unwrap();
        // Newer than everything shown: it belongs above, not on page 2.
        let newest = record(
            90_001,
            &fx.projects[0],
            start() + 59 * DAY,
            SessionStatus::Finished,
        );
        storage.sessions().insert(&newest).unwrap();
        // Just older than the last row shown: the next page's first.
        let at = last_shown.last_activity_ms.unwrap() - 60_000 - 1;
        let between = record(90_002, &fx.projects[0], at, SessionStatus::Finished);
        storage.sessions().insert(&between).unwrap();
    }
    let second = sessions_page_impl(
        state,
        &ended,
        &PageRequest::after(first.next_cursor.as_deref().unwrap(), 50),
    )
    .unwrap();
    let shown: HashSet<&str> = first.items.iter().map(|row| row.key.as_str()).collect();
    assert!(
        second
            .items
            .iter()
            .all(|row| !shown.contains(row.key.as_str()))
    );
    assert_eq!(second.items[0].key, "s/s-90002");
    assert!(second.items.iter().all(|row| row.key != "s/s-90001"));
    assert_eq!(second.total, first.total.map(|total| total + 2));
}

#[test]
fn scope_kind_and_search_narrow_the_list_server_side() {
    let fx = fixture();
    let state = &fx.state;
    let project = SessionsQuery {
        project_id: Some(fx.projects[3].clone()),
        ..SessionsQuery::default()
    };
    let (page, _) = timed("now: a project's Sessions tab, first page", || {
        sessions_page_impl(state, &project, &PageRequest::default()).unwrap()
    });
    assert_eq!(page.total, Some((SESSIONS / 20) as u64));
    assert!(page.items.iter().all(|row| {
        row.session.as_ref().unwrap().project_id.as_deref() == Some(fx.projects[3].as_str())
    }));

    let account = SessionsQuery {
        account_id: Some("acc-0".to_owned()),
        which: Some("open".to_owned()),
        ..SessionsQuery::default()
    };
    let open = sessions_page_impl(state, &account, &PageRequest::default()).unwrap();
    assert_eq!(
        open.total,
        Some(100),
        "every 100th is open, all on acc-0 (n % 5 == 0)"
    );

    let counts = sessions_counts_impl(state, &SessionsQuery::default()).unwrap();
    assert_eq!(counts.all, SESSIONS as u64);
    assert_eq!(counts.open, 100);
    assert_eq!(counts.archived, 200);
    assert_eq!(counts.ended, (SESSIONS - 300) as u64);

    // A date in words (UTC here): the sessions started that day.
    let day = SessionsQuery {
        text: Some(second_day()),
        utc_offset_min: Some(0),
        ..SessionsQuery::default()
    };
    let (matched, took) = timed("now: History search, first page", || {
        sessions_page_impl(state, &day, &PageRequest::default()).unwrap()
    });
    assert!(took < Duration::from_secs(1), "search took {took:?}");
    assert!(matched.total.unwrap() >= 240, "too few matches");
    let words = SessionsQuery {
        text: Some(format!("echo {}", second_day())),
        utc_offset_min: Some(0),
        ..SessionsQuery::default()
    };
    let narrowed = sessions_page_impl(state, &words, &PageRequest::default()).unwrap();
    assert!(narrowed.total.unwrap() < matched.total.unwrap());
    assert!(
        narrowed
            .items
            .iter()
            .all(|row| row.session.as_ref().unwrap().account_label == "Echo")
    );
}

#[test]
fn the_cost_table_comes_a_page_at_a_time_and_adds_up() {
    let fx = fixture();
    let state = &fx.state;
    let (whole, _) = timed("before: the whole cost table (cost_rollups)", || {
        commands::cost_rollups_impl(state, None)
    });
    let (first, took) = timed("now: the cost table's first page", || {
        commands::cost_rows_page_impl(state, None, &PageRequest::default()).unwrap()
    });
    // A row is one session's day, its models together (0.1.8).
    let session_days: HashSet<(String, Option<String>)> = whole
        .rows
        .iter()
        .map(|row| (row.key.clone(), row.day.clone()))
        .collect();
    assert_eq!(first.items.len(), 50);
    assert_eq!(first.total, Some(session_days.len() as u64));
    assert!(took < Duration::from_secs(1), "first page took {took:?}");

    let mut seen = HashSet::new();
    let mut input = 0_u64;
    let mut cursor: Option<String> = None;
    let mut last_day: Option<String> = None;
    loop {
        let page = commands::cost_rows_page_impl(
            state,
            None,
            &PageRequest {
                cursor: cursor.clone(),
                limit: Some(200),
            },
        )
        .unwrap();
        for row in &page.items {
            assert!(seen.insert((row.session.clone(), Some(row.day.clone()))));
            if let Some(before) = &last_day {
                assert!(before >= &row.day, "newest day first");
            }
            last_day = Some(row.day.clone());
            input += row.tokens.input;
        }
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(seen, session_days);
    assert_eq!(input, 30_000 * 100, "the pages add up to the whole");

    let one = commands::cost_rows_page_impl(state, Some("acc-1"), &PageRequest::default()).unwrap();
    assert!(one.total.unwrap() < session_days.len() as u64);
}

#[test]
fn the_app_and_the_rail_hold_the_working_set_never_the_history() {
    let fx = fixture();
    let state = &fx.state;
    let (rows, took) = timed("now: the app's list of sessions (the working set)", || {
        commands::instances_shared_snapshot(state).unwrap()
    });
    let open = rows.iter().filter(|row| row.status == "running").count();
    assert_eq!(open, 100, "every open session is in it");
    // Every session of the last day is in it (the rail's "ended today"
    // stays true); of the week before, the newest 200; nothing older.
    let now = Timestamp::now().millis();
    let last_day = (0..SESSIONS)
        .filter(|n| n % 100 != 0)
        .filter(|n| start() + i64::try_from(*n).unwrap() * 360_000 + 60_000 >= now - DAY)
        .count();
    let ended = rows.len() - open;
    assert!(
        ended >= last_day,
        "fewer ended rows than ended in the last day"
    );
    assert!(
        ended <= last_day + commands::RECENT_ENDED,
        "more ended rows than the working set holds"
    );
    assert!(rows.iter().all(|row| row.last_activity_ms.unwrap()
        >= now - commands::RECENT_ENDED_MS
        || row.status == "running"));
    assert!(took < Duration::from_secs(1), "{took:?}");
    // A session past it is read by its id (an old one opened from History).
    let old = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .get_many(&[SessionId::new("s-00001"), SessionId::new("gone")])
        .unwrap();
    assert_eq!(old.len(), 1);
}

#[test]
fn the_pools_answer_holds_counts_never_the_history_and_members_come_paged() {
    let fx = fixture();
    let state = &fx.state;
    let (info, took) = timed("now: the pools (pools_list)", || {
        pools::pools_list_impl(state).unwrap()
    });
    assert!(took < Duration::from_secs(1), "pools took {took:?}");
    assert!(info.placements.is_empty(), "every place is the default");
    let counted: u64 = info.pools.iter().map(|pool| u64::from(pool.members)).sum();
    assert_eq!(counted, (SESSIONS - 200) as u64, "archived ones aside");

    // Ten open sessions (every open one is project 0's) in a pool of yours.
    let made = pools::pool_create_impl(state, "Team", None).unwrap();
    let team = made
        .pools
        .iter()
        .find(|pool| pool.name == "Team")
        .unwrap()
        .id
        .clone();
    for n in (0..1000).step_by(100) {
        pools::pool_move_impl(state, &format!("s-{n:05}"), &team).unwrap();
    }
    let info = pools::pools_list_impl(state).unwrap();
    assert_eq!(
        info.placements.len(),
        10,
        "only the places that aren't the default"
    );
    let bytes = serde_json::to_string(&info).unwrap().len();
    eprintln!("[perf] the pools' answer: {bytes} bytes");
    assert!(bytes < 32_000, "the answer grew with the history");

    // Project 0's default pool: 490 sessions, a page at a time.
    let pool =
        juggler_core::storage::default_pool_id(Some(&ProjectId::new(fx.projects[0].as_str())));
    let count = info
        .pools
        .iter()
        .find(|row| row.id == pool)
        .unwrap()
        .members;
    assert_eq!(count, 490);
    let (first, took) = timed("now: a big pool's first page of members", || {
        pools::members::members_page(state, &pool, &PageRequest::default()).unwrap()
    });
    assert!(took < Duration::from_secs(1), "first page took {took:?}");
    assert_eq!(first.items.len(), 50);
    assert_eq!(first.total, Some(u64::from(count)));
    let mut seen = HashSet::new();
    let mut cursor: Option<String> = None;
    let mut last: Option<i64> = None;
    loop {
        let page = pools::members::members_page(
            state,
            &pool,
            &PageRequest {
                cursor: cursor.clone(),
                limit: Some(200),
            },
        )
        .unwrap();
        for row in &page.items {
            assert!(seen.insert(row.session.id.clone()), "a member came twice");
            let at = row.session.last_activity_ms.unwrap();
            if let Some(before) = last {
                assert!(before >= at, "members out of order");
            }
            last = Some(at);
        }
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(seen.len(), 490);
}
