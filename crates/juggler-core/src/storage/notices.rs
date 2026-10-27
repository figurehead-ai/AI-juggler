//! Notices (AJ-E2501, schema v37): the history of the one notification
//! model — what needs the user now, and what happened (a reset, a finish,
//! a crash and its restart, an update).
//!
//! - **Two sections.** A notice that *needs* the user stays open while its
//!   condition holds (a session asking, an account almost out of room)
//!   and is resolved when it ends; one open row per key at a time (the
//!   partial unique index), so the same need is never listed twice. An
//!   *earlier* notice is something that happened: it is told once — a
//!   key already written is never written again while it is kept.
//! - **Read, snoozed, dismissed** are the user's marks on a row; a
//!   **mute** (a project, a kind) is the user's word on what may
//!   interrupt. None of them changes what needs the user: the count
//!   stays the Needs you count.
//! - **Kept [`NOTICE_KEEP_DAYS`] days**, then purged ([`NoticesRepo::purge`]).
//! - Words only: titles and details are plain sentences the app wrote;
//!   `data` holds what a click opens and the facts a line is drawn from
//!   (a JSON object). Nothing here is sent anywhere.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;

/// How long a notice is kept, in days.
pub const NOTICE_KEEP_DAYS: i64 = 30;
/// The same, in milliseconds.
pub const NOTICE_KEEP_MS: i64 = NOTICE_KEEP_DAYS * 24 * 3_600_000;

/// v37: notices and the user's mutes.
pub(super) const MIGRATION: &str = "
CREATE TABLE notices (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    key           TEXT NOT NULL,
    kind          TEXT NOT NULL,
    origin        TEXT NOT NULL,
    needs         INTEGER NOT NULL,
    title         TEXT NOT NULL,
    detail        TEXT NOT NULL DEFAULT '',
    project_id    TEXT,
    project       TEXT,
    data          TEXT NOT NULL DEFAULT '{}',
    action        TEXT,
    meter         INTEGER,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    read_at       INTEGER,
    resolved_at   INTEGER,
    snoozed_until INTEGER,
    dismissed_at  INTEGER
);
CREATE UNIQUE INDEX idx_notices_open ON notices(key) WHERE resolved_at IS NULL;
CREATE INDEX idx_notices_key ON notices(key);
CREATE INDEX idx_notices_created ON notices(created_at, id);
CREATE TABLE notice_mutes (
    scope      TEXT NOT NULL,
    value      TEXT NOT NULL,
    label      TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (scope, value)
);
";

/// A notice to write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewNotice {
    /// One per condition (a need) or per happening (an earlier notice).
    pub key: String,
    /// What it is about (`asks`, `quota`, `finished`, …).
    pub kind: String,
    /// Who keeps it in step: `needs` (the Needs you list) or `app` (AI
    /// Juggler's own sources).
    pub origin: String,
    /// It needs the user while open (else it happened: Earlier).
    pub needs: bool,
    /// The line, in plain words.
    pub title: String,
    /// The second line.
    pub detail: String,
    /// The project it belongs to, when one does (for Mute this project).
    pub project_id: Option<String>,
    /// That project's name.
    pub project: Option<String>,
    /// What a click opens and the facts its line is drawn from (JSON).
    pub data: String,
    /// The one primary action's id.
    pub action: Option<String>,
    /// A meter, in percent (room left on an account).
    pub meter: Option<i64>,
}

/// One notice as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// Row id (newer rows have larger ids).
    pub id: i64,
    /// See [`NewNotice::key`].
    pub key: String,
    /// See [`NewNotice::kind`].
    pub kind: String,
    /// See [`NewNotice::origin`].
    pub origin: String,
    /// See [`NewNotice::needs`].
    pub needs: bool,
    /// See [`NewNotice::title`].
    pub title: String,
    /// See [`NewNotice::detail`].
    pub detail: String,
    /// See [`NewNotice::project_id`].
    pub project_id: Option<String>,
    /// See [`NewNotice::project`].
    pub project: Option<String>,
    /// See [`NewNotice::data`].
    pub data: String,
    /// See [`NewNotice::action`].
    pub action: Option<String>,
    /// See [`NewNotice::meter`].
    pub meter: Option<i64>,
    /// First written, epoch ms.
    pub created_at: i64,
    /// Last changed, epoch ms.
    pub updated_at: i64,
    /// When the user read it.
    pub read_at: Option<i64>,
    /// When its condition ended (a need), or when it was written (earlier).
    pub resolved_at: Option<i64>,
    /// Quiet until then.
    pub snoozed_until: Option<i64>,
    /// Its banner was closed then (it stays here).
    pub dismissed_at: Option<i64>,
}

