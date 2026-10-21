//! AJ-E1521 Req.1 — every connection on the app database waits for
//! another connection's lock instead of failing at once. The app opens
//! `juggler.sqlite` four times (storage, process ledger, cleanup audit,
//! cleanup settings); without a busy timeout a write that meets another
//! write fails with "database is locked".
//!
//! Each test holds the write lock on a second, plain connection, lets it
//! go after a short while on another thread, and expects the repository
//! to have waited for it. rusqlite already waits 5 s on ordinary
//! statements; the switch to WAL is the one SQLite answers "busy" at
//! once (it skips the busy handler there), so opening a database another
//! connection is writing failed before AJ-E1521.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::time::{Duration, Instant};

use juggler_core::storage::{
    BUSY_TIMEOUT, CleanupAuditRepo, CleanupSettingsRepo, CleanupTierSettings, ProcessLedgerRepo,
    Storage,
};
use rusqlite::Connection;

/// How long the other connection keeps the lock — well inside the
/// timeout, well above scheduling noise.
const HELD: Duration = Duration::from_millis(400);

/// Takes the database's write lock on its own connection and releases it
/// after [`HELD`]. Returns once the lock is held.
fn hold_write_lock(path: &Path) -> std::thread::JoinHandle<()> {
    let path = path.to_path_buf();
    let (locked, wait) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked.send(()).unwrap();
        std::thread::sleep(HELD);
        conn.execute_batch("COMMIT").unwrap();
    });
    wait.recv().unwrap();
    handle
}

#[test]
fn the_timeout_is_long_enough_for_a_long_write() {
    assert!(BUSY_TIMEOUT >= Duration::from_secs(2));
}

#[test]
fn a_storage_write_waits_for_another_connections_write() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&db).unwrap();
    let holder = hold_write_lock(&db);
    let started = Instant::now();
    storage
        .app_settings()
        .set("aj-e1521", "waited")
        .expect("the write waits for the lock instead of failing");
    assert!(started.elapsed() >= HELD / 2, "the write really waited");
    holder.join().unwrap();
    assert_eq!(
        storage.app_settings().get("aj-e1521").unwrap().as_deref(),
        Some("waited")
    );
}

#[test]
fn opening_the_database_waits_while_another_connection_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Connection::open(&db).unwrap());
    let holder = hold_write_lock(&db);
    // A fresh database: switching it to WAL needs the write lock.
    Storage::open(&db).expect("opening waits for the lock");
    holder.join().unwrap();

    // The other three switch their connection to WAL the same way.
    for (name, open) in [
        (
            "ledger",
            (|p: &Path| ProcessLedgerRepo::open(p).map(drop)) as fn(&Path) -> _,
        ),
        ("audit", |p: &Path| CleanupAuditRepo::open(p).map(drop)),
        ("settings", |p: &Path| {
            CleanupSettingsRepo::open(p).map(drop)
        }),
    ] {
        let fresh = dir.path().join(format!("{name}.sqlite"));
        drop(Connection::open(&fresh).unwrap());
        let holder = hold_write_lock(&fresh);
        open(&fresh).unwrap_or_else(|error| panic!("{name} waits for the lock: {error}"));
        holder.join().unwrap();
    }
}

#[test]
fn the_ledger_audit_and_cleanup_settings_wait_too() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());

    // Each opens its table on the shared file — a write.
    let holder = hold_write_lock(&db);
    ProcessLedgerRepo::open(&db).expect("the process ledger waits for the lock");
    holder.join().unwrap();

    let holder = hold_write_lock(&db);
    CleanupAuditRepo::open(&db).expect("the cleanup audit waits for the lock");
    holder.join().unwrap();

    let settings = CleanupSettingsRepo::open(&db).unwrap();
    let holder = hold_write_lock(&db);
    settings
        .set(
            "acct",
            &CleanupTierSettings {
                tier1_opt_in: true,
                tier2_opt_in: false,
            },
        )
        .expect("a cleanup-settings write waits for the lock");
    holder.join().unwrap();
}
