//! E19 storage (AJ-E1901): the tasks migration, and claims under load.
//!
//! - Upgrading from schema 34 (0.2's messages) keeps a copy of the
//!   database as it was, adds v35's sightings (D-41) and then v36's tasks,
//!   and leaves every pool, rule and message as it was; upgrading from 35
//!   (a database the Processes view made first) adds the tasks.
//! - N connections claiming one task at once: exactly one wins (the claim
//!   is one statement; SQLite's write lock orders them).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::{Arc, Barrier};

use juggler_core::storage::{Claim, NewTask, Storage, TASK_USER, TaskQuery, pre_migration_copies};
use juggler_core::tasks::{Priority, TaskKind, TaskStatus};
use rusqlite::{Connection, OpenFlags};

fn database_at(path: &Path, upto: i64) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(upto).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO pools (id, name, kind, project, created_at, access_level, hop_limit)
             VALUES ('pool-team', 'Team', 'user', NULL, 5, 'full', 3);
         INSERT INTO agent_rules (pool_id, action, made_by, created_at)
             VALUES ('pool-team', 'assign_task', 'user', 7);
         INSERT INTO agent_threads (id, pool_id, subject, started_by, created_at, updated_at)
             VALUES ('thr-1', 'pool-team', 'Start', 'session:s-1', 8, 8);",
    )
    .unwrap();
}

/// v35's tables (D-41's sightings, `e24-processes`).
const V35_TABLES: &[&str] = &["process_sightings", "process_marks"];
/// v36's tables (E19's tasks).
const V36_TABLES: &[&str] = &[
    "tasks",
    "task_links",
    "task_sessions",
    "task_events",
    "task_needs",
];

fn tables(path: &Path) -> Vec<String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn new<'a>(id: &'a str, pool: &'a str) -> NewTask<'a> {
    NewTask {
        id,
        pool_id: pool,
        kind: TaskKind::Task,
        parent_id: None,
        title: "Build the API",
        body: "",
        acceptance: "",
        priority: Priority::High,
        status: TaskStatus::Ready,
        assignee: None,
        created_by: TASK_USER,
        created_by_name: "You",
        external_url: None,
        blocked_by: &[],
    }
}

#[test]
fn upgrading_from_messages_keeps_a_copy_and_everything_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 34);
    let before = tables(&db);
    assert!(!before.contains(&"tasks".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    // v37 (AJ-E2501, notices), v38 (the pool canvas) and v39 (launch
    // settings' removed_at, D-60) came after E19's tables.
    assert_eq!(Storage::schema_version(), 39);
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, 34);
    assert_eq!(tables(&copies[0].1), before);
    // Both migrations ran: v35's sightings (D-41), then v36's tasks.
    let after = tables(&db);
    for table in [V35_TABLES, V36_TABLES].concat() {
        assert!(after.contains(&(*table).to_owned()), "{table} missing");
    }
    assert_eq!(storage.sightings().count().unwrap(), 0);

    let access = storage.agent_access().access("pool-team").unwrap();
    assert_eq!(access.level.unwrap().as_str(), "full");
    assert_eq!(
        storage
            .agent_access()
            .rules(Some("pool-team"))
            .unwrap()
            .len(),
        1
    );
    assert!(storage.messages().thread("thr-1").unwrap().is_some());

    let repo = storage.tasks();
    repo.create(&new("aj-7f3c", "pool-team")).unwrap();
    let listed = repo
        .list(
            &TaskQuery {
                pool_id: Some("pool-team"),
                ..TaskQuery::default()
            },
            10,
        )
        .unwrap();
    assert_eq!(listed[0].id, "aj-7f3c");
    assert_eq!(listed[0].status, TaskStatus::Ready);
}

#[test]
fn upgrading_from_sightings_adds_the_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 35);
    let before = tables(&db);
    for table in V35_TABLES {
        assert!(
            before.contains(&(*table).to_owned()),
            "{table} missing at v35"
        );
    }
    assert!(!before.contains(&"tasks".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    assert_eq!(pre_migration_copies(&db)[0].0, 35);
    let after = tables(&db);
    for table in [V35_TABLES, V36_TABLES].concat() {
        assert!(after.contains(&(*table).to_owned()), "{table} missing");
    }
    storage
        .tasks()
        .create(&new("aj-7f3c", "pool-team"))
        .unwrap();
    assert!(storage.tasks().get("aj-7f3c").unwrap().is_some());
}

#[test]
fn n_connections_claim_one_task_and_exactly_one_wins() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let pool = {
        let storage = Storage::open(&db).unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        storage.tasks().create(&new("aj-0001", &pool)).unwrap();
        pool
    };
    const N: usize = 8;
    let start = Arc::new(Barrier::new(N));
    // Each its own connection, opened before any of them starts.
    let storages: Vec<Storage> = (0..N).map(|_| Storage::open(&db).unwrap()).collect();
    let handles: Vec<_> = storages
        .into_iter()
        .enumerate()
        .map(|(n, storage)| {
            let (pool, start) = (pool.clone(), Arc::clone(&start));
            std::thread::spawn(move || {
                let me = format!("session:s-{n}");
                start.wait();
                storage
                    .tasks()
                    .claim(
                        &pool,
                        "aj-0001",
                        std::slice::from_ref(&me),
                        (&me, &me),
                        None,
                    )
                    .unwrap()
            })
        })
        .collect();
    let outcomes: Vec<Claim> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let won = outcomes
        .iter()
        .filter(|o| matches!(o, Claim::Won(_)))
        .count();
    assert_eq!(won, 1, "{outcomes:?}");
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o, Claim::Won(_) | Claim::Taken { .. })),
        "{outcomes:?}"
    );
    let storage = Storage::open(&db).unwrap();
    let claims = storage
        .tasks()
        .events("aj-0001", None, 50)
        .unwrap()
        .into_iter()
        .filter(|e| e.verb == "claimed")
        .count();
    assert_eq!(claims, 1, "one claim in the history");
}
