//! AJ-E3102 (schema v43, v44): what makes every page's counts cheap.
//!
//! - **Indexes on the sessions AI Juggler ran** (`instances`) by project,
//!   launch settings, App and Mac: a page's list or count narrows by an
//!   index, never by reading every row.
//! - **Stamps** (`relation_stamps`): one counter per source the relation
//!   counts are made from, moved by triggers whenever a row that decides a
//!   relation changes, whoever writes it. The app's relation-count cache
//!   compares them on each read, so a count is never older than the rows
//!   it heads (`juggler-app`'s `features::relations`). An update moves a
//!   stamp only when a value that decides a relation really changed (a
//!   session's heartbeat, usage or label never does).
//!   - `sessions`: a session added, removed, or its state, project,
//!     launch settings, App, Mac or conversation changed; a lifecycle
//!     step (archived, reviewed) recorded.
//!   - `folders`: a tool's folder or its launch settings added, removed,
//!     or pointed at another Account, tool or place.
//!   - `projects`: a project added, removed or moved; a worktree's home
//!     learned (it folds the worktree into its repository's project).
//!   - `pools` (v44, AJ-E3104): a pool added or deleted; a member row
//!     added, removed or moved; a session's Work item learned.
//!
//! Additive: nothing existing changes meaning. Each statement is made
//! only when missing, so a database an older build left half way (or a
//! test rewinding its version) runs it again safely.

use std::path::Path;

use rusqlite::{Connection, params};

use crate::error::CoreError;

/// v43: the indexes, the stamps and the triggers that move them.
pub(super) const MIGRATION: &str = "
CREATE INDEX IF NOT EXISTS idx_instances_project ON instances(project);
CREATE INDEX IF NOT EXISTS idx_instances_launch_recipe ON instances(launch_recipe_id);
CREATE INDEX IF NOT EXISTS idx_instances_app ON instances(app_id);
CREATE INDEX IF NOT EXISTS idx_instances_machine ON instances(machine);
CREATE TABLE IF NOT EXISTS relation_stamps (
    source TEXT PRIMARY KEY,
    n      INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO relation_stamps (source) VALUES ('sessions'), ('folders'), ('projects');
CREATE TRIGGER IF NOT EXISTS relation_stamp_instance_added AFTER INSERT ON instances
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'sessions'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_instance_removed AFTER DELETE ON instances
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'sessions'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_instance_changed
AFTER UPDATE OF status, project, launch_recipe_id, app_id, machine, conversation_id ON instances
WHEN OLD.status IS NOT NEW.status OR OLD.project IS NOT NEW.project OR OLD.launch_recipe_id IS NOT NEW.launch_recipe_id OR OLD.app_id IS NOT NEW.app_id OR OLD.machine IS NOT NEW.machine OR OLD.conversation_id IS NOT NEW.conversation_id
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'sessions'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_lifecycle_added AFTER INSERT ON instance_lifecycle
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'sessions'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_lifecycle_removed AFTER DELETE ON instance_lifecycle
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'sessions'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_folder_added AFTER INSERT ON accounts
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_folder_removed AFTER DELETE ON accounts
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_folder_changed
AFTER UPDATE OF provider_id, state_dir, subscription_id, removed_at ON accounts
WHEN OLD.provider_id IS NOT NEW.provider_id OR OLD.state_dir IS NOT NEW.state_dir OR OLD.subscription_id IS NOT NEW.subscription_id OR OLD.removed_at IS NOT NEW.removed_at
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_launch_added AFTER INSERT ON launch_recipes
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_launch_removed AFTER DELETE ON launch_recipes
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_launch_changed AFTER UPDATE OF account_id ON launch_recipes
WHEN OLD.account_id IS NOT NEW.account_id
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'folders'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_project_added AFTER INSERT ON projects
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_project_removed AFTER DELETE ON projects
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_project_moved AFTER UPDATE OF root ON projects
WHEN OLD.root IS NOT NEW.root
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_home_added AFTER INSERT ON worktree_homes
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_home_removed AFTER DELETE ON worktree_homes
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_home_changed AFTER UPDATE OF repo ON worktree_homes
WHEN OLD.repo IS NOT NEW.repo
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'projects'; END;
";

/// v44 (AJ-E3104): the `pools` stamp, moved whenever a session's pool
/// could change: a member row added, removed or moved (whoever writes it:
/// the user, an agent, a deleted pool's members going home), a session's
/// Work item learned (a member row follows the Work item), or a pool
/// added or deleted. Additive, and safe to run again.
pub(super) const POOLS_MIGRATION: &str = "
INSERT OR IGNORE INTO relation_stamps (source) VALUES ('pools');
CREATE TRIGGER IF NOT EXISTS relation_stamp_member_added AFTER INSERT ON pool_members
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_member_removed AFTER DELETE ON pool_members
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_member_changed
AFTER UPDATE OF pool_id, member_ref, left_at ON pool_members
WHEN OLD.pool_id IS NOT NEW.pool_id OR OLD.member_ref IS NOT NEW.member_ref OR OLD.left_at IS NOT NEW.left_at
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_epoch_added AFTER INSERT ON epochs
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_epoch_removed AFTER DELETE ON epochs
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_epoch_changed AFTER UPDATE OF work_id, session_ref ON epochs
WHEN OLD.work_id IS NOT NEW.work_id OR OLD.session_ref IS NOT NEW.session_ref
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_pool_added AFTER INSERT ON pools
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_pool_deleted AFTER UPDATE OF deleted_at ON pools
WHEN OLD.deleted_at IS NOT NEW.deleted_at
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
CREATE TRIGGER IF NOT EXISTS relation_stamp_pool_removed AFTER DELETE ON pools
BEGIN UPDATE relation_stamps SET n = n + 1 WHERE source = 'pools'; END;
";

/// Where each source's rows stand ([`RelationsRepo::stamps`]): any
/// difference from what a count was made from means it's read again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoredStamps {
    /// The sessions AI Juggler ran and their lifecycle.
    pub sessions: i64,
    /// The tools' folders and their launch settings.
    pub folders: i64,
    /// The projects and the worktrees' homes.
    pub projects: i64,
    /// The pools and who is in them (v44).
    pub pools: i64,
}

