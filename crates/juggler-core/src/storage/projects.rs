//! Project repository — the juggling anchor (ADR-0007).
//!
//! Projects are keyed by canonical root: launching an instance in a
//! directory "touches" its project (created on first use, activity
//! refreshed on every launch), so the UI can always answer "which
//! instances and accounts have worked here, and which is on it now".

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::CoreError;
use crate::model::{ProjectHistory, ProjectId, ProjectRecord, Timestamp};

/// Typed read/write access to the `projects` table.
#[derive(Debug)]
pub struct ProjectsRepo<'a> {
    conn: &'a Connection,
}

impl ProjectsRepo<'_> {
    /// Wraps an open connection.
    #[must_use]
    pub fn new(conn: &Connection) -> ProjectsRepo<'_> {
        ProjectsRepo { conn }
    }

    /// Creates-or-refreshes the project for a canonical `root` and returns
    /// it. `name` defaults to the final path component when creating.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures or when `root`
    /// has no final component usable as a name.
    pub fn touch(&self, root: &Path, name: Option<&str>) -> Result<ProjectRecord, CoreError> {
        let root_str = root.to_string_lossy().into_owned();
        let existing = self.by_root(root)?;
        let now = Timestamp::now();
        let record = if let Some(project) = existing {
            self.conn.execute(
                "UPDATE projects SET last_activity = ?2 WHERE id = ?1",
                params![project.id.as_str(), now.millis()],
            )?;
            ProjectRecord {
                last_activity: Some(now),
                ..project
            }
        } else {
            let fallback_name = root
                .file_name()
                .map_or_else(|| root_str.clone(), |n| n.to_string_lossy().into_owned());
            let name = name.unwrap_or(&fallback_name).to_string();
            let record = ProjectRecord {
                id: ProjectId::new(format!("proj-{}", suffix(now))),
                root: root.to_path_buf(),
                name,
                created_at: now,
                last_activity: Some(now),
                history: ProjectHistory::default(),
            };
            self.conn.execute(
                "INSERT INTO projects (id, root, name, created_at, last_activity)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    record.id.as_str(),
                    root_str,
                    record.name,
                    record.created_at.millis(),
                    record.last_activity.map(Timestamp::millis),
                ],
            )?;
            record
        };
        Ok(record)
    }

    /// Fetches a project by canonical root.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures or
    /// [`CoreError::Serialization`] for corrupt rows.
    pub fn by_root(&self, root: &Path) -> Result<Option<ProjectRecord>, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM projects WHERE root = ?1",
                params![root.to_string_lossy()],
                map_project,
            )
            .optional()
            .map_err(CoreError::from)
    }

    /// Fetches a project by id.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn get(&self, id: &ProjectId) -> Result<ProjectRecord, CoreError> {
        self.conn
            .query_row(
                "SELECT * FROM projects WHERE id = ?1",
                params![id.as_str()],
                map_project,
            )
            .optional()?
            .ok_or_else(|| super::accounts::not_found("project", id.as_str()))
    }

    /// Merges imported-history enrichment into the project row
    /// (backfill writes it once; AI-Juggler runs never touch it).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures or
    /// [`CoreError::Serialization`] for corrupt history JSON.
    pub fn merge_history(&self, id: &ProjectId, history: &ProjectHistory) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET history_json = ?2 WHERE id = ?1",
            params![id.as_str(), serde_json::to_string(history)?],
        )?;
        if changed == 0 {
            return Err(super::accounts::not_found("project", id.as_str()));
        }
        Ok(())
    }

    /// Lists projects, most recently active first.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures or
    /// [`CoreError::Serialization`] for corrupt rows.
    pub fn list(&self) -> Result<Vec<ProjectRecord>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM projects
             ORDER BY last_activity DESC, created_at DESC, id",
        )?;
        let rows = stmt.query_map([], map_project)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

/// Maps a row to a [`ProjectRecord`].
fn map_project(row: &Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let id: String = row.get("id")?;
    let root: String = row.get("root")?;
    Ok(ProjectRecord {
        id: ProjectId::new(id),
        root: std::path::PathBuf::from(root),
        name: row.get("name")?,
        created_at: Timestamp::from_millis(row.get("created_at")?),
        last_activity: row
            .get::<_, Option<i64>>("last_activity")?
            .map(Timestamp::from_millis),
        history: serde_json::from_str(&row.get::<_, String>("history_json")?).unwrap_or_default(),
    })
}

/// Time-based id suffix with a same-millisecond disambiguator (backfill
/// can create many projects in one millisecond).
fn suffix(now: Timestamp) -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let millis = u128::from(now.millis().max(0).cast_unsigned()) % 1_000_000_000_000;
    millis * 1_000 + (seq % 1_000)
}
