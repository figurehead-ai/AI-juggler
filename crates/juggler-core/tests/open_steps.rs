//! The first open after an update (0.4.1): its steps told as they start,
//! the tidy that gives the upgrade's freed room back (skipped, never
//! failing, when the disk is short, and tried again later), and a kill
//! during any step leaving data the next open recovers whole.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    clippy::disallowed_methods,
    reason = "a test (QUALITY.md §2); its raw connection builds the older database"
)]

use std::path::{Path, PathBuf};

use juggler_core::storage::{OpenStep, OpenWatch, Storage};
use rusqlite::Connection;

const HOLDERS: u64 = 120_000;

fn uuid(n: u64) -> String {
    let x = n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    format!(
        "{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}",
        x >> 32,
        (x >> 16) & 0xffff,
        x & 0xfff,
        (x >> 8) & 0xfff,
        n
    )
}

/// A database as v50 left it, with `HOLDERS` usage rows and holders.
fn v50_database(dir: &Path) -> PathBuf {
    let db = dir.join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(
        "PRAGMA auto_vacuum = NONE; VACUUM;
         DROP TRIGGER usage_events_unheld_insert; DROP TRIGGER usage_events_unheld_move;
         DROP TABLE usage_holds; DROP TABLE usage_keys;
         CREATE TABLE usage_holders (
             account TEXT NOT NULL, msg_uuid TEXT NOT NULL, session TEXT NOT NULL,
             ts INTEGER NOT NULL, PRIMARY KEY (account, msg_uuid, session)) WITHOUT ROWID;
         CREATE INDEX idx_usage_holders_session ON usage_holders(account, session, ts);
         CREATE INDEX idx_usage_holders_key ON usage_holders(msg_uuid);
         PRAGMA user_version = 50;",
    )
    .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    {
        let mut event = tx
            .prepare(
                "INSERT INTO usage_events (account, session, ts, output, source, confidence, msg_uuid)
                 VALUES ('acc', ?1, ?2, 1, 'transcript', 'measured', ?3)",
            )
            .unwrap();
        let mut holder = tx
            .prepare("INSERT INTO usage_holders VALUES ('acc', ?1, ?2, ?3)")
            .unwrap();
        for n in 0..HOLDERS {
            let (session, key, ts) = (
                uuid(1_000_000 + n / 300),
                uuid(n),
                1_790_000_000_000 + n as i64,
            );
            event.execute(rusqlite::params![session, ts, key]).unwrap();
            holder.execute(rusqlite::params![key, session, ts]).unwrap();
        }
    }
    tx.commit().unwrap();
    drop(conn);
    db
}

#[derive(Default)]
struct Seen {
    steps: Vec<OpenStep>,
    notes: Vec<String>,
    free: Option<u64>,
}

impl OpenWatch for Seen {
    fn step(&mut self, step: OpenStep) {
        self.steps.push(step);
    }
    fn free_bytes(&self, _: &Path) -> Option<u64> {
        self.free
    }
    fn note(&mut self, words: &str) {
        self.notes.push(words.to_owned());
    }
}

fn pragma(db: &Path, name: &str) -> i64 {
    Connection::open(db)
        .unwrap()
        .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .unwrap()
}

/// Every usage row and holder there, and SQLite's own check.
fn intact(db: &Path) {
    let conn = Connection::open(db).unwrap();
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
    assert_eq!(count("SELECT count(*) FROM usage_events"), HOLDERS as i64);
    assert_eq!(
        count(
            "SELECT count(*) FROM usage_holds h JOIN usage_keys m ON m.id = h.msg
             JOIN usage_keys s ON s.id = h.session"
        ),
        HOLDERS as i64
    );
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}

