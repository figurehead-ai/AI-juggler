//! Sightings (AJ-E2402, D-41a, schema v34): what AI Juggler saw of the
//! processes in sessions other apps run (the Claude app, the Codex app, a
//! terminal `claude`), so a helper that outlives its session can show as
//! "left behind by <session>".
//!
//! - A sighting is a process's pid, start time and boot, its program's
//!   name, the session and tool it belonged to, the app that ran that
//!   session, first and last seen, and a keyed hash of its command line.
//!   **Never the environment, never the command line itself**; the
//!   guardrail test checks the columns.
//! - Kept 7 days after it was last seen ([`SIGHTING_DAYS`]); nothing here
//!   is sent anywhere.
//! - The spawn ledger is never written for these processes (AJ-E1402
//!   Req.3): sightings are their own record.
//!
//! Marks are the user's word on one process: Keep (not a leftover) or
//! Remind me later (until a time). Keyed by the process's identity
//! (pid, start time, boot), so a reused pid is never marked.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;

/// How long a sighting is kept after it was last seen, in days.
pub const SIGHTING_DAYS: i64 = 7;
/// The same, in milliseconds.
pub const SIGHTING_KEEP_MS: i64 = SIGHTING_DAYS * 24 * 3_600_000;

/// v34: sightings of other apps' sessions' processes, and the user's marks
/// on processes (Keep, Remind me later). Both keyed by a process's
/// identity: pid, kernel start time and boot.
pub(super) const MIGRATION: &str = "
CREATE TABLE process_sightings (
    pid             INTEGER NOT NULL,
    start_time      INTEGER NOT NULL,
    boot_id         TEXT NOT NULL,
    exe_name        TEXT NOT NULL,
    cmd_hash        TEXT NOT NULL,
    tool            TEXT NOT NULL,
    session_id      TEXT NOT NULL,
    app             TEXT,
    session_root    INTEGER NOT NULL DEFAULT 0,
    parent_pid      INTEGER,
    first_seen      INTEGER NOT NULL,
    last_seen       INTEGER NOT NULL,
    PRIMARY KEY (pid, start_time, boot_id)
);
CREATE INDEX idx_process_sightings_seen ON process_sightings(last_seen);
CREATE INDEX idx_process_sightings_session ON process_sightings(session_id, boot_id);
CREATE TABLE process_marks (
    pid        INTEGER NOT NULL,
    start_time INTEGER NOT NULL,
    boot_id    TEXT NOT NULL,
    mark       TEXT NOT NULL,
    until_ms   INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (pid, start_time, boot_id)
);
";

/// The columns a sighting has, in order: the guardrail test compares the
/// table against this list, so a new column is a reviewed change.
pub const SIGHTING_COLUMNS: &[&str] = &[
    "pid",
    "start_time",
    "boot_id",
    "exe_name",
    "cmd_hash",
    "tool",
    "session_id",
    "app",
    "session_root",
    "parent_pid",
    "first_seen",
    "last_seen",
];

/// One process of a session another app runs, as AI Juggler saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sighting {
    /// Its pid.
    pub pid: u32,
    /// Its kernel start time, epoch seconds.
    pub start_time: i64,
    /// The boot it ran in (the OS boot time, epoch seconds, as text).
    pub boot_id: String,
    /// Its program's name (the executable's file name), never its path's
    /// folders or its arguments.
    pub exe_name: String,
    /// A keyed hash of its command line (hex); the line itself is never
    /// stored.
    pub cmd_hash: String,
    /// The tool of the session it belonged to (its adapter's id).
    pub tool: String,
    /// The tool's id for that session.
    pub session_id: String,
    /// The app that ran the session, by name, when known.
    pub app: Option<String>,
    /// It was the session's own process (the tool itself), not a helper.
    pub session_root: bool,
    /// Its parent when first seen.
    pub parent_pid: Option<u32>,
    /// First seen, epoch ms.
    pub first_seen: i64,
    /// Last seen alive, epoch ms.
    pub last_seen: i64,
}

