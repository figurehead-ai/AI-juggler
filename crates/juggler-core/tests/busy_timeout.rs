//! AJ-E1521 Req.1: the core storage waits for another connection's write
//! instead of failing at once with "database is locked". The app opens
//! several connections to one file (the store, the cleanup audit and
//! settings, the process ledger); a long write on one must not fail a
//! write on another.
//!
//! These pass on the code before AJ-E1521 too: rusqlite sets a 5 s busy
//! timeout on every open (`inner_connection.rs`), so the audit's "no busy
//! timeout" was a gap in what the code said, not in what it did. The
//! storage now sets `BUSY_TIMEOUT` itself, and these tests pin the
//! behaviour against a change of that default.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::time::{Duration, Instant};

use juggler_core::storage::{BUSY_TIMEOUT, CleanupAuditRepo, CleanupSettingsRepo, Storage};

/// Holds a write lock on `path` for `hold`, from another thread.
fn hold_write_lock(path: &std::path::Path, hold: Duration) -> std::thread::JoinHandle<()> {
    let path = path.to_path_buf();
    let (locked, wait) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked.send(()).unwrap();
        std::thread::sleep(hold);
        conn.execute_batch("COMMIT").unwrap();
    });
    wait.recv().unwrap();
    handle
}

#[test]
fn a_write_waits_out_another_connections_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    let writer = hold_write_lock(&path, Duration::from_millis(400));
    let started = Instant::now();
    storage
        .app_settings()
        .set("stuck_after_minutes", "10")
        .expect("waits for the lock instead of failing");
    assert!(
        started.elapsed() >= Duration::from_millis(300),
        "it waited: {:?}",
        started.elapsed()
    );
    writer.join().unwrap();
    assert_eq!(
        storage
            .app_settings()
            .get("stuck_after_minutes")
            .unwrap()
            .as_deref(),
        Some("10")
    );
}

#[test]
fn the_other_connections_to_the_file_wait_too() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let _storage = Storage::open(&path).unwrap();
    let audit = CleanupAuditRepo::open(&path).unwrap();
    let settings = CleanupSettingsRepo::open(&path).unwrap();
    let writer = hold_write_lock(&path, Duration::from_millis(300));
    audit.prune(0).expect("the audit waits");
    let current = settings.get("acc-work").unwrap();
    settings
        .set("acc-work", &current)
        .expect("the cleanup settings wait");
    writer.join().unwrap();
    assert!(
        BUSY_TIMEOUT >= Duration::from_secs(2),
        "at least the stats store's wait"
    );
}
