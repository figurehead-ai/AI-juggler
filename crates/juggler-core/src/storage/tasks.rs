//! E19 storage (schema v36): the tasks a pool shares.
//!
//! - **Tasks**: epics, stories and tasks with a parent, a short id
//!   (`aj-7f3c`), a title, a body, acceptance criteria, a priority, a
//!   stored status, an assignee (a member ref, so a switch keeps it) and
//!   an optional link to an issue elsewhere, with its title and state as
//!   last looked up.
//! - **Links** between tasks (blocks, relates to, duplicates). The status
//!   every read returns is the honest one (`tasks::effective`): a task an
//!   open task blocks reads as blocked.
//! - **A claim is one statement** ([`TasksRepo::claim`]): it takes a task
//!   only while it is ready, unblocked and no one else's, so of N agents
//!   claiming one task exactly one wins.
//! - **The sessions that worked a task** (`task_sessions`): who claimed it,
//!   who it was started for. A switch keeps the task (its assignee is the
//!   Work item) and the switch history shows each session's.
//! - **An append-only event log** (triggers refuse updates and deletes):
//!   every change, by whom.
//! - **What waits on the user** (`task_needs`): an agent's change above
//!   its pool's level, held for the user's OK, and an agent's request for
//!   a new session, which only the user starts (D-31).
//!
//! Nothing here starts, stops or types into a session, and nothing reads
//! quota (D-31).

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::error::CoreError;
use crate::model::Timestamp;
use crate::tasks::{LinkKind, Priority, TaskKind, TaskStatus, effective};

/// v36: the E19 tables.
pub(super) const MIGRATION: &str = "
CREATE TABLE tasks (
    seq           INTEGER PRIMARY KEY AUTOINCREMENT,
    id            TEXT NOT NULL UNIQUE,
    pool_id       TEXT NOT NULL REFERENCES pools(id),
    kind          TEXT NOT NULL,
    parent_id     TEXT,
    title         TEXT NOT NULL,
    body          TEXT NOT NULL DEFAULT '',
    acceptance    TEXT NOT NULL DEFAULT '',
    priority      INTEGER NOT NULL DEFAULT 2,
    status        TEXT NOT NULL,
    assignee_ref  TEXT,
    assignee_name TEXT,
    created_by    TEXT NOT NULL,
    created_by_name TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    closed_at     INTEGER,
    external_url  TEXT,
    external_title TEXT,
    external_state TEXT,
    external_checked_at INTEGER
);
CREATE INDEX idx_tasks_pool ON tasks(pool_id, status, priority, seq);
CREATE INDEX idx_tasks_assignee ON tasks(assignee_ref, status);
CREATE INDEX idx_tasks_parent ON tasks(parent_id);
CREATE TABLE task_links (
    from_id    TEXT NOT NULL REFERENCES tasks(id),
    to_id      TEXT NOT NULL REFERENCES tasks(id),
    kind       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, kind)
);
CREATE INDEX idx_task_links_to ON task_links(to_id, kind);
CREATE TABLE task_sessions (
    task_id    TEXT NOT NULL REFERENCES tasks(id),
    session_id TEXT NOT NULL,
    work_id    TEXT,
    how        TEXT NOT NULL,
    linked_at  INTEGER NOT NULL,
    PRIMARY KEY (task_id, session_id)
);
CREATE INDEX idx_task_sessions_session ON task_sessions(session_id);
CREATE TABLE task_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         INTEGER NOT NULL,
    pool_id    TEXT NOT NULL,
    task_id    TEXT NOT NULL,
    actor_ref  TEXT NOT NULL,
    actor_name TEXT NOT NULL,
    verb       TEXT NOT NULL,
    detail     TEXT
);
CREATE INDEX idx_task_events_task ON task_events(task_id, id);
CREATE TRIGGER task_events_no_update BEFORE UPDATE ON task_events BEGIN
    SELECT RAISE(ABORT, 'the event log is append-only');
END;
CREATE TRIGGER task_events_no_delete BEFORE DELETE ON task_events BEGIN
    SELECT RAISE(ABORT, 'the event log is append-only');
END;
CREATE TABLE task_needs (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    id          TEXT NOT NULL UNIQUE,
    pool_id     TEXT NOT NULL,
    task_id     TEXT,
    kind        TEXT NOT NULL,
    action      TEXT NOT NULL,
    actor_ref   TEXT NOT NULL,
    actor_name  TEXT NOT NULL,
    change      TEXT,
    summary     TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    answered_at INTEGER,
    answer      TEXT,
    session_id  TEXT
);
CREATE INDEX idx_task_needs_open ON task_needs(answered_at, seq);
";

/// Who the user is, as an actor or an assignee.
pub const TASK_USER: &str = "user";

/// The issue a task links to, as last looked up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct External {
    /// The link.
    pub url: String,
    /// Its title there, when looked up.
    pub title: Option<String>,
    /// Its state there (`open`, `closed`, `merged`), when looked up.
    pub state: Option<String>,
    /// When it was looked up, ms.
    pub checked_at: Option<i64>,
}

/// One task, epic or story.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Its row number (stable order).
    pub seq: i64,
    /// Its short id (`aj-7f3c`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// Epic, story or task.
    pub kind: TaskKind,
    /// The epic or story it belongs to.
    pub parent_id: Option<String>,
    /// Its title.
    pub title: String,
    /// What it asks.
    pub body: String,
    /// When it's done.
    pub acceptance: String,
    /// How urgent.
    pub priority: Priority,
    /// What was set.
    pub stored: TaskStatus,
    /// What is true ([`effective`]: blocked while an open task blocks it).
    pub status: TaskStatus,
    /// The open tasks that block it.
    pub blocked_by: Vec<String>,
    /// Who has it (a member ref, or [`TASK_USER`]).
    pub assignee_ref: Option<String>,
    /// Their name then.
    pub assignee_name: Option<String>,
    /// Who made it (a member ref, or [`TASK_USER`]).
    pub created_by: String,
    /// Their name then.
    pub created_by_name: String,
    /// When, ms.
    pub created_at: i64,
    /// Its last change, ms.
    pub updated_at: i64,
    /// When it was done or cancelled.
    pub closed_at: Option<i64>,
    /// The issue it comes from.
    pub external: Option<External>,
    /// Items under it, and how many of them are done.
    pub children: (u32, u32),
}

/// A task to write.
#[derive(Clone, Debug)]
pub struct NewTask<'a> {
    /// Its short id (free: [`TasksRepo::id_free`]).
    pub id: &'a str,
    /// Its pool.
    pub pool_id: &'a str,
    /// Its kind.
    pub kind: TaskKind,
    /// Its parent (in the same pool).
    pub parent_id: Option<&'a str>,
    /// Its title (clean).
    pub title: &'a str,
    /// Its body (clean).
    pub body: &'a str,
    /// Its acceptance criteria (clean).
    pub acceptance: &'a str,
    /// How urgent.
    pub priority: Priority,
    /// Where it starts (ready, or to do).
    pub status: TaskStatus,
    /// Who it's given to at once, (ref, name).
    pub assignee: Option<(&'a str, &'a str)>,
    /// Who makes it.
    pub created_by: &'a str,
    /// Their name.
    pub created_by_name: &'a str,
    /// The issue it comes from.
    pub external_url: Option<&'a str>,
    /// The tasks that block it (in the same pool).
    pub blocked_by: &'a [String],
}

