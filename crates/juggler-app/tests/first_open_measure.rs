//! The whole first open after the 0.4.1 update, measured at the Dev
//! database's size (run by hand: `--ignored --nocapture`). Synthetic, no
//! real data: a v49 database of about 950 MB (1 230 000 usage rows with
//! their details and 1 135 000 holders, uuid keys; 338 000 quota readings
//! of one Codex window): the time to a ready state (the safety copy,
//! V50, V51 and the tidy) and the file's size before and after.
#![allow(
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::disallowed_methods,
    reason = "a measurement (QUALITY.md §2), on a raw connection to build the v49 database"
)]

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
#[ignore = "a measurement: builds a 950 MB database"]
fn measure_the_first_open_at_dev_scale() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    drop(juggler_app::state::AppState::init(data.clone()).unwrap());
    let db = data.join("juggler.sqlite");
    {
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
             ALTER TABLE quota_samples DROP COLUMN last_seen;
             PRAGMA user_version = 49;",
        )
        .unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        {
            let mut event = tx
                .prepare(
                    "INSERT INTO usage_events (account, session, ts, model, input, output, cache_r, cache_w,
                         source, confidence, msg_uuid)
                     VALUES (?1, ?2, ?3, 'claude-opus-5-5', 1200, 340, 50000, 900, 'transcript', 'measured', ?4)",
                )
                .unwrap();
            let mut detail = tx
                .prepare(
                    "INSERT INTO usage_detail (account, msg_uuid, prompt_tokens, kind, root_session, usd, price_version)
                     VALUES (?1, ?2, 51200, 'main', ?3, 0.0123, '2026-11-01')",
                )
                .unwrap();
            let mut holder = tx
                .prepare("INSERT OR IGNORE INTO usage_holders VALUES (?1, ?2, ?3, ?4)")
                .unwrap();
            for n in 0..1_230_000_u64 {
                let account = format!("claude-code-57233{:07}", n % 5);
                let session = uuid(1_000_000_000 + n / 450);
                let key = uuid(n);
                let ts = 1_790_000_000_000 + n as i64 * 4_000;
                event
                    .execute(rusqlite::params![account, session, ts, key])
                    .unwrap();
                detail
                    .execute(rusqlite::params![account, key, session])
                    .unwrap();
                if n < 1_135_000 {
                    holder
                        .execute(rusqlite::params![account, key, session, ts])
                        .unwrap();
                }
            }
            let mut sample = tx
                .prepare(
                    "INSERT INTO quota_samples (account, window, ts, used_pct, resets_at, confidence)
                     VALUES ('codex-572347151000', 'weekly', ?1, ?2, 1810000000000, 'measured')",
                )
                .unwrap();
            let mut source = tx
                .prepare("INSERT INTO quota_sample_sources VALUES ('codex-572347151000', 'weekly', ?1, 'rollout')")
                .unwrap();
            let (mut at, mut used) = (1_800_000_000_000_i64, 0.0_f64);
            for n in 1..=338_000_u32 {
                at += if n % 400 == 0 { 45 * 60_000 } else { 12_000 };
                if n % 3_350 == 0 {
                    used += 1.0;
                }
                sample.execute(rusqlite::params![at, used]).unwrap();
                source.execute(rusqlite::params![at]).unwrap();
            }
        }
        tx.commit().unwrap();
        conn.execute_batch("VACUUM").unwrap();
    }
    // `AIJ_KEEP_V49=<file>`: a copy of the v49 database, for a walk's
    // first open after the update (synthetic, no real data).
    if let Some(keep) = std::env::var_os("AIJ_KEEP_V49") {
        std::fs::copy(&db, keep).unwrap();
    }
    let size = |path: &std::path::Path| std::fs::metadata(path).unwrap().len() as f64 / 1e6;
    // Each step's start, then the whole open's end.
    struct Clock(std::time::Instant, Vec<(String, std::time::Duration)>);
    impl juggler_core::storage::OpenWatch for Clock {
        fn step(&mut self, step: juggler_core::storage::OpenStep) {
            self.1.push((format!("{step:?}"), self.0.elapsed()));
        }
        fn free_bytes(&self, folder: &std::path::Path) -> Option<u64> {
            juggler_platform::room::free_space(folder).ok()
        }
    }
    // Cold: a clone of the file is a new file to the cache (macOS caches
    // by file, and APFS shares only the blocks), so its first open reads
    // from the disk as after a restart. Warm: a clone read through first.
    let open = |label: &str, data: &std::path::Path| {
        let db = data.join("juggler.sqlite");
        let before = size(&db);
        let mut clock = Clock(std::time::Instant::now(), Vec::new());
        drop(juggler_core::storage::Storage::open_watched(&db, &mut clock).unwrap());
        let opened = clock.0.elapsed();
        println!(
            "{label} first open at {before:.0} MB: steps {:?}; storage ready at {opened:?}; {:.0} MB after",
            clock.1,
            size(&db)
        );
    };
    let cold = dir.path().join("cold");
    let warm = dir.path().join("warm");
    for copy in [&cold, &warm] {
        std::fs::create_dir_all(copy).unwrap();
        std::fs::copy(&db, copy.join("juggler.sqlite")).unwrap();
    }
    open("cold", &cold);
    std::io::copy(
        &mut std::fs::File::open(warm.join("juggler.sqlite")).unwrap(),
        &mut std::io::sink(),
    )
    .unwrap();
    open("warm", &warm);
    let rest = std::time::Instant::now();
    let (state, notes) =
        juggler_app::startup::init_state(cold.clone(), std::time::Instant::now(), false).unwrap();
    println!(
        "the rest of the app's state after the cold open: {:?}; notes {notes:?}",
        rest.elapsed()
    );
    drop(state);
}
