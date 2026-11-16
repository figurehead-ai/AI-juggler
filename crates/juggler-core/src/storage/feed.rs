//! D-118 (AJ-E3702) — the activity feed's reads: one keyset read per
//! journal the feed shows, newest first, each bounded by its limit. F1
//! reads the journals that exist: notices (what happened, and needs once
//! they're resolved), session endings (`instance_lifecycle`), switches
//! (`switch_edges`), task status changes (`task_events`) and agents'
//! highlight messages (`agent_messages`). The app's feed module
//! (`juggler-app` `features/feed`) merges them into one page; a later
//! journal (F2's gaps, F3's git items) is one more read here and one more
//! source there.
//!
//! - **Keyset, never load-all.** Every read takes the page's start (a
//!   [`FeedKey`]: the time, the source's rank, its row id) and a limit, and
//!   reads at most that many rows past it, by an index on its time
//!   (`FEED_DERIVED`).
//! - **One order.** Newest first; at the same time, the higher source rank
//!   first, then the higher row id: total, so a page never repeats or
//!   skips a row whatever is written meanwhile.
//! - **Read only.** Nothing here writes a journal.

use rusqlite::{Connection, Row, ToSql, params_from_iter};

use crate::error::CoreError;

use super::notices::{COLUMNS as NOTICE_COLUMNS, Notice, notice as notice_row};

/// The indexes the feed's reads walk, made on every open (they only make
/// a read faster: no schema version of their own, so the 0.4 and 0.5
/// lines' numbered migrations are untouched).
pub(super) const FEED_DERIVED: &str = "
CREATE INDEX IF NOT EXISTS idx_notices_resolved ON notices(resolved_at, id);
CREATE INDEX IF NOT EXISTS idx_instance_lifecycle_at ON instance_lifecycle(at_ms);
CREATE INDEX IF NOT EXISTS idx_switch_edges_created ON switch_edges(created_at);
CREATE INDEX IF NOT EXISTS idx_task_events_at ON task_events(at, id);
CREATE INDEX IF NOT EXISTS idx_agent_messages_created ON agent_messages(created_at, seq);
";

/// Each journal's rank: at the same time, the higher rank comes first.
pub mod rank {
    /// `notices`.
    pub const NOTICES: i64 = 1;
    /// `instance_lifecycle` (endings and the user's marks).
    pub const LIFECYCLE: i64 = 2;
    /// `switch_edges`.
    pub const SWITCHES: i64 = 3;
    /// `task_events`.
    pub const TASKS: i64 = 4;
    /// `agent_messages`.
    pub const MESSAGES: i64 = 5;
}

/// A row's place in the feed: newest first, then the higher source rank,
/// then the higher row id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeedKey {
    /// When, epoch ms.
    pub at_ms: i64,
    /// The journal's [`rank`].
    pub source: i64,
    /// The row's id in its journal.
    pub rowid: i64,
}

impl FeedKey {
    /// The bounds a read of `source` takes to start strictly after `self`
    /// in the feed's order: `(time, row id)` for `at < time OR (at = time
    /// AND rowid < row id)`.
    #[must_use]
    pub const fn bounds_for(self, source: i64) -> (i64, i64) {
        let rowid = if source < self.source {
            i64::MAX
        } else if source == self.source {
            self.rowid
        } else {
            i64::MIN
        };
        (self.at_ms, rowid)
    }
}

/// What every read narrows to (none: everything).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedScope {
    /// One project, by id.
    pub project: Option<String>,
    /// One pool, by id.
    pub pool: Option<String>,
    /// The Account folders (`accounts.id`) of one Account or App.
    pub folders: Option<Vec<String>>,
}

