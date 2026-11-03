//! The settled hours, kept in the store (0.3.7): each profile's hourly
//! sums older than [`super::SETTLED_MS`], so a summary after a start reads a few
//! thousand rows instead of summing every message again, and the hours an
//! older message changed, so only those are summed again.
//!
//! Derived and outside the numbered schema, like the store's other derived
//! tables: made at each open, rebuilt from `usage_events` whenever its
//! state is missing. The triggers live in the file, so a write by any build
//! marks its hour; they're named apart from the settled-stamp triggers, so
//! a build that remakes those leaves these in place.

use rusqlite::{OptionalExtension, params};

use super::{GroupCost, HOUR_MS, HourRow, UsageHistoryRepo, count, tokens_at};
use crate::error::CoreError;
use crate::quota::CallKind;

/// Tables and triggers (see the module).
/// - `usage_hour_rollup`: a profile's settled [`HourRow`]s.
/// - `usage_rollup_state`: the hour they reach (exclusive), and whether
///   the oldest were left out past the row cap.
/// - `usage_rollup_dirty`: the hours whose messages changed since they
///   were summed: an older message written, changed or removed, or its
///   cost or conversation (`usage_detail`) changed. A change to a detail
///   also moves the settled stamp, as a change to its message does.
pub(crate) const ROLLUP_DERIVED: &str = "
CREATE TABLE IF NOT EXISTS usage_hour_rollup (
    account TEXT NOT NULL,
    hour INTEGER NOT NULL,
    session TEXT,
    root TEXT,
    kind TEXT NOT NULL,
    model TEXT,
    input INTEGER NOT NULL,
    output INTEGER NOT NULL,
    cache_r INTEGER NOT NULL,
    cache_w INTEGER NOT NULL,
    reasoning INTEGER NOT NULL,
    messages INTEGER NOT NULL,
    stored_usd REAL NOT NULL,
    un_input INTEGER NOT NULL,
    un_output INTEGER NOT NULL,
    un_cache_r INTEGER NOT NULL,
    un_cache_w INTEGER NOT NULL,
    un_reasoning INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_usage_hour_rollup ON usage_hour_rollup(account, hour);
CREATE TABLE IF NOT EXISTS usage_rollup_state (
    account TEXT PRIMARY KEY,
    cutoff INTEGER NOT NULL,
    cut INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS usage_rollup_dirty (
    account TEXT NOT NULL,
    hour INTEGER NOT NULL,
    PRIMARY KEY (account, hour)
) WITHOUT ROWID;
DROP TRIGGER IF EXISTS usage_rollup_event_insert;
CREATE TRIGGER usage_rollup_event_insert AFTER INSERT ON usage_events
WHEN NEW.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty VALUES (NEW.account, NEW.ts / 3600000);
END;
DROP TRIGGER IF EXISTS usage_rollup_event_update;
CREATE TRIGGER usage_rollup_event_update AFTER UPDATE ON usage_events
WHEN (OLD.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
       OR NEW.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000)
  AND (OLD.account, OLD.session, OLD.ts, OLD.model, OLD.input, OLD.output, OLD.cache_r, OLD.cache_w, OLD.reasoning)
      IS NOT (NEW.account, NEW.session, NEW.ts, NEW.model, NEW.input, NEW.output, NEW.cache_r, NEW.cache_w, NEW.reasoning)
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty VALUES (OLD.account, OLD.ts / 3600000);
    INSERT OR IGNORE INTO usage_rollup_dirty VALUES (NEW.account, NEW.ts / 3600000);
END;
DROP TRIGGER IF EXISTS usage_rollup_event_delete;
CREATE TRIGGER usage_rollup_event_delete AFTER DELETE ON usage_events
WHEN OLD.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty VALUES (OLD.account, OLD.ts / 3600000);
END;
DROP TRIGGER IF EXISTS usage_rollup_detail_insert;
CREATE TRIGGER usage_rollup_detail_insert AFTER INSERT ON usage_detail
WHEN EXISTS (SELECT 1 FROM usage_events e
    WHERE e.account = NEW.account AND e.msg_uuid = NEW.msg_uuid
      AND e.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000)
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty
        SELECT e.account, e.ts / 3600000 FROM usage_events e
        WHERE e.account = NEW.account AND e.msg_uuid = NEW.msg_uuid;
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
DROP TRIGGER IF EXISTS usage_rollup_detail_update;
CREATE TRIGGER usage_rollup_detail_update AFTER UPDATE ON usage_detail
WHEN (OLD.usd, OLD.kind, OLD.root_session) IS NOT (NEW.usd, NEW.kind, NEW.root_session)
  AND EXISTS (SELECT 1 FROM usage_events e
    WHERE e.account = NEW.account AND e.msg_uuid = NEW.msg_uuid
      AND e.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000)
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty
        SELECT e.account, e.ts / 3600000 FROM usage_events e
        WHERE e.account = NEW.account AND e.msg_uuid = NEW.msg_uuid;
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
DROP TRIGGER IF EXISTS usage_rollup_detail_delete;
CREATE TRIGGER usage_rollup_detail_delete AFTER DELETE ON usage_detail
WHEN EXISTS (SELECT 1 FROM usage_events e
    WHERE e.account = OLD.account AND e.msg_uuid = OLD.msg_uuid
      AND e.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000)
BEGIN
    INSERT OR IGNORE INTO usage_rollup_dirty
        SELECT e.account, e.ts / 3600000 FROM usage_events e
        WHERE e.account = OLD.account AND e.msg_uuid = OLD.msg_uuid;
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
";

/// A profile's settled hours as the store keeps them.
#[derive(Clone, Debug, PartialEq)]
pub struct Rollup {
    /// Hours before this (epoch ms, an hour boundary) are in `rows`.
    pub cutoff: i64,
    /// The oldest hours were left out (past the row cap).
    pub cut: bool,
    /// Oldest first.
    pub rows: Vec<HourRow>,
}

/// Which kept rows a [`UsageHistoryRepo::save_rollup`] replaces.
#[derive(Clone, Copy, Debug)]
pub enum Replace<'a> {
    /// All of the profile's: `rows` are its settled hours afresh.
    Everything,
    /// Those of these hours (epoch ms); `rows` are theirs, and any past
    /// the old cutoff.
    Hours(&'a [i64]),
}

fn sql(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl UsageHistoryRepo<'_> {
    /// `account`'s kept settled hours; `None` when none are kept (never
    /// summed yet, or the derived tables were made afresh).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn rollup(&self, account: &str) -> Result<Option<Rollup>, CoreError> {
        let state: Option<(i64, bool)> = self
            .conn
            .prepare_cached("SELECT cutoff, cut FROM usage_rollup_state WHERE account = ?1")?
            .query_row(params![account], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        let Some((cutoff, cut)) = state else {
            return Ok(None);
        };
        let rows = self
            .conn
            .prepare_cached(
                "SELECT hour, session, root, kind, model,
                        input, output, cache_r, cache_w, reasoning, messages, stored_usd,
                        un_input, un_output, un_cache_r, un_cache_w, un_reasoning
                 FROM usage_hour_rollup WHERE account = ?1 ORDER BY hour",
            )?
            .query_map(params![account], |row| {
                Ok(HourRow {
                    hour_ms: row.get(0)?,
                    session: row.get(1)?,
                    root: row.get(2)?,
                    kind: CallKind::parse(&row.get::<_, String>(3)?),
                    model: row.get(4)?,
                    tokens: tokens_at(row, 5)?,
                    messages: u32::try_from(count(row.get(10)?)).unwrap_or(u32::MAX),
                    cost: GroupCost {
                        stored_usd: row.get(11)?,
                        unstored: tokens_at(row, 12)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(Rollup { cutoff, cut, rows }))
    }

    /// The settled hours (epoch ms, ascending) whose messages changed since
    /// `account`'s were last taken, taken now: the caller sums them again.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; nothing is taken
    /// then.
    pub fn take_changed_hours(&self, account: &str) -> Result<Vec<i64>, CoreError> {
        let tx = rusqlite::Transaction::new_unchecked(
            self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        let hours = tx
            .prepare_cached("SELECT hour FROM usage_rollup_dirty WHERE account = ?1 ORDER BY hour")?
            .query_map(params![account], |row| row.get::<_, i64>(0))?
            .map(|hour| hour.map(|hour| hour.saturating_mul(HOUR_MS)))
            .collect::<Result<Vec<_>, _>>()?;
        tx.execute(
            "DELETE FROM usage_rollup_dirty WHERE account = ?1",
            params![account],
        )?;
        tx.commit()?;
        Ok(hours)
    }

    /// `account`'s rows in each of `hours` (epoch ms), summed afresh.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn hourly_in(&self, account: &str, hours: &[i64]) -> Result<Vec<HourRow>, CoreError> {
        let mut rows = Vec::new();
        for hour in hours {
            rows.extend(self.hourly(account, *hour, hour + HOUR_MS, usize::MAX)?.0);
        }
        Ok(rows)
    }

    /// Keeps `account`'s settled hours up to `cutoff` (see [`Replace`]),
    /// in one transaction.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures; nothing changes
    /// then.
    pub fn save_rollup(
        &self,
        account: &str,
        cutoff: i64,
        cut: bool,
        replace: Replace<'_>,
        rows: &[HourRow],
    ) -> Result<(), CoreError> {
        let tx = rusqlite::Transaction::new_unchecked(
            self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        match replace {
            Replace::Everything => {
                tx.execute(
                    "DELETE FROM usage_hour_rollup WHERE account = ?1",
                    params![account],
                )?;
            }
            Replace::Hours(hours) => {
                let mut clear = tx.prepare_cached(
                    "DELETE FROM usage_hour_rollup WHERE account = ?1 AND hour = ?2",
                )?;
                for hour in hours {
                    clear.execute(params![account, hour])?;
                }
            }
        }
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO usage_hour_rollup
                     (account, hour, session, root, kind, model, input, output, cache_r, cache_w,
                      reasoning, messages, stored_usd, un_input, un_output, un_cache_r,
                      un_cache_w, un_reasoning)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                         ?17, ?18)",
            )?;
            for row in rows {
                let (tokens, rest) = (row.tokens, row.cost.unstored);
                insert.execute(params![
                    account,
                    row.hour_ms,
                    row.session,
                    row.root,
                    row.kind.as_str(),
                    row.model,
                    sql(tokens.input),
                    sql(tokens.output),
                    sql(tokens.cache_read),
                    sql(tokens.cache_write),
                    sql(tokens.reasoning),
                    row.messages,
                    row.cost.stored_usd,
                    sql(rest.input),
                    sql(rest.output),
                    sql(rest.cache_read),
                    sql(rest.cache_write),
                    sql(rest.reasoning),
                ])?;
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO usage_rollup_state (account, cutoff, cut) VALUES (?1, ?2, ?3)",
            params![account, cutoff, cut],
        )?;
        tx.commit()?;
        Ok(())
    }
}
