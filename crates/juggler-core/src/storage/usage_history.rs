//! Usage history (0.1.5): the summary queries the Usage, project, account
//! and session pages draw from. Each one answers with a few rows per chart,
//! never the raw events: usage is summed per hour (or per day) in SQLite,
//! one profile at a time through the `(account, ts)` index, so a range's
//! cost grows with its active hours, not its messages. Nothing here writes,
//! and no index or table is added (the schema is the one main shares with
//! release/0.2). Costs come from `usage_detail`, the store's derived
//! per-call table (0.3.6), joined on the call's key.

use rusqlite::{Connection, params};

use crate::error::CoreError;
use crate::model::TokenCounts;
use crate::quota::{CallKind, QuotaSample, WindowKind};

/// An hour, in milliseconds.
pub const HOUR_MS: i64 = 3_600_000;

/// What a group of calls costs at list prices (0.3.6): the costs stored
/// with its calls, each priced at its own tier when it was read, and the
/// tokens of its calls with no stored cost (an unpriced model, or a call
/// stored before costs were), which only a price for their sum can cover.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroupCost {
    /// The calls' stored costs, summed (USD).
    pub stored_usd: f64,
    /// The tokens of the calls with no stored cost.
    pub unstored: TokenCounts,
}

impl GroupCost {
    /// The group's cost: the stored costs, plus `price` of the rest when
    /// there is any. `None` when that rest can't be priced (its model has
    /// no list price): never a partial figure.
    #[must_use]
    pub fn total(&self, price: impl FnOnce(TokenCounts) -> Option<f64>) -> Option<f64> {
        if self.unstored == TokenCounts::default() {
            return Some(self.stored_usd);
        }
        price(self.unstored).map(|rest| self.stored_usd + rest)
    }
}

/// One profile's usage of one session with one model in one hour.
#[derive(Clone, Debug, PartialEq)]
pub struct HourRow {
    /// The hour's start, epoch ms.
    pub hour_ms: i64,
    /// The session (the tool's conversation id); `None` when the event
    /// had none.
    pub session: Option<String>,
    /// The conversation the session's calls belong to, when it is
    /// another (a subagent's or an automatic review's thread): they count
    /// under it, not as a session of their own.
    pub root: Option<String>,
    /// What kind of calls these are.
    pub kind: CallKind,
    /// The model; `None` when the event didn't say.
    pub model: Option<String>,
    /// The tokens, each message counted once.
    pub tokens: TokenCounts,
    /// How many messages.
    pub messages: u32,
    /// What they cost.
    pub cost: GroupCost,
}

impl HourRow {
    /// The conversation these calls count under.
    #[must_use]
    pub fn conversation(&self) -> Option<&String> {
        self.root.as_ref().or(self.session.as_ref())
    }
}

/// One message of a session: what a turn used.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnRow {
    /// When, epoch ms.
    pub ts: i64,
    /// The model.
    pub model: Option<String>,
    /// The tokens.
    pub tokens: TokenCounts,
    /// What it cost.
    pub cost: GroupCost,
}

/// One session's usage with one model.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRow {
    /// The model.
    pub model: Option<String>,
    /// Its first use, epoch ms.
    pub first_ms: i64,
    /// The tokens.
    pub tokens: TokenCounts,
    /// How many messages.
    pub messages: u32,
    /// What they cost.
    pub cost: GroupCost,
}

/// The calls with their stored costs (`d`), for the summaries.
const WITH_COSTS: &str = "usage_events e
    LEFT JOIN usage_detail d ON d.account = e.account AND d.msg_uuid = e.msg_uuid";

/// A group's [`GroupCost`] columns (six), over [`WITH_COSTS`].
const COST_SUMS: &str = "TOTAL(d.usd),
    SUM(CASE WHEN d.usd IS NULL THEN e.input ELSE 0 END),
    SUM(CASE WHEN d.usd IS NULL THEN e.output ELSE 0 END),
    SUM(CASE WHEN d.usd IS NULL THEN e.cache_r ELSE 0 END),
    SUM(CASE WHEN d.usd IS NULL THEN e.cache_w ELSE 0 END),
    SUM(CASE WHEN d.usd IS NULL THEN e.reasoning ELSE 0 END)";

