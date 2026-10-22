//! Usage event & quota sample store (AJ-E606, deep/09 §Q8 schema):
//! append-only `usage_events` + `quota_samples` (+ the §Q8
//! `reset_queue` for AJ-E611), with `UNIQUE(account, msg_uuid)`
//! dedupe so resumed transcript files can never double-count. The
//! engine's maths (§Q3) lives in [`crate::quota`] as pure functions;
//! THIS module only persists and fetches.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::CoreError;
use crate::model::{Confidence, TokenCounts};
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
        let tx = self.conn.unchecked_transaction()?;
        let stored = store_in(&tx, events)?;
        tx.commit()?;
        Ok(stored)
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
        let tx = self.conn.unchecked_transaction()?;
        delete_in(&tx, sessions)?;
        let stored = store_in(&tx, events)?;
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
        let tx = self.conn.unchecked_transaction()?;
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
        let tx = self.conn.unchecked_transaction()?;
        let deleted = delete_in(&tx, sessions)?;
        tx.commit()?;
        Ok(deleted)
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

/// Who keeps a message two sessions share (a resumed copy repeats its
/// original's history): the session whose first message came first. A copy
/// keeps the original lines' times, so first messages tie; the copy then
/// goes on past the original's end, so the earlier last message wins, then
/// the smaller id. Each session's span counts the store and the whole
/// incoming batch, so the order files arrive in never changes the answer.
struct Shares<'c> {
    span_stored: rusqlite::Statement<'c>,
    move_to: rusqlite::Statement<'c>,
    span_in_batch: std::collections::HashMap<(String, String), (i64, i64)>,
}

impl<'c> Shares<'c> {
    fn new(conn: &'c Connection, events: &[UsageEvent]) -> Result<Self, CoreError> {
        let mut span_in_batch: std::collections::HashMap<(String, String), (i64, i64)> =
            std::collections::HashMap::new();
        for event in events {
            if let Some(session) = &event.session {
                span_in_batch
                    .entry((event.account.clone(), session.clone()))
                    .and_modify(|(first, last)| {
                        *first = (*first).min(event.ts);
                        *last = (*last).max(event.ts);
                    })
                    .or_insert((event.ts, event.ts));
            }
        }
        Ok(Self {
            span_stored: conn.prepare(
                "SELECT MIN(ts), MAX(ts) FROM usage_events WHERE account = ?1 AND session = ?2",
            )?,
            move_to: conn.prepare(
                "UPDATE usage_events SET session = ?3 WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            span_in_batch,
        })
    }

    /// A session's first and last message, stored or arriving.
    fn span(&mut self, account: &str, session: &str) -> Result<(i64, i64), CoreError> {
        let (min, max): (Option<i64>, Option<i64>) = self
            .span_stored
            .query_row(params![account, session], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        let batch = self
            .span_in_batch
            .get(&(account.to_owned(), session.to_owned()))
            .copied();
        let first = min.into_iter().chain(batch.map(|span| span.0)).min();
        let last = max.into_iter().chain(batch.map(|span| span.1)).max();
        Ok((first.unwrap_or(i64::MAX), last.unwrap_or(i64::MAX)))
    }

    /// Moves the message to `offered` when it, not `held`, came first.
    fn settle(
        &mut self,
        account: &str,
        key: Option<&str>,
        held: &str,
        offered: &str,
    ) -> Result<(), CoreError> {
        let (offered_first, offered_last) = self.span(account, offered)?;
        let (held_first, held_last) = self.span(account, held)?;
        if (offered_first, offered_last, offered) < (held_first, held_last, held) {
            self.move_to.execute(params![account, key, offered])?;
        }
        Ok(())
    }
}

fn store_in(conn: &Connection, events: &[UsageEvent]) -> Result<Vec<Stored>, CoreError> {
    let mut stored = Vec::with_capacity(events.len());
    let mut find = conn.prepare(
        "SELECT input, output, cache_r, cache_w, reasoning, session FROM usage_events
         WHERE account = ?1 AND msg_uuid = ?2",
    )?;
    let mut shares = Shares::new(conn, events)?;
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
    for event in events {
        let known = match &event.msg_uuid {
            Some(key) => find
                .query_row(params![event.account, key], |row| {
                    Ok((
                        TokenCounts {
                            input: from_sql(row.get(0)?),
                            output: from_sql(row.get(1)?),
                            cache_read: from_sql(row.get(2)?),
                            cache_write: from_sql(row.get(3)?),
                            reasoning: from_sql(row.get(4)?),
                        },
                        row.get::<_, Option<String>>(5)?,
                    ))
                })
                .optional()?,
            None => None,
        };
        let Some((was, held_by)) = known else {
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
        if let (Some(held), Some(offered)) = (&held_by, &event.session)
            && held != offered
        {
            shares.settle(&event.account, event.msg_uuid.as_deref(), held, offered)?;
        }
        if now != was {
            raise.execute(params![
                event.account,
                event.msg_uuid,
                as_sql(now.input),
                as_sql(now.output),
                as_sql(now.cache_read),
                as_sql(now.cache_write),
                as_sql(now.reasoning),
                event.model,
            ])?;
        }
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
    Ok(stored)
}

fn delete_in(
    conn: &Connection,
    sessions: &[(String, UsageSource, String)],
) -> Result<usize, CoreError> {
    let mut stmt = conn
        .prepare("DELETE FROM usage_events WHERE account = ?1 AND source = ?2 AND session = ?3")?;
    let mut deleted = 0;
    for (account, source, session) in sessions {
        deleted += stmt.execute(params![account, source.as_str(), session])?;
    }
    Ok(deleted)
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
}
