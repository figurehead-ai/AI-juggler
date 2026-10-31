//! Footprints (AJ-E1406, schema v29): how heavy each kind of session really
//! is on this Mac — peak tree memory, p95 tree CPU, peak helpers, how long
//! it ran — keyed by launch setting (`recipe:<id>`) or, for the AI tools AI
//! Juggler didn't start, by place (`observed:desktop`,
//! `observed:terminal`). Never a command line, folder or name.
//!
//! One row per session, updated in place while it runs (so a crash still
//! leaves its latest footprint); the newest 50 per key are kept, and a
//! deleted launch setting's or Account's go with it. Medians skip sessions
//! shorter than a minute and sessions that failed. Local only.

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// How many footprints are kept per key.
pub const FOOTPRINTS_PER_KEY: i64 = 50;
/// A session counts toward the medians once it has run this long.
pub const MIN_FOOTPRINT_MS: i64 = 60_000;

/// One session's footprint.
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    /// `recipe:<launch setting id>` or `observed:<place>`.
    pub key: String,
    /// The session (or observed tree) it belongs to — the upsert key.
    pub session_id: String,
    /// Provider id (`claude-code`, …).
    pub provider: String,
    /// The Account, when known.
    pub account_id: Option<String>,
    /// Peak tree memory, bytes.
    pub peak_rss: u64,
    /// p95 of the tree's CPU (percent of one core); `None` when never
    /// measured.
    pub p95_cpu: Option<f64>,
    /// Peak helper count.
    pub peak_helpers: u32,
    /// How long it has run, ms.
    pub duration_ms: i64,
    /// When this footprint was last written, epoch ms.
    pub recorded_at: i64,
}

/// What a key's footprints say.
#[derive(Clone, Debug, PartialEq)]
pub struct FootprintStats {
    /// The key.
    pub key: String,
    /// How many footprints it has.
    pub count: u32,
    /// Median of the peak memory, bytes.
    pub median_peak_rss: u64,
    /// Median of the p95 CPU, percent of one core; `None` when none was
    /// measured.
    pub median_p95_cpu: Option<f64>,
}

/// Typed access to the `footprints` table.
#[derive(Debug)]
pub struct FootprintsRepo<'a> {
    conn: &'a Connection,
}

/// The median of a sorted slice (the mean of the middle two when even).
fn median_u64(sorted: &[u64]) -> u64 {
    match sorted.len() {
        0 => 0,
        n if n % 2 == 1 => sorted[n / 2],
        n => u64::midpoint(sorted[n / 2 - 1], sorted[n / 2]),
    }
}

fn median_f64(sorted: &[f64]) -> Option<f64> {
    match sorted.len() {
        0 => None,
        n if n % 2 == 1 => Some(sorted[n / 2]),
        n => Some(f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])),
    }
}

