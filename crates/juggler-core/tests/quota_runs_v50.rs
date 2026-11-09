//! V50: a database stored before it has its runs of one reading collapsed
//! once on open, and every reader gives them back as each run's first and
//! last points (the runway's pace and the charts see what they saw).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    clippy::disallowed_methods,
    reason = "a test (QUALITY.md §2), and its raw connection rolls the database back a version as the other migration tests do"
)]

use juggler_core::model::Confidence;
use juggler_core::quota::{QuotaSample, WindowKind};
use juggler_core::storage::Storage;
use rusqlite::Connection;

#[test]
fn an_older_database_collapses_its_runs_and_reads_back_their_ends() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    {
        // As a v49 database: no last_seen, one row per reading.
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "ALTER TABLE quota_samples DROP COLUMN last_seen; PRAGMA user_version = 49;",
        )
        .unwrap();
        let put = |ts: i64, pct: f64, source: &str| {
            conn.execute(
                "INSERT INTO quota_samples (account, window, ts, used_pct, resets_at, confidence)
                 VALUES ('codex', 'weekly', ?1, ?2, 9000, 'measured')",
                rusqlite::params![ts, pct],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO quota_sample_sources (account, window, ts, source)
                 VALUES ('codex', 'weekly', ?1, ?2)",
                rusqlite::params![ts, source],
            )
            .unwrap();
        };
        for ts in (100..=1_000).step_by(100) {
            put(ts, 40.0, "rollout");
        }
        put(1_100, 41.0, "rollout");
        put(1_200, 41.0, "rollout");
        put(1_300, 41.0, "usage_cache");
    }
    let storage = Storage::open(&db).unwrap();
    let rows: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM quota_samples", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 3, "13 readings in three runs: three rows");
    let sources: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM quota_sample_sources", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(sources, 3, "each run's source stays with it");
    let read: Vec<(i64, f64)> = storage
        .usage()
        .samples_for_account("codex", WindowKind::Weekly { anchor_ms: None })
        .unwrap()
        .iter()
        .map(|sample: &QuotaSample| {
            assert_eq!(sample.confidence, Confidence::Measured);
            (sample.ts, sample.used_pct)
        })
        .collect();
    assert_eq!(
        read,
        [
            (100, 40.0),
            (1_000, 40.0),
            (1_100, 41.0),
            (1_200, 41.0),
            (1_300, 41.0)
        ],
        "each run's first and last; the source change keeps its own row"
    );
}

/// The rc6 walk's case, measured (run by hand: `--ignored --nocapture`):
/// 338 000 readings of one Codex weekly window, a reading every 12 s with
/// idle spells and 101 distinct values, as a v49 database; the file's
/// size before and after V50, and the runway and current reading the
/// same from the raw rows and from the stored runs.
#[test]
#[ignore = "a measurement: builds a large database"]
fn measure_a_codex_sized_window_before_and_after() {
    use juggler_core::quota::current_reading;
    use juggler_core::quota_runway::runway;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    drop(Storage::open(&db).unwrap());
    let reset = 1_800_000_000_000 + 120 * 86_400_000;
    let mut raw = Vec::new();
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "ALTER TABLE quota_samples DROP COLUMN last_seen; PRAGMA user_version = 49;",
        )
        .unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        let (mut at, mut used, mut n) = (1_800_000_000_000_i64, 0.0_f64, 0_u32);
        while raw.len() < 338_000 {
            n += 1;
            at += if n % 400 == 0 { 45 * 60_000 } else { 12_000 };
            if n % 3_350 == 0 && used < 100.0 {
                used += 1.0;
            }
            tx.execute(
                "INSERT INTO quota_samples (account, window, ts, used_pct, resets_at, confidence)
                 VALUES ('codex', 'weekly', ?1, ?2, ?3, 'measured')",
                rusqlite::params![at, used, reset],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO quota_sample_sources (account, window, ts, source) VALUES ('codex', 'weekly', ?1, 'rollout')",
                rusqlite::params![at],
            )
            .unwrap();
            raw.push(QuotaSample {
                ts: at,
                used_pct: used,
                resets_at: Some(reset),
                confidence: Confidence::Measured,
                held_until: None,
            });
        }
        tx.commit().unwrap();
        conn.execute_batch("VACUUM").unwrap();
    }
    let before = std::fs::metadata(&db).unwrap().len();
    let opened = std::time::Instant::now();
    let storage = Storage::open(&db).unwrap();
    let took = opened.elapsed();
    let rows: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM quota_samples", [], |row| row.get(0))
        .unwrap();
    drop(storage);
    Connection::open(&db)
        .unwrap()
        .execute_batch("VACUUM")
        .unwrap();
    let after = std::fs::metadata(&db).unwrap().len();
    let storage = Storage::open(&db).unwrap();
    let stored = storage
        .usage()
        .samples_since("codex", WindowKind::Weekly { anchor_ms: None }, i64::MIN)
        .unwrap();
    let now = raw.last().unwrap().ts + 60_000;
    let week = 7 * 86_400_000;
    let (a, b) = (
        runway(&raw, &[], week, now).unwrap(),
        runway(&stored, &[], week, now).unwrap(),
    );
    assert_eq!(
        (a.used_pct, a.pct_per_hour, a.exhausts_at_ms, a.starts_ms),
        (b.used_pct, b.pct_per_hour, b.exhausts_at_ms, b.starts_ms)
    );
    let (x, y) = (
        current_reading(&raw).unwrap(),
        current_reading(&stored).unwrap(),
    );
    assert_eq!((x.ts, x.used_pct), (y.ts, y.used_pct));
    println!(
        "V50 on a Codex-sized window: {} readings -> {rows} rows; file {:.1} MB -> {:.1} MB; upgrade took {took:?}; runway and current reading the same",
        raw.len(),
        before as f64 / 1e6,
        after as f64 / 1e6
    );
}