/// The user's word on one process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    /// Keep: it isn't a leftover; never offered for cleanup, never tidied.
    Keep,
    /// Remind me later: out of Needs attention until a time.
    Snooze,
}

impl MarkKind {
    /// As stored.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Snooze => "snooze",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "keep" => Some(Self::Keep),
            "snooze" => Some(Self::Snooze),
            _ => None,
        }
    }
}

/// One mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessMark {
    /// The process's pid.
    pub pid: u32,
    /// Its kernel start time, epoch seconds.
    pub start_time: i64,
    /// Its boot.
    pub boot_id: String,
    /// Keep or Remind me later.
    pub kind: MarkKind,
    /// Until when, epoch ms (Remind me later).
    pub until_ms: Option<i64>,
    /// When the user chose it, epoch ms.
    pub created_at: i64,
}

/// Typed access to the sightings and marks tables.
#[derive(Debug)]
pub struct SightingsRepo<'a> {
    conn: &'a Connection,
}

fn sighting_of(row: &Row<'_>) -> rusqlite::Result<Sighting> {
    Ok(Sighting {
        pid: row.get("pid")?,
        start_time: row.get("start_time")?,
        boot_id: row.get("boot_id")?,
        exe_name: row.get("exe_name")?,
        cmd_hash: row.get("cmd_hash")?,
        tool: row.get("tool")?,
        session_id: row.get("session_id")?,
        app: row.get("app")?,
        session_root: row.get::<_, i64>("session_root")? != 0,
        parent_pid: row.get("parent_pid")?,
        first_seen: row.get("first_seen")?,
        last_seen: row.get("last_seen")?,
    })
}

fn mark_of(row: &Row<'_>) -> rusqlite::Result<Option<ProcessMark>> {
    let kind: String = row.get("mark")?;
    Ok(MarkKind::parse(&kind).map(|kind| ProcessMark {
        pid: row.get("pid").unwrap_or_default(),
        start_time: row.get("start_time").unwrap_or_default(),
        boot_id: row.get("boot_id").unwrap_or_default(),
        kind,
        until_ms: row.get("until_ms").unwrap_or_default(),
        created_at: row.get("created_at").unwrap_or_default(),
    }))
}

