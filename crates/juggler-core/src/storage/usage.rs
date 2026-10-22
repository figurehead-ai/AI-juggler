//! Usage event & quota sample store (AJ-E606, deep/09 §Q8 schema):
//! append-only `usage_events` + `quota_samples` (+ the §Q8
//! `reset_queue` for AJ-E611), with `UNIQUE(account, msg_uuid)`
//! dedupe so resumed transcript files can never double-count. The
//! engine's maths (§Q3) lives in [`crate::quota`] as pure functions;
//! THIS module only persists and fetches.

use std::collections::{BTreeSet, HashMap, HashSet};

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
        let stored = store_in(&tx, events, Removed::default())?;
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
        let (deleted, removed) = delete_in(&tx, sessions)?;
        store_in(&tx, &[], removed)?;
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

/// Tables derived from the transcripts and rollouts, kept outside the
/// numbered schema: reading the files again (the pipeline's re-read)
/// rebuilds them, so they need no migration of their own.
/// - `usage_holders`: every session that offered each keyed message,
///   whichever one keeps it.
/// - `usage_spans`: each session's standing under the shared-message rule
///   ([`Span`]).
pub(crate) const DERIVED: &str = "
CREATE TABLE IF NOT EXISTS usage_holders (
    account TEXT NOT NULL,
    msg_uuid TEXT NOT NULL,
    session TEXT NOT NULL,
    ts INTEGER NOT NULL,
    PRIMARY KEY (account, msg_uuid, session)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_usage_holders_session ON usage_holders(account, session, ts);
CREATE TABLE IF NOT EXISTS usage_spans (
    account TEXT NOT NULL,
    session TEXT NOT NULL,
    first INTEGER NOT NULL,
    last INTEGER NOT NULL,
    own_start INTEGER,
    PRIMARY KEY (account, session)
) WITHOUT ROWID;
";

/// One session's standing for the messages it shares, from every message
/// it offered (kept or not).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Span {
    /// Its first message.
    first: i64,
    /// Its last message.
    last: i64,
    /// Its first message no other session holds; `None` when it has none.
    own_start: Option<i64>,
}

impl Span {
    /// Who keeps a message two sessions share (a resumed copy or a fork
    /// repeats its original's history, with the original times): the
    /// smallest of these.
    /// 1. The earliest first message.
    /// 2. The session whose own messages start latest, none counting as
    ///    latest: a copy starts its own right after it was made, while the
    ///    original has none then, or starts them later.
    /// 3. Between sessions with none of their own, the earlier last
    ///    message (in a chain of copies, the shorter one is older).
    /// 4. The smaller id.
    ///
    /// None of these moves as a session grows past its own start, so a
    /// message's keeper doesn't either.
    ///
    /// Why 2 holds: Claude's resume writes a copy holding everything up to
    /// the point it was resumed, so the original has messages of its own
    /// only after the fork. That puts the shared part on the original in a
    /// chain of copies, in a fork, and when the original never writes again.
    ///
    /// Known limit: when the original writes first after a fork and the
    /// copy writes later, the copy's own start is the later one, and it is
    /// credited the shared part. Only the split between the two sessions is
    /// affected; an account's totals never are (each message is counted
    /// once, whoever keeps it).
    fn rank<'s>(&self, session: &'s str) -> (i64, std::cmp::Reverse<i64>, i64, &'s str) {
        (
            self.first,
            std::cmp::Reverse(self.own_start.unwrap_or(i64::MAX)),
            if self.own_start.is_none() {
                self.last
            } else {
                0
            },
            session,
        )
    }

    /// Whether a change from `before` can change who keeps a message.
    fn moved_from(&self, before: Option<Self>) -> bool {
        before.is_none_or(|was| {
            was.first != self.first
                || was.own_start != self.own_start
                || (self.own_start.is_none() && was.last != self.last)
        })
    }
}

type SessionKey = (String, String);

/// Settles who keeps each shared message (see [`Span::rank`]). Every
/// session that offers a message is recorded, kept or not; each pass
/// updates the standing of the sessions it touched and settles every
/// shared message whose keeper could have changed. The answer depends
/// only on what has been offered, never on the order or the passes.
struct Owners<'c> {
    hold: rusqlite::Statement<'c>,
    holders_of: rusqlite::Statement<'c>,
    load_span: rusqlite::Statement<'c>,
    save_span: rusqlite::Statement<'c>,
    count_span: rusqlite::Statement<'c>,
    shared_keys: rusqlite::Statement<'c>,
    move_to: rusqlite::Statement<'c>,
    /// Standing as stored when the pass first touched each session.
    before: HashMap<SessionKey, Option<Span>>,
    /// Standing now.
    spans: HashMap<SessionKey, Span>,
    /// Sessions whose own start must be counted again in full (one of
    /// their own messages became shared, or a session they shared with
    /// was removed).
    recount: HashSet<SessionKey>,
    /// Shared messages to settle: (account, key).
    keys: BTreeSet<(String, String)>,
}

