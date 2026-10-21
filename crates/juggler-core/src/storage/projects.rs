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
                default_account: None,
                pinned_at: None,
                forgotten_at: None,
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

    /// Removes a project nothing refers to — no session and no work item
    /// (a history-only row the backfill made). Returns whether it went.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn delete_if_unused(&self, id: &ProjectId) -> Result<bool, CoreError> {
        let changed = self.conn.execute(
            "DELETE FROM projects WHERE id = ?1
               AND NOT EXISTS (SELECT 1 FROM instances WHERE project = ?1)
               AND NOT EXISTS (SELECT 1 FROM work_items WHERE project = ?1)",
            params![id.as_str()],
        )?;
        Ok(changed > 0)
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

    /// Sets (or clears, with `None`) the account a project's new
    /// sessions start with (AJ-E1501 Req.4).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when the project is missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_default_account(
        &self,
        id: &ProjectId,
        account: Option<&crate::model::AccountId>,
    ) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET default_account = ?2 WHERE id = ?1",
            params![id.as_str(), account.map(crate::model::AccountId::as_str)],
        )?;
        super::accounts::require_changed(changed, "project", id.as_str())
    }

    /// Renames a project (its label; the folder is never renamed).
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn rename(&self, id: &ProjectId, name: &str) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET name = ?2 WHERE id = ?1",
            params![id.as_str(), name],
        )?;
        super::accounts::require_changed(changed, "project", id.as_str())
    }

    /// Pins a project to the top of the rail (now), or unpins it.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_pinned(&self, id: &ProjectId, pinned: bool) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET pinned_at = ?2 WHERE id = ?1",
            params![id.as_str(), pinned.then(|| Timestamp::now().millis())],
        )?;
        super::accounts::require_changed(changed, "project", id.as_str())
    }

    /// Forgets a project (now; hidden until used again), or brings it
    /// back. A forgotten project loses its pin.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn set_forgotten(&self, id: &ProjectId, forgotten: bool) -> Result<(), CoreError> {
        let changed = if forgotten {
            self.conn.execute(
                "UPDATE projects SET forgotten_at = ?2, pinned_at = NULL WHERE id = ?1",
                params![id.as_str(), Timestamp::now().millis()],
            )?
        } else {
            self.conn.execute(
                "UPDATE projects SET forgotten_at = NULL WHERE id = ?1",
                params![id.as_str()],
            )?
        };
        super::accounts::require_changed(changed, "project", id.as_str())
    }

    /// Repoints a project at the folder it moved to. Refused (a
    /// constraint error) when another project already has that root.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::NotFound`] when missing, or
    /// [`CoreError::Storage`] on SQLite failures.
    pub fn relocate(&self, id: &ProjectId, root: &Path) -> Result<(), CoreError> {
        let changed = self.conn.execute(
            "UPDATE projects SET root = ?2 WHERE id = ?1",
            params![id.as_str(), root.to_string_lossy()],
        )?;
        super::accounts::require_changed(changed, "project", id.as_str())
    }

    /// Clears an account as every project's default (it was removed).
    /// Returns how many projects had it.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Storage`] on SQLite failures.
    pub fn clear_default_account(
        &self,
        account: &crate::model::AccountId,
    ) -> Result<usize, CoreError> {
        Ok(self.conn.execute(
            "UPDATE projects SET default_account = NULL WHERE default_account = ?1",
            params![account.as_str()],
        )?)
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

/// The repository a git worktree belongs to: `path`, or the nearest
/// folder above it, holds a `.git` FILE pointing at
/// `<repo>/.git/worktrees/<name>`. Returns `<repo>` — so a session in a
/// worktree (Claude Code's `.claude/worktrees/…`, other tools'
/// `…/worktrees/…`) belongs to its project, not to a project of its own.
/// `None` for an ordinary folder or repository. Reads only `.git` files.
#[must_use]
pub fn worktree_repo_root(path: &Path) -> Option<std::path::PathBuf> {
    let mut dir = Some(path);
    for _ in 0..12 {
        let here = dir?;
        let dot_git = here.join(".git");
        if dot_git.is_dir() {
            return None; // a repository's own root
        }
        if dot_git.is_file() {
            let text = std::fs::read_to_string(&dot_git).ok()?;
            let gitdir = text
                .lines()
                .find_map(|line| line.strip_prefix("gitdir:"))?
                .trim();
            // Relative (submodules, `worktree.useRelativePaths`) resolves
            // from the `.git` file's own folder, never the app's cwd.
            let gitdir = here.join(gitdir);
            // `<repo>/.git/worktrees/<name>`, by path components (any
            // separator): the repository is the parent of that `.git`.
            let repo = gitdir.ancestors().find_map(|ancestor| {
                let parent = ancestor.parent()?;
                (ancestor.file_name()? == "worktrees" && parent.file_name()? == ".git")
                    .then(|| parent.parent().map(Path::to_path_buf))
                    .flatten()
            })?;
            // One spelling per repository (a symlinked ~/Dev, /var vs
            // /private/var): the root is UNIQUE by string.
            return std::fs::canonicalize(&repo)
                .ok()
                .filter(|repo| repo.is_dir());
        }
        dir = here.parent();
    }
    None
}

/// AJ-E1529: the project a folder belongs to.
///
/// - An agent worktree, `<repo>/.claude/worktrees/<name>/…` (or
///   `.codex/…`), belongs to `<repo>` by its path: whether it is still
///   there, was removed (only its transcripts remain) or was pruned (the
///   folder remains, its `.git` file doesn't).
/// - A linked worktree elsewhere belongs to its repository through its
///   `.git` file ([`worktree_repo_root`]).
/// - A folder inside a repository belongs to the repository.
/// - Anything else is its own project.
///
/// Reads only `.git` entries; never writes.
#[must_use]
pub fn project_root(path: &Path) -> std::path::PathBuf {
    if let Some(repo) = agent_worktree_repo(path) {
        return repo;
    }
    if let Some(repo) = worktree_repo_root(path) {
        return repo;
    }
    path.ancestors()
        .skip(1)
        .take(12)
        .find(|ancestor| ancestor.join(".git").is_dir())
        .map_or_else(
            || path.to_path_buf(),
            |repo| std::fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf()),
        )
}

