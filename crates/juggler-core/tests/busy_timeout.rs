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

/// #126's CI: a usage write read before it wrote (its deferred transaction
/// began as a read), so when another connection (the process ledger) wrote
/// and committed meanwhile, SQLite answered "database is locked" at once
/// instead of waiting. Usage writes take the write lock first.
#[test]
fn a_usage_write_waits_for_another_connection_that_changes_the_database() {
    use juggler_core::model::{Confidence, TokenCounts};
    use juggler_core::quota::{UsageEvent, UsageSource};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&db).unwrap();
    let event = |key: &str| UsageEvent {
        account: "acc".into(),
        session: Some("s".into()),
        ts: 1,
        model: None,
        tokens: TokenCounts {
            output: 1,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: Some(key.into()),
    };
    storage.usage().store_events(&[event("a")]).unwrap();
    for round in 0..3 {
        let path = db.clone();
        let (locked, wait) = std::sync::mpsc::channel();
        let other = std::thread::spawn(move || {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('other', ?1)",
                [round.to_string()],
            )
            .unwrap();
            locked.send(()).unwrap();
            std::thread::sleep(HELD);
            conn.execute_batch("COMMIT").unwrap();
        });
        wait.recv().unwrap();
        storage
            .usage()
            .store_events(&[event("a"), event(&format!("b{round}"))])
            .expect("the usage write waits for the other write");
        storage
            .usage()
            .replace_session_events(
                &[("acc".into(), UsageSource::Transcript, "s".into())],
                &[event("a")],
            )
            .expect("so does a re-read");
        other.join().unwrap();
    }
}

/// One store write, run against a competing writer.
type Write<'a> = Box<dyn Fn() -> Result<(), juggler_core::error::CoreError> + 'a>;

/// The other stores' write transactions, audited after #126's CI (only the
/// usage store read before it wrote): each begins with a write, so SQLite
/// takes the lock with a fresh snapshot and waits its turn. Each is run
/// while another connection holds the write lock with a change pending.
#[test]
fn every_store_write_waits_for_another_connection_that_changes_the_database() {
    use juggler_core::model::SessionId;
    use juggler_core::storage::SyncEntry;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&db).unwrap();
    let ledger = ProcessLedgerRepo::open(&db).unwrap();
    let session = SessionId::new("s");
    let entry = SyncEntry {
        seq: 1,
        kind: "instance".into(),
        record_id: "r".into(),
        payload: vec![1],
        wall_ms: 1,
    };
    let helper = || juggler_core::storage::LedgerRow {
        pid: Some(4242),
        pgid: Some(4242),
        started_at: Some(1),
        exe_path: "/bin/sh".into(),
        cmdline_hash: "h".into(),
        cmdline_trunc: "sh".into(),
        boot_id: "boot".into(),
        session_id: "s".into(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: None,
        project_id: None,
        machine_id: "m".into(),
        root_pid: None,
        parent_pid: None,
        depth: 1,
        role: juggler_core::storage::ROLE_HELPER.into(),
        state: String::new(),
        listens_on: None,
        cwd: "/".into(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: 1,
        last_seen: 1,
    };
    let writes: Vec<(&str, Write<'_>)> = vec![
        (
            "transcript index append",
            Box::new(|| {
                storage
                    .transcript_index()
                    .append(&session, "/t.jsonl", &[(0, 4, "text".into())], 4)
            }),
        ),
        (
            "transcript index forget",
            Box::new(|| storage.transcript_index().forget(&session)),
        ),
        (
            "fleet outbox",
            Box::new(|| {
                storage
                    .fleet_sync()
                    .outbox_append("instance", "r", b"x", 1)
                    .map(drop)
            }),
        ),
        (
            "fleet replica epoch",
            Box::new(|| storage.fleet_sync().replica_set_epoch("peer", "e1")),
        ),
        (
            "fleet replica apply",
            Box::new(|| {
                storage
                    .fleet_sync()
                    .replica_apply("peer", &entry, 1)
                    .map(drop)
            }),
        ),
        (
            "fleet replica forget",
            Box::new(|| storage.fleet_sync().replica_forget("peer")),
        ),
        (
            "ledger ports",
            Box::new(|| ledger.apply_ports(&[(1, "boot", "[]")])),
        ),
        (
            "ledger helpers",
            Box::new(|| ledger.upsert_helpers("s", &[helper()])),
        ),
    ];
    for (round, (name, write)) in writes.iter().enumerate() {
        let path = db.clone();
        let (locked, wait) = std::sync::mpsc::channel();
        let other = std::thread::spawn(move || {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('other', ?1)",
                [round.to_string()],
            )
            .unwrap();
            locked.send(()).unwrap();
            std::thread::sleep(HELD);
            conn.execute_batch("COMMIT").unwrap();
        });
        wait.recv().unwrap();
        write().unwrap_or_else(|error| panic!("{name} waits for the other write: {error}"));
        other.join().unwrap();
    }
}

/// The audit (#130 review L10): an upgrade's migrations wait for another
/// connection that is writing (opening a database a version behind).
#[test]
fn a_migration_waits_for_another_connection_that_changes_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    // One version behind: the last migration (v34, AJ-E2402 sightings) is
    // undone by hand and runs again on open.
    let version: i64 = Connection::open(&db)
        .unwrap()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        version, 34,
        "a new migration landed: undo it here instead (this test's undo is v34's)"
    );
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch("DROP TABLE process_marks; DROP TABLE process_sightings;")
            .unwrap();
        conn.pragma_update(None, "user_version", version - 1)
            .unwrap();
    }
    let path = db.clone();
    let (locked, wait) = std::sync::mpsc::channel();
    let other = std::thread::spawn(move || {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('other', '1')",
            [],
        )
        .unwrap();
        locked.send(()).unwrap();
        std::thread::sleep(HELD);
        conn.execute_batch("COMMIT").unwrap();
    });
    wait.recv().unwrap();
    let storage = Storage::open(&db).expect("the migration waits its turn");
    assert!(storage.migrated_on_open());
    other.join().unwrap();
}
