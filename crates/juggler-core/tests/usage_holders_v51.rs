//! V51 measured at the Dev database's scale (run by hand: `--ignored
//! --nocapture`): 900 000 usage rows and 830 000 holders with uuid keys, as
//! a v50 database; the file's size, the upgrade's time, and what a VACUUM
//! would cost after it. Synthetic: no real data.
#![allow(
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::disallowed_methods,
    reason = "a measurement (QUALITY.md §2), on a raw connection to build the v50 database"
)]

use juggler_core::storage::Storage;
use rusqlite::Connection;

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

#[test]
#[ignore = "a measurement: builds a large database"]
fn measure_v51_at_dev_scale() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "DROP TRIGGER usage_events_unheld_insert; DROP TRIGGER usage_events_unheld_move;
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
                     VALUES (?1, ?2, ?3, 1, 'transcript', 'measured', ?4)",
                )
                .unwrap();
            let mut holder = tx
                .prepare("INSERT OR IGNORE INTO usage_holders VALUES (?1, ?2, ?3, ?4)")
                .unwrap();
            for n in 0..900_000_u64 {
                let account = format!("claude-code-57233{:07}", n % 5);
                let session = uuid(1_000_000_000 + n / 450);
                let key = uuid(n);
                let ts = 1_790_000_000_000 + i64::try_from(n).unwrap() * 4_000;
                event
                    .execute(rusqlite::params![account, session, ts, key])
                    .unwrap();
                if n < 830_000 {
                    holder
                        .execute(rusqlite::params![account, key, session, ts])
                        .unwrap();
                }
            }
        }
        tx.commit().unwrap();
        conn.execute_batch("VACUUM").unwrap();
    }
    let size = |db: &std::path::Path| std::fs::metadata(db).unwrap().len() as f64 / 1e6;
    let before = size(&db);
    let opened = std::time::Instant::now();
    let storage = Storage::open(&db).unwrap();
    let upgrade = opened.elapsed();
    assert!(storage.migrated_on_open());
    drop(storage);
    let unvacuumed = size(&db);
    let free: i64 = Connection::open(&db)
        .unwrap()
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .unwrap();
    let vacuumed = std::time::Instant::now();
    Connection::open(&db)
        .unwrap()
        .execute_batch("VACUUM")
        .unwrap();
    let vacuum = vacuumed.elapsed();
    let after = size(&db);
    let holds: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM usage_holds", [], |row| row.get(0))
        .unwrap();
    assert_eq!(holds, 830_000);
    println!(
        "V51 at Dev scale: {before:.1} MB -> upgrade {upgrade:?} -> {unvacuumed:.1} MB with {free} free pages -> VACUUM {vacuum:?} -> {after:.1} MB"
    );
}
