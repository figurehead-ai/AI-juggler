//! V52 (AJ-E3202 req. 3, 0.4.1): the name a folder was accepted under at
//! import. A v51 database, as 0.4.0 left it, opens, upgrades once, and
//! reads its Accounts back whole; the step is safe to run again.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_methods,
    reason = "a migration test: a raw connection builds the v51 database"
)]

use juggler_core::model::AccountId;
use juggler_core::storage::Storage;
use rusqlite::Connection;

/// A database as 0.4.0 left it: the first 51 migrations, and two folders.
fn v51_database(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations().iter().enumerate().take(51) {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
             VALUES ('work', 'claude-code', 'Work', NULL, '/lab/.claude-work', 1),
                    ('side', 'claude-code', 'Side', 'max-20x', '/lab/.claude-side', 2);",
    )
    .unwrap();
}

#[test]
fn a_v51_database_upgrades_to_v52_and_reads_its_accounts_back() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    v51_database(&db);

    let storage = Storage::open(&db).expect("0.4.0's database opens in 0.4.1");
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), 52);
    let accounts = storage.accounts().list().unwrap();
    let ids: Vec<&str> = accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["work", "side"]);
    assert_eq!(accounts[0].label, "Work");
    assert_eq!(accounts[1].plan.as_ref().unwrap().as_str(), "max-20x");
    assert!(
        storage.accounts().import_names().unwrap().is_empty(),
        "nothing is guessed for the folders already there"
    );
    storage
        .accounts()
        .set_import_name(&AccountId::new("work"), Some(" Work "))
        .unwrap();
    assert!(
        storage
            .accounts()
            .set_import_name(&AccountId::new("nope"), Some("x"))
            .is_err()
    );
    drop(storage);

    let storage = Storage::open(&db).unwrap();
    assert!(!storage.migrated_on_open(), "v52 runs once");
    let names = storage.accounts().import_names().unwrap();
    assert_eq!(names.get("work").map(String::as_str), Some("Work"));
    assert_eq!(names.len(), 1);
    storage
        .accounts()
        .set_import_name(&AccountId::new("work"), None)
        .unwrap();
    assert!(storage.accounts().import_names().unwrap().is_empty());
}

/// A database already at v52 put back a version (as the upgrade tests
/// build their older databases) runs v52 again without failing.
#[test]
fn v52_is_safe_to_run_again() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&db).unwrap();
    storage.accounts().list().unwrap();
    drop(storage);
    Connection::open(&db)
        .unwrap()
        .pragma_update(None, "user_version", 51)
        .unwrap();
    let storage = Storage::open(&db).expect("v52 again");
    assert_eq!(storage.user_version().unwrap(), 52);
}

/// rc27's v52 added `accounts.import_name` instead of the table, and the
/// Dev data it opened is at v52 that way: no migration runs on it again.
/// The current code opens it cleanly, makes the table, reads its Accounts
/// back and saves an accepted name; the old column is left alone.
#[test]
fn rc27s_v52_data_opens_and_saves_an_accepted_name() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    v51_database(&db);
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "ALTER TABLE accounts ADD COLUMN import_name TEXT;
             UPDATE accounts SET import_name = 'Work' WHERE id = 'work';
             PRAGMA user_version = 52;",
        )
        .unwrap();
    }

    let storage = Storage::open(&db).expect("rc27's v52 data opens");
    assert!(
        !storage.migrated_on_open(),
        "already v52: no migration runs"
    );
    let ids: Vec<String> = storage
        .accounts()
        .list()
        .unwrap()
        .into_iter()
        .map(|a| a.id.as_str().to_owned())
        .collect();
    assert_eq!(ids, ["work", "side"]);
    assert!(storage.accounts().import_names().unwrap().is_empty());
    storage
        .accounts()
        .set_import_name(&AccountId::new("side"), Some("Side"))
        .unwrap();
    drop(storage);

    let storage = Storage::open(&db).unwrap();
    let names = storage.accounts().import_names().unwrap();
    assert_eq!(names.get("side").map(String::as_str), Some("Side"));
    assert_eq!(names.len(), 1);
}
