//! The settled hours: a profile's hourly sums older than six hours rarely
//! change, so they're summed once and kept, in the store (its derived
//! `usage_hour_rollup`, so a start reads them back instead of summing every
//! message again) and in memory; each summary reads only the recent hours
//! live. When an older message changes, by any build, the store marks its
//! hour and the settled-history stamp moves: only the marked hours are
//! summed again.
//!
//! The maintainer's Mac (0.3.7, 281K Codex calls over six weeks): summing
//! them all took 0.4 s warm and 1.8 s cold; reading the kept hours back
//! takes a few milliseconds.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use juggler_core::storage::{HOUR_MS, HourRow, Replace, Rollup, SETTLED_MS, Storage};

use crate::error::AppResult;

/// How far the kept hours may fall behind before the hours since are
/// summed and added.
const REFRESH_MS: i64 = 6 * HOUR_MS;

/// Past this many changed hours, a profile's settled hours are summed
/// afresh (one pass beats as many hour by hour: a re-read of history).
const MOST_CHANGED_HOURS: usize = 200;

/// One profile's settled hours.
type Kept = Rollup;

/// One profile's kept hours, and the settled-history stamp's count of
/// changes they were brought up to date at (the stamp is the store's; each
/// profile catches up on its own changes when it's read).
struct Synced {
    at: i64,
    kept: Kept,
}