/// What a list holds.
#[derive(Clone, Copy, Debug, Default)]
pub struct TaskQuery<'a> {
    /// One pool's.
    pub pool_id: Option<&'a str>,
    /// Those any of these hold.
    pub assignees: Option<&'a [String]>,
    /// Those in these (honest) statuses.
    pub statuses: Option<&'a [TaskStatus]>,
    /// Those of these kinds.
    pub kinds: Option<&'a [TaskKind]>,
    /// Those under this item; `Some("")` for those with no parent.
    pub parent: Option<&'a str>,
    /// Those whose title has these words.
    pub words: Option<&'a str>,
    /// Those after this one in the list's order (the previous page's last).
    pub after: Option<TaskKey>,
}

/// A task's place in a list: ready work first (its honest status's rank),
/// then the most urgent, then the newest. Pages start after one (a keyset
/// cursor: a task added or changed meanwhile never repeats or skips one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskKey {
    /// `TaskStatus::rank` of its honest status.
    pub rank: u8,
    /// `Priority::rank`.
    pub priority: u8,
    /// Its row number, newest first (negated in the order).
    pub seq: i64,
}

impl Task {
    /// Its place in a list.
    #[must_use]
    pub fn key(&self) -> TaskKey {
        TaskKey {
            rank: self.status.rank(),
            priority: self.priority.rank(),
            seq: self.seq,
        }
    }
}

/// A change to a task. `None` leaves a field as it is.
#[derive(Clone, Debug, Default)]
pub struct TaskChange<'a> {
    /// A new title (clean).
    pub title: Option<&'a str>,
    /// A new body (clean).
    pub body: Option<&'a str>,
    /// New acceptance criteria (clean).
    pub acceptance: Option<&'a str>,
    /// A new priority.
    pub priority: Option<Priority>,
    /// A new status.
    pub status: Option<TaskStatus>,
    /// A new parent; `Some(None)` takes it out.
    pub parent: Option<Option<&'a str>>,
    /// A new link to an issue; `Some(None)` removes it.
    pub external_url: Option<Option<&'a str>>,
    /// Its holder gives it back: no one holds it, and it's ready again.
    pub release: bool,
    /// A note with the change (a hand-off's), in the history.
    pub note: Option<&'a str>,
}

/// How a claim went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claim {
    /// It's the claimer's now (or already was).
    Won(Box<Task>),
    /// Someone else has it.
    Taken {
        /// Their name.
        by: String,
    },
    /// Open tasks block it.
    Blocked {
        /// Their ids.
        by: Vec<String>,
    },
    /// It isn't ready (to do, in review, done…), or it's an epic.
    NotReady(TaskStatus),
    /// There's no such task in the pool.
    Gone,
}

/// One link, from a task's side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskLink {
    /// The link's kind.
    pub kind: LinkKind,
    /// `true` when this task is the link's first end (it blocks, it
    /// duplicates…); `false` when it's the second (it's blocked by…).
    pub outgoing: bool,
    /// The other task.
    pub other_id: String,
    /// Its title.
    pub other_title: String,
    /// Its honest status.
    pub other_status: TaskStatus,
}

/// A session that worked a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSession {
    /// The task.
    pub task_id: String,
    /// The session.
    pub session_id: String,
    /// Its Work item, when it had one.
    pub work_id: Option<String>,
    /// `claimed`, `started_for`, `assigned` or `worked`.
    pub how: String,
    /// When, ms.
    pub linked_at: i64,
}

/// One event in a task's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEvent {
    /// Its id (the cursor).
    pub id: i64,
    /// When, ms.
    pub at: i64,
    /// The task.
    pub task_id: String,
    /// Who (a member ref, or [`TASK_USER`]).
    pub actor_ref: String,
    /// Their name then.
    pub actor_name: String,
    /// What happened (`created`, `claimed`, `status`, `assigned`, …).
    pub verb: String,
    /// In words, when there's more to say.
    pub detail: Option<String>,
}

/// Something about tasks that waits on the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskNeed {
    /// Its id (`tneed-…`).
    pub id: String,
    /// Its pool.
    pub pool_id: String,
    /// The task it's about (none for a held new task).
    pub task_id: Option<String>,
    /// `ask` (a change held for the user's OK) or `launch` (a request
    /// for a new session, which only the user starts).
    pub kind: String,
    /// The catalog's word for the action.
    pub action: String,
    /// Who asked (a member ref).
    pub actor_ref: String,
    /// Their name then.
    pub actor_name: String,
    /// The held change, as the app wrote it (JSON).
    pub change: Option<String>,
    /// What it asks, in one line.
    pub summary: String,
    /// When, ms.
    pub created_at: i64,
}

/// Typed access to the E19 tables.
#[derive(Debug)]
pub struct TasksRepo<'a> {
    conn: &'a Connection,
}

/// The honest status of `t`, as SQL.
const EFFECTIVE: &str = "CASE WHEN t.status IN ('todo','ready') AND EXISTS (
        SELECT 1 FROM task_links l JOIN tasks b ON b.id = l.from_id
         WHERE l.to_id = t.id AND l.kind = 'blocks' AND b.status NOT IN ('done','cancelled'))
    THEN 'blocked' ELSE t.status END";

/// The most links one task's read shows (its first ones): a task's
/// answer to the UI stays small however many an agent adds.
pub const LINKS_READ: u32 = 100;
/// The most open blockers a task's row names (its honest status counts
/// them all).
pub const BLOCKERS_SHOWN: usize = 20;

/// The open blockers of `t`, as a JSON array.
const BLOCKERS: &str =
    "(SELECT json_group_array(l.from_id) FROM task_links l JOIN tasks b ON b.id = l.from_id
      WHERE l.to_id = t.id AND l.kind = 'blocks' AND b.status NOT IN ('done','cancelled'))";

/// A list's order: ready work first (as `TaskStatus::rank`).
const RANK: &str = "CASE eff WHEN 'ready' THEN 0 WHEN 'in_progress' THEN 1 WHEN 'in_review' THEN 2
    WHEN 'blocked' THEN 3 WHEN 'todo' THEN 4 WHEN 'done' THEN 5 ELSE 6 END";

fn select_sql() -> String {
    format!(
        "SELECT t.seq, t.id, t.pool_id, t.kind, t.parent_id, t.title, t.body, t.acceptance, t.priority,
                t.status, t.assignee_ref, t.assignee_name, t.created_by, t.created_by_name, t.created_at,
                t.updated_at, t.closed_at, t.external_url, t.external_title, t.external_state,
                t.external_checked_at, {BLOCKERS} AS blockers,
                (SELECT COUNT(*) FROM tasks c WHERE c.parent_id = t.id) AS children,
                (SELECT COUNT(*) FROM tasks c WHERE c.parent_id = t.id AND c.status = 'done') AS done,
                {EFFECTIVE} AS eff
           FROM tasks t"
    )
}