#[test]
fn the_first_open_after_an_update_tells_each_step_and_tidies() {
    let dir = tempfile::tempdir().unwrap();
    let db = v50_database(dir.path());
    let before = std::fs::metadata(&db).unwrap().len();
    let mut seen = Seen {
        free: Some(u64::MAX),
        ..Seen::default()
    };
    let storage = Storage::open_watched(&db, &mut seen).unwrap();
    // The rc8 walk: the running app's WAL stayed as big as the file after
    // the upgrade and the VACUUM. Measured while it's still open (closing
    // the last connection empties it anyway).
    let wal = std::fs::metadata(db.with_file_name("juggler.sqlite-wal")).map_or(0, |m| m.len());
    assert_eq!(wal, 0, "the open leaves its WAL empty");
    drop(storage);
    assert_eq!(
        seen.steps,
        [OpenStep::SafetyCopy, OpenStep::Updating, OpenStep::Tidying]
    );
    assert_eq!(
        pragma(&db, "auto_vacuum"),
        2,
        "from now on it gives room back a little at a time"
    );
    assert!(
        std::fs::metadata(&db).unwrap().len() < before,
        "the freed room is given back"
    );
    intact(&db);
    // The next open has nothing to do.
    let mut again = Seen::default();
    drop(Storage::open_watched(&db, &mut again).unwrap());
    assert!(again.steps.is_empty(), "{:?}", again.steps);
}

#[test]
fn a_short_disk_skips_the_tidy_says_why_and_a_later_open_does_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = v50_database(dir.path());
    let mut short = Seen {
        free: Some(1_000),
        ..Seen::default()
    };
    drop(Storage::open_watched(&db, &mut short).unwrap());
    assert_eq!(
        short.steps,
        [OpenStep::SafetyCopy, OpenStep::Updating],
        "upgraded, not tidied"
    );
    assert!(
        short
            .notes
            .iter()
            .any(|note| note.contains("not enough free disk space")),
        "{:?}",
        short.notes
    );
    assert_eq!(pragma(&db, "auto_vacuum"), 0);
    intact(&db);
    let mut roomy = Seen {
        free: Some(u64::MAX),
        ..Seen::default()
    };
    drop(Storage::open_watched(&db, &mut roomy).unwrap());
    assert_eq!(roomy.steps, [OpenStep::Tidying]);
    assert_eq!(pragma(&db, "auto_vacuum"), 2);
    intact(&db);
}

#[test]
fn a_new_database_gives_room_back_a_little_at_a_time_from_the_start() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    assert_eq!(pragma(&db, "auto_vacuum"), 2);
}

/// The child of [`a_kill_during_any_step_leaves_data_the_next_open_recovers`]:
/// opens the database named in its environment, writing a mark as the
/// named step starts.
#[test]
#[ignore = "run by its parent test, which kills it mid-step"]
fn killed_child() {
    let (Ok(db), Ok(at), Ok(mark)) = (
        std::env::var("AIJ_KILL_DB"),
        std::env::var("AIJ_KILL_AT"),
        std::env::var("AIJ_KILL_MARK"),
    ) else {
        return;
    };
    struct Mark(String, String);
    impl OpenWatch for Mark {
        fn step(&mut self, step: OpenStep) {
            if format!("{step:?}") == self.0 {
                std::fs::write(&self.1, "here").unwrap();
            }
        }
        fn free_bytes(&self, _: &Path) -> Option<u64> {
            Some(u64::MAX)
        }
    }
    let _ = Storage::open_watched(Path::new(&db), &mut Mark(at, mark));
    // Held open, so a kill always lands while the open is unfinished or
    // just done: the parent decides.
    std::thread::sleep(std::time::Duration::from_secs(30));
}

#[test]
fn a_kill_during_any_step_leaves_data_the_next_open_recovers() {
    for step in ["SafetyCopy", "Updating", "Tidying"] {
        for delay_ms in [0_u64, 40, 150] {
            let dir = tempfile::tempdir().unwrap();
            let db = v50_database(dir.path());
            let mark = dir.path().join("mark");
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "killed_child",
                    "--ignored",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("AIJ_KILL_DB", &db)
                .env("AIJ_KILL_AT", step)
                .env("AIJ_KILL_MARK", &mark)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while !mark.exists() && std::time::Instant::now() < deadline {
                // poll: the child's mark, bounded by the deadline.
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(mark.exists(), "{step}: the child reached the step");
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            child.kill().unwrap();
            child.wait().unwrap();
            // The next open finishes the job, or finds it done; nothing lost.
            let mut next = Seen {
                free: Some(u64::MAX),
                ..Seen::default()
            };
            drop(
                Storage::open_watched(&db, &mut next)
                    .unwrap_or_else(|e| panic!("{step} +{delay_ms} ms: {e}")),
            );
            intact(&db);
            assert_eq!(pragma(&db, "user_version"), Storage::schema_version());
        }
    }
}