/// One mute: a project or a kind that may not interrupt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeMute {
    /// `project` | `kind`.
    pub scope: String,
    /// The project's id, or the kind.
    pub value: String,
    /// How Settings names it.
    pub label: String,
    /// Muted at, epoch ms.
    pub created_at: i64,
}

const COLUMNS: &str = "id, key, kind, origin, needs, title, detail, project_id, project, data, \
    action, meter, created_at, updated_at, read_at, resolved_at, snoozed_until, dismissed_at";

fn notice(row: &Row<'_>) -> rusqlite::Result<Notice> {
    Ok(Notice {
        id: row.get("id")?,
        key: row.get("key")?,
        kind: row.get("kind")?,
        origin: row.get("origin")?,
        needs: row.get::<_, i64>("needs")? != 0,
        title: row.get("title")?,
        detail: row.get("detail")?,
        project_id: row.get("project_id")?,
        project: row.get("project")?,
        data: row.get("data")?,
        action: row.get("action")?,
        meter: row.get("meter")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        read_at: row.get("read_at")?,
        resolved_at: row.get("resolved_at")?,
        snoozed_until: row.get("snoozed_until")?,
        dismissed_at: row.get("dismissed_at")?,
    })
}

/// What writing a need did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Written {
    /// A new row (unread).
    New(i64),
    /// The open row's words or facts changed.
    Changed(i64),
    /// Nothing changed.
    Same(i64),
}

impl Written {
    /// The row's id.
    #[must_use]
    pub const fn id(self) -> i64 {
        match self {
            Self::New(id) | Self::Changed(id) | Self::Same(id) => id,
        }
    }
}

/// Typed access to the `notices` and `notice_mutes` tables.
#[derive(Debug)]
pub struct NoticesRepo<'a> {
    conn: &'a Connection,
}