impl<'a> SightingsRepo<'a> {
    /// Wraps the store's connection.
    #[must_use]
    pub const fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Records new sightings and moves known ones' last seen to theirs, in
    /// one transaction. A known sighting keeps its first seen, its session
    /// and its hash (a process doesn't change sessions).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn record(&self, sightings: &[Sighting]) -> Result<(), CoreError> {
        if sightings.is_empty() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO process_sightings
                    (pid, start_time, boot_id, exe_name, cmd_hash, tool, session_id, app,
                     session_root, parent_pid, first_seen, last_seen)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT (pid, start_time, boot_id)
                 DO UPDATE SET last_seen = MAX(last_seen, excluded.last_seen)",
            )?;
            for sighting in sightings {
                stmt.execute(params![
                    sighting.pid,
                    sighting.start_time,
                    sighting.boot_id,
                    sighting.exe_name,
                    sighting.cmd_hash,
                    sighting.tool,
                    sighting.session_id,
                    sighting.app,
                    i64::from(sighting.session_root),
                    sighting.parent_pid,
                    sighting.first_seen,
                    sighting.last_seen,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Every sighting of `boot_id` last seen at or after `since_ms` (the
    /// ones whose process may still run).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn seen_since(&self, boot_id: &str, since_ms: i64) -> Result<Vec<Sighting>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM process_sightings WHERE boot_id = ?1 AND last_seen >= ?2")?;
        let rows = stmt
            .query_map(params![boot_id, since_ms], sighting_of)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// One process's sighting, by its identity.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn get(
        &self,
        pid: u32,
        start_time: i64,
        boot_id: &str,
    ) -> Result<Option<Sighting>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT * FROM process_sightings
                 WHERE pid = ?1 AND start_time = ?2 AND boot_id = ?3",
                params![pid, start_time, boot_id],
                sighting_of,
            )
            .optional()?)
    }

    /// The latest sighting of `pid` in `boot_id`, whatever its start time
    /// (the stop path then checks the live process against it).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn latest_of(&self, pid: u32, boot_id: &str) -> Result<Option<Sighting>, CoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT * FROM process_sightings WHERE pid = ?1 AND boot_id = ?2
                 ORDER BY start_time DESC LIMIT 1",
                params![pid, boot_id],
                sighting_of,
            )
            .optional()?)
    }

    /// One page of sightings last seen at or after `since_ms`, newest last
    /// seen first, after `after` (`(last_seen, pid)` of the previous
    /// page's last row).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn page(
        &self,
        since_ms: i64,
        after: Option<(i64, u32)>,
        limit: usize,
    ) -> Result<Vec<Sighting>, CoreError> {
        let (seen, pid) = after.unwrap_or((i64::MAX, u32::MAX));
        let mut stmt = self.conn.prepare(
            "SELECT * FROM process_sightings
             WHERE last_seen >= ?1 AND (last_seen < ?2 OR (last_seen = ?2 AND pid < ?3))
             ORDER BY last_seen DESC, pid DESC
             LIMIT ?4",
        )?;
        let rows = stmt
            .query_map(
                params![
                    since_ms,
                    seen,
                    pid,
                    i64::try_from(limit).unwrap_or(i64::MAX)
                ],
                sighting_of,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The session's own process(es) as seen in `boot_id`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn session_roots(
        &self,
        session_id: &str,
        boot_id: &str,
    ) -> Result<Vec<Sighting>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM process_sightings
             WHERE session_id = ?1 AND boot_id = ?2 AND session_root = 1",
        )?;
        let rows = stmt
            .query_map(params![session_id, boot_id], sighting_of)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Forgets sightings last seen before `before_ms`, and marks made
    /// before it or whose time has passed. Returns the sightings removed.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn prune(&self, before_ms: i64, now_ms: i64) -> Result<usize, CoreError> {
        let removed = self.conn.execute(
            "DELETE FROM process_sightings WHERE last_seen < ?1",
            params![before_ms],
        )?;
        self.conn.execute(
            "DELETE FROM process_marks
             WHERE created_at < ?1 OR (until_ms IS NOT NULL AND until_ms < ?2)",
            params![before_ms, now_ms],
        )?;
        Ok(removed)
    }

    /// How many sightings are kept.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn count(&self) -> Result<u64, CoreError> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM process_sightings", [], |row| {
                    row.get(0)
                })?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    /// Sets the user's mark on one process (replacing any).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn set_mark(&self, mark: &ProcessMark) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO process_marks (pid, start_time, boot_id, mark, until_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (pid, start_time, boot_id)
             DO UPDATE SET mark = excluded.mark, until_ms = excluded.until_ms,
                           created_at = excluded.created_at",
            params![
                mark.pid,
                mark.start_time,
                mark.boot_id,
                mark.kind.as_str(),
                mark.until_ms,
                mark.created_at
            ],
        )?;
        Ok(())
    }

    /// Clears the user's mark on one process.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn clear_mark(&self, pid: u32, start_time: i64, boot_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM process_marks WHERE pid = ?1 AND start_time = ?2 AND boot_id = ?3",
            params![pid, start_time, boot_id],
        )?;
        Ok(())
    }

    /// Every mark of `boot_id` still in force at `now_ms`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn marks(&self, boot_id: &str, now_ms: i64) -> Result<Vec<ProcessMark>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM process_marks
             WHERE boot_id = ?1 AND (until_ms IS NULL OR until_ms >= ?2)",
        )?;
        let rows = stmt
            .query_map(params![boot_id, now_ms], mark_of)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().flatten().collect())
    }
}
