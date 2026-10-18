//! AJ-E1002: git worktree isolation for parallel Instances.
//!
//! A second Instance on a project that already has a live one is offered
//! (default on) its own git worktree on a `juggler/<id>` branch, at a path
//! the user sees and confirms — so two Accounts never edit one checkout.
//! Status (branch, dirty count, ahead/behind) is read with
//! `git status --porcelain=v2 --branch` at most every 30 s per folder (on
//! focus sooner), never on the rail's hot path. Removing a worktree never
//! deletes uncommitted work silently: with changes present it needs the
//! folder's name typed back, and the branch is always kept.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Status is re-read at most this often per folder.
const STATUS_TTL: Duration = Duration::from_secs(30);
/// A forced refresh (on focus) is honoured at most this often.
const FORCED_MIN: Duration = Duration::from_secs(2);
/// The app setting listing the worktrees Juggler created.
const WORKTREES_KEY: &str = "juggler_worktrees";

/// A folder's git state.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GitStatus {
    /// The branch (`None` when detached).
    pub branch: Option<String>,
    /// Changed tracked files (staged or not).
    pub changed: u64,
    /// Untracked files.
    pub untracked: u64,
    /// Commits ahead of the upstream (`None` without an upstream).
    pub ahead: Option<u64>,
    /// Commits behind the upstream.
    pub behind: Option<u64>,
}

/// Parses `git status --porcelain=v2 --branch` (pure).
#[must_use]
pub fn parse_porcelain(text: &str) -> GitStatus {
    let mut status = GitStatus::default();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            status.branch = (head != "(detached)").then(|| head.to_owned());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut parts = ab.split_whitespace();
            status.ahead = parts
                .next()
                .and_then(|a| a.trim_start_matches('+').parse().ok());
            status.behind = parts
                .next()
                .and_then(|b| b.trim_start_matches('-').parse().ok());
        } else if line.starts_with("? ") {
            status.untracked += 1;
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            status.changed += 1;
        }
    }
    status
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The repository's top level for `dir`, when it is inside one.
#[must_use]
pub fn repo_root(dir: &Path) -> Option<PathBuf> {
    git(dir, &["rev-parse", "--show-toplevel"]).map(|out| PathBuf::from(out.trim()))
}

/// Reads `dir`'s git status now (a subprocess — never per rail render).
#[must_use]
pub fn read_status(dir: &Path) -> Option<GitStatus> {
    git(
        dir,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
        ],
    )
    .map(|out| parse_porcelain(&out))
}

/// Folder → (when read, status); `None` = not a git folder.
type StatusCache = Mutex<HashMap<PathBuf, (Instant, Option<GitStatus>)>>;

fn cache() -> &'static StatusCache {
    static CACHE: OnceLock<StatusCache> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

fn in_flight() -> &'static Mutex<std::collections::HashSet<PathBuf>> {
    static IN_FLIGHT: OnceLock<Mutex<std::collections::HashSet<PathBuf>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(Mutex::default)
}

/// The cached status for `dir` (rail / inspector reads). A missing or
/// stale entry is refreshed in the background — the caller never waits
/// on git.
pub(crate) fn cached_status(dir: &Path) -> Option<GitStatus> {
    let entry = cache().lock().ok().and_then(|map| map.get(dir).cloned());
    let stale = entry
        .as_ref()
        .is_none_or(|(at, _)| at.elapsed() >= STATUS_TTL);
    if stale {
        let claimed = in_flight()
            .lock()
            .is_ok_and(|mut set| set.insert(dir.to_path_buf()));
        if claimed {
            let dir = dir.to_path_buf();
            std::thread::spawn(move || {
                let status = read_status(&dir);
                if let Ok(mut map) = cache().lock() {
                    map.insert(dir.clone(), (Instant::now(), status));
                }
                if let Ok(mut set) = in_flight().lock() {
                    set.remove(&dir);
                }
            });
        }
    }
    entry.and_then(|(_, status)| status)
}

/// Re-reads `dir`'s status now (on focus), honoured at most every 2 s.
pub(crate) fn refresh_status(dir: &Path) -> Option<GitStatus> {
    if let Ok(map) = cache().lock()
        && let Some((at, status)) = map.get(dir)
        && at.elapsed() < FORCED_MIN
    {
        return status.clone();
    }
    let status = read_status(dir);
    if let Ok(mut map) = cache().lock() {
        map.insert(dir.to_path_buf(), (Instant::now(), status.clone()));
    }
    status
}

