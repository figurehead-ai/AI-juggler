//! The settled hours, kept in memory: a profile's hourly sums older than
//! six hours rarely change, so they're read from SQLite once and kept,
//! and each summary reads only the recent hours live. The store's
//! settled-history stamp (`UsageHistoryRepo::settled_stamp`) moves on any
//! write to an older message, by any build, and empties the memory.
//!
//! A year of a busy profile is read in about 90 ms once; after that a
//! range costs what its recent hours do (a millisecond or two).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use juggler_core::storage::{HOUR_MS, HourRow, SETTLED_MS, Storage};

use crate::error::AppResult;

/// How far the kept hours may fall behind before they're read again.
const REFRESH_MS: i64 = 6 * HOUR_MS;

/// One profile's settled hours.
struct Kept {
    /// Hours before this are kept.
    cutoff: i64,
    /// Oldest first.
    rows: Vec<HourRow>,
    /// The oldest hours were left out (past the row cap).
    cut: bool,
}

#[derive(Default)]
struct Store {
    stamp: Option<(i64, i64)>,
    profiles: HashMap<String, Kept>,
}

/// One store per data folder (tests run several apps in one process).
fn stores() -> &'static Mutex<HashMap<PathBuf, Store>> {
    static STORES: OnceLock<Mutex<HashMap<PathBuf, Store>>> = OnceLock::new();
    STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The settled cutoff for `now`: an hour boundary at least six hours ago.
#[must_use]
pub fn cutoff_for(now: i64) -> i64 {
    let at = now - SETTLED_MS;
    at - at.rem_euclid(HOUR_MS)
}