fn task_of(row: &Row<'_>) -> rusqlite::Result<Task> {
    let stored = TaskStatus::stored(&row.get::<_, String>(9)?);
    let mut blocked_by: Vec<String> =
        serde_json::from_str(&row.get::<_, String>(21)?).unwrap_or_default();
    let open_blockers = blocked_by.len();
    blocked_by.truncate(BLOCKERS_SHOWN);
    let url: Option<String> = row.get(17)?;
    Ok(Task {
        seq: row.get(0)?,
        id: row.get(1)?,
        pool_id: row.get(2)?,
        kind: TaskKind::parse(&row.get::<_, String>(3)?).unwrap_or(TaskKind::Task),
        parent_id: row.get(4)?,
        title: row.get(5)?,
        body: row.get(6)?,
        acceptance: row.get(7)?,
        priority: Priority::of_rank(row.get(8)?),
        status: effective(stored, open_blockers),
        stored,
        blocked_by,
        assignee_ref: row.get(10)?,
        assignee_name: row.get(11)?,
        created_by: row.get(12)?,
        created_by_name: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        closed_at: row.get(16)?,
        external: match url {
            Some(url) => Some(External {
                url,
                title: row.get(18)?,
                state: row.get(19)?,
                checked_at: row.get(20)?,
            }),
            None => None,
        },
        children: (row.get(22)?, row.get(23)?),
    })
}

fn json<T: serde::Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_owned())
}

/// The most rows one read returns: the IPC's largest page (200) and the
/// one more it reads to know whether another page follows.
pub const READ_MAX: u32 = 201;

/// A page size within 1..=[`READ_MAX`].
fn page(limit: u32) -> i64 {
    i64::from(limit.clamp(1, READ_MAX))
}

fn collect<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>, CoreError> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn log_in(
    tx: &Connection,
    now: i64,
    pool: &str,
    task: &str,
    actor: &str,
    name: &str,
    verb: &str,
    detail: Option<&str>,
) -> Result<(), CoreError> {
    tx.execute(
        "INSERT INTO task_events (at, pool_id, task_id, actor_ref, actor_name, verb, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![now, pool, task, actor, name, verb, detail],
    )?;
    Ok(())
}

