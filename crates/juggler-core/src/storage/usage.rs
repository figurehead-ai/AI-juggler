//! Usage event & quota sample store (AJ-E606, deep/09 §Q8 schema):
//! append-only `usage_events` + `quota_samples` (+ the §Q8
//! `reset_queue` for AJ-E611), with `UNIQUE(account, msg_uuid)`
//! dedupe so resumed transcript files can never double-count, and (0.3.8)
//! one row per keyed call across accounts: a conversation copied into
//! another account's folder outside AI Juggler (a Dev build's switch, a
//! folder copied by hand, another Mac) repeats the vendor's own call keys,
//! and the call was billed once. The
//! engine's maths (§Q3) lives in [`crate::quota`] as pure functions;
//! THIS module only persists and fetches.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;
use crate::model::{Confidence, TokenCounts};
use crate::quota::{QuotaSample, UsageEvent, UsageSource, WindowKind};

mod cost_pages;
pub use cost_pages::CostDay;
mod detail;
pub use detail::RepriceCursor;
use detail::{Details, EVENT_COLUMNS, event_row};
mod owners;
use owners::{Owners, Rows, SessionKey};

/// One window's readings, oldest first, each with its source (`None`:
/// stored before sources were kept).
pub type SourcedSamples = Vec<(QuotaSample, Option<UsageSource>)>;

/// Append-only usage store over an open connection (single writer,
/// like every storage repo).
pub struct UsageRepo<'a> {
    conn: &'a Connection,
}

