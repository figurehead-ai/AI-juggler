//! Usage history (0.1.5): the summary queries the Usage, project, account
//! and session pages draw from. Each one answers with a few rows per chart,
//! never the raw events: usage is summed per hour (or per day) in SQLite,
//! one profile at a time through the `(account, ts)` index, so a range's
//! cost grows with its active hours, not its messages. Nothing here writes,
//! and no index or table is added (the schema is the one main shares with
//! release/0.2).

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::model::TokenCounts;
use crate::quota::{QuotaSample, WindowKind};

/// An hour, in milliseconds.
pub const HOUR_MS: i64 = 3_600_000;

/// One profile's usage of one session with one model in one hour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HourRow {
    /// The hour's start, epoch ms.
    pub hour_ms: i64,
    /// The session (the tool's conversation id); `None` when the event
    /// had none.
    pub session: Option<String>,
    /// The model; `None` when the event didn't say.
    pub model: Option<String>,
    /// The tokens, each message counted once.
    pub tokens: TokenCounts,
    /// How many messages.
    pub messages: u32,
}

/// One message of a session: what a turn used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnRow {
    /// When, epoch ms.
    pub ts: i64,
    /// The model.
    pub model: Option<String>,
    /// The tokens.
    pub tokens: TokenCounts,
}

/// One session's usage with one model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRow {
    /// The model.
    pub model: Option<String>,
    /// Its first use, epoch ms.
    pub first_ms: i64,
    /// The tokens.
    pub tokens: TokenCounts,
    /// How many messages.
    pub messages: u32,
}

/// A move of a piece of work from one account to another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchRow {
    /// When the move was made, epoch ms.
    pub at_ms: i64,
    /// The project the work belongs to.
    pub project: String,
    /// The profile it left.
    pub from_account: String,
    /// The profile it went to.
    pub to_account: String,
    /// `complete`, `failed`, … as the switch graph records it.
    pub state: String,
}

/// How old a message must be before the summaries may keep its hour in
/// memory: six hours (later messages still arrive for the current one).
pub const SETTLED_MS: i64 = 6 * HOUR_MS;

/// Kept outside the numbered schema, like the usage store's other derived
/// tables (made again at each open, harmless to builds that don't know
/// them): a counter of changes to messages older than [`SETTLED_MS`]. A
/// summary kept in memory for settled hours stays good while it doesn't
/// move. The triggers live in the file, so a write by any build counts.
/// `token` tells one store from another (a restored one starts afresh).
pub(crate) const HISTORY_DERIVED: &str = "
CREATE TABLE IF NOT EXISTS usage_history_epoch (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    token INTEGER NOT NULL,
    changed INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO usage_history_epoch (id, token, changed) VALUES (1, random(), 0);
DROP TRIGGER IF EXISTS usage_history_settled_insert;
CREATE TRIGGER usage_history_settled_insert AFTER INSERT ON usage_events
WHEN NEW.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
BEGIN
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
DROP TRIGGER IF EXISTS usage_history_settled_update;
CREATE TRIGGER usage_history_settled_update AFTER UPDATE ON usage_events
WHEN (OLD.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
       OR NEW.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000)
  AND (OLD.account, OLD.session, OLD.ts, OLD.model, OLD.input, OLD.output, OLD.cache_r, OLD.cache_w, OLD.reasoning)
      IS NOT (NEW.account, NEW.session, NEW.ts, NEW.model, NEW.input, NEW.output, NEW.cache_r, NEW.cache_w, NEW.reasoning)
BEGIN
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
DROP TRIGGER IF EXISTS usage_history_settled_delete;
CREATE TRIGGER usage_history_settled_delete AFTER DELETE ON usage_events
WHEN OLD.ts < CAST(strftime('%s', 'now') AS INTEGER) * 1000 - 21600000
BEGIN
    UPDATE usage_history_epoch SET changed = changed + 1 WHERE id = 1;
END;
";

/// Read-only summaries over the usage store.
pub struct UsageHistoryRepo<'a> {
    conn: &'a Connection,
}

fn count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn sample(row: &rusqlite::Row<'_>) -> rusqlite::Result<QuotaSample> {
    Ok(QuotaSample {
        ts: row.get(0)?,
        used_pct: row.get(1)?,
        resets_at: row.get(2)?,
        confidence: crate::model::Confidence::Measured,
    })
}

