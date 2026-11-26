//! ADR-0039 §3 (AJ-E2027): the one table of schedules. Each row names a
//! registered handler and carries typed data (never code): its trigger,
//! modifiers and missed-run policy as typed JSON the app validates, its
//! place in time (`base_ms`, `armed_ms`) and its persisted next due time,
//! recomputed by the app on launch, wake and a clock or zone change.
//!
//! - **Bounded reads.** The scheduler reads the due rows (by an index on
//!   `next_due_ms`) and the soonest due time; the lists read a page at a
//!   time by `(created_ms, id)`, newest first.
//! - **Ended rows stay a while** (`ended_ms`), so the list can say "done";
//!   [`SchedulesRepo::prune_ended`] removes them after their keep.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;

/// V57: the `schedules` table (ADR-0039 §3); safe to run again.
pub const MIGRATION: &str = "
CREATE TABLE IF NOT EXISTS schedules (
    id TEXT PRIMARY KEY,
    owner TEXT NOT NULL,
    handler TEXT NOT NULL,
    payload TEXT NOT NULL,
    trigger TEXT NOT NULL,
    modifiers TEXT NOT NULL,
    missed_policy TEXT NOT NULL,
    next_due_ms INTEGER,
    occurrence_ms INTEGER,
    base_ms INTEGER NOT NULL,
    armed_ms INTEGER,
    last_run_ms INTEGER,
    last_outcome TEXT,
    retries INTEGER NOT NULL DEFAULT 0,
    paused INTEGER NOT NULL DEFAULT 0,
    user_visible INTEGER NOT NULL,
    created_by TEXT NOT NULL CHECK (created_by IN ('user', 'app', 'bot')),
    source TEXT NOT NULL DEFAULT 'ai_juggler',
    title TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    updated_ms INTEGER NOT NULL,
    ended_ms INTEGER
);
CREATE INDEX IF NOT EXISTS idx_schedules_due ON schedules(next_due_ms)
    WHERE next_due_ms IS NOT NULL AND ended_ms IS NULL;
CREATE INDEX IF NOT EXISTS idx_schedules_list ON schedules(user_visible, created_ms, id);
CREATE INDEX IF NOT EXISTS idx_schedules_owner ON schedules(owner, handler);
";

/// How long an ended schedule stays listed (as "done").
pub const SCHEDULE_ENDED_KEEP_MS: i64 = 7 * 86_400_000;
/// The most rows one read returns.
pub const SCHEDULES_READ_MAX: u32 = 500;

const COLUMNS: &str = "id, owner, handler, payload, trigger, modifiers, missed_policy,
    next_due_ms, occurrence_ms, base_ms, armed_ms, last_run_ms, last_outcome, retries, paused,
    user_visible, created_by, source, title, created_ms, updated_ms, ended_ms";

/// One schedule as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleRow {
    /// Its unique id (actions target it by this).
    pub id: String,
    /// The module that owns it.
    pub owner: String,
    /// The registered handler that runs it.
    pub handler: String,
    /// The handler's typed data, JSON.
    pub payload: String,
    /// The trigger, JSON.
    pub trigger: String,
    /// The modifiers, JSON.
    pub modifiers: String,
    /// The missed-run policy, JSON.
    pub missed_policy: String,
    /// When it is next due; `None` when nothing is to come yet.
    pub next_due_ms: Option<i64>,
    /// The raw occurrence the next due time serves.
    pub occurrence_ms: Option<i64>,
    /// Occurrences after this are still to come.
    pub base_ms: i64,
    /// When an event or idle trigger was armed.
    pub armed_ms: Option<i64>,
    /// When it last ran.
    pub last_run_ms: Option<i64>,
    /// How its last run went (a code).
    pub last_outcome: Option<String>,
    /// Retries of a failed run so far.
    pub retries: u32,
    /// Paused by its owner or the user.
    pub paused: bool,
    /// Shown in the user's list (else Settings › Advanced's).
    pub user_visible: bool,
    /// `user`, `app` or `bot` (reserved for 0.6).
    pub created_by: String,
    /// Whose schedule: `ai_juggler`, or later another tool's.
    pub source: String,
    /// Plain words, for the lists.
    pub title: String,
    /// When it was made.
    pub created_ms: i64,
    /// When it last changed.
    pub updated_ms: i64,
    /// When it ended (its lifetime over); `None` while it lives.
    pub ended_ms: Option<i64>,
}