impl FootprintsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> FootprintsRepo<'_> {
        FootprintsRepo { conn }
    }

    /// Writes a session's footprint (a running session's is replaced by
    /// its newer one; peaks never go down), then keeps the newest
    /// [`FOOTPRINTS_PER_KEY`] for its key.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn record(&self, footprint: &Footprint) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO footprints
                (key, session_id, provider, account_id, peak_rss, p95_cpu, peak_helpers, duration_ms, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(session_id) DO UPDATE SET
                key = excluded.key,
                peak_rss = MAX(peak_rss, excluded.peak_rss),
                p95_cpu = COALESCE(excluded.p95_cpu, p95_cpu),
                peak_helpers = MAX(peak_helpers, excluded.peak_helpers),
                duration_ms = MAX(duration_ms, excluded.duration_ms),
                recorded_at = excluded.recorded_at",
            params![
                footprint.key,
                footprint.session_id,
                footprint.provider,
                footprint.account_id,
                i64::try_from(footprint.peak_rss).unwrap_or(i64::MAX),
                footprint.p95_cpu,
                footprint.peak_helpers,
                footprint.duration_ms,
                footprint.recorded_at,
            ],
        )?;
        self.conn.execute(
            "DELETE FROM footprints WHERE key = ?1 AND session_id NOT IN (
                SELECT session_id FROM footprints WHERE key = ?1
                ORDER BY recorded_at DESC, session_id DESC LIMIT ?2)",
            params![footprint.key, FOOTPRINTS_PER_KEY],
        )?;
        Ok(())
    }

    /// Every footprint of one key, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn of_key(&self, key: &str) -> Result<Vec<Footprint>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT key, session_id, provider, account_id, peak_rss, p95_cpu, peak_helpers, duration_ms, recorded_at
             FROM footprints WHERE key = ?1 ORDER BY recorded_at DESC, session_id DESC",
        )?;
        let rows = stmt.query_map(params![key], |row| {
            Ok(Footprint {
                key: row.get(0)?,
                session_id: row.get(1)?,
                provider: row.get(2)?,
                account_id: row.get(3)?,
                peak_rss: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                p95_cpu: row.get(5)?,
                peak_helpers: row.get(6)?,
                duration_ms: row.get(7)?,
                recorded_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// What every key's footprints say (median peak memory, median p95
    /// CPU, how many), keys sorted. A session shorter than
    /// [`MIN_FOOTPRINT_MS`], or one that failed, isn't counted: neither is
    /// what a launch setting typically weighs.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn stats(&self) -> Result<Vec<FootprintStats>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.key, f.peak_rss, f.p95_cpu FROM footprints f
             LEFT JOIN instances i ON i.id = f.session_id
             WHERE f.duration_ms >= ?1 AND (i.status IS NULL OR i.status != 'failed')
             ORDER BY f.key",
        )?;
        let rows = stmt.query_map(params![MIN_FOOTPRINT_MS], |row| {
            Ok((
                row.get::<_, String>(0)?,
                u64::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
                row.get::<_, Option<f64>>(2)?,
            ))
        })?;
        let mut grouped: std::collections::BTreeMap<String, (Vec<u64>, Vec<f64>)> =
            std::collections::BTreeMap::new();
        for row in rows {
            let (key, rss, cpu) = row?;
            let entry = grouped.entry(key).or_default();
            entry.0.push(rss);
            if let Some(cpu) = cpu {
                entry.1.push(cpu);
            }
        }
        Ok(grouped
            .into_iter()
            .map(|(key, (mut rss, mut cpu))| {
                rss.sort_unstable();
                cpu.sort_by(f64::total_cmp);
                FootprintStats {
                    key,
                    count: u32::try_from(rss.len()).unwrap_or(u32::MAX),
                    median_peak_rss: median_u64(&rss),
                    median_p95_cpu: median_f64(&cpu),
                }
            })
            .collect())
    }
}

impl FootprintsRepo<'_> {
    /// Deletes the footprints of launch settings and Accounts that no
    /// longer exist or were removed; returns how many went.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn forget_deleted(&self) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "DELETE FROM footprints
             WHERE (key LIKE 'recipe:%' AND substr(key, 8) NOT IN (SELECT id FROM launch_recipes WHERE removed_at IS NULL))
                OR (account_id IS NOT NULL AND account_id NOT IN (SELECT id FROM accounts))",
            [],
        )?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::storage::Storage;

    fn footprint(session: &str, key: &str, rss: u64, cpu: Option<f64>, at: i64) -> Footprint {
        Footprint {
            key: key.into(),
            session_id: session.into(),
            provider: "claude-code".into(),
            account_id: Some("acc".into()),
            peak_rss: rss,
            p95_cpu: cpu,
            peak_helpers: 2,
            duration_ms: 60_000,
            recorded_at: at,
        }
    }

    #[test]
    fn a_running_session_updates_its_one_row_and_peaks_never_drop() {
        let storage = Storage::open_in_memory().unwrap();
        let repo = storage.footprints();
        repo.record(&footprint("s1", "recipe:a", 900, Some(40.0), 1))
            .unwrap();
        // Ten minutes later it uses less now, but its peak stays.
        let mut later = footprint("s1", "recipe:a", 600, Some(55.0), 2);
        later.peak_helpers = 1;
        later.duration_ms = 600_000;
        repo.record(&later).unwrap();
        let rows = repo.of_key("recipe:a").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peak_rss, 900);
        assert_eq!(rows[0].p95_cpu, Some(55.0));
        assert_eq!(rows[0].peak_helpers, 2);
        assert_eq!(rows[0].duration_ms, 600_000);
        assert_eq!(rows[0].recorded_at, 2);
    }

    #[test]
    fn medians_per_key_from_real_rows() {
        let storage = Storage::open_in_memory().unwrap();
        let repo = storage.footprints();
        for (n, rss) in [1_000_u64, 3_000, 2_000, 10_000].into_iter().enumerate() {
            repo.record(&footprint(
                &format!("a{n}"),
                "recipe:a",
                rss,
                Some(10.0 * (n as f64 + 1.0)),
                n as i64,
            ))
            .unwrap();
        }
        repo.record(&footprint("b0", "observed:desktop", 5_000, None, 9))
            .unwrap();
        let stats = repo.stats().unwrap();
        assert_eq!(stats.len(), 2);
        let desktop = &stats[0];
        assert_eq!(desktop.key, "observed:desktop");
        assert_eq!(desktop.count, 1);
        assert_eq!(desktop.median_p95_cpu, None);
        let a = &stats[1];
        assert_eq!(a.count, 4);
        assert_eq!(
            a.median_peak_rss, 2_500,
            "the middle two of 1k, 2k, 3k, 10k"
        );
        assert_eq!(a.median_p95_cpu, Some(25.0));
    }

    #[test]
    fn only_the_newest_fifty_per_key_are_kept() {
        let storage = Storage::open_in_memory().unwrap();
        let repo = storage.footprints();
        for n in 0..60 {
            repo.record(&footprint(&format!("s{n:02}"), "recipe:a", 100, None, n))
                .unwrap();
        }
        repo.record(&footprint("other", "recipe:b", 100, None, 0))
            .unwrap();
        let rows = repo.of_key("recipe:a").unwrap();
        assert_eq!(rows.len(), 50);
        assert_eq!(
            rows.last().unwrap().session_id,
            "s10",
            "the oldest ten went"
        );
        assert_eq!(
            repo.of_key("recipe:b").unwrap().len(),
            1,
            "other keys untouched"
        );
    }
}