/// A session's facts, as its row joins them (what the feed names it by).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedSession {
    /// The session.
    pub id: String,
    /// The name the user gave it.
    pub label: Option<String>,
    /// Its working folder.
    pub cwd: Option<String>,
    /// `terminal` | `desktop`.
    pub kind: Option<String>,
    /// Its Account folder (`accounts.id`).
    pub folder: Option<String>,
    /// That folder's tool (`claude-code`, `codex`…).
    pub provider: Option<String>,
    /// That folder's name.
    pub folder_label: Option<String>,
    /// Its project, by id and name.
    pub project_id: Option<String>,
    /// See `project_id`.
    pub project: Option<String>,
    /// When it started, epoch ms.
    pub started_ms: Option<i64>,
}

/// A notice, with its feed key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedNotice {
    /// Where it sits ([`rank::NOTICES`], at its `resolved_at`).
    pub key: FeedKey,
    /// The notice.
    pub notice: Notice,
    /// The session it names (`data.target.session_id`), when one is kept.
    pub session: Option<FeedSession>,
}

/// One lifecycle event of a session (an ending, or the user's mark).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedLifecycle {
    /// Where it sits.
    pub key: FeedKey,
    /// The event as kept (`ended: error (exit code 1)`, `reviewed`).
    pub event: String,
    /// The session.
    pub session: FeedSession,
}

/// One switch (an edge of the switch graph).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedSwitch {
    /// Where it sits (its `created_at`: the edge keeps no other time
    /// until F2's G2).
    pub key: FeedKey,
    /// The edge.
    pub id: String,
    /// The work it moved: its title and project.
    pub work_title: String,
    /// See `work_title`.
    pub project_id: Option<String>,
    /// See `work_title`.
    pub project: Option<String>,
    /// From and to: the Account folder, its name and its tool.
    pub from_folder: String,
    /// See `from_folder`.
    pub from_label: Option<String>,
    /// See `from_folder`.
    pub from_provider: String,
    /// See `from_folder`.
    pub to_folder: String,
    /// See `from_folder`.
    pub to_label: Option<String>,
    /// See `from_folder`.
    pub to_provider: String,
    /// How it moved (`resume`, `pack`…).
    pub mechanism: String,
    /// `staged` | `verified` | `launched` | `confirmed` | `failed`.
    pub state: String,
    /// Why it failed, in words, when it did.
    pub fail_reason: Option<String>,
    /// What it said it would carry.
    pub fidelity: Option<String>,
}

/// One task status change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedTaskEvent {
    /// Where it sits.
    pub key: FeedKey,
    /// The pool, by id and name.
    pub pool_id: String,
    /// See `pool_id`.
    pub pool: Option<String>,
    /// The task, by id and title.
    pub task_id: String,
    /// See `task_id`.
    pub title: Option<String>,
    /// Who changed it, by ref and name.
    pub actor_ref: String,
    /// See `actor_ref`.
    pub actor_name: String,
    /// The change as logged (`In progress → Done`).
    pub detail: Option<String>,
}

/// One agent message of a highlight kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedMessage {
    /// Where it sits.
    pub key: FeedKey,
    /// The pool, by id and name.
    pub pool_id: String,
    /// See `pool_id`.
    pub pool: Option<String>,
    /// The thread, by id, subject and its messages in all.
    pub thread_id: String,
    /// See `thread_id`.
    pub thread: String,
    /// See `thread_id`.
    pub thread_count: i64,
    /// The sender, by ref, kind (`agent` | `user`) and name.
    pub sender_ref: String,
    /// See `sender_ref`.
    pub sender_kind: String,
    /// See `sender_ref`.
    pub sender_name: String,
    /// `handoff` | `review_request` | `question` | `status`.
    pub kind: String,
    /// Its body (already redacted when it was written).
    pub body: String,
}

/// The feed's reads.
#[derive(Debug)]
pub struct FeedRepo<'a> {
    conn: &'a Connection,
}

impl super::Storage {
    /// The activity feed's keyset reads over the journals (D-118).
    #[must_use]
    pub fn feed(&self) -> FeedRepo<'_> {
        FeedRepo::new(&self.conn)
    }
}