/// Where a schedule is in time, as the scheduler writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleTiming {
    /// The next due time.
    pub next_due_ms: Option<i64>,
    /// Its raw occurrence.
    pub occurrence_ms: Option<i64>,
    /// Handled up to here.
    pub base_ms: i64,
    /// Armed at.
    pub armed_ms: Option<i64>,
}

/// A page's start: newest first by `(created_ms, id)`.
pub type ScheduleKey = (i64, String);

fn row(r: &Row<'_>) -> rusqlite::Result<ScheduleRow> {
    Ok(ScheduleRow {
        id: r.get(0)?,
        owner: r.get(1)?,
        handler: r.get(2)?,
        payload: r.get(3)?,
        trigger: r.get(4)?,
        modifiers: r.get(5)?,
        missed_policy: r.get(6)?,
        next_due_ms: r.get(7)?,
        occurrence_ms: r.get(8)?,
        base_ms: r.get(9)?,
        armed_ms: r.get(10)?,
        last_run_ms: r.get(11)?,
        last_outcome: r.get(12)?,
        retries: r.get(13)?,
        paused: r.get(14)?,
        user_visible: r.get(15)?,
        created_by: r.get(16)?,
        source: r.get(17)?,
        title: r.get(18)?,
        created_ms: r.get(19)?,
        updated_ms: r.get(20)?,
        ended_ms: r.get(21)?,
    })
}

/// The schedules table.
#[derive(Debug)]
pub struct SchedulesRepo<'c> {
    conn: &'c Connection,
}