fn tokens_at(row: &rusqlite::Row<'_>, first: usize) -> rusqlite::Result<TokenCounts> {
    Ok(TokenCounts {
        input: count(row.get(first)?),
        output: count(row.get(first + 1)?),
        cache_read: count(row.get(first + 2)?),
        cache_write: count(row.get(first + 3)?),
        reasoning: count(row.get(first + 4)?),
    })
}

impl UsageHistoryRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> UsageHistoryRepo<'_> {
        UsageHistoryRepo { conn }
    }

    /// One profile's usage in `[start, end)`, summed per hour, session and
    /// model, at most `limit` rows (the newest hours first when it's cut).
    /// Says whether rows were left out.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn hourly(
        &self,
        account: &str,
        start_ms: i64,
        end_ms: i64,
        limit: usize,
    ) -> Result<(Vec<HourRow>, bool), CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ts / 3600000 AS h, session, model,
                    SUM(input), SUM(output), SUM(cache_r), SUM(cache_w), SUM(reasoning), COUNT(*)
             FROM usage_events
             WHERE account = ?1 AND ts >= ?2 AND ts < ?3
             GROUP BY h, session, model
             ORDER BY h DESC
             LIMIT ?4",
        )?;
        let wanted = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
        let mut rows = stmt
            .query_map(params![account, start_ms, end_ms, wanted], |row| {
                Ok(HourRow {
                    hour_ms: row.get::<_, i64>(0)?.saturating_mul(HOUR_MS),
                    session: row.get(1)?,
                    model: row.get(2)?,
                    tokens: tokens_at(row, 3)?,
                    messages: u32::try_from(row.get::<_, i64>(8)?).unwrap_or(u32::MAX),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let cut = rows.len() > limit;
        rows.truncate(limit);
        Ok((rows, cut))
    }

    /// One session's messages on one profile, oldest first: the newest
    /// `limit` of them. Says whether older ones were left out.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn session_turns(
        &self,
        account: &str,
        session: &str,
        limit: usize,
    ) -> Result<(Vec<TurnRow>, bool), CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ts, model, input, output, cache_r, cache_w, reasoning
             FROM usage_events
             WHERE account = ?1 AND session = ?2
             ORDER BY ts DESC
             LIMIT ?3",
        )?;
        let wanted = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
        let mut rows = stmt
            .query_map(params![account, session, wanted], |row| {
                Ok(TurnRow {
                    ts: row.get(0)?,
                    model: row.get(1)?,
                    tokens: tokens_at(row, 2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let cut = rows.len() > limit;
        rows.truncate(limit);
        rows.reverse();
        Ok((rows, cut))
    }

    /// The settled-history stamp: the store's token and how many times a
    /// message older than [`SETTLED_MS`] was written, changed or removed.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn settled_stamp(&self) -> Result<(i64, i64), CoreError> {
        self.conn
            .prepare_cached("SELECT token, changed FROM usage_history_epoch WHERE id = 1")?
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(CoreError::Storage)
    }

    /// The first usage of a profile, epoch ms (the `(account, ts)` index
    /// answers it at once); `None` when it has none.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn first_ts(&self, account: &str) -> Result<Option<i64>, CoreError> {
        self.conn
            .prepare_cached("SELECT MIN(ts) FROM usage_events WHERE account = ?1")?
            .query_row(params![account], |row| row.get(0))
            .map_err(CoreError::Storage)
    }

    /// One profile's tokens since `since_ms`, and how much of that one
    /// session used (a window's share).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn share_since(
        &self,
        account: &str,
        session: &str,
        since_ms: i64,
    ) -> Result<(TokenCounts, TokenCounts), CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT session = ?2,
                    SUM(input), SUM(output), SUM(cache_r), SUM(cache_w), SUM(reasoning)
             FROM usage_events
             WHERE account = ?1 AND ts >= ?3
             GROUP BY session = ?2",
        )?;
        let mut all = TokenCounts::default();
        let mut mine = TokenCounts::default();
        let rows = stmt.query_map(params![account, session, since_ms], |row| {
            Ok((row.get::<_, Option<bool>>(0)?, tokens_at(row, 1)?))
        })?;
        for row in rows {
            let (is_mine, tokens) = row?;
            add(&mut all, tokens);
            if is_mine == Some(true) {
                add(&mut mine, tokens);
            }
        }
        Ok((all, mine))
    }

    /// A window's readings in `[start, end)`, oldest first, at most
    /// `limit` (the oldest left out past it).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn samples_between(
        &self,
        account: &str,
        kind: WindowKind,
        start_ms: i64,
        end_ms: i64,
        limit: usize,
    ) -> Result<Vec<QuotaSample>, CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ts, used_pct, resets_at
             FROM quota_samples
             WHERE account = ?1 AND window = ?2 AND ts >= ?3 AND ts < ?4
             ORDER BY ts DESC
             LIMIT ?5",
        )?;
        let wanted = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut rows = stmt
            .query_map(
                params![account, kind.as_str(), start_ms, end_ms, wanted],
                sample,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        rows.reverse();
        Ok(rows)
    }

    /// A window's newest reading at or before `at_ms`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn latest_sample(
        &self,
        account: &str,
        kind: WindowKind,
        at_ms: i64,
    ) -> Result<Option<QuotaSample>, CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT ts, used_pct, resets_at
             FROM quota_samples
             WHERE account = ?1 AND window = ?2 AND ts <= ?3
             ORDER BY ts DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![account, kind.as_str(), at_ms], sample)?;
        rows.next().transpose().map_err(CoreError::Storage)
    }

    /// One session's totals per model, with each model's first use (the
    /// instant it's priced at).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn session_by_model(
        &self,
        account: &str,
        session: &str,
    ) -> Result<Vec<ModelRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT model, MIN(ts),
                    SUM(input), SUM(output), SUM(cache_r), SUM(cache_w), SUM(reasoning), COUNT(*)
             FROM usage_events
             WHERE account = ?1 AND session = ?2
             GROUP BY model",
        )?;
        let rows = stmt.query_map(params![account, session], |row| {
            Ok(ModelRow {
                model: row.get(0)?,
                first_ms: row.get(1)?,
                tokens: tokens_at(row, 2)?,
                messages: u32::try_from(row.get::<_, i64>(7)?).unwrap_or(u32::MAX),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// The moves between accounts made in `[start, end)`, oldest first, at
    /// most `limit`; only one project's when `project` is given.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn switches_between(
        &self,
        start_ms: i64,
        end_ms: i64,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SwitchRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT e.created_at, w.project, e.from_account, e.to_account, e.state
             FROM switch_edges e JOIN work_items w ON w.id = e.work_id
             WHERE e.created_at >= ?1 AND e.created_at < ?2 AND (?3 IS NULL OR w.project = ?3)
             ORDER BY e.created_at
             LIMIT ?4",
        )?;
        let wanted = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = stmt.query_map(params![start_ms, end_ms, project, wanted], |row| {
            Ok(SwitchRow {
                at_ms: row.get(0)?,
                project: row.get(1)?,
                from_account: row.get(2)?,
                to_account: row.get(3)?,
                state: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }
}

/// Adds `more` to `into`, saturating.
pub fn add(into: &mut TokenCounts, more: TokenCounts) {
    into.input = into.input.saturating_add(more.input);
    into.output = into.output.saturating_add(more.output);
    into.cache_read = into.cache_read.saturating_add(more.cache_read);
    into.cache_write = into.cache_write.saturating_add(more.cache_write);
    into.reasoning = into.reasoning.saturating_add(more.reasoning);
}

/// At most `limit` items, evenly spaced, the first and last kept.
#[must_use]
pub fn thin<T>(rows: Vec<T>, limit: usize) -> Vec<T> {
    let n = rows.len();
    if n <= limit || limit < 2 {
        return rows;
    }
    let mut kept = Vec::with_capacity(limit);
    let mut next = 0usize;
    for (index, row) in rows.into_iter().enumerate() {
        // The i-th kept item sits at round(i × (n − 1) / (limit − 1)).
        if index == (next * (n - 1) + (limit - 1) / 2) / (limit - 1) {
            kept.push(row);
            next += 1;
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinning_keeps_both_ends_and_the_limit() {
        let rows: Vec<usize> = (0..1000).collect();
        let kept = thin(rows, 50);
        assert_eq!(kept.len(), 50);
        assert_eq!(kept.first(), Some(&0));
        assert_eq!(kept.last(), Some(&999));
        assert!(kept.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(thin(vec![1, 2, 3], 50), vec![1, 2, 3]);
    }
}