impl<'c> Owners<'c> {
    fn new(conn: &'c Connection) -> Result<Self, CoreError> {
        Ok(Self {
            hold: conn.prepare(
                "INSERT OR IGNORE INTO usage_holders (account, msg_uuid, session, ts)
                 VALUES (?1, ?2, ?3, ?4)",
            )?,
            holders_of: conn.prepare(
                "SELECT session, ts FROM usage_holders WHERE account = ?1 AND msg_uuid = ?2",
            )?,
            load_span: conn.prepare(
                "SELECT first, last, own_start FROM usage_spans WHERE account = ?1 AND session = ?2",
            )?,
            save_span: conn.prepare(
                "INSERT OR REPLACE INTO usage_spans (account, session, first, last, own_start)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?,
            count_span: conn.prepare(
                "SELECT MIN(h.ts), MAX(h.ts), MIN(CASE WHEN NOT EXISTS (
                     SELECT 1 FROM usage_holders o
                     WHERE o.account = h.account AND o.msg_uuid = h.msg_uuid
                       AND o.session <> h.session) THEN h.ts END)
                 FROM usage_holders h WHERE h.account = ?1 AND h.session = ?2",
            )?,
            shared_keys: conn.prepare(
                "SELECT h.msg_uuid FROM usage_holders h
                 WHERE h.account = ?1 AND h.session = ?2 AND EXISTS (
                     SELECT 1 FROM usage_holders o
                     WHERE o.account = h.account AND o.msg_uuid = h.msg_uuid
                       AND o.session <> h.session)",
            )?,
            move_to: conn.prepare(
                "UPDATE usage_events SET session = ?3
                 WHERE account = ?1 AND msg_uuid = ?2 AND session IS NOT ?3",
            )?,
            before: HashMap::new(),
            spans: HashMap::new(),
            recount: HashSet::new(),
            keys: BTreeSet::new(),
        })
    }

    /// The stored standing, or one counted from the holders (a session
    /// recorded before standings were kept).
    fn stored(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        if let Some(span) = self
            .load_span
            .query_row(params![who.0, who.1], |row| {
                Ok(Span {
                    first: row.get(0)?,
                    last: row.get(1)?,
                    own_start: row.get(2)?,
                })
            })
            .optional()?
        {
            return Ok(Some(span));
        }
        self.counted(who)
    }

    /// The standing counted in full from the holders.
    fn counted(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        let (first, last, own_start): (Option<i64>, Option<i64>, Option<i64>) =
            self.count_span.query_row(params![who.0, who.1], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;
        Ok(first.zip(last).map(|(first, last)| Span {
            first,
            last,
            own_start,
        }))
    }

    /// The standing as this pass sees it (touching the session).
    fn span(&mut self, who: &SessionKey) -> Result<Option<Span>, CoreError> {
        if let Some(span) = self.spans.get(who) {
            return Ok(Some(*span));
        }
        let stored = self.stored(who)?;
        self.before.insert(who.clone(), stored);
        if let Some(span) = stored {
            self.spans.insert(who.clone(), span);
        }
        Ok(stored)
    }

    fn holders(&mut self, account: &str, key: &str) -> Result<Vec<(String, i64)>, CoreError> {
        let rows = self
            .holders_of
            .query_map(params![account, key], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// `session` offered the message `key` at `ts`.
    fn offer(&mut self, account: &str, key: &str, session: &str, ts: i64) -> Result<(), CoreError> {
        if self.hold.execute(params![account, key, session, ts])? == 0 {
            return Ok(()); // it had offered it before
        }
        let who = (account.to_owned(), session.to_owned());
        let holders = self.holders(account, key)?;
        let own = holders.len() == 1;
        let was = self.span(&who)?;
        let span = was.map_or(
            Span {
                first: ts,
                last: ts,
                own_start: own.then_some(ts),
            },
            |was| Span {
                first: was.first.min(ts),
                last: was.last.max(ts),
                own_start: if own {
                    Some(was.own_start.map_or(ts, |start| start.min(ts)))
                } else {
                    was.own_start
                },
            },
        );
        self.spans.insert(who, span);
        if own {
            return Ok(());
        }
        // A message that was another session's own is now shared: its
        // own start may have been this one.
        if holders.len() == 2
            && let Some((other, at)) = holders.iter().find(|(holder, _)| holder != session)
        {
            let other = (account.to_owned(), other.clone());
            if self.span(&other)?.and_then(|span| span.own_start) == Some(*at) {
                self.recount.insert(other);
            }
        }
        self.keys.insert((account.to_owned(), key.to_owned()));
        Ok(())
    }

    /// Settles every shared message whose keeper could have changed, and
    /// stores the new standings.
    fn finish(mut self) -> Result<(), CoreError> {
        for who in std::mem::take(&mut self.recount) {
            self.span(&who)?;
            match self.counted(&who)? {
                Some(span) => {
                    self.spans.insert(who, span);
                }
                None => {
                    self.spans.remove(&who);
                }
            }
        }
        let touched: Vec<(SessionKey, Span)> = self
            .spans
            .iter()
            .map(|(who, span)| (who.clone(), *span))
            .collect();
        for (who, span) in touched {
            let before = self.before.get(&who).copied().flatten();
            if before != Some(span) {
                self.save_span.execute(params![
                    who.0,
                    who.1,
                    span.first,
                    span.last,
                    span.own_start
                ])?;
            }
            if span.moved_from(before) {
                let keys = self
                    .shared_keys
                    .query_map(params![who.0, who.1], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                self.keys
                    .extend(keys.into_iter().map(|key| (who.0.clone(), key)));
            }
        }
        for (account, key) in std::mem::take(&mut self.keys) {
            let mut best: Option<(Span, String)> = None;
            for (holder, _) in self.holders(&account, &key)? {
                let Some(span) = self.span(&(account.clone(), holder.clone()))? else {
                    continue;
                };
                if best
                    .as_ref()
                    .is_none_or(|(kept, by)| span.rank(&holder) < kept.rank(by))
                {
                    best = Some((span, holder));
                }
            }
            if let Some((_, keeper)) = best {
                self.move_to.execute(params![account, key, keeper])?;
            }
        }
        Ok(())
    }
}

/// Stores `events`, then settles what removing sessions left (see
/// [`Removed`]).
fn store_in(
    conn: &Connection,
    events: &[UsageEvent],
    removed: Removed,
) -> Result<Vec<Stored>, CoreError> {
    let mut stored = Vec::with_capacity(events.len());
    let mut find = conn.prepare(
        "SELECT input, output, cache_r, cache_w, reasoning FROM usage_events
         WHERE account = ?1 AND msg_uuid = ?2",
    )?;
    let mut owners = Owners::new(conn)?;
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
    for event in events {
        let known = match &event.msg_uuid {
            Some(key) => find
                .query_row(params![event.account, key], |row| {
                    Ok(TokenCounts {
                        input: from_sql(row.get(0)?),
                        output: from_sql(row.get(1)?),
                        cache_read: from_sql(row.get(2)?),
                        cache_write: from_sql(row.get(3)?),
                        reasoning: from_sql(row.get(4)?),
                    })
                })
                .optional()?,
            None => None,
        };
        if let (Some(key), Some(session)) = (&event.msg_uuid, &event.session) {
            owners.offer(&event.account, key, session, event.ts)?;
        }
        let Some(was) = known else {
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
    owners.finish()?;
    Ok(stored)
}

/// What removing sessions leaves to settle: sessions that shared messages
/// with them (their standing is counted again), and the shared messages
/// they held (kept, and given to the best holder left).
#[derive(Default)]
struct Removed {
    recheck: Vec<SessionKey>,
    keys: Vec<(String, String)>,
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
        "SELECT h.msg_uuid, o.session FROM usage_holders h JOIN usage_holders o
           ON o.account = h.account AND o.msg_uuid = h.msg_uuid AND o.session <> h.session
         WHERE h.account = ?1 AND h.session = ?2",
    )?;
    let mut forget =
        conn.prepare("DELETE FROM usage_holders WHERE account = ?1 AND session = ?2")?;
    let mut forget_span =
        conn.prepare("DELETE FROM usage_spans WHERE account = ?1 AND session = ?2")?;
    let mut delete = conn.prepare(
        "DELETE FROM usage_events
         WHERE account = ?1 AND source = ?2 AND session = ?3 AND (msg_uuid IS NULL OR NOT EXISTS (
             SELECT 1 FROM usage_holders o
             WHERE o.account = usage_events.account AND o.msg_uuid = usage_events.msg_uuid))",
    )?;
    let mut deleted = 0;
    let mut recheck = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for (account, source, session) in sessions {
        let pairs = shared
            .query_map(params![account, session], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (key, other) in pairs {
            keys.insert((account.clone(), key));
            recheck.insert((account.clone(), other));
        }
        forget.execute(params![account, session])?;
        forget_span.execute(params![account, session])?;
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
}