/// The branch of a folder from its `.git` — a directory, or (for a
/// linked worktree) a file pointing at its gitdir. No subprocess.
#[must_use]
pub fn branch_of(dir: &Path) -> Option<String> {
    let dot_git = dir.join(".git");
    let head_path = if dot_git.is_file() {
        let text = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = text.trim().strip_prefix("gitdir: ")?.trim();
        let gitdir = Path::new(gitdir);
        let gitdir = if gitdir.is_absolute() {
            gitdir.to_path_buf()
        } else {
            dir.join(gitdir)
        };
        gitdir.join("HEAD")
    } else {
        dot_git.join("HEAD")
    };
    let head = std::fs::read_to_string(head_path).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

/// Whether `dir` is a linked worktree (its `.git` is a file).
#[must_use]
pub fn is_linked_worktree(dir: &Path) -> bool {
    dir.join(".git").is_file()
}

// ------------------------------------------------------------- the offer

/// The isolation offer for a launch into a folder another Instance is
/// already working in.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct IsolationOfferInfo {
    /// The folder's name.
    pub project_name: String,
    /// Who is already working there (Account labels).
    pub in_use_by: Vec<String>,
    /// Isolation is possible (a git project).
    pub available: bool,
    /// Why not, in words (`Unavailable — not a git project`).
    pub unavailable_reason: Option<String>,
    /// Where the worktree would be created (shown; the user confirms).
    pub proposed_path: Option<String>,
    /// Its branch (`juggler/<id>`).
    pub proposed_branch: Option<String>,
    /// The branch it starts from.
    pub base_branch: Option<String>,
}