/// A read's SQL, built clause by clause, its values bound in order.
struct Query {
    sql: String,
    values: Vec<Box<dyn ToSql>>,
}

impl Query {
    fn new(select: &str) -> Self {
        Self {
            sql: select.to_owned(),
            values: Vec::new(),
        }
    }

    /// `AND <clause>`, its `?` bound to `values` in turn.
    fn and(&mut self, clause: &str, values: Vec<Box<dyn ToSql>>) {
        self.sql.push_str(" AND ");
        self.sql.push_str(clause);
        self.values.extend(values);
    }

    /// `AND <column> IN (?, ?, …)`; an empty list matches nothing.
    fn and_in(&mut self, column: &str, list: &[String]) {
        if list.is_empty() {
            self.sql.push_str(" AND 0");
            return;
        }
        let marks = vec!["?"; list.len()].join(", ");
        self.and(
            &format!("{column} IN ({marks})"),
            list.iter()
                .map(|value| Box::new(value.clone()) as Box<dyn ToSql>)
                .collect(),
        );
    }

    /// The keyset start and the order, then the limit.
    fn page(
        mut self,
        at: &str,
        rowid: &str,
        before: Option<FeedKey>,
        source: i64,
        limit: usize,
    ) -> Self {
        let (time, row) = before.map_or((i64::MAX, i64::MAX), |key| key.bounds_for(source));
        self.and(
            &format!("({at} < ? OR ({at} = ? AND {rowid} < ?))"),
            vec![Box::new(time), Box::new(time), Box::new(row)],
        );
        for part in [" ORDER BY ", at, " DESC, ", rowid, " DESC LIMIT ?"] {
            self.sql.push_str(part);
        }
        self.values
            .push(Box::new(i64::try_from(limit).unwrap_or(i64::MAX)));
        self
    }