#[derive(Default)]
struct Store {
    token: Option<i64>,
    profiles: HashMap<String, Synced>,
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

/// `profile`'s settled hours up to `target`: kept ones brought up to date
/// (changed hours summed again, the hours since added), else summed afresh;
/// what changed is kept in the store too.
fn bring_up_to_date(
    storage: &Storage,
    profile: &str,
    kept: Option<Kept>,
    target: i64,
    cap: usize,
) -> AppResult<Kept> {
    let history = storage.usage_history();
    let changed = history.take_changed_hours(profile)?;
    let kept = kept.map_or_else(|| history.rollup(profile), |kept| Ok(Some(kept)))?;
    let Some(mut kept) = kept.filter(|kept| kept.cutoff <= target) else {
        return afresh(storage, profile, target, cap);
    };
    let changed: Vec<i64> = changed
        .into_iter()
        .filter(|hour| *hour < kept.cutoff)
        .collect();
    if changed.len() > MOST_CHANGED_HOURS {
        return afresh(storage, profile, target, cap);
    }
    let mut added = history.hourly_in(profile, &changed)?;
    let mut cutoff = kept.cutoff;
    if kept.cutoff + REFRESH_MS <= target {
        let (since, _) = history.hourly(profile, kept.cutoff, target, usize::MAX)?;
        added.extend(since);
        cutoff = target;
    }
    if changed.is_empty() && cutoff == kept.cutoff {
        return Ok(kept);
    }
    history.save_rollup(profile, cutoff, kept.cut, Replace::Hours(&changed), &added)?;
    let gone: BTreeSet<i64> = changed.iter().copied().collect();
    kept.rows.retain(|row| !gone.contains(&row.hour_ms));
    kept.rows.extend(added);
    kept.rows.sort_by_key(|row| row.hour_ms);
    kept.cutoff = cutoff;
    if kept.rows.len() > cap {
        // The oldest go past the cap, as a fresh sum would leave them out.
        let over = kept.rows.len() - cap;
        kept.rows.drain(..over);
        kept.cut = true;
    }
    Ok(kept)
}

/// `profile`'s settled hours summed from every message, and kept.
fn afresh(storage: &Storage, profile: &str, target: i64, cap: usize) -> AppResult<Kept> {
    let history = storage.usage_history();
    let (mut rows, cut) = history.hourly(profile, i64::MIN, target, cap)?;
    rows.reverse();
    history.save_rollup(profile, target, cut, Replace::Everything, &rows)?;
    Ok(Kept {
        cutoff: target,
        cut,
        rows,
    })
}

/// One profile's hourly rows in `[start, end)`: the settled ones kept, the
/// recent ones live. Says whether rows were left out.
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
    // Another store (a restored one) keeps nothing of this one's.
    if store.token != Some(stamp.0) {
        store.profiles.clear();
        store.token = Some(stamp.0);
    }
    let synced = match store.profiles.remove(profile) {
        // Nothing older changed and the kept hours are near enough.
        Some(synced) if synced.at == stamp.1 && synced.kept.cutoff + REFRESH_MS > target => synced,
        other => Synced {
            at: stamp.1,
            kept: bring_up_to_date(
                storage,
                profile,
                other.map(|synced| synced.kept),
                target,
                cap,
            )?,
        },
    };
    let kept = &store
        .profiles
        .entry(profile.to_owned())
        .or_insert(synced)
        .kept;
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
#[allow(
    clippy::disallowed_methods,
    reason = "test fixtures build databases raw (other tools' files, old schemas)"
)]
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
            detail: juggler_core::quota::UsageDetail::default(),
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

    fn event_on(account: &str, ts: i64, uuid: &str, input: u64) -> UsageEvent {
        UsageEvent {
            account: account.into(),
            ..event(ts, uuid, input)
        }
    }

    fn sum(storage: &Storage, data: &Path, profile: &str, now: i64) -> (u64, f64) {
        let (rows, _) = hourly(
            storage,
            data,
            profile,
            now - 30 * 86_400_000,
            now + 1,
            1_000,
            now,
        )
        .unwrap();
        (
            rows.iter().map(|row| row.tokens.input).sum(),
            rows.iter().map(|row| row.cost.stored_usd).sum(),
        )
    }

    /// 0.3.7: the settled hours are kept in the store, so a start reads
    /// them back instead of summing every message; a change made while the
    /// app was closed (by any build) is caught by its hour.
    #[test]
    fn a_start_reads_the_kept_hours_and_catches_up_on_what_changed() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("j.db");
        let now = juggler_core::model::Timestamp::now().millis();
        let old = now - 3 * 86_400_000;
        {
            let storage = Storage::open(&db).unwrap();
            storage
                .usage()
                .insert_events(&[event(old, "a", 100), event(old - 86_400_000, "b", 20)])
                .unwrap();
            assert_eq!(sum(&storage, dir.path(), "work", now).0, 120);
        }
        // A start: nothing in memory (a fresh data folder key), the store
        // as it was. The kept hours come back without a sum of every
        // message: proven by removing a message behind the triggers' back
        // (a fresh sum would leave it out).
        let restarted = dir.path().join("restarted");
        let storage = Storage::open(&db).unwrap();
        {
            let raw = rusqlite::Connection::open(&db).unwrap();
            raw.execute_batch(
                "DROP TRIGGER usage_rollup_event_delete;
                 DELETE FROM usage_events WHERE msg_uuid = 'b';",
            )
            .unwrap();
        }
        assert_eq!(
            sum(&storage, &restarted, "work", now).0,
            120,
            "the kept hours"
        );
        // A change while closed, through the triggers: its hour only.
        drop(storage);
        let storage = Storage::open(&db).unwrap();
        storage
            .usage()
            .insert_events(&[event(old + 60_000, "c", 5)])
            .unwrap();
        let again = dir.path().join("again");
        assert_eq!(sum(&storage, &again, "work", now).0, 125);
    }

    /// The stamp is the store's, the catching up each profile's own: a
    /// change to one profile's older hour is caught however the profiles
    /// are read.
    #[test]
    fn each_profile_catches_up_on_its_own_changes() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("j.db")).unwrap();
        let now = juggler_core::model::Timestamp::now().millis();
        let old = now - 3 * 86_400_000;
        storage
            .usage()
            .insert_events(&[event_on("a", old, "a1", 10), event_on("b", old, "b1", 20)])
            .unwrap();
        assert_eq!(sum(&storage, dir.path(), "a", now).0, 10);
        assert_eq!(sum(&storage, dir.path(), "b", now).0, 20);
        storage
            .usage()
            .insert_events(&[event_on("b", old + 1_000, "b2", 300)])
            .unwrap();
        assert_eq!(sum(&storage, dir.path(), "a", now).0, 10);
        assert_eq!(
            sum(&storage, dir.path(), "b", now).0,
            320,
            "b's change, after a was read"
        );
    }

    /// A cost that moves without its tokens (prices changed) reaches the
    /// kept hours too.
    #[test]
    fn a_new_price_reaches_the_kept_hours() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("j.db")).unwrap();
        let now = juggler_core::model::Timestamp::now().millis();
        let old = now - 3 * 86_400_000;
        let priced = UsageEvent {
            model: Some("unlisted-model".into()),
            ..event(old, "p", 1_000_000)
        };
        storage.usage().insert_events(&[priced]).unwrap();
        assert_eq!(
            sum(&storage, dir.path(), "work", now).1,
            0.0,
            "no list price"
        );
        storage
            .usage()
            .pricing_override_upsert(&juggler_core::cost_ledger::LedgerPriceRow {
                model: "unlisted-model".to_owned(),
                effective_from_ms: 0,
                input_per_1k: 0.002,
                output_per_1k: 0.0,
                cache_read_per_1k: 0.0,
                cache_write_per_1k: 0.0,
                cache_write_1h_per_1k: None,
                long_context: None,
                fast: None,
                us_multiplier: None,
                source_url: "test".to_owned(),
                captured: "2026-11-03".to_owned(),
            })
            .unwrap();
        let mut cursor = None;
        while let Some(next) = storage.usage().reprice(cursor, 100).unwrap() {
            cursor = Some(next);
        }
        let (_, usd) = sum(&storage, dir.path(), "work", now);
        assert!((usd - 2.0).abs() < 1e-9, "{usd}");
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
            "client-a", "client-b", "lab", "personal", "work", "work-2", "openai-1",
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