/// `<repo>` of `<repo>/.claude/worktrees/<name>[/…]` (or `.codex`), when
/// `<repo>` is a repository on this Mac.
fn agent_worktree_repo(path: &Path) -> Option<std::path::PathBuf> {
    let parts: Vec<_> = path.components().collect();
    let at = parts.windows(3).position(|window| {
        matches!(window[0].as_os_str().to_str(), Some(".claude" | ".codex"))
            && window[1].as_os_str() == "worktrees"
    })?;
    let repo: std::path::PathBuf = parts[..at].iter().collect();
    repo.join(".git")
        .exists()
        .then(|| std::fs::canonicalize(&repo).unwrap_or(repo))
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
        default_account: row
            .get::<_, Option<String>>("default_account")?
            .map(crate::model::AccountId::new),
        pinned_at: row
            .get::<_, Option<i64>>("pinned_at")?
            .map(Timestamp::from_millis),
        forgotten_at: row
            .get::<_, Option<i64>>("forgotten_at")?
            .map(Timestamp::from_millis),
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

#[cfg(test)]
mod project_root_tests {
    use super::project_root;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.test",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    }

    /// AJ-E1529: every shape of worktree seen on a real Mac folds into its
    /// repository: present, removed, pruned; and a subfolder of a repo.
    #[test]
    fn worktrees_present_removed_and_pruned_fold_into_their_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("app");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        std::fs::write(repo.join("a"), "a").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "a"]);
        let repo = std::fs::canonicalize(&repo).unwrap();
        let present = repo.join(".claude/worktrees/present");
        let removed = repo.join(".claude/worktrees/removed");
        let pruned = repo.join(".claude/worktrees/pruned");
        for (path, branch) in [(&present, "p"), (&removed, "r"), (&pruned, "q")] {
            git(
                &repo,
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    branch,
                    path.to_str().unwrap(),
                ],
            );
        }
        git(
            &repo,
            &["worktree", "remove", "--force", removed.to_str().unwrap()],
        );
        std::fs::remove_file(pruned.join(".git")).unwrap();
        assert_eq!(project_root(&present), repo);
        assert_eq!(
            project_root(&removed),
            repo,
            "removed: only its transcripts remain"
        );
        assert_eq!(
            project_root(&pruned),
            repo,
            "pruned: the folder remains, git forgot it"
        );
        assert_eq!(project_root(&present.join("src")), repo);
        std::fs::create_dir_all(repo.join("web/src")).unwrap();
        assert_eq!(
            project_root(&repo.join("web/src")),
            repo,
            "a folder inside a repo is the repo"
        );
        assert_eq!(project_root(&repo), repo);
        let plain = dir.path().join("notes");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(project_root(&plain), plain, "not a repo: its own project");
        let gone = dir.path().join("gone/.claude/worktrees/x");
        assert_eq!(
            project_root(&gone),
            gone,
            "a worktree path without its repo stays as it is"
        );
    }
}
