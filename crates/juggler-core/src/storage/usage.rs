//! Usage event & quota sample store (AJ-E606, deep/09 §Q8 schema):
//! append-only `usage_events` + `quota_samples` (+ the §Q8
//! `reset_queue` for AJ-E611), with `UNIQUE(account, msg_uuid)`
//! dedupe so resumed transcript files can never double-count. The
//! engine's maths (§Q3) lives in [`crate::quota`] as pure functions;
//! THIS module only persists and fetches.

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::model::Confidence;
use crate::quota::{QuotaSample, UsageEvent, UsageSource, WindowKind};

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

    /// Appends events in ONE transaction; each row is
    /// `INSERT OR IGNORE` against `UNIQUE(account, msg_uuid)` — a
    /// re-ingested file dedupes to zero. Returns how many rows were
    /// NEWLY inserted (the ingest counters' honest denominator).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; the
    /// transaction rolls back atomically.
    pub fn insert_events(&self, events: &[UsageEvent]) -> Result<usize, CoreError> {
        let mut inserted = 0usize;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO usage_events
                     (account, session, ts, model, input, output, cache_r, cache_w,
                      reasoning, credits, usd_est, source, confidence, msg_uuid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            for event in events {
                let source = event.source.as_str();
                let confidence = confidence_name(event.confidence);
                inserted += stmt
                    .execute(params![
                        event.account,
                        event.session,
                        event.ts,
                        event.model,
                        i64::try_from(event.tokens.input).unwrap_or(i64::MAX),
                        i64::try_from(event.tokens.output).unwrap_or(i64::MAX),
                        i64::try_from(event.tokens.cache_read).unwrap_or(i64::MAX),
                        i64::try_from(event.tokens.cache_write).unwrap_or(i64::MAX),
                        i64::try_from(event.tokens.reasoning).unwrap_or(i64::MAX),
                        event.credits,
                        event.usd_est,
                        source,
                        confidence,
                        event.msg_uuid,
                    ])
                    .map(|changed| usize::from(changed > 0))?;
            }
        }
        tx.commit()?;
        Ok(inserted)
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
        let mut stmt = self.conn.prepare(
            "SELECT account, session, ts, model, input, output, cache_r, cache_w,
                    reasoning, credits, usd_est, source, confidence, msg_uuid
             FROM usage_events
             WHERE account = ?1 AND ts >= ?2
             ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![account, since_ms], |row| {
            Ok(UsageEvent {
                account: row.get(0)?,
                session: row.get(1)?,
                ts: row.get(2)?,
                model: row.get(3)?,
                tokens: crate::model::TokenCounts {
                    input: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                    output: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
                    cache_read: u64::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                    cache_write: u64::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                    reasoning: u64::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                },
                credits: row.get(9)?,
                usd_est: row.get(10)?,
                source: UsageSource::parse(&row.get::<_, String>(11)?)
                    .unwrap_or(UsageSource::Transcript),
                confidence: parse_confidence(&row.get::<_, String>(12)?),
                msg_uuid: row.get(13)?,
            })
        })?;
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
        self.conn
            .execute(
                "INSERT OR REPLACE INTO quota_samples
                     (account, window, ts, used_pct, resets_at, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    account,
                    kind.as_str(),
                    sample.ts,
                    sample.used_pct,
                    sample.resets_at,
                    confidence_name(sample.confidence),
                ],
            )
            .map(|_| ())
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
        let mut stmt = self.conn.prepare(
            "SELECT account, session, ts, model, input, output, cache_r, cache_w,
                    reasoning, credits, usd_est, source, confidence, msg_uuid
             FROM usage_events
             WHERE account = ?1 AND ts > ?2 AND ts <= ?3
             ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![account, lo_ms, hi_ms], |row| {
            Ok(UsageEvent {
                account: row.get(0)?,
                session: row.get(1)?,
                ts: row.get(2)?,
                model: row.get(3)?,
                tokens: crate::model::TokenCounts {
                    input: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                    output: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
                    cache_read: u64::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                    cache_write: u64::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                    reasoning: u64::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                },
                credits: row.get(9)?,
                usd_est: row.get(10)?,
                source: UsageSource::parse(&row.get::<_, String>(11)?)
                    .unwrap_or(UsageSource::Transcript),
                confidence: parse_confidence(&row.get::<_, String>(12)?),
                msg_uuid: row.get(13)?,
            })
        })?;
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

/// Stable confidence names for storage.
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
}