fn is_live(status: juggler_core::model::SessionStatus) -> bool {
    use juggler_core::model::SessionStatus;
    matches!(
        status,
        SessionStatus::Starting
            | SessionStatus::Running
            | SessionStatus::WaitingUser
            | SessionStatus::Stuck
    )
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn short_id() -> String {
    let mut bytes = [0_u8; 3];
    let _ = getrandom::fill(&mut bytes);
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Core: the isolation offer for launching into `cwd`, or `None` when no
/// live Instance works there.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failures.
pub fn isolation_offer_impl(
    state: &Arc<AppState>,
    cwd: &str,
) -> AppResult<Option<IsolationOfferInfo>> {
    let target = canonical(Path::new(cwd));
    let rows = crate::commands::instances_shared_snapshot(state)?;
    let in_use_by: Vec<String> = rows
        .iter()
        .filter(|row| {
            juggler_core::model::SessionStatus::parse(&row.status).is_some_and(is_live)
                && canonical(Path::new(&row.cwd)) == target
        })
        .map(|row| row.account_label.clone())
        .collect();
    if in_use_by.is_empty() {
        return Ok(None);
    }
    let name = target.file_name().map_or_else(
        || cwd.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Some(root) = repo_root(&target) else {
        return Ok(Some(IsolationOfferInfo {
            project_name: name,
            in_use_by,
            available: false,
            unavailable_reason: Some("Unavailable — not a git project".to_owned()),
            proposed_path: None,
            proposed_branch: None,
            base_branch: None,
        }));
    };
    let short = short_id();
    let parent = root
        .parent()
        .map_or_else(|| root.clone(), Path::to_path_buf);
    let root_name = root.file_name().map_or_else(
        || "project".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let path = parent.join(format!("{root_name}.juggler")).join(&short);
    Ok(Some(IsolationOfferInfo {
        project_name: name,
        in_use_by,
        available: true,
        unavailable_reason: None,
        proposed_path: Some(path.to_string_lossy().into_owned()),
        proposed_branch: Some(format!("juggler/{short}")),
        base_branch: branch_of(&root),
    }))
}

/// A worktree Juggler created (for the removal guard).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
struct JugglerWorktree {
    path: String,
    branch: String,
    repo_root: String,
    created_at_ms: i64,
}

fn created(state: &Arc<AppState>) -> Vec<JugglerWorktree> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(WORKTREES_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_created(state: &Arc<AppState>, rows: &[JugglerWorktree]) -> AppResult<()> {
    let raw = serde_json::to_string(rows).map_err(|e| AppError::Io {
        context: "worktrees",
        message: e.to_string(),
    })?;
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(WORKTREES_KEY, &raw)?;
    Ok(())
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation {
        subject: "worktree",
        message: message.into(),
    }
}

/// Core: creates the worktree the user confirmed — a new `branch` from
/// the current HEAD of `cwd`'s repository, at `path` (which must not
/// exist). Returns the path to launch in.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a non-git folder, an existing
/// path or branch, or a branch outside `juggler/`; [`AppError::Io`] when
/// git refuses.
pub fn worktree_create_impl(
    state: &Arc<AppState>,
    cwd: &str,
    path: &str,
    branch: &str,
) -> AppResult<String> {
    let root =
        repo_root(Path::new(cwd)).ok_or_else(|| invalid("Unavailable — not a git project"))?;
    if !branch.starts_with("juggler/") || branch.contains("..") || branch.contains(' ') {
        return Err(invalid("the branch must be juggler/<name>"));
    }
    let target = PathBuf::from(path);
    if target.exists() {
        return Err(invalid(format!(
            "{path} already exists — nothing was created"
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::io("create the worktree folder", &e))?;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["worktree", "add", "-b", branch])
        .arg(&target)
        .output()
        .map_err(|e| AppError::io("run git", &e))?;
    if !output.status.success() {
        return Err(AppError::Io {
            context: "create the worktree",
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let mut rows = created(state);
    rows.push(JugglerWorktree {
        path: target.to_string_lossy().into_owned(),
        branch: branch.to_owned(),
        repo_root: root.to_string_lossy().into_owned(),
        created_at_ms: juggler_core::model::Timestamp::now().millis(),
    });
    save_created(state, &rows)?;
    Ok(target.to_string_lossy().into_owned())
}

/// What removing a worktree would do, in words, before anything runs.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WorktreeRemovalInfo {
    /// The worktree.
    pub path: String,
    /// Its folder name — typed back to confirm a discard.
    pub folder: String,
    /// Its branch (always kept).
    pub branch: String,
    /// Changed tracked files that would be discarded.
    #[ts(type = "number")]
    pub changed: u64,
    /// Untracked files that would be discarded.
    #[ts(type = "number")]
    pub untracked: u64,
    /// Removing needs the folder name typed back (something would be lost).
    pub needs_confirmation: bool,
    /// Plain words: what is discarded, what is kept.
    pub summary: String,
    /// An Instance still runs in it (removal refused).
    pub in_use: bool,
}

fn removal_info(state: &Arc<AppState>, path: &str) -> AppResult<WorktreeRemovalInfo> {
    let row = created(state)
        .into_iter()
        .find(|row| row.path == path)
        .ok_or_else(|| invalid("only worktrees AI Juggler created can be removed here"))?;
    let status = read_status(Path::new(path)).unwrap_or_default();
    let target = canonical(Path::new(path));
    let in_use = crate::commands::instances_shared_snapshot(state)?
        .iter()
        .any(|instance| {
            juggler_core::model::SessionStatus::parse(&instance.status).is_some_and(is_live)
                && canonical(Path::new(&instance.cwd)) == target
        });
    let lost = status.changed + status.untracked;
    let summary = if lost == 0 {
        format!(
            "No uncommitted work. The branch {} stays — its commits are kept.",
            row.branch
        )
    } else {
        format!(
            "Discards {} changed and {} untracked {} — they are not committed anywhere. The branch {} stays with its commits.",
            status.changed,
            status.untracked,
            if lost == 1 { "file" } else { "files" },
            row.branch
        )
    };
    Ok(WorktreeRemovalInfo {
        folder: Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: row.path,
        branch: row.branch,
        changed: status.changed,
        untracked: status.untracked,
        needs_confirmation: lost > 0,
        summary,
        in_use,
    })
}

/// Core: the removal preview.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a worktree Juggler didn't create.
pub fn worktree_removal_preview_impl(
    state: &Arc<AppState>,
    path: &str,
) -> AppResult<WorktreeRemovalInfo> {
    removal_info(state, path)
}

/// Core: removes a worktree Juggler created. With uncommitted work in
/// it, `confirm` must be its folder name typed back; the branch is
/// always kept.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when an Instance still runs in it or
/// the confirmation is missing; [`AppError::Io`] when git refuses.
pub fn worktree_remove_impl(
    state: &Arc<AppState>,
    path: &str,
    confirm: Option<&str>,
) -> AppResult<()> {
    let info = removal_info(state, path)?;
    if info.in_use {
        return Err(invalid(
            "an Instance still runs in this worktree — stop it first",
        ));
    }
    if info.needs_confirmation && confirm != Some(info.folder.as_str()) {
        return Err(invalid(format!(
            "type «{}» to discard {} uncommitted {} — nothing was removed",
            info.folder,
            info.changed + info.untracked,
            if info.changed + info.untracked == 1 {
                "file"
            } else {
                "files"
            }
        )));
    }
    let rows = created(state);
    let row = rows
        .iter()
        .find(|row| row.path == path)
        .cloned()
        .ok_or_else(|| invalid("unknown worktree"))?;
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&row.repo_root)
        .args(["worktree", "remove"]);
    if info.needs_confirmation {
        command.arg("--force");
    }
    let output = command
        .arg(path)
        .output()
        .map_err(|e| AppError::io("run git", &e))?;
    if !output.status.success() {
        return Err(AppError::Io {
            context: "remove the worktree",
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let remaining: Vec<JugglerWorktree> = rows.into_iter().filter(|row| row.path != path).collect();
    save_created(state, &remaining)
}

/// Whether `path` is a worktree Juggler created.
pub(crate) fn juggler_created(state: &Arc<AppState>, path: &Path) -> bool {
    let target = canonical(path);
    created(state)
        .iter()
        .any(|row| canonical(Path::new(&row.path)) == target)
}

/// Per-Instance git status, as the inspector shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GitStatusInfo {
    /// The branch (`null` = detached).
    pub branch: Option<String>,
    /// Changed tracked files.
    #[ts(type = "number")]
    pub changed: u64,
    /// Untracked files.
    #[ts(type = "number")]
    pub untracked: u64,
    /// Ahead of upstream.
    #[ts(type = "number | null")]
    pub ahead: Option<u64>,
    /// Behind upstream.
    #[ts(type = "number | null")]
    pub behind: Option<u64>,
    /// It runs in a linked worktree.
    pub worktree: bool,
    /// A worktree AI Juggler created (removable here).
    pub created_by_juggler: bool,
    /// The folder.
    pub path: String,
}

/// Core: re-reads an Instance's git status now (on focus; at most every
/// 2 s), `None` for a non-git folder.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] for an unknown Instance.
pub fn git_status_refresh_impl(
    state: &Arc<AppState>,
    session_id: &str,
) -> AppResult<Option<GitStatusInfo>> {
    let cwd = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(session_id))?
            .cwd
    };
    Ok(refresh_status(&cwd).map(|status| GitStatusInfo {
        branch: status.branch,
        changed: status.changed,
        untracked: status.untracked,
        ahead: status.ahead,
        behind: status.behind,
        worktree: is_linked_worktree(&cwd),
        created_by_juggler: juggler_created(state, &cwd),
        path: cwd.to_string_lossy().into_owned(),
    }))
}

// ---------------------------------------------------------- the commands

/// AJ-E1002: the isolation offer for launching into `cwd`.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`isolation_offer_impl`].
#[tauri::command]
pub fn isolation_offer(
    state: tauri::State<'_, Arc<AppState>>,
    cwd: String,
) -> AppResult<Option<IsolationOfferInfo>> {
    isolation_offer_impl(state.inner(), &cwd)
}

/// AJ-E1002: creates the confirmed worktree.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_create_impl`].
#[tauri::command]
pub fn worktree_create(
    state: tauri::State<'_, Arc<AppState>>,
    cwd: String,
    path: String,
    branch: String,
) -> AppResult<String> {
    worktree_create_impl(state.inner(), &cwd, &path, &branch)
}

/// AJ-E1002: what removing a worktree would discard.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_removal_preview_impl`].
#[tauri::command]
pub fn worktree_removal_preview(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> AppResult<WorktreeRemovalInfo> {
    worktree_removal_preview_impl(state.inner(), &path)
}

/// AJ-E1002: removes a worktree AI Juggler created.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_remove_impl`].
#[tauri::command]
pub fn worktree_remove(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
    confirm: Option<String>,
) -> AppResult<()> {
    worktree_remove_impl(state.inner(), &path, confirm.as_deref())
}

/// AJ-E1002: an Instance's git status now (on focus).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`git_status_refresh_impl`].
#[tauri::command]
pub fn git_status_refresh(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Option<GitStatusInfo>> {
    git_status_refresh_impl(state.inner(), &session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_v2_reads_branch_dirty_untracked_and_ahead_behind() {
        let text = "# branch.oid 1234\n# branch.head juggler/a1b2c3\n# branch.upstream origin/main\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/app.ts\n2 R. N... 100644 100644 100644 a b R100 new.ts\told.ts\n? notes.md\n? tmp/\n";
        let status = parse_porcelain(text);
        assert_eq!(status.branch.as_deref(), Some("juggler/a1b2c3"));
        assert_eq!(status.changed, 2);
        assert_eq!(status.untracked, 2);
        assert_eq!((status.ahead, status.behind), (Some(2), Some(1)));
        let detached = parse_porcelain("# branch.head (detached)\n");
        assert_eq!(detached.branch, None);
        assert_eq!(detached.ahead, None, "no upstream → unknown, never 0");
    }
}