fn cost_at(row: &rusqlite::Row<'_>, first: usize) -> rusqlite::Result<GroupCost> {
    Ok(GroupCost {
        stored_usd: row.get(first)?,
        unstored: tokens_at(row, first + 1)?,
    })
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
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT e.ts / 3600000 AS h, e.session, e.model, COALESCE(d.kind, 'main') AS k,
                    MAX(d.root_session),
                    SUM(e.input), SUM(e.output), SUM(e.cache_r), SUM(e.cache_w), SUM(e.reasoning),
                    COUNT(*), {COST_SUMS}
             FROM {WITH_COSTS}
             WHERE e.account = ?1 AND e.ts >= ?2 AND e.ts < ?3
             GROUP BY h, e.session, e.model, k
             ORDER BY h DESC
             LIMIT ?4"
        ))?;
        let wanted = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
        let mut rows = stmt
            .query_map(params![account, start_ms, end_ms, wanted], |row| {
                let session: Option<String> = row.get(1)?;
                let root: Option<String> = row.get(4)?;
                Ok(HourRow {
                    hour_ms: row.get::<_, i64>(0)?.saturating_mul(HOUR_MS),
                    root: root.filter(|root| Some(root) != session.as_ref()),
                    session,
                    model: row.get(2)?,
                    kind: CallKind::parse(&row.get::<_, String>(3)?),
                    tokens: tokens_at(row, 5)?,
                    messages: u32::try_from(row.get::<_, i64>(10)?).unwrap_or(u32::MAX),
                    cost: cost_at(row, 11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let cut = rows.len() > limit;
        rows.truncate(limit);
        Ok((rows, cut))
    }

    /// One session's messages on one profile, oldest first: the newest
    /// `limit` of them. A conversation's include its subagents' and its
    /// automatic reviews' (0.3.6). Says whether older ones were left out.
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
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT e.ts, e.model, e.input, e.output, e.cache_r, e.cache_w, e.reasoning, {COST_SUMS}
             FROM {WITH_COSTS}
             WHERE e.account = ?1 AND (e.session = ?2 OR d.root_session = ?2)
             GROUP BY e.id
             ORDER BY e.ts DESC
             LIMIT ?3"
        ))?;
        let wanted = i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX);
        let mut rows = stmt
            .query_map(params![account, session, wanted], |row| {
                Ok(TurnRow {
                    ts: row.get(0)?,
                    model: row.get(1)?,
                    tokens: tokens_at(row, 2)?,
                    cost: cost_at(row, 7)?,
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
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT e.session = ?2 OR d.root_session IS ?2 AS mine,
                    SUM(e.input), SUM(e.output), SUM(e.cache_r), SUM(e.cache_w), SUM(e.reasoning)
             FROM {WITH_COSTS}
             WHERE e.account = ?1 AND e.ts >= ?3
             GROUP BY mine"
        ))?;
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
    /// instant the calls with no stored cost are priced at); a
    /// conversation's include its subagents' and automatic reviews'.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn session_by_model(
        &self,
        account: &str,
        session: &str,
    ) -> Result<Vec<ModelRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT e.model, MIN(e.ts),
                    SUM(e.input), SUM(e.output), SUM(e.cache_r), SUM(e.cache_w), SUM(e.reasoning),
                    COUNT(*), {COST_SUMS}
             FROM {WITH_COSTS}
             WHERE e.account = ?1 AND (e.session = ?2 OR d.root_session = ?2)
             GROUP BY e.model"
        ))?;
        let rows = stmt.query_map(params![account, session], |row| {
            Ok(ModelRow {
                model: row.get(0)?,
                first_ms: row.get(1)?,
                tokens: tokens_at(row, 2)?,
                messages: u32::try_from(row.get::<_, i64>(7)?).unwrap_or(u32::MAX),
                cost: cost_at(row, 8)?,
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
