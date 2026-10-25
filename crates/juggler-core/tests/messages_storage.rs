//! E18 storage (AJ-E1801): the messages migration.
//!
//! - Upgrading from schema 33 (0.2's pools) keeps a copy of the database
//!   as it was, adds the messages tables, and leaves every pool, member,
//!   level and rule as it was.
//! - Upgrading straight from 0.1.0 (schema 32) runs both 0.2 migrations at
//!   once and keeps 0.1.0's copy, so "Go back to an earlier version" can
//!   still return to 0.1.0.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_core::storage::{RecipientState, Storage, pre_migration_copies};
use rusqlite::{Connection, OpenFlags};

/// A database exactly as schema `upto` left it, with a pool, a member
/// and a rule when pools exist.
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
        "INSERT INTO projects (id, root, name, created_at) VALUES ('p-shop', '/tmp/shop', 'shop', 1);",
    )
    .unwrap();
    if upto >= 33 {
        conn.execute_batch(
            "INSERT INTO pools (id, name, kind, project, created_at, access_level, hop_limit)
                 VALUES ('pool-team', 'Team', 'user', NULL, 5, 'full', 3);
             INSERT INTO pool_members (pool_id, member_ref, role, joined_at)
                 VALUES ('pool-team', 'session:s-1', 'lead', 6);
             INSERT INTO agent_rules (pool_id, action, made_by, created_at)
                 VALUES ('pool-team', 'assign_task', 'user', 7);",
        )
        .unwrap();
    }
}

fn tables(path: &Path) -> Vec<String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn upgrading_from_pools_keeps_a_copy_and_every_pool_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 33);
    let before = tables(&db);
    assert!(!before.contains(&"agent_messages".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), 34);
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, 33);
    assert_eq!(tables(&copies[0].1), before);

    // The pools are as they were.
    let access = storage.agent_access().access("pool-team").unwrap();
    assert_eq!(access.level.unwrap().as_str(), "full");
    assert_eq!(access.hop_limit, 3);
    assert_eq!(
        storage
            .agent_access()
            .rules(Some("pool-team"))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(storage.pools().members().unwrap().len(), 1);

    // And the messages tables work.
    let repo = storage.messages();
    repo.create_thread("thr-1", "pool-team", "Start", "session:s-1")
        .unwrap();
    let recipients = vec![(
        "session:s-2".to_owned(),
        "Builder".to_owned(),
        RecipientState::Unread,
    )];
    repo.insert_message(&juggler_core::storage::NewMessage {
        id: "msg-1",
        thread_id: "thr-1",
        pool_id: "pool-team",
        sender_ref: "session:s-1",
        sender_kind: "agent",
        sender_name: "Planner",
        kind: "handoff",
        subject: "Start on the API",
        body: "The schema is in the shared doc.",
        body_sha256: "h",
        hop: 1,
        reply_to: None,
        to_all: false,
        redacted: 0,
        recipients: &recipients,
    })
    .unwrap();
    assert_eq!(repo.search("pool-team", "schema", 5).unwrap().len(), 1);
}

#[test]
fn upgrading_from_0_1_runs_both_and_keeps_0_1s_copy() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, 32);
    let storage = Storage::open(&db).unwrap();
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, 32, "0.1.0 opens its copy");
    assert!(tables(&db).contains(&"agent_messages".to_owned()));
    assert!(tables(&db).contains(&"pools".to_owned()));
}