/// A connection of its own on the database, for the stamps alone. In WAL
/// a reader never waits on a writer, so a count is checked against the
/// committed rows even while the app's connection is busy with a long
/// write: a stale count is never served for want of a lock.
#[derive(Debug)]
pub struct StampReader {
    conn: Connection,
}

/// How long the stamp reader waits on a lock (only a checkpoint's brief
/// one; a writer never holds it up in WAL).
const READER_WAIT: std::time::Duration = std::time::Duration::from_millis(250);

impl StampReader {
    /// A reader on the database at `path` (already opened and migrated by
    /// [`super::Storage::open`]); it only reads.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] when SQLite can't open it.
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let conn = super::open_hardened(path)?;
        conn.busy_timeout(READER_WAIT)?;
        conn.pragma_update(None, "query_only", "ON")?;
        Ok(Self { conn })
    }

    /// Where each source's committed rows stand now.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn stamps(&self) -> Result<StoredStamps, CoreError> {
        RelationsRepo::new(&self.conn).stamps()
    }
}

/// What a relation needs of one session AI Juggler ran, without its row
/// (no JSON read): [`RelationsRepo::session_facets`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFacet {
    /// The session.
    pub id: String,
    /// Its project, when it has one.
    pub project: Option<String>,
    /// Its launch settings.
    pub launch_settings: String,
    /// The Mac it ran on.
    pub machine: String,
    /// Its status as stored (`SessionStatus`'s words).
    pub status: String,
    /// When it was last active (else started), epoch ms.
    pub last_ms: i64,
}

/// An indexed column of `instances` a count can narrow by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceColumn {
    /// `project`.
    Project,
    /// `launch_recipe_id`.
    LaunchSettings,
    /// `app_id`.
    App,
    /// `machine`.
    Machine,
}

impl InstanceColumn {
    /// Every one, each with its own index.
    pub const ALL: [Self; 4] = [
        Self::Project,
        Self::LaunchSettings,
        Self::App,
        Self::Machine,
    ];

    /// Its column's name (never from input).
    #[must_use]
    pub const fn column(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::LaunchSettings => "launch_recipe_id",
            Self::App => "app_id",
            Self::Machine => "machine",
        }
    }

    /// The index the migration made for it.
    #[must_use]
    pub const fn index(self) -> &'static str {
        match self {
            Self::Project => "idx_instances_project",
            Self::LaunchSettings => "idx_instances_launch_recipe",
            Self::App => "idx_instances_app",
            Self::Machine => "idx_instances_machine",
        }
    }
}

/// Reads for the relation counts.
#[derive(Debug)]
pub struct RelationsRepo<'a> {
    conn: &'a Connection,
}

impl RelationsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> RelationsRepo<'_> {
        RelationsRepo { conn }
    }

    /// Where each source stands now: one small read.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn stamps(&self) -> Result<StoredStamps, CoreError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT source, n FROM relation_stamps")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut stamps = StoredStamps::default();
        for row in rows {
            let (source, n) = row?;
            match source.as_str() {
                "sessions" => stamps.sessions = n,
                "folders" => stamps.folders = n,
                "projects" => stamps.projects = n,
                "pools" => stamps.pools = n,
                _ => {}
            }
        }
        Ok(stamps)
    }

    /// Every session's relations and state, one narrow pass (the
    /// relation-count cache's rebuild).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn session_facets(&self) -> Result<Vec<SessionFacet>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, launch_recipe_id, machine, status, \
             COALESCE(last_activity, started_at) FROM instances",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionFacet {
                id: row.get(0)?,
                project: row.get(1)?,
                launch_settings: row.get(2)?,
                machine: row.get(3)?,
                status: row.get(4)?,
                last_ms: row.get(5)?,
            })
        })?;
        rows.map(|row| row.map_err(CoreError::Storage)).collect()
    }

    /// How many sessions have `value` in `column`, by its index (a
    /// reference count beside the cache's).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn count_where(&self, column: InstanceColumn, value: &str) -> Result<u64, CoreError> {
        let n: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM instances WHERE {} = ?1",
                column.column()
            ),
            params![value],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(n).unwrap_or_default())
    }

    /// How SQLite reads [`Self::count_where`] for `column` (its plan's
    /// words): the tests hold each to its index.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn count_plan(&self, column: InstanceColumn) -> Result<String, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "EXPLAIN QUERY PLAN SELECT COUNT(*) FROM instances WHERE {} = ?1",
            column.column()
        ))?;
        let rows = stmt.query_map(params![""], |row| row.get::<_, String>(3))?;
        let mut plan = Vec::new();
        for row in rows {
            plan.push(row?);
        }
        Ok(plan.join("; "))
    }
}
