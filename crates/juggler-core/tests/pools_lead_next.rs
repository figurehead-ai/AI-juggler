//! v53 (0.4.6, AJ-E3011): a pool may wait for an App's next session to
//! lead. D-92 holds: one session leads.
//!
//! - A v52 database upgrades once; the step is safe to run again.
//! - The claim takes the Lead only while the pool waits for that App and
//!   has no Lead; the wait ends with it.
//! - Two sessions claiming at once, on two connections: only one leads.

#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a storage test: raw connections build the older database"
)]

use std::path::Path;
use std::sync::{Arc, Barrier};

use juggler_core::model::SessionId;
use juggler_core::storage::{MemberRef, PoolRole, Scope, Storage};
use rusqlite::Connection;

/// A database at v52, as 0.4.5 left it.
fn v52_database(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations().iter().enumerate().take(52) {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
}

fn session(id: &str) -> MemberRef {
    MemberRef::Session(SessionId::new(id))
}

/// A user pool with the App `cli:work` as a Member, waiting for its next
/// session to lead.
fn waiting_pool(storage: &Storage) -> String {
    let pool = storage
        .pools()
        .create_scoped("Team", &Scope::Every)
        .unwrap();
    storage
        .pools()
        .join(
            &pool.id,
            &MemberRef::App("cli:work".into()),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    storage
        .pools()
        .set_lead_next(&pool.id, Some("cli:work"))
        .unwrap();
    pool.id
}

fn leads(storage: &Storage, pool: &str) -> Vec<String> {
    storage
        .pools()
        .members()
        .unwrap()
        .into_iter()
        .filter(|row| row.pool_id == pool && row.role == PoolRole::Lead)
        .map(|row| row.member.to_wire())
        .collect()
}

#[test]
fn a_v52_database_upgrades_and_the_step_runs_again_safely() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    v52_database(&db);
    let storage = Storage::open(&db).expect("0.4.5's database opens in 0.4.6");
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    let pool = waiting_pool(&storage);
    assert_eq!(
        storage.pools().lead_next(&pool).unwrap().as_deref(),
        Some("cli:work")
    );
    drop(storage);
    // Run from v52 again: the table is made only if it isn't there.
    Connection::open(&db)
        .unwrap()
        .pragma_update(None, "user_version", 52)
        .unwrap();
    let storage = Storage::open(&db).expect("v53 again");
    assert_eq!(
        storage.pools().lead_next(&pool).unwrap().as_deref(),
        Some("cli:work")
    );
}

#[test]
fn the_next_session_of_the_app_leads_and_the_wait_ends() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    let pool = waiting_pool(&storage);
    assert_eq!(
        storage.pools().waiting_for_lead("cli:work").unwrap(),
        std::slice::from_ref(&pool)
    );

    // Another App's session doesn't take it.
    assert!(
        !storage
            .pools()
            .claim_lead_next(&pool, "cli:side", &session("s-side"))
            .unwrap()
    );
    assert!(leads(&storage, &pool).is_empty());

    assert!(
        storage
            .pools()
            .claim_lead_next(&pool, "cli:work", &session("s-1"))
            .unwrap()
    );
    assert_eq!(leads(&storage, &pool), ["session:s-1"]);
    assert_eq!(
        storage.pools().lead_next(&pool).unwrap(),
        None,
        "the wait ends"
    );
    assert!(
        storage
            .pools()
            .waiting_for_lead("cli:work")
            .unwrap()
            .is_empty()
    );

    // The next one is a Member through its App, as before.
    assert!(
        !storage
            .pools()
            .claim_lead_next(&pool, "cli:work", &session("s-2"))
            .unwrap()
    );
    assert_eq!(leads(&storage, &pool), ["session:s-1"]);
}

#[test]
fn a_pool_that_has_a_lead_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    let pool = waiting_pool(&storage);
    storage
        .pools()
        .join(&pool, &session("s-lead"), &[], PoolRole::Lead, None)
        .unwrap();
    assert!(
        !storage
            .pools()
            .claim_lead_next(&pool, "cli:work", &session("s-1"))
            .unwrap()
    );
    assert_eq!(leads(&storage, &pool), ["session:s-lead"]);
}

#[test]
fn a_session_named_already_takes_the_lead_on_its_own_row() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    let pool = waiting_pool(&storage);
    storage
        .pools()
        .join(&pool, &session("s-1"), &[], PoolRole::Reviewer, None)
        .unwrap();
    assert!(
        storage
            .pools()
            .claim_lead_next(&pool, "cli:work", &session("s-1"))
            .unwrap()
    );
    let rows: Vec<_> = storage
        .pools()
        .members()
        .unwrap()
        .into_iter()
        .filter(|row| row.pool_id == pool && row.member == session("s-1"))
        .collect();
    assert_eq!(rows.len(), 1, "one active row per pool and member");
    assert_eq!(rows[0].role, PoolRole::Lead);
}

#[test]
fn a_deleted_pool_waits_for_no_one() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    let pool = waiting_pool(&storage);
    storage.pools().delete(&pool).unwrap();
    assert!(
        storage
            .pools()
            .waiting_for_lead("cli:work")
            .unwrap()
            .is_empty()
    );
    assert!(
        !storage
            .pools()
            .claim_lead_next(&pool, "cli:work", &session("s-1"))
            .unwrap()
    );
}

#[test]
fn two_sessions_starting_at_once_only_one_leads() {
    for _ in 0..20 {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("juggler.sqlite");
        let pool = waiting_pool(&Storage::open(&db).unwrap());
        let start = Arc::new(Barrier::new(2));
        // Each on its own connection, as two writers would be (opened
        // first, one after the other: opening is not what's raced).
        let claims: Vec<_> = ["s-a", "s-b"]
            .into_iter()
            .map(|id| {
                let storage = Storage::open(&db).unwrap();
                let (pool, start) = (pool.clone(), Arc::clone(&start));
                std::thread::spawn(move || {
                    start.wait();
                    storage
                        .pools()
                        .claim_lead_next(&pool, "cli:work", &session(id))
                        .unwrap()
                })
            })
            .collect();
        let won: Vec<bool> = claims
            .into_iter()
            .map(|claim| claim.join().unwrap())
            .collect();
        assert_eq!(won.iter().filter(|won| **won).count(), 1, "{won:?}");
        let storage = Storage::open(&db).unwrap();
        assert_eq!(leads(&storage, &pool).len(), 1);
        assert_eq!(storage.pools().lead_next(&pool).unwrap(), None);
    }
}