    fn rows<T>(
        &self,
        conn: &Connection,
        map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, CoreError> {
        let mut statement = conn.prepare(&self.sql)?;
        let rows = statement.query_map(params_from_iter(self.values.iter()), map)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }
}

/// The session columns every session read joins (`s`, `a`, `pr`).
const SESSION_COLUMNS: &str = "s.id AS s_id, s.label AS s_label, s.cwd AS s_cwd, s.kind AS s_kind, \
    p.account_id AS s_folder, a.provider_id AS s_provider, a.label AS s_folder_label, \
    s.project AS s_project_id, pr.name AS s_project, s.started_at AS s_started";
const SESSION_JOINS: &str = "LEFT JOIN launch_recipes p ON p.id = s.launch_recipe_id \
    LEFT JOIN accounts a ON a.id = p.account_id LEFT JOIN projects pr ON pr.id = s.project";

fn session_of(row: &Row<'_>) -> rusqlite::Result<Option<FeedSession>> {
    let Some(id) = row.get::<_, Option<String>>("s_id")? else {
        return Ok(None);
    };
    Ok(Some(FeedSession {
        id,
        label: row.get("s_label")?,
        cwd: row.get("s_cwd")?,
        kind: row.get("s_kind")?,
        folder: row.get("s_folder")?,
        provider: row.get("s_provider")?,
        folder_label: row.get("s_folder_label")?,
        project_id: row.get("s_project_id")?,
        project: row.get("s_project")?,
        started_ms: row.get("s_started")?,
    }))
}

/// The project and folders clauses for a read whose session is `s`.
fn session_scope(query: &mut Query, scope: &FeedScope, project: &str) {
    if let Some(id) = &scope.project {
        query.and(&format!("{project} = ?"), vec![Box::new(id.clone())]);
    }
    if let Some(folders) = &scope.folders {
        query.and_in("p.account_id", folders);
    }
}

impl FeedRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> FeedRepo<'_> {
        FeedRepo { conn }
    }

    /// Notices that happened, newest first, after `before`: earlier ones
    /// of the kinds `earlier` (when written), and needs of the kinds
    /// `answered` once resolved. An open need is never here: the feed pins
    /// it from the Needs you list.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn notices(
        &self,
        before: Option<FeedKey>,
        scope: &FeedScope,
        earlier: &[String],
        answered: &[String],
        limit: usize,
    ) -> Result<Vec<FeedNotice>, CoreError> {
        if scope.pool.is_some() && scope.project.is_none() && scope.folders.is_none() {
            // A pool's notices are its messages and tasks: their own reads.
            return Ok(Vec::new());
        }
        let columns = NOTICE_COLUMNS
            .split(", ")
            .map(|column| format!("n.{}", column.trim()))
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = Query::new(&format!(
            "SELECT {columns}, {SESSION_COLUMNS} FROM notices n \
             LEFT JOIN instances s ON s.id = json_extract(n.data, '$.target.session_id') {SESSION_JOINS} \
             WHERE n.resolved_at IS NOT NULL"
        ));
        if earlier.is_empty() && answered.is_empty() {
            return Ok(Vec::new());
        }
        let (earlier_marks, mut values) = in_list(earlier);
        let (answered_marks, answered_values) = in_list(answered);
        values.extend(answered_values);
        query.and(
            &format!("((n.needs = 0 AND n.kind IN ({earlier_marks})) OR (n.needs = 1 AND n.kind IN ({answered_marks})))"),
            values,
        );
        if let Some(id) = &scope.project {
            query.and(
                "(n.project_id = ? OR s.project = ?)",
                vec![Box::new(id.clone()), Box::new(id.clone())],
            );
        }
        if let Some(folders) = &scope.folders {
            let (marks, values) = either_in(folders);
            query.and(
                &format!("(p.account_id IN ({marks}) OR json_extract(n.data, '$.target.account_id') IN ({marks}))"),
                values,
            );
        }
        query
            .page("n.resolved_at", "n.id", before, rank::NOTICES, limit)
            .rows(self.conn, |row| {
                let notice = notice_row(row)?;
                Ok(FeedNotice {
                    key: FeedKey {
                        at_ms: notice.resolved_at.unwrap_or(notice.created_at),
                        source: rank::NOTICES,
                        rowid: notice.id,
                    },
                    session: session_of(row)?,
                    notice,
                })
            })
    }

    /// Session lifecycle events whose text starts with one of `prefixes`
    /// (`ended: finished`, `reviewed`…), newest first, after `before`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn lifecycle(
        &self,
        before: Option<FeedKey>,
        scope: &FeedScope,
        prefixes: &[String],
        limit: usize,
    ) -> Result<Vec<FeedLifecycle>, CoreError> {
        if scope.pool.is_some() || prefixes.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = Query::new(&format!(
            "SELECT l.rowid AS l_rowid, l.event AS l_event, l.at_ms AS l_at, {SESSION_COLUMNS} \
             FROM instance_lifecycle l JOIN instances s ON s.id = l.session_id {SESSION_JOINS} WHERE 1"
        ));
        let likes = vec!["l.event LIKE ? ESCAPE '\\'"; prefixes.len()].join(" OR ");
        query.and(
            &format!("({likes})"),
            prefixes
                .iter()
                .map(|prefix| Box::new(format!("{}%", escape_like(prefix))) as Box<dyn ToSql>)
                .collect(),
        );
        session_scope(&mut query, scope, "s.project");
        query
            .page("l.at_ms", "l.rowid", before, rank::LIFECYCLE, limit)
            .rows(self.conn, |row| {
                Ok(FeedLifecycle {
                    key: FeedKey {
                        at_ms: row.get("l_at")?,
                        source: rank::LIFECYCLE,
                        rowid: row.get("l_rowid")?,
                    },
                    event: row.get("l_event")?,
                    session: session_of(row)?.unwrap_or_default(),
                })
            })
    }

    /// Switches that finished (confirmed) or failed, newest first, after
    /// `before`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn switches(
        &self,
        before: Option<FeedKey>,
        scope: &FeedScope,
        limit: usize,
    ) -> Result<Vec<FeedSwitch>, CoreError> {
        if scope.pool.is_some() {
            return Ok(Vec::new());
        }
        let mut query = Query::new(
            "SELECT e.rowid AS e_rowid, e.id, e.created_at, w.title, w.project, pr.name AS project_name, \
             e.from_account, fa.label AS from_label, e.from_provider, e.to_account, ta.label AS to_label, \
             e.to_provider, e.mechanism, e.state, e.fail_reason, e.fidelity_shown \
             FROM switch_edges e JOIN work_items w ON w.id = e.work_id \
             LEFT JOIN projects pr ON pr.id = w.project \
             LEFT JOIN accounts fa ON fa.id = e.from_account LEFT JOIN accounts ta ON ta.id = e.to_account \
             WHERE e.state IN ('confirmed', 'failed')",
        );
        if let Some(id) = &scope.project {
            query.and("w.project = ?", vec![Box::new(id.clone())]);
        }
        if let Some(folders) = &scope.folders {
            let (marks, values) = either_in(folders);
            query.and(
                &format!("(e.from_account IN ({marks}) OR e.to_account IN ({marks}))"),
                values,
            );
        }
        query
            .page("e.created_at", "e.rowid", before, rank::SWITCHES, limit)
            .rows(self.conn, |row| {
                Ok(FeedSwitch {
                    key: FeedKey {
                        at_ms: row.get("created_at")?,
                        source: rank::SWITCHES,
                        rowid: row.get("e_rowid")?,
                    },
                    id: row.get("id")?,
                    work_title: row.get("title")?,
                    project_id: row.get("project")?,
                    project: row.get("project_name")?,
                    from_folder: row.get("from_account")?,
                    from_label: row.get("from_label")?,
                    from_provider: row.get("from_provider")?,
                    to_folder: row.get("to_account")?,
                    to_label: row.get("to_label")?,
                    to_provider: row.get("to_provider")?,
                    mechanism: row.get("mechanism")?,
                    state: row.get("state")?,
                    fail_reason: row.get("fail_reason")?,
                    fidelity: row.get("fidelity_shown")?,
                })
            })
    }

    /// Task status changes into one of `to` (`Done`, `In review`,
    /// `Blocked`: the words the log keeps), newest first, after `before`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn task_events(
        &self,
        before: Option<FeedKey>,
        scope: &FeedScope,
        to: &[String],
        limit: usize,
    ) -> Result<Vec<FeedTaskEvent>, CoreError> {
        if scope.folders.is_some() || to.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = Query::new(
            "SELECT te.id, te.at, te.pool_id, po.name AS pool, te.task_id, t.title, te.actor_ref, \
             te.actor_name, te.detail FROM task_events te LEFT JOIN tasks t ON t.id = te.task_id \
             LEFT JOIN pools po ON po.id = te.pool_id WHERE te.verb = 'status'",
        );
        let likes = vec!["te.detail LIKE ? ESCAPE '\\'"; to.len()].join(" OR ");
        query.and(
            &format!("({likes})"),
            to.iter()
                .map(|word| Box::new(format!("%→ {}", escape_like(word))) as Box<dyn ToSql>)
                .collect(),
        );
        pool_scope(&mut query, scope, "te.pool_id");
        query
            .page("te.at", "te.id", before, rank::TASKS, limit)
            .rows(self.conn, |row| {
                Ok(FeedTaskEvent {
                    key: FeedKey {
                        at_ms: row.get("at")?,
                        source: rank::TASKS,
                        rowid: row.get("id")?,
                    },
                    pool_id: row.get("pool_id")?,
                    pool: row.get("pool")?,
                    task_id: row.get("task_id")?,
                    title: row.get("title")?,
                    actor_ref: row.get("actor_ref")?,
                    actor_name: row.get("actor_name")?,
                    detail: row.get("detail")?,
                })
            })
    }

    /// Agents' messages of `kinds`, newest first, after `before`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn messages(
        &self,
        before: Option<FeedKey>,
        scope: &FeedScope,
        kinds: &[String],
        limit: usize,
    ) -> Result<Vec<FeedMessage>, CoreError> {
        if scope.folders.is_some() || kinds.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = Query::new(
            "SELECT m.seq, m.created_at, m.pool_id, po.name AS pool, m.thread_id, th.subject AS thread, \
             th.count AS thread_count, m.sender_ref, m.sender_kind, m.sender_name, m.kind, m.body \
             FROM agent_messages m JOIN agent_threads th ON th.id = m.thread_id \
             LEFT JOIN pools po ON po.id = m.pool_id WHERE 1",
        );
        query.and_in("m.kind", kinds);
        pool_scope(&mut query, scope, "m.pool_id");
        query
            .page("m.created_at", "m.seq", before, rank::MESSAGES, limit)
            .rows(self.conn, |row| {
                Ok(FeedMessage {
                    key: FeedKey {
                        at_ms: row.get("created_at")?,
                        source: rank::MESSAGES,
                        rowid: row.get("seq")?,
                    },
                    pool_id: row.get("pool_id")?,
                    pool: row.get("pool")?,
                    thread_id: row.get("thread_id")?,
                    thread: row.get("thread")?,
                    thread_count: row.get("thread_count")?,
                    sender_ref: row.get("sender_ref")?,
                    sender_kind: row.get("sender_kind")?,
                    sender_name: row.get("sender_name")?,
                    kind: row.get("kind")?,
                    body: row.get("body")?,
                })
            })
    }
}