impl UsageRepo<'_> {
    /// Wraps an open connection.
    pub fn new(conn: &Connection) -> UsageRepo<'_> {
        UsageRepo { conn }
    }

    /// Appends events in ONE transaction (see [`Self::store_events`]).
    /// Returns how many rows were NEWLY inserted (the ingest counters'
    /// honest denominator).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the
    /// transaction rolls back atomically.
    pub fn insert_events(&self, events: &[UsageEvent]) -> Result<usize, CoreError> {
        Ok(self
            .store_events(events)?
            .into_iter()
            .filter(|stored| stored.new)
            .count())
    }

    /// Stores events in ONE transaction, keyed `UNIQUE(account,
    /// msg_uuid)`. A new key inserts. A key already stored keeps the larger
    /// figure of each class: one response can reach the store over two tail
    /// passes (its first lines in one, a later block with the final output
    /// in the next), and the first figure must not stand. A re-read of the
    /// same lines changes nothing. Says, per event, whether it was new and
    /// what it added.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the
    /// transaction rolls back atomically.
    pub fn store_events(&self, events: &[UsageEvent]) -> Result<Vec<Stored>, CoreError> {
        let tx = write_transaction(self.conn)?;
        let stored = store_in(&tx, events, Removed::default())?;
        tx.commit()?;
        Ok(stored)
    }

    /// Whether usage rows were written that the holders don't account for
    /// (a build without them wrote here): every transcript and rollout
    /// should be read again.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn derived_dirty(&self) -> Result<bool, CoreError> {
        self.conn
            .query_row("SELECT dirty FROM usage_derived WHERE id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map(|dirty| dirty.unwrap_or(0) != 0)
            .map_err(CoreError::Storage)
    }

    /// The accounts whose usage keys name their own session (their tool's
    /// adapter says so): no holders are kept for them, and any kept before
    /// go. An account that leaves the set has none, so the holders are
    /// marked dirty and everything is read again. Writes only when the set
    /// changed.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set_unshared_accounts(&self, accounts: &BTreeSet<String>) -> Result<(), CoreError> {
        let now: BTreeSet<String> = self
            .conn
            .prepare("SELECT account FROM usage_unshared")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<_, _>>()?;
        if &now == accounts {
            return Ok(());
        }
        let tx = write_transaction(self.conn)?;
        if now.iter().any(|account| !accounts.contains(account)) {
            tx.execute("UPDATE usage_derived SET dirty = 1 WHERE id = 1", [])?;
        }
        tx.execute("DELETE FROM usage_unshared", [])?;
        for account in accounts {
            tx.execute(
                "INSERT INTO usage_unshared (account) VALUES (?1)",
                params![account],
            )?;
            tx.execute(
                "DELETE FROM usage_holders WHERE account = ?1",
                params![account],
            )?;
            tx.execute(
                "DELETE FROM usage_spans WHERE account = ?1",
                params![account],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 0.3.8: before everything is read again, the sessions' standings go
    /// and are counted afresh from the holders as they're needed: a
    /// standing kept by an older build counted only its own account's
    /// sessions as sharing a message.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn forget_standings(&self) -> Result<(), CoreError> {
        self.conn
            .execute("DELETE FROM usage_spans", [])
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// 0.3.8: settles every keyed call that more than one account holds,
    /// so its one row is the keeper's: what a re-read of the files on disk
    /// doesn't reach (a copy whose file is gone). Returns how many calls
    /// it settled.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; nothing changes
    /// then.
    pub fn settle_copies(&self) -> Result<usize, CoreError> {
        let tx = write_transaction(self.conn)?;
        let keys: Vec<String> = tx
            .prepare(
                "SELECT DISTINCT a.msg_uuid FROM usage_holders a
                 WHERE EXISTS (SELECT 1 FROM usage_holders b
                     WHERE b.msg_uuid = a.msg_uuid AND b.account <> a.account)",
            )?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<_, _>>()?;
        let mut count = keys.len();
        let mut owners = Owners::new(&tx)?;
        owners.keys.extend(keys);
        owners.finish()?;
        // Unshared accounts keep no holders: their copies are found by key
        // among themselves, and the row stored first stays.
        let copies: Vec<(String, String)> = tx
            .prepare(
                "SELECT msg_uuid, source FROM usage_events
                 WHERE msg_uuid IS NOT NULL
                   AND account IN (SELECT account FROM usage_unshared)
                 GROUP BY msg_uuid, source HAVING COUNT(*) > 1",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        count += copies.len();
        let mut rows = Rows::new(&tx)?;
        for (key, source) in copies {
            let (account, session): (String, Option<String>) = tx.query_row(
                "SELECT account, session FROM usage_events
                 WHERE msg_uuid = ?1 AND source = ?2
                   AND account IN (SELECT account FROM usage_unshared)
                 ORDER BY id LIMIT 1",
                params![key, source],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            let Some(session) = session else {
                continue; // no session to keep it on: left as it is
            };
            let first = (account, session);
            let accounts: BTreeSet<String> = tx
                .prepare(
                    "SELECT account FROM usage_events
                     WHERE msg_uuid = ?1 AND source = ?2
                       AND account IN (SELECT account FROM usage_unshared)",
                )?
                .query_map(params![key, source], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;
            rows.give(&key, &accounts, &first)?;
        }
        drop(rows);
        tx.commit()?;
        Ok(count)
    }

    /// Marks the holders whole again (after reading everything again).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn clear_derived_dirty(&self) -> Result<(), CoreError> {
        self.conn
            .execute("UPDATE usage_derived SET dirty = 0 WHERE id = 1", [])
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// [`Self::delete_session_events`] and then [`Self::store_events`],
    /// in ONE transaction: the re-read replaces a session's rows, or
    /// nothing changes.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; nothing is
    /// deleted or stored then.
    pub fn replace_session_events(
        &self,
        sessions: &[(String, UsageSource, String)],
        events: &[UsageEvent],
    ) -> Result<Vec<Stored>, CoreError> {
        let tx = write_transaction(self.conn)?;
        let (_, removed) = delete_in(&tx, sessions)?;
        let stored = store_in(&tx, events, removed)?;
        tx.commit()?;
        Ok(stored)
    }

    /// Appends quota samples in ONE transaction (`INSERT OR REPLACE` on
    /// the sample key, as [`Self::insert_sample`]).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn insert_samples(
        &self,
        samples: &[(String, WindowKind, QuotaSample)],
    ) -> Result<(), CoreError> {
        let tx = write_transaction(self.conn)?;
        for (account, kind, sample) in samples {
            UsageRepo { conn: &tx }.insert_sample(account, *kind, sample)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Deletes the stored events of these `(account, source, session)`
    /// triples, in ONE transaction: the one-time re-read that replaces
    /// rows counted wrongly before (0.0.13: Claude content blocks keyed
    /// per line, Codex's nested token classes). Only the pipeline calls
    /// it, for sessions whose files it is reading again in the same pass.
    /// Returns how many rows went.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the
    /// transaction rolls back atomically.
    pub fn delete_session_events(
        &self,
        sessions: &[(String, UsageSource, String)],
    ) -> Result<usize, CoreError> {
        let tx = write_transaction(self.conn)?;
        let (deleted, removed) = delete_in(&tx, sessions)?;
        store_in(&tx, &[], removed)?;
        tx.commit()?;
        Ok(deleted)
    }

    /// P21's leftovers: a Codex compaction row (`…/compacted/…`) that an
    /// older build stored for another thread's model call, in a session
    /// whose rollout is gone (so no re-read replaces it). One goes only
    /// when its tokens equal, class by class, a stored record's
    /// (`thread/turn/response`): that record is the call, counted where it
    /// was written. A gone file's other rows stay, as they always do, and
    /// a session in `present` is never touched. Returns how many went.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; nothing is
    /// deleted then.
    pub fn forget_duplicated_compactions(
        &self,
        account: &str,
        present: &BTreeSet<String>,
    ) -> Result<usize, CoreError> {
        let tx = write_transaction(self.conn)?;
        let candidates = {
            let mut find = tx.prepare(
                "SELECT e.id, e.session, e.msg_uuid FROM usage_events e
                 WHERE e.account = ?1 AND e.session IS NOT NULL
                   AND e.msg_uuid LIKE '%/compacted/%'
                   AND EXISTS (SELECT 1 FROM usage_events r
                       WHERE r.account = e.account AND r.msg_uuid IS NOT NULL
                         AND r.msg_uuid NOT LIKE '%/compacted/%'
                         AND r.input = e.input AND r.output = e.output
                         AND r.cache_r = e.cache_r AND r.cache_w = e.cache_w
                         AND r.reasoning = e.reasoning)",
            )?;
            find.query_map(params![account], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        let mut gone = 0;
        for (id, session, key) in candidates {
            if present.contains(&session) {
                continue;
            }
            gone += tx.execute("DELETE FROM usage_events WHERE id = ?1", params![id])?;
            tx.execute(
                "DELETE FROM usage_detail WHERE account = ?1 AND msg_uuid = ?2",
                params![account, key],
            )?;
            tx.execute(
                "DELETE FROM usage_holders WHERE account = ?1 AND msg_uuid = ?2 AND session = ?3",
                params![account, key, session],
            )?;
        }
        tx.commit()?;
        Ok(gone)
    }

    /// Every event for one account since `since_ms` (the window fold
    /// fetches a 7 d tail; older rows are audit, never maths).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn events_for_account(
        &self,
        account: &str,
        since_ms: i64,
    ) -> Result<Vec<UsageEvent>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {EVENT_COLUMNS} WHERE e.account = ?1 AND e.ts >= ?2 ORDER BY e.ts"
        ))?;
        let rows = stmt.query_map(params![account, since_ms], event_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Appends one vendor quota sample (`INSERT OR REPLACE` on the
    /// `(account, window, ts)` primary key — re-reads of the same
    /// instant are idempotent).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn insert_sample(
        &self,
        account: &str,
        kind: WindowKind,
        sample: &QuotaSample,
    ) -> Result<(), CoreError> {
        self.insert_named_sample(account, kind.as_str(), sample)
    }

    /// [`Self::insert_sample`] under a stored window name (an all-models
    /// window's, or a model's own week's).
    fn insert_named_sample(
        &self,
        account: &str,
        window: &str,
        sample: &QuotaSample,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO quota_samples
                     (account, window, ts, used_pct, resets_at, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    account,
                    window,
                    sample.ts,
                    sample.used_pct,
                    sample.resets_at,
                    confidence_name(sample.confidence),
                ],
            )
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// Appends vendor samples with where each came from, in ONE
    /// transaction (`INSERT OR REPLACE` on both rows: reading the same
    /// instant again changes nothing).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the transaction
    /// rolls back atomically.
    pub fn insert_sourced_samples(
        &self,
        samples: &[(String, WindowKind, QuotaSample, UsageSource)],
    ) -> Result<(), CoreError> {
        let tx = write_transaction(self.conn)?;
        for (account, kind, sample, source) in samples {
            UsageRepo { conn: &tx }.insert_sourced(account, kind.as_str(), sample, *source)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Appends readings of weekly limits on one model only (`(account,
    /// model, sample, source)`), each under its model's own window name
    /// ([`crate::quota::model_week_window`]), in ONE transaction.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the transaction
    /// rolls back atomically.
    pub fn insert_model_week_samples(
        &self,
        samples: &[(String, String, QuotaSample, UsageSource)],
    ) -> Result<(), CoreError> {
        if samples.is_empty() {
            return Ok(());
        }
        let tx = write_transaction(self.conn)?;
        for (account, model, sample, source) in samples {
            let window = crate::quota::model_week_window(model);
            UsageRepo { conn: &tx }.insert_sourced(account, &window, sample, *source)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn insert_sourced(
        &self,
        account: &str,
        window: &str,
        sample: &QuotaSample,
        source: UsageSource,
    ) -> Result<(), CoreError> {
        self.insert_named_sample(account, window, sample)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO quota_sample_sources (account, window, ts, source)
             VALUES (?1, ?2, ?3, ?4)",
            params![account, window, sample.ts, source.as_str()],
        )?;
        Ok(())
    }

    /// Every model's own weekly limit readings for one account, by model
    /// (sorted), each oldest first with its source.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn model_week_samples_for_account(
        &self,
        account: &str,
    ) -> Result<Vec<(String, SourcedSamples)>, CoreError> {
        let prefix = crate::quota::MODEL_WEEK_PREFIX;
        let mut stmt = self.conn.prepare(
            "SELECT q.window, q.ts, q.used_pct, q.resets_at, q.confidence, s.source
             FROM quota_samples q
             LEFT JOIN quota_sample_sources s
               ON s.account = q.account AND s.window = q.window AND s.ts = q.ts
             WHERE q.account = ?1 AND substr(q.window, 1, ?2) = ?3
             ORDER BY q.window, q.ts",
        )?;
        let prefix_len = i64::try_from(prefix.len()).unwrap_or(i64::MAX);
        let rows = stmt.query_map(params![account, prefix_len, prefix], |row| {
            Ok((
                row.get::<_, String>(0)?,
                QuotaSample {
                    ts: row.get(1)?,
                    used_pct: row.get(2)?,
                    resets_at: row.get(3)?,
                    confidence: parse_confidence(&row.get::<_, String>(4)?),
                },
                row.get::<_, Option<String>>(5)?
                    .as_deref()
                    .and_then(UsageSource::parse),
            ))
        })?;
        let mut out: Vec<(String, SourcedSamples)> = Vec::new();
        for row in rows {
            let (window, sample, source) = row?;
            let Some(model) = window.strip_prefix(prefix).map(str::to_owned) else {
                continue;
            };
            match out.last_mut() {
                Some((last, samples)) if *last == model => samples.push((sample, source)),
                _ => out.push((model, vec![(sample, source)])),
            }
        }
        Ok(out)
    }

    /// [`Self::sourced_samples_for_account`] from `since_ms` on, with the
    /// newest one before it (what an older reading said): the rc6 walk's
    /// Codex window had 338 915 samples, every one read on each alerts
    /// pass under the storage lock. Through the key's index, so only the
    /// span is read.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn sourced_samples_since(
        &self,
        account: &str,
        kind: WindowKind,
        since_ms: i64,
    ) -> Result<Vec<(QuotaSample, Option<UsageSource>)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT q.ts, q.used_pct, q.resets_at, q.confidence, s.source
             FROM quota_samples q
             LEFT JOIN quota_sample_sources s
               ON s.account = q.account AND s.window = q.window AND s.ts = q.ts
             WHERE q.account = ?1 AND q.window = ?2
               AND q.ts >= COALESCE(
                 (SELECT MAX(ts) FROM quota_samples
                   WHERE account = ?1 AND window = ?2 AND ts < ?3), ?3)
             ORDER BY q.ts",
        )?;
        let rows = stmt.query_map(params![account, kind.as_str(), since_ms], |row| {
            Ok((
                QuotaSample {
                    ts: row.get(0)?,
                    used_pct: row.get(1)?,
                    resets_at: row.get(2)?,
                    confidence: parse_confidence(&row.get::<_, String>(3)?),
                },
                row.get::<_, Option<String>>(4)?
                    .as_deref()
                    .and_then(UsageSource::parse),
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// [`Self::samples_for_account`] with each sample's source, oldest
    /// first (`None`: stored before sources were kept).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn sourced_samples_for_account(
        &self,
        account: &str,
        kind: WindowKind,
    ) -> Result<Vec<(QuotaSample, Option<UsageSource>)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT q.ts, q.used_pct, q.resets_at, q.confidence, s.source
             FROM quota_samples q
             LEFT JOIN quota_sample_sources s
               ON s.account = q.account AND s.window = q.window AND s.ts = q.ts
             WHERE q.account = ?1 AND q.window = ?2
             ORDER BY q.ts",
        )?;
        let rows = stmt.query_map(params![account, kind.as_str()], |row| {
            Ok((
                QuotaSample {
                    ts: row.get(0)?,
                    used_pct: row.get(1)?,
                    resets_at: row.get(2)?,
                    confidence: parse_confidence(&row.get::<_, String>(3)?),
                },
                row.get::<_, Option<String>>(4)?
                    .as_deref()
                    .and_then(UsageSource::parse),
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Every sample for one account and window (the §Q8 store keys
    /// `(account, window, ts)` — a five-hour sample must never pollute
    /// the weekly reading).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn samples_for_account(
        &self,
        account: &str,
        kind: WindowKind,
    ) -> Result<Vec<QuotaSample>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT ts, used_pct, resets_at, confidence
             FROM quota_samples
             WHERE account = ?1 AND window = ?2
             ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![account, kind.as_str()], |row| {
            Ok(QuotaSample {
                ts: row.get(0)?,
                used_pct: row.get(1)?,
                resets_at: row.get(2)?,
                confidence: parse_confidence(&row.get::<_, String>(3)?),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Events for one account inside `(lo, hi]` (the CSV export walks
    /// month windows through this — chunked reads, never a full-table
    /// materialisation; the `(account, ts)` index serves it).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn events_between(
        &self,
        account: &str,
        lo_ms: i64,
        hi_ms: i64,
    ) -> Result<Vec<UsageEvent>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {EVENT_COLUMNS} WHERE e.account = ?1 AND e.ts > ?2 AND e.ts <= ?3 ORDER BY e.ts"
        ))?;
        let rows = stmt.query_map(params![account, lo_ms, hi_ms], event_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Every account id present in `usage_events` (the export's scope
    /// when no account is named).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn usage_accounts(&self) -> Result<Vec<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT account FROM usage_events ORDER BY account")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Upserts one user pricing override (same model + instant ⇒
    /// replace).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn pricing_override_upsert(
        &self,
        row: &crate::cost_ledger::LedgerPriceRow,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO pricing_overrides
                     (model, effective_from_ms, input_per_1k, output_per_1k,
                      cache_read_per_1k, cache_write_per_1k, source_url, captured)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.model,
                    row.effective_from_ms,
                    row.input_per_1k,
                    row.output_per_1k,
                    row.cache_read_per_1k,
                    row.cache_write_per_1k,
                    row.source_url,
                    row.captured,
                ],
            )
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// Deletes one override (exact model + instant).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn pricing_override_delete(
        &self,
        model: &str,
        effective_from_ms: i64,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "DELETE FROM pricing_overrides WHERE model = ?1 AND effective_from_ms = ?2",
                params![model, effective_from_ms],
            )
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// All user overrides.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn pricing_overrides(&self) -> Result<Vec<crate::cost_ledger::LedgerPriceRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT model, effective_from_ms, input_per_1k, output_per_1k,
                    cache_read_per_1k, cache_write_per_1k, source_url, captured
             FROM pricing_overrides ORDER BY model, effective_from_ms",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::cost_ledger::LedgerPriceRow {
                model: row.get(0)?,
                effective_from_ms: row.get(1)?,
                input_per_1k: row.get(2)?,
                output_per_1k: row.get(3)?,
                cache_read_per_1k: row.get(4)?,
                cache_write_per_1k: row.get(5)?,
                // An override is one set of rates: no tiers.
                cache_write_1h_per_1k: None,
                long_context: None,
                fast: None,
                us_multiplier: None,
                source_url: row.get(6)?,
                captured: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Queues a resume for an exhausted session (AJ-E611's reset
    /// queue; written by the alert FSM when it lands).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn queue_reset(
        &self,
        id: &str,
        session: &str,
        account: &str,
        resume_at_ms: i64,
    ) -> Result<(), CoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO reset_queue (id, session, account, resume_at, state, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'queued', ?5)",
                params![id, session, account, resume_at_ms, resume_at_ms],
            )
            .map(|_| ())
            .map_err(CoreError::Storage)
    }

    /// Queued resumes whose time has come, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn pending_resumes(
        &self,
        now_ms: i64,
    ) -> Result<Vec<(String, String, String, i64)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session, account, resume_at FROM reset_queue
             WHERE state = 'queued' AND resume_at <= ?1
             ORDER BY resume_at",
        )?;
        let rows = stmt.query_map(params![now_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// The reminders still to act on (0.3.2): queued ones, and due ones
    /// already notified, until the user opens or closes them; soonest
    /// first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_reminders(&self) -> Result<Vec<(String, String, String, i64)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session, account, resume_at FROM reset_queue
             WHERE state IN ('queued', 'notified')
             ORDER BY resume_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Advances a reset-queue row (`queued|notified|resumed|dismissed`).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set_reset_state(&self, id: &str, state: &str) -> Result<(), CoreError> {
        self.conn
            .execute(
                "UPDATE reset_queue SET state = ?2 WHERE id = ?1",
                params![id, state],
            )
            .map(|_| ())
            .map_err(CoreError::Storage)
    }
}

/// A write transaction that takes the write lock as it begins (`BEGIN
/// IMMEDIATE`). A deferred one begins as a read, and these writes read
/// first (who holds a message, where each session stands): when another
/// connection on the file (the process ledger, the cleanup audit) wrote and
/// committed meanwhile, SQLite answered "database is locked" at once
/// instead of waiting (#126's CI). Taken first, the lock waits its turn
/// under the busy timeout, and the reads see the latest data.
fn write_transaction(conn: &Connection) -> Result<rusqlite::Transaction<'_>, CoreError> {
    rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)
        .map_err(CoreError::Storage)
}

/// Stable confidence names for storage.
/// What one stored event did: whether its key was new, and the tokens it
/// added (all of them when new; the increase when it raised a stored row;
/// none for a repeat).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Stored {
    /// The key was not stored before.
    pub new: bool,
    /// Tokens this write added to the store.
    pub added: TokenCounts,
}

fn as_sql(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

fn from_sql(count: i64) -> u64 {
    u64::try_from(count).unwrap_or_default()
}

/// Tables derived from the transcripts and rollouts, kept outside the
/// numbered schema: reading the files again (the pipeline's re-read)
/// rebuilds them, so they need no migration of their own.
/// - `usage_holders`: every session that offered each keyed message,
///   whichever one keeps it, in any account (0.3.8: a key is the vendor's
///   own id for one call, so the holders of a key across accounts are
///   copies of one call; the tools' keys never take each other's form).
/// - `usage_spans`: each session's standing under the shared-message rule
///   ([`Span`]).
/// - `usage_unshared`: accounts whose usage keys name their own session (a
///   tool's adapter says so): no holders are kept for them.
/// - `usage_derived`: one row, `dirty` once something wrote usage rows the
///   holders don't account for. A build that predates these tables (gone
///   back to, then forward again) does exactly that, and the triggers below
///   fire for it too, since they live in the file: a row inserted, or moved
///   to a session, that holds no holder row. This build records the holder
///   first, so its own writes never set it. The pipeline reads everything
///   again when it is set ([`UsageRepo::derived_dirty`]). The triggers are
///   dropped and made again at each open, so a change to them always
///   lands.
/// - `usage_detail` (0.3.7, the usage audit): each keyed call's facts
///   beyond its token classes ([`UsageDetail`]) and its list-price cost,
///   priced per call with the prices of `price_version`. Rows a build
///   without it wrote (or calls with no key) have none: they are priced
///   from their tokens at the standard rates, as before.
/// - `usage_pricing`: the prices every stored cost was last made with
///   ([`PricingCatalog::version`]); another means [`UsageRepo::reprice`]
///   prices them again.
/// - `quota_sample_sources`: where each `quota_samples` row came from (a
///   status line, a rollout, the tool's desktop app, its own cached
///   reading, a limit hit), keyed like the row. Kept beside the numbered
///   table, so no migration of its own; a row without one predates it.
pub(crate) const DERIVED: &str = "
CREATE TABLE IF NOT EXISTS quota_sample_sources (
    account TEXT NOT NULL,
    window TEXT NOT NULL,
    ts INTEGER NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (account, window, ts)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS usage_holders (
    account TEXT NOT NULL,
    msg_uuid TEXT NOT NULL,
    session TEXT NOT NULL,
    ts INTEGER NOT NULL,
    PRIMARY KEY (account, msg_uuid, session)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_usage_holders_session ON usage_holders(account, session, ts);
CREATE INDEX IF NOT EXISTS idx_usage_holders_key ON usage_holders(msg_uuid);
CREATE TABLE IF NOT EXISTS usage_spans (
    account TEXT NOT NULL,
    session TEXT NOT NULL,
    first INTEGER NOT NULL,
    last INTEGER NOT NULL,
    own_start INTEGER,
    PRIMARY KEY (account, session)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS usage_unshared (
    account TEXT PRIMARY KEY
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS usage_derived (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    dirty INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS usage_detail (
    account TEXT NOT NULL,
    msg_uuid TEXT NOT NULL,
    cache_w_1h INTEGER NOT NULL DEFAULT 0,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    speed TEXT,
    geo TEXT,
    kind TEXT NOT NULL DEFAULT 'main',
    root_session TEXT,
    usd REAL,
    price_version TEXT,
    PRIMARY KEY (account, msg_uuid)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS usage_pricing (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    version TEXT
);
INSERT OR IGNORE INTO usage_derived (id, dirty) VALUES (1, 0);
DROP TRIGGER IF EXISTS usage_events_unheld_insert;
CREATE TRIGGER usage_events_unheld_insert AFTER INSERT ON usage_events
WHEN NEW.msg_uuid IS NOT NULL AND NEW.session IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM usage_holders h
    WHERE h.account = NEW.account AND h.msg_uuid = NEW.msg_uuid AND h.session = NEW.session)
  AND NOT EXISTS (SELECT 1 FROM usage_unshared u WHERE u.account = NEW.account)
BEGIN
    UPDATE usage_derived SET dirty = 1 WHERE id = 1;
END;
DROP TRIGGER IF EXISTS usage_events_unheld_move;
CREATE TRIGGER usage_events_unheld_move AFTER UPDATE OF session ON usage_events
WHEN NEW.msg_uuid IS NOT NULL AND NEW.session IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM usage_holders h
    WHERE h.account = NEW.account AND h.msg_uuid = NEW.msg_uuid AND h.session = NEW.session)
  AND NOT EXISTS (SELECT 1 FROM usage_unshared u WHERE u.account = NEW.account)
BEGIN
    UPDATE usage_derived SET dirty = 1 WHERE id = 1;
END;
";

/// Stores `events`, then settles what removing sessions left (see
/// [`Removed`]).
#[allow(clippy::too_many_lines)] // one pass: each call inserted or raised, its holders and detail kept
fn store_in(
    conn: &Connection,
    events: &[UsageEvent],
    removed: Removed,
) -> Result<Vec<Stored>, CoreError> {
    let mut stored = Vec::with_capacity(events.len());
    let mut owners = Owners::new(conn)?;
    let mut rows = Rows::new(conn)?;
    let unshared: BTreeSet<String> = conn
        .prepare("SELECT account FROM usage_unshared")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    owners.recount.extend(removed.recheck);
    owners.keys.extend(removed.keys);
    let mut insert = conn.prepare(
        "INSERT OR IGNORE INTO usage_events
             (account, session, ts, model, input, output, cache_r, cache_w,
              reasoning, credits, usd_est, source, confidence, msg_uuid)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )?;
    let mut raise = conn.prepare(
        "UPDATE usage_events
         SET input = ?3, output = ?4, cache_r = ?5, cache_w = ?6, reasoning = ?7,
             model = COALESCE(model, ?8)
         WHERE account = ?1 AND msg_uuid = ?2",
    )?;
    let mut details = Details::new(conn)?;
    for event in events {
        let shared = !unshared.contains(&event.account);
        // 0.3.8: a key another account of the tool holds is the same call
        // (a copy of the conversation made outside AI Juggler), so its row
        // is found there. A shared account's holders say where; the row is
        // settled onto one keeper below. An unshared account's keys name
        // their own thread, so a copy repeats the thread too: the account
        // that stored the call first keeps it.
        let known = match &event.msg_uuid {
            Some(key) => {
                let accounts = if shared {
                    owners.holding_accounts(key)?
                } else {
                    unshared.clone()
                };
                rows.find_among(key, event.source.as_str(), &event.account, &accounts)?
            }
            None => None,
        };
        if let (Some(key), Some(session)) = (&event.msg_uuid, &event.session)
            && shared
        {
            owners.offer(&event.account, key, session, event.ts)?;
        }
        let Some((row_account, was)) = known else {
            insert.execute(params![
                event.account,
                event.session,
                event.ts,
                event.model,
                as_sql(event.tokens.input),
                as_sql(event.tokens.output),
                as_sql(event.tokens.cache_read),
                as_sql(event.tokens.cache_write),
                as_sql(event.tokens.reasoning),
                event.credits,
                event.usd_est,
                event.source.as_str(),
                confidence_name(event.confidence),
                event.msg_uuid,
            ])?;
            details.keep(&event.account, event, event.tokens, event.model.as_deref())?;
            stored.push(Stored {
                new: true,
                added: event.tokens,
            });
            continue;
        };
        let now = TokenCounts {
            input: was.input.max(event.tokens.input),
            output: was.output.max(event.tokens.output),
            cache_read: was.cache_read.max(event.tokens.cache_read),
            cache_write: was.cache_write.max(event.tokens.cache_write),
            reasoning: was.reasoning.max(event.tokens.reasoning),
        };
        if now != was {
            raise.execute(params![
                row_account,
                event.msg_uuid,
                as_sql(now.input),
                as_sql(now.output),
                as_sql(now.cache_read),
                as_sql(now.cache_write),
                as_sql(now.reasoning),
                event.model,
            ])?;
        }
        details.keep(&row_account, event, now, event.model.as_deref())?;
        stored.push(Stored {
            new: false,
            added: TokenCounts {
                input: now.input - was.input,
                output: now.output - was.output,
                cache_read: now.cache_read - was.cache_read,
                cache_write: now.cache_write - was.cache_write,
                reasoning: now.reasoning - was.reasoning,
            },
        });
    }
    owners.finish()?;
    Ok(stored)
}

/// What removing sessions leaves to settle: sessions that shared messages
/// with them (their standing is counted again), and the shared messages
/// they held (kept, and given to the best holder left).
#[derive(Default)]
struct Removed {
    recheck: Vec<SessionKey>,
    keys: Vec<String>,
}

/// Deletes what the sessions offered, and their rows that no other session
/// holds; returns how many rows went. A row another session also holds
/// stays (an account's total never drops because one holder's file was
/// read again without it) and is settled again.
fn delete_in(
    conn: &Connection,
    sessions: &[(String, UsageSource, String)],
) -> Result<(usize, Removed), CoreError> {
    let mut shared = conn.prepare(
        "SELECT h.msg_uuid, o.account, o.session FROM usage_holders h JOIN usage_holders o
           ON o.msg_uuid = h.msg_uuid AND (o.account <> h.account OR o.session <> h.session)
         WHERE h.account = ?1 AND h.session = ?2",
    )?;
    let mut forget =
        conn.prepare("DELETE FROM usage_holders WHERE account = ?1 AND session = ?2")?;
    let mut forget_span =
        conn.prepare("DELETE FROM usage_spans WHERE account = ?1 AND session = ?2")?;
    let mut delete = conn.prepare(
        "DELETE FROM usage_events
         WHERE account = ?1 AND source = ?2 AND session = ?3 AND (msg_uuid IS NULL OR NOT EXISTS (
             SELECT 1 FROM usage_holders o WHERE o.msg_uuid = usage_events.msg_uuid))",
    )?;
    // The same rows' details, first (they go with their rows).
    let mut delete_details = conn.prepare(
        "DELETE FROM usage_detail
         WHERE account = ?1 AND msg_uuid IN (
             SELECT e.msg_uuid FROM usage_events e
             WHERE e.account = ?1 AND e.source = ?2 AND e.session = ?3 AND e.msg_uuid IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM usage_holders o WHERE o.msg_uuid = e.msg_uuid))",
    )?;
    let mut deleted = 0;
    let mut recheck = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for (account, source, session) in sessions {
        let pairs = shared
            .query_map(params![account, session], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (row.get::<_, String>(1)?, row.get::<_, String>(2)?),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (key, other) in pairs {
            keys.insert(key);
            recheck.insert(other);
        }
        forget.execute(params![account, session])?;
        forget_span.execute(params![account, session])?;
        delete_details.execute(params![account, source.as_str(), session])?;
        deleted += delete.execute(params![account, source.as_str(), session])?;
    }
    for (account, _, session) in sessions {
        recheck.remove(&(account.clone(), session.clone()));
    }
    Ok((
        deleted,
        Removed {
            recheck: recheck.into_iter().collect(),
            keys: keys.into_iter().collect(),
        },
    ))
}

fn confidence_name(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Measured => "measured",
        Confidence::Computed => "computed",
        Confidence::Estimated => "estimated",
    }
}

/// Parses a stored confidence name; unknown values degrade to the
/// honest default (Estimated — never overstate trust).
fn parse_confidence(value: &str) -> Confidence {
    match value {
        "measured" => Confidence::Measured,
        "computed" => Confidence::Computed,
        _ => Confidence::Estimated,
    }
}

/// The settings key holding one account's user-confirmed cap.
#[must_use]
pub fn cap_settings_key(account: &str, kind: WindowKind) -> String {
    format!("quota_cap:{account}:{}", kind.as_str())
}

/// The settings key holding one account's weekly phase anchor (`ZCode`
/// purchase instant), when the user confirmed one.
#[must_use]
pub fn anchor_settings_key(account: &str) -> String {
    format!("quota_anchor:{account}")
}

// Window-name constants are the storage vocabulary; keep them pinned
// to WindowKind::as_str at the type level via tests.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_names_match_kind_vocabulary() {
        // The quota_samples.window vocabulary IS WindowKind::as_str.
        assert_eq!("five_hour", WindowKind::FiveHour.as_str());
        assert_eq!("weekly", WindowKind::Weekly { anchor_ms: None }.as_str());
    }

    #[test]
    fn settings_keys_are_stable_and_scoped() {
        assert_eq!(
            cap_settings_key("acc-1", WindowKind::FiveHour),
            "quota_cap:acc-1:five_hour"
        );
        assert_eq!(anchor_settings_key("acc-1"), "quota_anchor:acc-1");
    }

    #[test]
    fn a_sample_keeps_where_it_came_from_and_reading_it_again_changes_nothing() {
        let storage = crate::storage::Storage::open_in_memory().unwrap();
        let usage = storage.usage();
        let sample = |ts: i64| QuotaSample {
            ts,
            used_pct: 40.0,
            resets_at: None,
            confidence: Confidence::Measured,
        };
        // One from before sources were kept.
        usage
            .insert_sample("acc", WindowKind::FiveHour, &sample(1))
            .unwrap();
        let batch = vec![
            (
                "acc".to_owned(),
                WindowKind::FiveHour,
                sample(2),
                UsageSource::DesktopApp,
            ),
            (
                "acc".to_owned(),
                WindowKind::Weekly { anchor_ms: None },
                sample(2),
                UsageSource::UsageCache,
            ),
        ];
        usage.insert_sourced_samples(&batch).unwrap();
        usage.insert_sourced_samples(&batch).unwrap();
        let five = usage
            .sourced_samples_for_account("acc", WindowKind::FiveHour)
            .unwrap();
        assert_eq!(
            five.iter().map(|(s, src)| (s.ts, *src)).collect::<Vec<_>>(),
            vec![(1, None), (2, Some(UsageSource::DesktopApp))]
        );
        let week = usage
            .sourced_samples_for_account("acc", WindowKind::Weekly { anchor_ms: None })
            .unwrap();
        assert_eq!(week.len(), 1);
        assert_eq!(week[0].1, Some(UsageSource::UsageCache));
        // The plain read sees the same rows.
        assert_eq!(
            usage
                .samples_for_account("acc", WindowKind::FiveHour)
                .unwrap()
                .len(),
            2
        );
    }

    /// rc6 walk: the reading takes the span it needs and the newest
    /// sample before it, never every sample ever stored.
    #[test]
    fn samples_since_take_the_span_and_the_newest_one_before() {
        let storage = crate::storage::Storage::open_in_memory().unwrap();
        let usage = storage.usage();
        for ts in [10, 20, 30, 40, 50] {
            usage
                .insert_sample(
                    "acc",
                    WindowKind::FiveHour,
                    &QuotaSample {
                        ts,
                        used_pct: ts as f64,
                        resets_at: None,
                        confidence: Confidence::Measured,
                    },
                )
                .unwrap();
        }
        let since = |at: i64| -> Vec<i64> {
            usage
                .sourced_samples_since("acc", WindowKind::FiveHour, at)
                .unwrap()
                .iter()
                .map(|(sample, _)| sample.ts)
                .collect()
        };
        assert_eq!(since(35), [30, 40, 50], "the span, and the one before it");
        assert_eq!(
            since(30),
            [20, 30, 40, 50],
            "the newest before the span's start, always"
        );
        assert_eq!(
            since(5),
            [10, 20, 30, 40, 50],
            "nothing before: from the first"
        );
        assert_eq!(since(60), [50], "nothing in the span: the newest alone");
        assert_eq!(
            usage
                .sourced_samples_since("other", WindowKind::FiveHour, 35)
                .unwrap()
                .len(),
            0
        );
    }

    /// A model's own week is a window of its own: its readings never mix
    /// with the all-models week's, and each model keeps its own.
    #[test]
    fn a_models_own_week_is_kept_apart_from_the_all_models_week() {
        let storage = crate::storage::Storage::open_in_memory().unwrap();
        let usage = storage.usage();
        let sample = |ts: i64, used_pct: f64| QuotaSample {
            ts,
            used_pct,
            resets_at: Some(ts + 1_000),
            confidence: Confidence::Measured,
        };
        let rows = vec![
            (
                "acc".to_owned(),
                "Fable".to_owned(),
                sample(1, 10.0),
                UsageSource::UsageCache,
            ),
            (
                "acc".to_owned(),
                "Fable".to_owned(),
                sample(2, 12.0),
                UsageSource::UsageCache,
            ),
            (
                "acc".to_owned(),
                "Opus".to_owned(),
                sample(2, 3.0),
                UsageSource::UsageCache,
            ),
            (
                "other".to_owned(),
                "Fable".to_owned(),
                sample(2, 90.0),
                UsageSource::UsageCache,
            ),
        ];
        usage.insert_model_week_samples(&rows).unwrap();
        usage.insert_model_week_samples(&rows).unwrap();
        let weeks = usage.model_week_samples_for_account("acc").unwrap();
        let shape: Vec<(&str, Vec<(i64, f64)>)> = weeks
            .iter()
            .map(|(model, samples)| {
                (
                    model.as_str(),
                    samples.iter().map(|(s, _)| (s.ts, s.used_pct)).collect(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                ("Fable", vec![(1, 10.0), (2, 12.0)]),
                ("Opus", vec![(2, 3.0)])
            ]
        );
        assert_eq!(weeks[0].1[0].1, Some(UsageSource::UsageCache));
        assert!(
            usage
                .samples_for_account("acc", WindowKind::Weekly { anchor_ms: None })
                .unwrap()
                .is_empty(),
            "never read as the all-models week"
        );
    }
}
