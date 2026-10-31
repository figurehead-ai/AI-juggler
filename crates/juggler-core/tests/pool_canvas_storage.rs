//! v38 (0.3, redesign §5): the pool canvas's storage.
//!
//! - Upgrading from v37 keeps a copy of the database as it was and adds
//!   the canvas's table, leaving every pool as it was.
//! - A member's place is one row, moved in place; a place off the canvas
//!   is refused; Tidy up forgets a pool's rows and no other pool's.
//! - Hand-offs count only the tasks the receiver still holds; "moving"
//!   counts tasks in progress per pool.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_core::storage::{POSITION_LIMIT, Storage, pre_migration_copies};
use rusqlite::Connection;

fn database_at(path: &Path, upto: i64) {
    let conn = Connection::open(path).unwrap();
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
        "INSERT INTO pools (id, name, kind, project, created_at) VALUES ('pool-team', 'Team', 'user', NULL, 5);
         INSERT INTO pools (id, name, kind, project, created_at) VALUES ('pool-docs', 'Docs', 'user', NULL, 6);",
    )
    .unwrap();
}

#[test]
fn upgrading_from_notices_keeps_a_copy_and_adds_the_canvas() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 37);
    let storage = Storage::open(&db).unwrap();
    // To the newest schema (v38 added the canvas; later ones come too).
    assert_eq!(
        storage.user_version().unwrap(),
        i64::try_from(Storage::migrations().len()).unwrap()
    );
    assert_eq!(pre_migration_copies(&db)[0].0, 37);
    assert_eq!(storage.pools().get("pool-team").unwrap().name, "Team");
    assert!(
        storage
            .pool_canvas()
            .positions("pool-team")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_place_is_one_row_moved_in_place_and_tidy_up_forgets_one_pool() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 38);
    let storage = Storage::open(&db).unwrap();
    let canvas = storage.pool_canvas();
    canvas.place("pool-team", "work:w1", 10.0, 20.0).unwrap();
    canvas.place("pool-team", "work:w1", 30.0, -5.0).unwrap();
    canvas.place("pool-team", "session:s2", 0.0, 0.0).unwrap();
    canvas.place("pool-docs", "session:s9", 1.0, 1.0).unwrap();
    let team = canvas.positions("pool-team").unwrap();
    assert_eq!(team.len(), 2);
    let moved = team.iter().find(|row| row.member_ref == "work:w1").unwrap();
    assert_eq!((moved.x, moved.y), (30.0, -5.0));

    for (x, y) in [
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
        (POSITION_LIMIT * 2.0, 0.0),
    ] {
        assert!(
            canvas.place("pool-team", "work:w1", x, y).is_err(),
            "off the canvas"
        );
    }
    assert_eq!(canvas.forget("pool-team").unwrap(), 2);
    assert!(canvas.positions("pool-team").unwrap().is_empty());
    assert_eq!(
        canvas.positions("pool-docs").unwrap().len(),
        1,
        "another pool's stay"
    );
}

#[test]
fn handoffs_count_what_the_receiver_still_holds_and_moving_counts_in_progress() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 38);
    let storage = Storage::open(&db).unwrap();
    let insert_task = |id: &str, status: &str, holder: Option<(&str, &str)>| {
        storage
            .raw_conn()
            .execute(
                "INSERT INTO tasks (id, pool_id, kind, title, status, assignee_ref, assignee_name,
                                    created_by, created_by_name, created_at, updated_at)
                 VALUES (?1, 'pool-team', 'task', 'T', ?2, ?3, ?4, 'session:lead', 'Lead', 1, 1)",
                rusqlite::params![id, status, holder.map(|h| h.0), holder.map(|h| h.1)],
            )
            .unwrap();
    };
    let event = |task: &str, at: i64, detail: &str| {
        storage
            .raw_conn()
            .execute(
                "INSERT INTO task_events (at, pool_id, task_id, actor_ref, actor_name, verb, detail)
                 VALUES (?1, 'pool-team', ?2, 'session:lead', 'Lead', 'assigned', ?3)",
                rusqlite::params![at, task, detail],
            )
            .unwrap();
    };
    insert_task("aj-1", "in_progress", Some(("session:b", "Builder")));
    insert_task("aj-2", "ready", Some(("session:c", "Copy")));
    insert_task("aj-3", "in_progress", None);
    event("aj-1", 100, "Builder");
    // Given to Builder, now held by Copy: no longer Builder's hand-off.
    event("aj-2", 110, "Builder");
    event("aj-2", 120, "Copy");
    let canvas = storage.pool_canvas();
    let mut handoffs = canvas.task_handoffs("pool-team", 0, 50).unwrap();
    handoffs.sort_by(|a, b| a.to_ref.cmp(&b.to_ref));
    let pairs: Vec<(&str, &str, u32)> = handoffs
        .iter()
        .map(|h| (h.from_ref.as_str(), h.to_ref.as_str(), h.count))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("session:lead", "session:b", 1),
            ("session:lead", "session:c", 1)
        ]
    );
    assert!(
        canvas
            .task_handoffs("pool-team", 200, 50)
            .unwrap()
            .is_empty(),
        "since bounds it"
    );
    let moving = canvas
        .moving(&["pool-team".to_owned(), "pool-docs".to_owned()])
        .unwrap();
    assert_eq!(moving, vec![("pool-team".to_owned(), 2)]);
    let stream = canvas.stream("pool-team", 10).unwrap();
    assert_eq!(stream.len(), 3);
    assert_eq!(stream[0].at, 120, "newest first");
    assert_eq!(stream[0].source, "task");
}