impl NoticesRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> NoticesRepo<'_> {
        NoticesRepo { conn }
    }

    fn insert(
        &self,
        notice: &NewNotice,
        now: i64,
        resolved: Option<i64>,
    ) -> Result<i64, CoreError> {
        self.conn.execute(
            "INSERT INTO notices (key, kind, origin, needs, title, detail, project_id, project, \
             data, action, meter, created_at, updated_at, resolved_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?13)",
            params![
                notice.key,
                notice.kind,
                notice.origin,
                i64::from(notice.needs),
                notice.title,
                notice.detail,
                notice.project_id,
                notice.project,
                notice.data,
                notice.action,
                notice.meter,
                now,
                resolved,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Writes something that happened, once: a key already kept is never
    /// written again. `Ok(None)` when it was already told.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn record(&self, notice: &NewNotice, now: i64) -> Result<Option<i64>, CoreError> {
        let told: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM notices WHERE key = ?1)",
            params![notice.key],
            |row| row.get(0),
        )?;
        if told {
            return Ok(None);
        }
        let earlier = NewNotice {
            needs: false,
            ..notice.clone()
        };
        self.insert(&earlier, now, Some(now)).map(Some)
    }

    /// Keeps a need open: a new row the first time its key is seen open,
    /// else the open row's words and facts brought up to date (its read,
    /// snooze and dismiss marks kept).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open(&self, notice: &NewNotice, now: i64) -> Result<Written, CoreError> {
        let open = self.open_by_key(&notice.key)?;
        let Some(open) = open else {
            let need = NewNotice {
                needs: true,
                ..notice.clone()
            };
            return self.insert(&need, now, None).map(Written::New);
        };
        let same = open.title == notice.title
            && open.detail == notice.detail
            && open.project_id == notice.project_id
            && open.project == notice.project
            && open.data == notice.data
            && open.action == notice.action
            && open.meter == notice.meter
            && open.kind == notice.kind;
        if same {
            return Ok(Written::Same(open.id));
        }
        self.conn.execute(
            "UPDATE notices SET kind = ?2, title = ?3, detail = ?4, project_id = ?5, project = ?6, \
             data = ?7, action = ?8, meter = ?9, updated_at = ?10 WHERE id = ?1",
            params![
                open.id,
                notice.kind,
                notice.title,
                notice.detail,
                notice.project_id,
                notice.project,
                notice.data,
                notice.action,
                notice.meter,
                now,
            ],
        )?;
        Ok(Written::Changed(open.id))
    }

    /// The open need with this key.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_by_key(&self, key: &str) -> Result<Option<Notice>, CoreError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM notices WHERE key = ?1 AND resolved_at IS NULL"),
                params![key],
                notice,
            )
            .optional()
            .map_err(CoreError::Storage)
    }

    /// Ends the open need with this key; `true` when one was open.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn resolve(&self, key: &str, now: i64) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "UPDATE notices SET resolved_at = ?2, updated_at = ?2 WHERE key = ?1 AND resolved_at IS NULL",
            params![key, now],
        )?;
        Ok(changed > 0)
    }

    /// Ends every open need of `origin` whose key isn't in `keep` (the
    /// list it keeps in step with no longer has them). Returns how many.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn resolve_missing(
        &self,
        origin: &str,
        keep: &[String],
        now: i64,
    ) -> Result<usize, CoreError> {
        let open = self.open_needs()?;
        let mut ended = 0;
        for row in open
            .iter()
            .filter(|row| row.origin == origin && !keep.contains(&row.key))
        {
            ended += self.conn.execute(
                "UPDATE notices SET resolved_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![row.id, now],
            )?;
        }
        Ok(ended)
    }

    /// Every open need, oldest first (the list is bounded by what is
    /// actually waiting on the user).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_needs(&self) -> Result<Vec<Notice>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM notices WHERE needs = 1 AND resolved_at IS NULL ORDER BY created_at, id"
        ))?;
        let rows = stmt.query_map([], notice)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// How many needs are open (the Needs you count).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn open_count(&self) -> Result<u64, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notices WHERE needs = 1 AND resolved_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// One page of what happened, newest first: `limit + 1` rows after
    /// `after` (`(created_at, id)` of the last row shown), so the caller
    /// knows whether another page follows.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn earlier(
        &self,
        after: Option<(i64, i64)>,
        limit: usize,
    ) -> Result<Vec<Notice>, CoreError> {
        let (at, id) = after.unwrap_or((i64::MAX, i64::MAX));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM notices WHERE needs = 0 \
             AND (created_at < ?1 OR (created_at = ?1 AND id < ?2)) \
             ORDER BY created_at DESC, id DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                at,
                id,
                i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX)
            ],
            notice,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// How many earlier notices are kept.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn earlier_count(&self) -> Result<u64, CoreError> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM notices WHERE needs = 0", [], |row| {
                    row.get(0)
                })?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// What happened since `since` (epoch ms), by kind: `(kind, count)`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn earlier_kinds_since(&self, since: i64) -> Result<Vec<(String, u64)>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT kind, COUNT(*) AS n FROM notices WHERE needs = 0 AND created_at >= ?1 \
             GROUP BY kind ORDER BY kind",
        )?;
        let rows = stmt.query_map(params![since], |row| {
            Ok((
                row.get::<_, String>("kind")?,
                u64::try_from(row.get::<_, i64>("n")?).unwrap_or(0),
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// One notice by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, id: i64) -> Result<Option<Notice>, CoreError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM notices WHERE id = ?1"),
                params![id],
                notice,
            )
            .optional()
            .map_err(CoreError::Storage)
    }

    /// Marks one notice read; `false` when there is no such notice.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_read(&self, id: i64, now: i64) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "UPDATE notices SET read_at = COALESCE(read_at, ?2) WHERE id = ?1",
            params![id, now],
        )?;
        Ok(changed > 0)
    }

    /// Marks everything read. Returns how many were unread.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mark_all_read(&self, now: i64) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "UPDATE notices SET read_at = ?1 WHERE read_at IS NULL",
            params![now],
        )?)
    }

    /// Quiet until `until` (epoch ms); `None` wakes it.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn snooze(&self, id: i64, until: Option<i64>) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "UPDATE notices SET snoozed_until = ?2 WHERE id = ?1",
            params![id, until],
        )?;
        Ok(changed > 0)
    }

    /// Closes its banner (the notice stays in the centre).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn dismiss(&self, id: i64, now: i64) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "UPDATE notices SET dismissed_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(changed > 0)
    }

    /// Forgets what is older than `before` (epoch ms): earlier notices
    /// written then, and needs that ended then. An open need is never
    /// forgotten, however old. Returns how many rows went.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn purge(&self, before: i64) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "DELETE FROM notices WHERE resolved_at IS NOT NULL AND resolved_at < ?1",
            params![before],
        )?)
    }

    /// Every mute, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mutes(&self) -> Result<Vec<NoticeMute>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT scope, value, label, created_at FROM notice_mutes ORDER BY created_at, scope, value",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(NoticeMute {
                scope: row.get("scope")?,
                value: row.get("value")?,
                label: row.get("label")?,
                created_at: row.get("created_at")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CoreError::Storage)
    }

    /// Mutes a project or a kind (again: its label is brought up to date).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn mute(&self, scope: &str, value: &str, label: &str, now: i64) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO notice_mutes (scope, value, label, created_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(scope, value) DO UPDATE SET label = excluded.label",
            params![scope, value, label, now],
        )?;
        Ok(())
    }

    /// Lets a project or a kind interrupt again; `false` when it wasn't muted.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn unmute(&self, scope: &str, value: &str) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "DELETE FROM notice_mutes WHERE scope = ?1 AND value = ?2",
            params![scope, value],
        )?;
        Ok(changed > 0)
    }
}