impl<'c> SchedulesRepo<'c> {
    pub(crate) const fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }

    /// Adds a schedule.
    ///
    /// # Errors
    ///
    /// Storage failures, an id already taken among them.
    pub fn insert(&self, s: &ScheduleRow) -> Result<(), CoreError> {
        self.conn.execute(
            &format!(
                "INSERT INTO schedules ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                 ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"
            ),
            params![
                s.id,
                s.owner,
                s.handler,
                s.payload,
                s.trigger,
                s.modifiers,
                s.missed_policy,
                s.next_due_ms,
                s.occurrence_ms,
                s.base_ms,
                s.armed_ms,
                s.last_run_ms,
                s.last_outcome,
                s.retries,
                s.paused,
                s.user_visible,
                s.created_by,
                s.source,
                s.title,
                s.created_ms,
                s.updated_ms,
                s.ended_ms,
            ],
        )?;
        Ok(())
    }

    /// One schedule.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn get(&self, id: &str) -> Result<Option<ScheduleRow>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM schedules WHERE id = ?1"),
                [id],
                row,
            )
            .optional()?)
    }

    /// The live, unpaused schedules due by `now_ms`, soonest first, at
    /// most `limit`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn due(&self, now_ms: i64, limit: u32) -> Result<Vec<ScheduleRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM schedules
             WHERE next_due_ms IS NOT NULL AND ended_ms IS NULL AND next_due_ms <= ?1
               AND paused = 0
             ORDER BY next_due_ms, id LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![now_ms, limit.min(SCHEDULES_READ_MAX)], row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The soonest due time of a live, unpaused schedule.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn soonest(&self) -> Result<Option<i64>, CoreError> {
        Ok(self.conn.query_row(
            "SELECT min(next_due_ms) FROM schedules
             WHERE next_due_ms IS NOT NULL AND ended_ms IS NULL AND paused = 0",
            [],
            |r| r.get(0),
        )?)
    }

    /// The live schedules after `after_id` by id, at most `limit` (the
    /// recompute walks them all in pages).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn live_after(&self, after_id: &str, limit: u32) -> Result<Vec<ScheduleRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM schedules WHERE ended_ms IS NULL AND id > ?1
             ORDER BY id LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![after_id, limit.min(SCHEDULES_READ_MAX)], row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// A page of one list (the user's, or the app's own), newest first,
    /// after `after`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn page(
        &self,
        user_visible: bool,
        after: Option<&ScheduleKey>,
        limit: u32,
    ) -> Result<Vec<ScheduleRow>, CoreError> {
        let (at, id) = after.map_or((None, ""), |(at, id)| (Some(*at), id.as_str()));
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM schedules WHERE user_visible = ?1
               AND (?2 IS NULL OR created_ms < ?2 OR (created_ms = ?2 AND id < ?3))
             ORDER BY created_ms DESC, id DESC LIMIT ?4"
        ))?;
        let rows = stmt
            .query_map(
                params![user_visible, at, id, limit.min(SCHEDULES_READ_MAX)],
                row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The schedules of one owner and handler (an owner keeps its own
    /// app-made schedules, at most a few).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn of_handler(&self, owner: &str, handler: &str) -> Result<Vec<ScheduleRow>, CoreError> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM schedules WHERE owner = ?1 AND handler = ?2
             ORDER BY created_ms, id LIMIT ?3"
        ))?;
        let rows = stmt
            .query_map(params![owner, handler, SCHEDULES_READ_MAX], row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// How many live schedules the user's list holds.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn live_user_count(&self) -> Result<u32, CoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM schedules WHERE user_visible = 1 AND ended_ms IS NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    /// The user's live schedules: how many run, how many are paused.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn user_counts(&self) -> Result<(u32, u32), CoreError> {
        let (active, paused): (i64, i64) = self.conn.query_row(
            "SELECT coalesce(sum(paused = 0), 0), coalesce(sum(paused = 1), 0) FROM schedules
             WHERE user_visible = 1 AND ended_ms IS NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok((
            u32::try_from(active).unwrap_or(u32::MAX),
            u32::try_from(paused).unwrap_or(u32::MAX),
        ))
    }

    /// Writes where a schedule is in time.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn set_timing(&self, id: &str, t: ScheduleTiming, now_ms: i64) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE schedules SET next_due_ms = ?2, occurrence_ms = ?3, base_ms = ?4,
                 armed_ms = ?5, updated_ms = ?6
             WHERE id = ?1",
            params![
                id,
                t.next_due_ms,
                t.occurrence_ms,
                t.base_ms,
                t.armed_ms,
                now_ms
            ],
        )? > 0)
    }

    /// Writes a run's end: its outcome, the retries so far, where the
    /// schedule is now, and its end when its lifetime is over.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn record_run(
        &self,
        id: &str,
        ran_ms: Option<i64>,
        outcome: &str,
        retries: u32,
        t: ScheduleTiming,
        ended_ms: Option<i64>,
    ) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE schedules SET last_run_ms = coalesce(?2, last_run_ms), last_outcome = ?3,
                 retries = ?4, next_due_ms = ?5, occurrence_ms = ?6, base_ms = ?7,
                 armed_ms = ?8, ended_ms = ?9, updated_ms = coalesce(?2, updated_ms)
             WHERE id = ?1",
            params![
                id,
                ran_ms,
                outcome,
                retries,
                t.next_due_ms,
                t.occurrence_ms,
                t.base_ms,
                t.armed_ms,
                ended_ms
            ],
        )? > 0)
    }

    /// Edits a schedule's timing spec, title and data, with its new place.
    ///
    /// # Errors
    ///
    /// Storage failures.
    #[allow(
        clippy::too_many_arguments,
        reason = "one row's editable fields, written together"
    )]
    pub fn edit(
        &self,
        id: &str,
        trigger: &str,
        modifiers: &str,
        payload: &str,
        title: &str,
        t: ScheduleTiming,
        now_ms: i64,
    ) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE schedules SET trigger = ?2, modifiers = ?3, payload = ?4, title = ?5,
                 next_due_ms = ?6, occurrence_ms = ?7, base_ms = ?8, armed_ms = ?9,
                 retries = 0, updated_ms = ?10, ended_ms = NULL
             WHERE id = ?1",
            params![
                id,
                trigger,
                modifiers,
                payload,
                title,
                t.next_due_ms,
                t.occurrence_ms,
                t.base_ms,
                t.armed_ms,
                now_ms
            ],
        )? > 0)
    }

    /// Pauses or resumes one schedule.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn set_paused(&self, id: &str, paused: bool, now_ms: i64) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE schedules SET paused = ?2, updated_ms = ?3 WHERE id = ?1 AND paused != ?2",
            params![id, paused, now_ms],
        )? > 0)
    }

    /// Pauses or resumes every live schedule in the user's list; how many
    /// changed.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn set_paused_user(&self, paused: bool, now_ms: i64) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "UPDATE schedules SET paused = ?1, updated_ms = ?2
             WHERE user_visible = 1 AND ended_ms IS NULL AND paused != ?1",
            params![paused, now_ms],
        )?)
    }

    /// Removes one schedule.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn remove(&self, id: &str) -> Result<bool, CoreError> {
        Ok(self
            .conn
            .execute("DELETE FROM schedules WHERE id = ?1", [id])?
            > 0)
    }

    /// Removes the schedules that ended before `before_ms`; how many went.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn prune_ended(&self, before_ms: i64) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "DELETE FROM schedules WHERE ended_ms IS NOT NULL AND ended_ms < ?1",
            [before_ms],
        )?)
    }
}
