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

/// git, from the one place AI Juggler trusts it (C2), or the plain words
/// for a Mac that has none.
fn git_command() -> Result<Command, AppError> {
    juggler_adapter::system_tools::git()
        .map(Command::new)
        .ok_or_else(|| AppError::Io {
            context: "run git",
            message: juggler_adapter::system_tools::GIT_MISSING.to_owned(),
        })
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = git_command()
        .ok()?
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

/// `git` in `dir`, its raw stdout when it exits with one of `codes`.
fn git_bytes(dir: &Path, args: &[&str], codes: &[i32]) -> Option<Vec<u8>> {
    let output = git_command()
        .ok()?
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .code()
        .is_some_and(|code| codes.contains(&code))
        .then_some(output.stdout)
}

/// The most a pack's uncommitted-changes patch may weigh; a larger one
/// stays out and the pack says so.
pub const PATCH_MAX_BYTES: usize = 4 * 1024 * 1024;

/// The most untracked files a patch adds one by one.
const PATCH_MAX_UNTRACKED: usize = 200;

/// ADR-0009: `dir`'s uncommitted changes as one patch for a pack's
/// `diffs/`: the tracked changes against HEAD, then each untracked file as
/// an addition. Read-only (the index is never written). `None` when there
/// is nothing to stage, outside a repository (or before its first
/// commit), or over [`PATCH_MAX_BYTES`] / [`PATCH_MAX_UNTRACKED`] (live
/// lab 13: a dirty pack's `diffs/` was always empty).
#[must_use]
pub fn uncommitted_patch(dir: &Path) -> Option<Vec<u8>> {
    let mut patch = git_bytes(dir, &["diff", "HEAD", "--binary"], &[0])?;
    let untracked = git(dir, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    let files: Vec<&str> = untracked.split('\0').filter(|f| !f.is_empty()).collect();
    if files.len() > PATCH_MAX_UNTRACKED {
        return None;
    }
    for file in files {
        // `--no-index` exits 1 when the two differ: that is the patch.
        let added = git_bytes(
            dir,
            &["diff", "--no-index", "--binary", "--", "/dev/null", file],
            &[0, 1],
        )?;
        patch.extend(added);
        if patch.len() > PATCH_MAX_BYTES {
            return None;
        }
    }
    (!patch.is_empty()).then_some(patch)
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

/// AJ-E1541: what has changed in a project and isn't committed yet, for
/// a handover: the branch, up to [`CHANGES_SHOWN`] files as git names them
/// (`M calc.py`, `?? test_calc.py`), how many more, and the diff stat.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct WorkChanges {
    /// The branch, when on one.
    pub branch: Option<String>,
    /// The changed files, as `git status --short` lines.
    pub files: Vec<String>,
    /// Changed files not listed.
    pub more: usize,
    /// `git diff --shortstat HEAD`: `2 files changed, 12 insertions(+)`.
    pub stat: Option<String>,
}

/// Files a handover lists by name; the rest are counted.
pub const CHANGES_SHOWN: usize = 30;

/// Reads `dir`'s uncommitted changes now (two git subprocesses). `None`
/// outside a repository.
#[must_use]
pub fn work_changes(dir: &Path) -> Option<WorkChanges> {
    let short = git(dir, &["status", "--short", "--untracked-files=normal"])?;
    let lines: Vec<String> = short
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let more = lines.len().saturating_sub(CHANGES_SHOWN);
    Some(WorkChanges {
        branch: branch_of(dir),
        files: lines.into_iter().take(CHANGES_SHOWN).collect(),
        more,
        stat: git(dir, &["diff", "--shortstat", "HEAD"])
            .map(|out| out.trim().to_owned())
            .filter(|out| !out.is_empty()),
    })
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

/// The worktrees AI Juggler made, `(path, branch)`, as recorded.
pub(crate) fn created_worktrees(state: &Arc<AppState>) -> Vec<(String, String)> {
    created(state)
        .into_iter()
        .map(|row| (row.path, row.branch))
        .collect()
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
    let output = git_command()?
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
            "a session still runs in this worktree — stop it first",
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
    let mut command = git_command()?;
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
    Ok(status_info(state, &cwd))
}

/// `dir`'s git status now (at most every 2 s) as the UI shows it, `None`
/// for a folder that isn't a git checkout: a session's, or a project's.
pub(crate) fn status_info(state: &Arc<AppState>, dir: &Path) -> Option<GitStatusInfo> {
    refresh_status(dir).map(|status| GitStatusInfo {
        branch: status.branch,
        changed: status.changed,
        untracked: status.untracked,
        ahead: status.ahead,
        behind: status.behind,
        worktree: is_linked_worktree(dir),
        created_by_juggler: juggler_created(state, dir),
        path: dir.to_string_lossy().into_owned(),
    })
}

// ---------------------------------------------------------- the commands

/// AJ-E1002: the isolation offer for launching into `cwd`.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`isolation_offer_impl`].
#[tauri::command]
pub async fn isolation_offer(
    state: tauri::State<'_, Arc<AppState>>,
    cwd: String,
) -> AppResult<Option<IsolationOfferInfo>> {
    crate::ipc::blocking(state.inner(), "isolation offer", move |state| {
        isolation_offer_impl(state, &cwd)
    })
    .await
}

/// AJ-E1002: creates the confirmed worktree.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_create_impl`].
#[tauri::command]
pub async fn worktree_create(
    state: tauri::State<'_, Arc<AppState>>,
    cwd: String,
    path: String,
    branch: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "worktree create", move |state| {
        worktree_create_impl(state, &cwd, &path, &branch)
    })
    .await
}

/// AJ-E1002: what removing a worktree would discard.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_removal_preview_impl`].
#[tauri::command]
pub async fn worktree_removal_preview(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> AppResult<WorktreeRemovalInfo> {
    crate::ipc::blocking(state.inner(), "worktree removal preview", move |state| {
        worktree_removal_preview_impl(state, &path)
    })
    .await
}

/// AJ-E1002: removes a worktree AI Juggler created.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`worktree_remove_impl`].
#[tauri::command]
pub async fn worktree_remove(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
    confirm: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "worktree remove", move |state| {
        worktree_remove_impl(state, &path, confirm.as_deref())
    })
    .await
}

/// AJ-E1002: an Instance's git status now (on focus).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`git_status_refresh_impl`].
#[tauri::command]
pub async fn git_status_refresh(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Option<GitStatusInfo>> {
    crate::ipc::blocking(state.inner(), "git status refresh", move |state| {
        git_status_refresh_impl(state, &session_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live lab 13: a dirty pack's `diffs/` was always empty. The patch
    /// holds the tracked change and the untracked file, and reading it
    /// writes nothing to the repository.
    #[test]
    fn the_uncommitted_patch_holds_changed_and_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let run = |args: &[&str]| {
            let ok = git_command()
                .unwrap()
                .arg("-C")
                .arg(repo)
                .args(args)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "the fixture's git step runs");
        };
        run(&["init", "-q"]);
        std::fs::write(repo.join("calc.py"), "def add(a, b):\n    return a + b\n").unwrap();
        run(&["add", "-A"]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "start",
        ]);
        assert!(uncommitted_patch(repo).is_none(), "a clean tree stages nothing");

        std::fs::write(
            repo.join("calc.py"),
            "def add(a, b):\n    return a + b\n\ndef sub(a, b):\n    return a - b\n",
        )
        .unwrap();
        std::fs::write(repo.join("test_calc.py"), "import calc\n").unwrap();
        let status_before = git(repo, &["status", "--porcelain"]).unwrap();
        let patch = String::from_utf8(uncommitted_patch(repo).unwrap()).unwrap();
        assert!(patch.contains("+def sub(a, b):"), "the tracked change");
        assert!(
            patch.contains("+++ b/test_calc.py") && patch.contains("+import calc"),
            "the untracked file, as an addition"
        );
        assert_eq!(
            git(repo, &["status", "--porcelain"]).unwrap(),
            status_before,
            "reading the patch changes nothing"
        );
    }

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