/// One profile's hourly rows in `[start, end)`: the settled ones from
/// memory, the recent ones live. Says whether rows were left out.
///
/// # Errors
///
/// Storage errors.
pub fn hourly(
    storage: &Storage,
    data_dir: &Path,
    profile: &str,
    start: i64,
    end: i64,
    cap: usize,
    now: i64,
) -> AppResult<(Vec<HourRow>, bool)> {
    let history = storage.usage_history();
    let stamp = history.settled_stamp()?;
    let target = cutoff_for(now);
    let mut all = stores()
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    let store = all.entry(data_dir.to_path_buf()).or_default();
    if store.stamp != Some(stamp) {
        store.profiles.clear();
        store.stamp = Some(stamp);
    }
    let stale = store
        .profiles
        .get(profile)
        .is_none_or(|kept| kept.cutoff + REFRESH_MS <= target);
    if stale {
        let (mut rows, cut) = history.hourly(profile, i64::MIN, target, cap)?;
        rows.reverse();
        store.profiles.insert(
            profile.to_owned(),
            Kept {
                cutoff: target,
                rows,
                cut,
            },
        );
    }
    let kept = &store.profiles[profile];
    let cutoff = kept.cutoff;
    let mut out: Vec<HourRow> = if start < cutoff {
        let from = kept.rows.partition_point(|row| row.hour_ms < start);
        let to = kept
            .rows
            .partition_point(|row| row.hour_ms < end.min(cutoff));
        kept.rows[from..to].to_vec()
    } else {
        Vec::new()
    };
    let mut cut = kept.cut && start < cutoff;
    drop(all);
    if end > cutoff {
        let (live, live_cut) = history.hourly(profile, start.max(cutoff), end, cap)?;
        cut |= live_cut;
        out.extend(live);
    }
    Ok((out, cut))
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_core::model::{Confidence, TokenCounts};
    use juggler_core::quota::{UsageEvent, UsageSource};

    fn event(ts: i64, uuid: &str, input: u64) -> UsageEvent {
        UsageEvent {
            account: "work".into(),
            session: Some("s-1".into()),
            ts,
            model: Some("opus".into()),
            tokens: TokenCounts {
                input,
                ..TokenCounts::default()
            },
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: Confidence::Measured,
            msg_uuid: Some(uuid.into()),
        }
    }

    #[test]
    fn settled_hours_come_from_memory_until_an_old_message_changes() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("j.db")).unwrap();
        let now = juggler_core::model::Timestamp::now().millis();
        let old = now - 3 * 86_400_000;
        storage
            .usage()
            .insert_events(&[event(old, "a", 100), event(now - 60_000, "b", 7)])
            .unwrap();
        let read = |storage: &Storage| {
            hourly(
                storage,
                dir.path(),
                "work",
                now - 10 * 86_400_000,
                now + 1,
                1_000,
                now,
            )
            .unwrap()
            .0
            .iter()
            .map(|row| row.tokens.input)
            .sum::<u64>()
        };
        assert_eq!(read(&storage), 107);
        // A recent message is read live: no stamp change needed.
        storage
            .usage()
            .insert_events(&[event(now - 30_000, "c", 3)])
            .unwrap();
        assert_eq!(read(&storage), 110);
        // An old one moves the stamp, so the kept hours are read again.
        let before = storage.usage_history().settled_stamp().unwrap();
        storage
            .usage()
            .insert_events(&[event(old + 1_000, "d", 1_000)])
            .unwrap();
        assert_ne!(storage.usage_history().settled_stamp().unwrap(), before);
        assert_eq!(read(&storage), 1_110);
    }

    /// The page's budget on a million messages over a year on seven
    /// profiles: `cargo test -p juggler-app --lib settled -- --ignored --nocapture`.
    #[test]
    #[ignore = "builds a million-message store; the budget run"]
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn a_page_of_summaries_holds_its_budget_on_a_million_messages() {
        use std::time::Instant;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j.db");
        drop(Storage::open(&path).unwrap());
        let now = juggler_core::model::Timestamp::now().millis();
        let day = 86_400_000;
        let profiles = [
            "client-a", "client-b", "lab", "personal", "work", "work-2", "codex",
        ];
        {
            let mut conn = rusqlite::Connection::open(&path).unwrap();
            let tx = conn.transaction().unwrap();
            let mut insert = tx
                .prepare("INSERT INTO usage_events (account, session, ts, model, input, output, cache_r, cache_w, reasoning, source, confidence, msg_uuid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, 'transcript', 'measured', ?8)")
                .unwrap();
            let mut seed: u64 = 7;
            let mut next = move || {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                (seed >> 33) as i64
            };
            for d in 0..365i64 {
                for (p, profile) in profiles.iter().enumerate() {
                    let sessions = if (d + 4) % 7 >= 5 { 1 } else { 5 };
                    for s in 0..sessions {
                        let begin = now - (365 - d) * day + (8 + next() % 10) * HOUR_MS;
                        for m in 0..(80 + next() % 40) {
                            let input = 2_000 + next() % 8_000;
                            insert
                                .execute(rusqlite::params![
                                    profile,
                                    format!("{profile}-{d}-{s}"),
                                    begin + m * 100_000,
                                    ["opus", "sonnet", "haiku"][(s + p) % 3],
                                    input,
                                    input / 5,
                                    input * 12,
                                    format!("{profile}-{d}-{s}-{m}")
                                ])
                                .unwrap();
                        }
                    }
                }
            }
            drop(insert);
            tx.commit().unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        let page = |span: i64| {
            let start = Instant::now();
            let mut rows = 0;
            for profile in profiles {
                // The range and the span before it, as a page asks.
                rows += hourly(&storage, dir.path(), profile, now - span, now, 60_000, now)
                    .unwrap()
                    .0
                    .len();
                rows += hourly(
                    &storage,
                    dir.path(),
                    profile,
                    now - 2 * span,
                    now - span,
                    60_000,
                    now,
                )
                .unwrap()
                .0
                .len();
            }
            (start.elapsed().as_secs_f64() * 1000.0, rows)
        };
        let (cold, _) = page(366 * day);
        eprintln!("first read of a year of settled hours, 7 profiles: {cold:.1} ms (once)");
        for (name, span) in [
            ("24 h", day),
            ("7 days", 7 * day),
            ("30 days", 30 * day),
            ("90 days", 90 * day),
            ("all", 366 * day),
        ] {
            let (ms, rows) = page(span);
            eprintln!("{name}: {ms:.1} ms for 14 reads ({rows} hourly rows)");
            assert!(ms < 50.0, "{name} took {ms:.1} ms for a page");
        }
    }

    #[test]
    fn the_cutoff_is_an_hour_boundary_six_hours_back() {
        let now = 100 * HOUR_MS + 1_234;
        assert_eq!(cutoff_for(now), 94 * HOUR_MS);
    }
}
