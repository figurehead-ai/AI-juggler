//! AJ-E2026: commits that survive a power cut on macOS. A plain `fsync`
//! there leaves the drive's cache unflushed; only `F_FULLFSYNC` flushes
//! it. Every connection AI Juggler opens sets `checkpoint_fullfsync`, and
//! `Storage::make_durable` forces a checkpoint after the writes that must
//! not be lost. Its cost is measured and printed (the manifest records it).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test: a failed step fails the test"
)]

use juggler_core::storage::{Storage, open_hardened};

fn pragma(conn: &rusqlite::Connection, name: &str) -> i64 {
    conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .unwrap()
}

#[test]
fn every_connection_checkpoints_with_full_fsync() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    assert_eq!(pragma(storage.raw_conn(), "checkpoint_fullfsync"), 1);
    // Any other connection on the file (the ledger, the audits, a reader).
    let other = open_hardened(&path).unwrap();
    assert_eq!(pragma(&other, "checkpoint_fullfsync"), 1);
    // WAL with NORMAL: commits don't sync; checkpoints do, fully.
    assert_eq!(pragma(storage.raw_conn(), "synchronous"), 1);
}

#[test]
fn make_durable_checkpoints_and_its_cost_is_small() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    storage
        .raw_conn()
        .execute_batch("CREATE TABLE IF NOT EXISTS durable_probe (v TEXT); INSERT INTO durable_probe VALUES ('x');")
        .unwrap();
    let began = std::time::Instant::now();
    storage.make_durable().unwrap();
    let ms = began.elapsed().as_secs_f64() * 1000.0;
    println!("make_durable: {ms:.1} ms");
    assert!(ms < 2_000.0, "a durable checkpoint took {ms} ms");
}