/// The kind and pool of a task, when it's there.
fn kind_and_pool(tx: &Connection, id: &str) -> Result<Option<(TaskKind, String)>, CoreError> {
    Ok(tx
        .query_row(
            "SELECT kind, pool_id FROM tasks WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .map(|(kind, pool)| (TaskKind::parse(&kind).unwrap_or(TaskKind::Task), pool)))
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation {
        subject: "task",
        message: message.into(),
    }
}

/// Whether `parent` may hold an item of `kind` in `pool`, and isn't one
/// of the item's own (a loop).
fn check_parent(
    tx: &Connection,
    pool: &str,
    item: Option<&str>,
    kind: TaskKind,
    parent: &str,
) -> Result<(), CoreError> {
    let Some((parent_kind, parent_pool)) = kind_and_pool(tx, parent)? else {
        return Err(invalid(format!("there's no {parent} in this pool")));
    };
    if parent_pool != pool {
        return Err(invalid(format!("there's no {parent} in this pool")));
    }
    if !kind.parents().contains(&parent_kind) {
        return Err(invalid(format!(
            "a {} goes under {}, not a {}",
            kind.as_str(),
            match kind {
                TaskKind::Story => "an epic",
                _ => "an epic or a story",
            },
            parent_kind.as_str()
        )));
    }
    if let Some(item) = item {
        let mut at = Some(parent.to_owned());
        while let Some(id) = at {
            if id == item {
                return Err(invalid("an item can't go under itself"));
            }
            at = tx
                .query_row(
                    "SELECT parent_id FROM tasks WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
        }
    }
    Ok(())
}

impl TasksRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> TasksRepo<'_> {
        TasksRepo { conn }
    }

    /// Whether no task has this id yet.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn id_free(&self, id: &str) -> Result<bool, CoreError> {
        let taken: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(taken == 0)
    }

    /// Writes a task with its blockers, and logs it, in one transaction.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for a parent or blocker that isn't in the
    /// pool, or a parent of the wrong kind; storage failures (a pool that
    /// isn't there is a constraint error).
    pub fn create(&self, task: &NewTask<'_>) -> Result<Task, CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        if let Some(parent) = task.parent_id {
            check_parent(&tx, task.pool_id, None, task.kind, parent)?;
        }
        tx.execute(
            "INSERT INTO tasks (id, pool_id, kind, parent_id, title, body, acceptance, priority, status,
                 assignee_ref, assignee_name, created_by, created_by_name, created_at, updated_at, external_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15)",
            params![
                task.id,
                task.pool_id,
                task.kind.as_str(),
                task.parent_id,
                task.title,
                task.body,
                task.acceptance,
                task.priority.rank(),
                task.status.as_str(),
                task.assignee.map(|(r, _)| r),
                task.assignee.map(|(_, n)| n),
                task.created_by,
                task.created_by_name,
                now,
                task.external_url,
            ],
        )?;
        log_in(
            &tx,
            now,
            task.pool_id,
            task.id,
            task.created_by,
            task.created_by_name,
            "created",
            Some(task.title),
        )?;
        for blocker in task.blocked_by {
            Self::link_in(
                &tx,
                now,
                blocker,
                task.id,
                LinkKind::Blocks,
                task.created_by,
                task.created_by_name,
            )?;
        }
        tx.commit()?;
        self.get(task.id)?
            .ok_or_else(|| super::accounts::not_found("task", task.id))
    }

    /// A task, when it's there.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn get(&self, id: &str) -> Result<Option<Task>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("{} WHERE t.id = ?1", select_sql()),
                params![id],
                task_of,
            )
            .optional()?)
    }

    /// One page of tasks, ready work first, then by priority, newest
    /// first within each, after `query.after` when it says.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn list(&self, query: &TaskQuery<'_>, limit: u32) -> Result<Vec<Task>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT * FROM (SELECT *, {RANK} AS rk FROM ({}
              WHERE (?1 IS NULL OR t.pool_id = ?1)
                AND (?2 IS NULL OR t.assignee_ref IN (SELECT value FROM json_each(?2)))
                AND (?4 IS NULL OR t.kind IN (SELECT value FROM json_each(?4)))
                AND (?5 IS NULL OR (?5 = '' AND t.parent_id IS NULL) OR t.parent_id = ?5)
                AND (?6 IS NULL OR t.title LIKE '%' || ?6 || '%' ESCAPE '\\')))
              WHERE (?3 IS NULL OR eff IN (SELECT value FROM json_each(?3)))
                AND (?8 IS NULL OR (rk, priority, -seq) > (?8, ?9, ?10))
              ORDER BY rk, priority, seq DESC LIMIT ?7",
            select_sql()
        ))?;
        let statuses = query
            .statuses
            .map(|list| json(&list.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
        let kinds = query
            .kinds
            .map(|list| json(&list.iter().map(|k| k.as_str()).collect::<Vec<_>>()));
        let words = query.words.map(|words| {
            words
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        });
        let rows = stmt.query_map(
            params![
                query.pool_id,
                query.assignees.map(json),
                statuses,
                kinds,
                query.parent,
                words,
                page(limit),
                query.after.map(|key| key.rank),
                query.after.map(|key| key.priority),
                query.after.map(|key| -key.seq),
            ],
            task_of,
        )?;
        collect(rows)
    }

    /// How many items of a pool (of `kinds`, or all) are in each honest
    /// status.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn counts(
        &self,
        pool_id: &str,
        kinds: Option<&[TaskKind]>,
    ) -> Result<Vec<(TaskStatus, u32)>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT eff, COUNT(*) FROM (SELECT {EFFECTIVE} AS eff FROM tasks t WHERE t.pool_id = ?1
                 AND (?2 IS NULL OR t.kind IN (SELECT value FROM json_each(?2))))
              GROUP BY eff"
        ))?;
        let kinds = kinds.map(|list| json(&list.iter().map(|k| k.as_str()).collect::<Vec<_>>()));
        let rows = stmt.query_map(params![pool_id, kinds], |row| {
            Ok((TaskStatus::stored(&row.get::<_, String>(0)?), row.get(1)?))
        })?;
        let mut counts = collect(rows)?;
        counts.sort_by_key(|(status, _)| status.rank());
        Ok(counts)
    }

    /// Takes a task for `who` (ref, name), in one statement: only while
    /// it is ready, no open task blocks it, it isn't an epic and no one
    /// else has it. Its session is linked, and the claim logged.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn claim(
        &self,
        pool_id: &str,
        id: &str,
        refs: &[String],
        who: (&str, &str),
        session: Option<(&str, Option<&str>)>,
    ) -> Result<Claim, CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        let won = tx.execute(
            "UPDATE tasks SET status = 'in_progress', assignee_ref = ?3, assignee_name = ?4, updated_at = ?5
              WHERE id = ?1 AND pool_id = ?2 AND status = 'ready' AND kind != 'epic'
                AND (assignee_ref IS NULL OR assignee_ref IN (SELECT value FROM json_each(?6)))
                AND NOT EXISTS (SELECT 1 FROM task_links l JOIN tasks b ON b.id = l.from_id
                     WHERE l.to_id = ?1 AND l.kind = 'blocks' AND b.status NOT IN ('done','cancelled'))",
            params![id, pool_id, who.0, who.1, now, json(&refs)],
        )? == 1;
        if won {
            log_in(&tx, now, pool_id, id, who.0, who.1, "claimed", None)?;
            if let Some((session, work)) = session {
                Self::link_session_in(&tx, now, id, session, work, "claimed")?;
            }
            tx.commit()?;
        } else {
            drop(tx);
        }
        let Some(task) = self.get(id)?.filter(|task| task.pool_id == pool_id) else {
            return Ok(Claim::Gone);
        };
        let mine = task
            .assignee_ref
            .as_ref()
            .is_some_and(|holder| refs.contains(holder) || holder == who.0);
        Ok(if won || (mine && task.stored == TaskStatus::InProgress) {
            Claim::Won(Box::new(task))
        } else if task.stored == TaskStatus::InProgress && !mine {
            Claim::Taken {
                by: task.assignee_name.unwrap_or_else(|| "someone".to_owned()),
            }
        } else if !task.blocked_by.is_empty() {
            Claim::Blocked {
                by: task.blocked_by,
            }
        } else if task.assignee_ref.is_some() && !mine {
            Claim::Taken {
                by: task.assignee_name.unwrap_or_else(|| "someone".to_owned()),
            }
        } else {
            Claim::NotReady(if task.kind == TaskKind::Epic {
                TaskStatus::Todo
            } else {
                task.status
            })
        })
    }

    /// Changes a task and logs each change, in one transaction.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a task that isn't there;
    /// [`CoreError::Validation`] for a parent that can't hold it, or "in
    /// progress" with no one on it; storage failures.
    pub fn update(
        &self,
        id: &str,
        change: &TaskChange<'_>,
        actor: (&str, &str),
    ) -> Result<Task, CoreError> {
        let now = Timestamp::now().millis();
        let task = self
            .get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))?;
        let tx = self.conn.unchecked_transaction()?;
        let log = |verb: &str, detail: Option<&str>| {
            log_in(&tx, now, &task.pool_id, id, actor.0, actor.1, verb, detail)
        };
        let mut edited = Vec::new();
        for (column, value, word) in [
            ("title", change.title, "title"),
            ("body", change.body, "description"),
            ("acceptance", change.acceptance, "acceptance criteria"),
        ] {
            if let Some(value) = value {
                tx.execute(
                    &format!("UPDATE tasks SET {column} = ?2 WHERE id = ?1"),
                    params![id, value],
                )?;
                edited.push(word);
            }
        }
        if let Some(priority) = change.priority.filter(|p| *p != task.priority) {
            tx.execute(
                "UPDATE tasks SET priority = ?2 WHERE id = ?1",
                params![id, priority.rank()],
            )?;
            edited.push(priority.as_str());
        }
        if let Some(parent) = change.parent {
            if let Some(parent) = parent {
                check_parent(&tx, &task.pool_id, Some(id), task.kind, parent)?;
            }
            tx.execute(
                "UPDATE tasks SET parent_id = ?2 WHERE id = ?1",
                params![id, parent],
            )?;
            log("parent", Some(parent.unwrap_or("none")))?;
        }
        if let Some(url) = change.external_url {
            tx.execute(
                "UPDATE tasks SET external_url = ?2, external_title = NULL, external_state = NULL,
                        external_checked_at = NULL WHERE id = ?1",
                params![id, url],
            )?;
            log("linked_issue", Some(url.unwrap_or("removed")))?;
        }
        if !edited.is_empty() {
            log("edited", Some(&edited.join(", ")))?;
        }
        let mut status = change.status;
        let mut holder = task.assignee_ref.clone();
        if change.release && task.assignee_ref.is_some() {
            tx.execute(
                "UPDATE tasks SET assignee_ref = NULL, assignee_name = NULL WHERE id = ?1",
                params![id],
            )?;
            holder = None;
            log("released", change.note)?;
            if status.is_none() && task.stored.is_open() {
                status = Some(TaskStatus::Ready);
            }
        } else if let Some(note) = change.note {
            log("noted", Some(note))?;
        }
        if let Some(to) = status.filter(|to| *to != task.stored) {
            if to == TaskStatus::InProgress && holder.is_none() && task.kind == TaskKind::Task {
                return Err(invalid("in progress needs someone on it: claim it"));
            }
            tx.execute(
                "UPDATE tasks SET status = ?2,
                        closed_at = CASE WHEN ?2 IN ('done','cancelled') THEN ?3 ELSE NULL END
                  WHERE id = ?1",
                params![id, to.as_str(), now],
            )?;
            log(
                "status",
                Some(&format!("{} → {}", task.status.words(), to.words())),
            )?;
        }
        tx.execute(
            "UPDATE tasks SET updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))
    }

    /// The user's action on a task (D-44), checked against the task as it
    /// is now ([`crate::tasks::may_user`]) and logged as one event, in one
    /// transaction:
    ///
    /// - **Approve**: in review → done.
    /// - **Send back** (`note` needed): in review → in progress, its holder
    ///   kept (ready, when no one holds it).
    /// - **Free**: its holder cleared, only while it's still `seen_holder`
    ///   (the holder whose sessions the caller found not running); in
    ///   progress goes back to ready, any other status stays.
    /// - **Cancel**: open → cancelled.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a task that isn't there;
    /// [`CoreError::Validation`] with the rule, in words, when the task
    /// isn't in a state for it (or its holder changed); storage failures.
    pub fn user_action(
        &self,
        id: &str,
        action: crate::tasks::UserAction,
        note: Option<&str>,
        seen_holder: Option<&str>,
        holder_running: bool,
        actor: (&str, &str),
    ) -> Result<Task, CoreError> {
        use crate::tasks::UserAction;
        let now = Timestamp::now().millis();
        let task = self
            .get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))?;
        crate::tasks::may_user(
            action,
            task.stored,
            task.assignee_ref.is_some(),
            holder_running,
        )
        .map_err(invalid)?;
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        let tx = self.conn.unchecked_transaction()?;
        let (verb, detail) = match action {
            UserAction::Approve | UserAction::Cancel => {
                let to = if action == UserAction::Approve {
                    TaskStatus::Done
                } else {
                    TaskStatus::Cancelled
                };
                tx.execute(
                    "UPDATE tasks SET status = ?2, closed_at = ?3, updated_at = ?3 WHERE id = ?1",
                    params![id, to.as_str(), now],
                )?;
                (action.verb(), note.map(str::to_owned))
            }
            UserAction::SendBack => {
                let note = note.ok_or_else(|| invalid("say what should change: a short note"))?;
                let to = if task.assignee_ref.is_some() {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::Ready
                };
                tx.execute(
                    "UPDATE tasks SET status = ?2, closed_at = NULL, updated_at = ?3 WHERE id = ?1",
                    params![id, to.as_str(), now],
                )?;
                (action.verb(), Some(note.to_owned()))
            }
            UserAction::Free => {
                if task.assignee_ref.as_deref() != seen_holder {
                    return Err(invalid("someone else holds it now: look again"));
                }
                tx.execute(
                    "UPDATE tasks SET assignee_ref = NULL, assignee_name = NULL, updated_at = ?2,
                            status = CASE WHEN status = 'in_progress' THEN 'ready' ELSE status END
                      WHERE id = ?1",
                    params![id, now],
                )?;
                (action.verb(), task.assignee_name.clone())
            }
        };
        log_in(
            &tx,
            now,
            &task.pool_id,
            id,
            actor.0,
            actor.1,
            verb,
            detail.as_deref(),
        )?;
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))
    }

    /// Gives a task to `to` (ref, name), or to no one, and logs it. A task
    /// someone was working goes back to ready for its new holder to start
    /// (or for anyone, with no one): its state stays honest.
    ///
    /// # Errors
    ///
    /// [`CoreError::NotFound`] for a task that isn't there; storage
    /// failures.
    pub fn assign(
        &self,
        id: &str,
        to: Option<(&str, &str)>,
        actor: (&str, &str),
    ) -> Result<Task, CoreError> {
        let now = Timestamp::now().millis();
        let task = self
            .get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))?;
        if task.assignee_ref.as_deref() == to.map(|(r, _)| r) {
            return Ok(task);
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE tasks SET assignee_ref = ?2, assignee_name = ?3, updated_at = ?4,
                    status = CASE WHEN status = 'in_progress' THEN 'ready' ELSE status END
              WHERE id = ?1",
            params![id, to.map(|(r, _)| r), to.map(|(_, n)| n), now],
        )?;
        match to {
            Some((_, name)) => log_in(
                &tx,
                now,
                &task.pool_id,
                id,
                actor.0,
                actor.1,
                "assigned",
                Some(name),
            )?,
            None => log_in(
                &tx,
                now,
                &task.pool_id,
                id,
                actor.0,
                actor.1,
                "unassigned",
                None,
            )?,
        }
        tx.commit()?;
        self.get(id)?
            .ok_or_else(|| super::accounts::not_found("task", id))
    }

    #[allow(clippy::too_many_arguments)]
    fn link_in(
        tx: &Transaction<'_>,
        now: i64,
        from: &str,
        to: &str,
        kind: LinkKind,
        actor: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        let (Some((_, from_pool)), Some((_, to_pool))) =
            (kind_and_pool(tx, from)?, kind_and_pool(tx, to)?)
        else {
            return Err(invalid("both tasks must be in this pool"));
        };
        if from_pool != to_pool {
            return Err(invalid("both tasks must be in this pool"));
        }
        if kind == LinkKind::Blocks {
            let mut stmt =
                tx.prepare("SELECT from_id, to_id FROM task_links WHERE kind = 'blocks'")?;
            let edges = collect(stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?)?;
            if crate::tasks::would_cycle(&edges, from, to) {
                return Err(invalid(format!(
                    "{from} can't block {to}: {to} already waits on {from}"
                )));
            }
        } else if from == to {
            return Err(invalid("a task can't link to itself"));
        }
        let added = tx.execute(
            "INSERT OR IGNORE INTO task_links (from_id, to_id, kind, created_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![from, to, kind.as_str(), now, actor],
        )?;
        if added > 0 {
            log_in(
                tx,
                now,
                &to_pool,
                to,
                actor,
                name,
                "linked",
                Some(&format!("{from} {} {to}", kind.as_str())),
            )?;
        }
        Ok(())
    }

    /// Links two tasks of one pool (`from` blocks, relates to or
    /// duplicates `to`), and logs it on `to`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] for tasks in different pools, a task
    /// linked to itself or a loop of blockers; storage failures.
    pub fn link(
        &self,
        from: &str,
        to: &str,
        kind: LinkKind,
        actor: (&str, &str),
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        Self::link_in(
            &tx,
            Timestamp::now().millis(),
            from,
            to,
            kind,
            actor.0,
            actor.1,
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Takes a link back, and logs it on `to`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn unlink(
        &self,
        from: &str,
        to: &str,
        kind: LinkKind,
        actor: (&str, &str),
    ) -> Result<(), CoreError> {
        let now = Timestamp::now().millis();
        let tx = self.conn.unchecked_transaction()?;
        let removed = tx.execute(
            "DELETE FROM task_links WHERE from_id = ?1 AND to_id = ?2 AND kind = ?3",
            params![from, to, kind.as_str()],
        )?;
        if removed > 0
            && let Some((_, pool)) = kind_and_pool(&tx, to)?
        {
            log_in(
                &tx,
                now,
                &pool,
                to,
                actor.0,
                actor.1,
                "unlinked",
                Some(&format!("{from} {} {to}", kind.as_str())),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A task's links, both ways: the first `LINKS_READ`.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn links(&self, id: &str) -> Result<Vec<TaskLink>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT l.kind, l.from_id = ?1, t.id, t.title, t.status, {EFFECTIVE}
               FROM task_links l JOIN tasks t ON t.id = CASE WHEN l.from_id = ?1 THEN l.to_id ELSE l.from_id END
              WHERE l.from_id = ?1 OR l.to_id = ?1 ORDER BY l.created_at LIMIT ?2"
        ))?;
        collect(stmt.query_map(params![id, LINKS_READ], |row| {
            Ok(TaskLink {
                kind: LinkKind::parse(&row.get::<_, String>(0)?).unwrap_or(LinkKind::RelatesTo),
                outgoing: row.get(1)?,
                other_id: row.get(2)?,
                other_title: row.get(3)?,
                other_status: TaskStatus::stored(&row.get::<_, String>(5)?),
            })
        })?)
    }

    fn link_session_in(
        tx: &Connection,
        now: i64,
        task: &str,
        session: &str,
        work: Option<&str>,
        how: &str,
    ) -> Result<(), CoreError> {
        tx.execute(
            "INSERT OR IGNORE INTO task_sessions (task_id, session_id, work_id, how, linked_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task, session, work, how, now],
        )?;
        Ok(())
    }

    /// Records that a session worked a task (`how`: `claimed`,
    /// `started_for`, `assigned`, `worked`, `switched`: a switch carried
    /// it there); the first word stays.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn link_session(
        &self,
        task: &str,
        session: &str,
        work: Option<&str>,
        how: &str,
    ) -> Result<(), CoreError> {
        Self::link_session_in(
            self.conn,
            Timestamp::now().millis(),
            task,
            session,
            work,
            how,
        )
    }

    /// The sessions that worked a task, first first.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn sessions_of(&self, task: &str) -> Result<Vec<TaskSession>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, session_id, work_id, how, linked_at FROM task_sessions
              WHERE task_id = ?1 ORDER BY linked_at LIMIT 50",
        )?;
        collect(stmt.query_map(params![task], |row| {
            Ok(TaskSession {
                task_id: row.get(0)?,
                session_id: row.get(1)?,
                work_id: row.get(2)?,
                how: row.get(3)?,
                linked_at: row.get(4)?,
            })
        })?)
    }

    /// The tasks each of `sessions` worked: (session, task id, title,
    /// honest status), newest link first.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn of_sessions(
        &self,
        sessions: &[String],
    ) -> Result<Vec<(String, String, String, TaskStatus)>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT s.session_id, t.id, t.title, {EFFECTIVE}
               FROM task_sessions s JOIN tasks t ON t.id = s.task_id
              WHERE s.session_id IN (SELECT value FROM json_each(?1))
              ORDER BY s.linked_at DESC LIMIT 500"
        ))?;
        collect(stmt.query_map(params![json(&sessions)], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                TaskStatus::stored(&row.get::<_, String>(3)?),
            ))
        })?)
    }

    /// The task any of `refs` is on now (in progress or in review), the
    /// latest changed.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn held_by(&self, refs: &[String]) -> Result<Option<Task>, CoreError> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "{} WHERE t.assignee_ref IN (SELECT value FROM json_each(?1))
                       AND t.status IN ('in_progress','in_review','blocked')
                     ORDER BY t.status = 'in_progress' DESC, t.updated_at DESC LIMIT 1",
                    select_sql()
                ),
                params![json(&refs)],
                task_of,
            )
            .optional()?)
    }

    /// Records the issue's title and state as just looked up.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn set_external(
        &self,
        id: &str,
        title: Option<&str>,
        state: Option<&str>,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE tasks SET external_title = ?2, external_state = ?3, external_checked_at = ?4
              WHERE id = ?1 AND external_url IS NOT NULL",
            params![id, title, state, Timestamp::now().millis()],
        )?;
        Ok(())
    }

    /// One page of a task's history, newest first; `before` is the
    /// previous page's last id.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn events(
        &self,
        task: &str,
        before: Option<i64>,
        limit: u32,
    ) -> Result<Vec<TaskEvent>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, task_id, actor_ref, actor_name, verb, detail FROM task_events
              WHERE task_id = ?1 AND (?2 IS NULL OR id < ?2) ORDER BY id DESC LIMIT ?3",
        )?;
        collect(stmt.query_map(params![task, before, page(limit)], |row| {
            Ok(TaskEvent {
                id: row.get(0)?,
                at: row.get(1)?,
                task_id: row.get(2)?,
                actor_ref: row.get(3)?,
                actor_name: row.get(4)?,
                verb: row.get(5)?,
                detail: row.get(6)?,
            })
        })?)
    }

    /// Writes one event in a task's history.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn log(
        &self,
        pool: &str,
        task: &str,
        actor: (&str, &str),
        verb: &str,
        detail: Option<&str>,
    ) -> Result<(), CoreError> {
        log_in(
            self.conn,
            Timestamp::now().millis(),
            pool,
            task,
            actor.0,
            actor.1,
            verb,
            detail,
        )
    }

    /// Records something that waits on the user.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn insert_need(&self, need: &TaskNeed) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO task_needs (id, pool_id, task_id, kind, action, actor_ref, actor_name, change, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                need.id,
                need.pool_id,
                need.task_id,
                need.kind,
                need.action,
                need.actor_ref,
                need.actor_name,
                need.change,
                need.summary,
                need.created_at
            ],
        )?;
        Ok(())
    }

    /// What still waits on the user, oldest first, one page.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn open_needs(&self, offset: u32, limit: u32) -> Result<Vec<TaskNeed>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pool_id, task_id, kind, action, actor_ref, actor_name, change, summary, created_at
               FROM task_needs WHERE answered_at IS NULL ORDER BY seq LIMIT ?1 OFFSET ?2",
        )?;
        collect(stmt.query_map(params![page(limit), offset], |row| {
            Ok(TaskNeed {
                id: row.get(0)?,
                pool_id: row.get(1)?,
                task_id: row.get(2)?,
                kind: row.get(3)?,
                action: row.get(4)?,
                actor_ref: row.get(5)?,
                actor_name: row.get(6)?,
                change: row.get(7)?,
                summary: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?)
    }

    /// One open need, when it's still open.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn open_need(&self, id: &str) -> Result<Option<TaskNeed>, CoreError> {
        let seq: Option<i64> = self
            .conn
            .query_row(
                "SELECT seq FROM task_needs WHERE id = ?1 AND answered_at IS NULL",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(seq) = seq else {
            return Ok(None);
        };
        let offset: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM task_needs WHERE answered_at IS NULL AND seq < ?1",
            params![seq],
            |row| row.get(0),
        )?;
        Ok(self.open_needs(offset, 1)?.into_iter().next())
    }

    /// Whether the same request from the same member is already open (a
    /// repeated ask isn't asked twice).
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn need_open_for(
        &self,
        actor: &str,
        action: &str,
        task: Option<&str>,
    ) -> Result<Option<String>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM task_needs WHERE answered_at IS NULL AND actor_ref = ?1 AND action = ?2
                   AND task_id IS ?3 AND kind = 'launch' LIMIT 1",
                params![actor, action, task],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Closes an open need with the user's answer (and the session started
    /// for it); `false` when it wasn't open.
    ///
    /// # Errors
    ///
    /// Storage failures.
    pub fn close_need(
        &self,
        id: &str,
        answer: &str,
        session: Option<&str>,
    ) -> Result<bool, CoreError> {
        Ok(self.conn.execute(
            "UPDATE task_needs SET answered_at = ?2, answer = ?3, session_id = ?4
              WHERE id = ?1 AND answered_at IS NULL",
            params![id, Timestamp::now().millis(), answer, session],
        )? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn storage() -> (Storage, String) {
        let storage = Storage::open_in_memory().unwrap();
        let pool = storage.pools().create("Team", None).unwrap().id;
        (storage, pool)
    }

    fn new<'a>(
        pool: &'a str,
        id: &'a str,
        kind: TaskKind,
        blocked_by: &'a [String],
    ) -> NewTask<'a> {
        NewTask {
            id,
            pool_id: pool,
            kind,
            parent_id: None,
            title: id,
            body: "",
            acceptance: "",
            priority: Priority::Normal,
            status: TaskStatus::Ready,
            assignee: None,
            created_by: TASK_USER,
            created_by_name: "You",
            external_url: None,
            blocked_by,
        }
    }

    #[test]
    fn ready_is_honest_about_open_blockers() {
        let (storage, pool) = storage();
        let repo = storage.tasks();
        repo.create(&new(&pool, "aj-0001", TaskKind::Task, &[]))
            .unwrap();
        let blocked = vec!["aj-0001".to_owned()];
        let two = repo
            .create(&new(&pool, "aj-0002", TaskKind::Task, &blocked))
            .unwrap();
        assert_eq!(
            (two.stored, two.status),
            (TaskStatus::Ready, TaskStatus::Blocked)
        );
        assert_eq!(two.blocked_by, blocked);
        let ready = repo
            .list(
                &TaskQuery {
                    pool_id: Some(&pool),
                    statuses: Some(&[TaskStatus::Ready]),
                    ..TaskQuery::default()
                },
                50,
            )
            .unwrap();
        assert_eq!(
            ready.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["aj-0001"]
        );
        repo.update(
            "aj-0001",
            &TaskChange {
                status: Some(TaskStatus::Done),
                ..TaskChange::default()
            },
            (TASK_USER, "You"),
        )
        .unwrap();
        let two = repo.get("aj-0002").unwrap().unwrap();
        assert_eq!(two.status, TaskStatus::Ready, "its blocker is done");
        assert!(
            repo.link("aj-0002", "aj-0001", LinkKind::Blocks, (TASK_USER, "You"))
                .is_err(),
            "no loops: aj-0001 already blocks aj-0002"
        );
        assert!(
            repo.link(
                "aj-0002",
                "aj-0002",
                LinkKind::RelatesTo,
                (TASK_USER, "You")
            )
            .is_err()
        );
        repo.link(
            "aj-0002",
            "aj-0001",
            LinkKind::RelatesTo,
            (TASK_USER, "You"),
        )
        .unwrap();
        let links = repo.links("aj-0001").unwrap();
        assert_eq!(links.len(), 2);
        assert!(
            links
                .iter()
                .any(|l| l.kind == LinkKind::Blocks && l.outgoing)
        );
    }

    #[test]
    fn a_tasks_read_stays_small_however_many_links_it_has() {
        let (storage, pool) = storage();
        let repo = storage.tasks();
        let ids: Vec<String> = (0..120).map(|n| format!("aj-{:04x}", 0x100 + n)).collect();
        for id in &ids {
            repo.create(&new(&pool, id, TaskKind::Task, &[])).unwrap();
        }
        repo.create(&new(&pool, "aj-0001", TaskKind::Task, &[]))
            .unwrap();
        for id in &ids {
            repo.link(id, "aj-0001", LinkKind::Blocks, (TASK_USER, "You"))
                .unwrap();
        }
        let task = repo.get("aj-0001").unwrap().unwrap();
        // Its honest status counts every blocker; its row names the first 20.
        assert_eq!(task.status, TaskStatus::Blocked);
        assert_eq!(task.blocked_by.len(), BLOCKERS_SHOWN);
        assert_eq!(repo.links("aj-0001").unwrap().len(), LINKS_READ as usize);
    }

    #[test]
    fn a_claim_takes_a_ready_task_once() {
        let (storage, pool) = storage();
        let repo = storage.tasks();
        repo.create(&new(&pool, "aj-0001", TaskKind::Task, &[]))
            .unwrap();
        let a = vec!["session:a".to_owned()];
        let b = vec!["session:b".to_owned()];
        let won = repo
            .claim(
                &pool,
                "aj-0001",
                &a,
                ("session:a", "Alpha"),
                Some(("a", None)),
            )
            .unwrap();
        assert!(
            matches!(&won, Claim::Won(task) if task.status == TaskStatus::InProgress),
            "{won:?}"
        );
        assert_eq!(
            repo.claim(&pool, "aj-0001", &b, ("session:b", "Beta"), None)
                .unwrap(),
            Claim::Taken { by: "Alpha".into() }
        );
        assert!(matches!(
            repo.claim(&pool, "aj-0001", &a, ("session:a", "Alpha"), None)
                .unwrap(),
            Claim::Won(_)
        ));
        assert_eq!(
            repo.claim(
                "pool-default:none",
                "aj-0001",
                &b,
                ("session:b", "Beta"),
                None
            )
            .unwrap(),
            Claim::Gone,
            "another pool's"
        );
        assert_eq!(repo.sessions_of("aj-0001").unwrap()[0].how, "claimed");
        let events = repo.events("aj-0001", None, 10).unwrap();
        assert_eq!(events[0].verb, "claimed");
        assert!(
            storage
                .raw_conn()
                .execute("DELETE FROM task_events", [])
                .is_err()
        );
        assert!(
            storage
                .raw_conn()
                .execute("UPDATE task_events SET verb = 'x'", [])
                .is_err()
        );
    }

    #[test]
    fn releasing_gives_it_back_and_epics_hold_stories() {
        let (storage, pool) = storage();
        let repo = storage.tasks();
        repo.create(&new(&pool, "aj-e001", TaskKind::Epic, &[]))
            .unwrap();
        let mut story = new(&pool, "aj-5001", TaskKind::Story, &[]);
        story.parent_id = Some("aj-e001");
        repo.create(&story).unwrap();
        let mut task = new(&pool, "aj-0001", TaskKind::Task, &[]);
        task.parent_id = Some("aj-5001");
        repo.create(&task).unwrap();
        let mut wrong = new(&pool, "aj-5002", TaskKind::Story, &[]);
        wrong.parent_id = Some("aj-0001");
        assert!(repo.create(&wrong).is_err(), "a story goes under an epic");
        assert_eq!(repo.get("aj-e001").unwrap().unwrap().children, (1, 0));
        assert!(matches!(
            repo.claim(&pool, "aj-e001", &[], ("session:a", "A"), None)
                .unwrap(),
            Claim::NotReady(_)
        ));
        repo.claim(
            &pool,
            "aj-0001",
            &["session:a".to_owned()],
            ("session:a", "A"),
            None,
        )
        .unwrap();
        let back = repo
            .update(
                "aj-0001",
                &TaskChange {
                    release: true,
                    note: Some("the schema isn't settled"),
                    ..TaskChange::default()
                },
                ("session:a", "A"),
            )
            .unwrap();
        assert_eq!((back.status, back.assignee_ref), (TaskStatus::Ready, None));
        assert_eq!(
            repo.events("aj-0001", None, 1).unwrap()[0].verb,
            "status",
            "released, then ready again"
        );
        assert!(
            repo.update(
                "aj-0001",
                &TaskChange {
                    status: Some(TaskStatus::InProgress),
                    ..TaskChange::default()
                },
                (TASK_USER, "You"),
            )
            .is_err(),
            "in progress needs someone on it"
        );
    }

    #[test]
    fn the_users_actions_move_only_from_their_state_and_log_one_event() {
        use crate::tasks::UserAction;
        let (storage, pool) = storage();
        let repo = storage.tasks();
        let you = (TASK_USER, "You");
        for id in ["aj-0001", "aj-0002", "aj-0003"] {
            repo.create(&new(&pool, id, TaskKind::Task, &[])).unwrap();
            repo.claim(
                &pool,
                id,
                &["session:a".to_owned()],
                ("session:a", "Alpha"),
                None,
            )
            .unwrap();
        }
        let review = |id: &str| {
            repo.update(
                id,
                &TaskChange {
                    status: Some(TaskStatus::InReview),
                    ..TaskChange::default()
                },
                ("session:a", "Alpha"),
            )
            .unwrap()
        };
        // Not in review: neither approved nor sent back.
        for action in [UserAction::Approve, UserAction::SendBack] {
            assert!(
                repo.user_action("aj-0001", action, Some("x"), None, true, you)
                    .is_err(),
                "only from in review"
            );
        }
        review("aj-0001");
        let done = repo
            .user_action("aj-0001", UserAction::Approve, None, None, true, you)
            .unwrap();
        assert_eq!(done.status, TaskStatus::Done);
        assert_eq!(repo.events("aj-0001", None, 1).unwrap()[0].verb, "approved");
        // Send back: a note is needed; the holder keeps it.
        review("aj-0002");
        assert!(
            repo.user_action("aj-0002", UserAction::SendBack, Some(" "), None, true, you)
                .is_err(),
            "a note is needed"
        );
        let back = repo
            .user_action(
                "aj-0002",
                UserAction::SendBack,
                Some("the refunds path still uses floats"),
                None,
                true,
                you,
            )
            .unwrap();
        assert_eq!(back.status, TaskStatus::InProgress);
        assert_eq!(back.assignee_ref.as_deref(), Some("session:a"));
        let event = repo.events("aj-0002", None, 1).unwrap().remove(0);
        assert_eq!(event.verb, "sent_back");
        assert_eq!(
            event.detail.as_deref(),
            Some("the refunds path still uses floats")
        );
        assert_eq!(event.actor_ref, TASK_USER);
        // Free: never while its holder runs, never a holder that changed.
        assert!(
            repo.user_action(
                "aj-0003",
                UserAction::Free,
                None,
                Some("session:a"),
                true,
                you
            )
            .is_err(),
            "its session runs"
        );
        assert!(
            repo.user_action(
                "aj-0003",
                UserAction::Free,
                None,
                Some("session:b"),
                false,
                you
            )
            .is_err(),
            "someone else holds it now"
        );
        let freed = repo
            .user_action(
                "aj-0003",
                UserAction::Free,
                None,
                Some("session:a"),
                false,
                you,
            )
            .unwrap();
        assert_eq!(
            (freed.status, freed.assignee_ref),
            (TaskStatus::Ready, None)
        );
        assert_eq!(repo.events("aj-0003", None, 1).unwrap()[0].verb, "freed");
        assert!(
            repo.user_action("aj-0003", UserAction::Free, None, None, false, you)
                .is_err(),
            "no one holds it"
        );
        // Cancel: any open task, once.
        let cancelled = repo
            .user_action("aj-0003", UserAction::Cancel, None, None, false, you)
            .unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
        assert!(
            repo.user_action("aj-0003", UserAction::Cancel, None, None, false, you)
                .is_err(),
            "closed already"
        );
        assert!(
            repo.user_action("aj-0001", UserAction::Cancel, None, None, false, you)
                .is_err(),
            "done is closed"
        );
    }

    #[test]
    fn a_thousand_tasks_list_quickly() {
        let (storage, pool) = storage();
        let repo = storage.tasks();
        for n in 0..1000 {
            let id = format!("aj-{n:04x}");
            let blocked = if n % 10 == 1 {
                vec![format!("aj-{:04x}", n - 1)]
            } else {
                Vec::new()
            };
            repo.create(&new(&pool, &id, TaskKind::Task, &blocked))
                .unwrap();
        }
        let started = std::time::Instant::now();
        let all = repo
            .list(
                &TaskQuery {
                    pool_id: Some(&pool),
                    ..TaskQuery::default()
                },
                200,
            )
            .unwrap();
        let mut ids: std::collections::HashSet<String> = all.iter().map(|t| t.id.clone()).collect();
        let mut last = all.last().map(Task::key);
        while ids.len() < 1000 {
            let next = repo
                .list(
                    &TaskQuery {
                        pool_id: Some(&pool),
                        after: last,
                        ..TaskQuery::default()
                    },
                    200,
                )
                .unwrap();
            assert!(!next.is_empty());
            for task in &next {
                assert!(ids.insert(task.id.clone()), "{} twice", task.id);
            }
            last = next.last().map(Task::key);
        }
        let seen = ids.len();
        let took = started.elapsed();
        assert_eq!(all[0].status, TaskStatus::Ready, "ready work first");
        assert_eq!(seen, 1000);
        assert!(took < std::time::Duration::from_millis(100), "{took:?}");
        let counts = repo.counts(&pool, None).unwrap();
        assert_eq!(
            counts,
            vec![(TaskStatus::Ready, 900), (TaskStatus::Blocked, 100)]
        );
    }
}