/// The marks of one `IN (…)` list and its values (an empty list: one
/// value no row holds, so it matches nothing).
fn in_list(list: &[String]) -> (String, Vec<Box<dyn ToSql>>) {
    let list: Vec<String> = if list.is_empty() {
        vec![String::new()]
    } else {
        list.to_vec()
    };
    let marks = vec!["?"; list.len()].join(", ");
    (
        marks,
        list.into_iter()
            .map(|value| Box::new(value) as Box<dyn ToSql>)
            .collect(),
    )
}

/// The marks of one `IN (…)` list and its values twice over, for a clause
/// that tests two columns against `list`.
fn either_in(list: &[String]) -> (String, Vec<Box<dyn ToSql>>) {
    let (marks, mut values) = in_list(list);
    values.extend(in_list(list).1);
    (marks, values)
}

/// The pool and project clauses of a pool's journal (`po` is its pool).
fn pool_scope(query: &mut Query, scope: &FeedScope, pool: &str) {
    if let Some(id) = &scope.pool {
        query.and(&format!("{pool} = ?"), vec![Box::new(id.clone())]);
    }
    if let Some(id) = &scope.project {
        query.and("po.project = ?", vec![Box::new(id.clone())]);
    }
}

/// `text` safe inside a `LIKE … ESCAPE '\'` pattern.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::FeedKey;

    #[test]
    fn a_start_names_the_rows_after_it_in_each_source() {
        let key = FeedKey {
            at_ms: 1_000,
            source: 3,
            rowid: 40,
        };
        // A lower rank comes after it at the same time: all of its rows.
        assert_eq!(key.bounds_for(2), (1_000, i64::MAX));
        // The same source: the rows below its id.
        assert_eq!(key.bounds_for(3), (1_000, 40));
        // A higher rank came before it at that time: none of them.
        assert_eq!(key.bounds_for(4), (1_000, i64::MIN));
    }

    #[test]
    fn like_patterns_escape_their_wildcards() {
        assert_eq!(super::escape_like("50%_a\\b"), "50\\%\\_a\\\\b");
    }
}
