//! Paths the user chose (AJ-E2003 R6).
//!
//! A command never trusts a path, URL or program the page hands it: the
//! page could be running a script it shouldn't. The native pickers open
//! from Rust ([`pick_path`]), and what the user picks, or drops onto the
//! window ([`record_dropped`]), is remembered here for a while. A command
//! that takes a path asks [`vouch`]: was it picked (of the right kind) or
//! dropped, or is it a folder AI Juggler already knows (a project's, a
//! session's, an Account's)? Anything else is refused.

use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::Deserialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// How long a pick stays good for: the sheet or dialog it was picked for
/// is answered well within it.
const KEPT_FOR: Duration = Duration::from_mins(30);
/// The most picks remembered at once.
const MOST: usize = 64;

/// What the user is picking, which decides the picker and what the pick
/// may be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub enum PickKind {
    /// A folder to work in (a project, a session's folder, an import).
    Folder,
    /// A handover folder to open.
    PackFolder,
    /// A backup to restore or check.
    BackupFile,
    /// Where to save a backup.
    BackupDest,
    /// Where to save the recovery code.
    RecoveryCodeDest,
    /// Where to save a diagnostics zip.
    DiagnosticsDest,
}

/// How a path reached AI Juggler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Picked(PickKind),
    Dropped,
}

struct Chosen {
    path: PathBuf,
    source: Source,
    at: Instant,
}

static CHOSEN: Mutex<Vec<Chosen>> = Mutex::new(Vec::new());

/// `path` made absolute and plain (no `.` or `..`), without touching the
/// disk: a save destination doesn't exist yet. A relative path is refused.
fn plain(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Some(out)
}

fn remember(path: &Path, source: Source) {
    let Some(path) = plain(path) else { return };
    let mut chosen = CHOSEN.lock().unwrap_or_else(PoisonError::into_inner);
    chosen.retain(|c| c.at.elapsed() < KEPT_FOR && c.path != path);
    chosen.push(Chosen {
        path,
        source,
        at: Instant::now(),
    });
    let extra = chosen.len().saturating_sub(MOST);
    chosen.drain(..extra);
}

/// Paths dropped onto a window (its drag-and-drop event, in Rust).
pub fn record_dropped(paths: &[PathBuf]) {
    for path in paths {
        remember(path, Source::Dropped);
    }
}

fn was(path: &Path, ok: impl Fn(Source) -> bool) -> bool {
    let Some(path) = plain(path) else { return false };
    CHOSEN
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .any(|c| c.at.elapsed() < KEPT_FOR && c.path == path && ok(c.source))
}

/// Where a command's path may come from.
#[derive(Debug, Clone, Copy)]
pub struct Allowed<'a> {
    /// Picks of these kinds.
    pub picked: &'a [PickKind],
    /// A folder dropped onto the window.
    pub dropped: bool,
}

/// `path` as given, when the user picked or dropped it as `allowed` says,
/// or `known` vouches for it; otherwise the refusal.
///
/// # Errors
///
/// [`AppError::Validation`] for a path the user didn't choose and AI
/// Juggler doesn't know.
pub fn vouch(path: &str, allowed: Allowed<'_>, known: impl FnOnce(&Path) -> bool) -> AppResult<PathBuf> {
    let given = PathBuf::from(path);
    let chosen = was(&given, |source| match source {
        Source::Picked(kind) => allowed.picked.contains(&kind),
        Source::Dropped => allowed.dropped,
    });
    if chosen || plain(&given).is_some_and(|p| known(&p)) {
        return Ok(given);
    }
    Err(AppError::Validation {
        subject: "path",
        message: "Choose that place again: AI Juggler only uses folders and files you picked or it already knows.".to_owned(),
    })
}

/// Whether `path` is `root` or inside it (both plain, absolute).
#[must_use]
pub fn within(path: &Path, root: &Path) -> bool {
    match (plain(path), plain(root)) {
        (Some(path), Some(root)) => path.starts_with(&root),
        _ => false,
    }
}

/// Opens the native picker for `kind` and remembers what the user picks;
/// `None` when they cancel. `file_name` is a save picker's suggestion.
///
/// # Errors
///
/// [`AppError::Refused`] through the read-only test bridge (a picker
/// starts a write).
#[tauri::command]
pub async fn pick_path(app: tauri::AppHandle, kind: PickKind, file_name: Option<String>) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt as _;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file();
        let chosen = match kind {
            PickKind::Folder => dialog.blocking_pick_folder(),
            PickKind::PackFolder => dialog.set_title("Open a handover").blocking_pick_folder(),
            PickKind::BackupFile => dialog
                .add_filter("AI Juggler backup", &["ajbackup"])
                .blocking_pick_file(),
            PickKind::BackupDest => dialog
                .add_filter("AI Juggler backup", &["ajbackup"])
                .set_file_name(file_name.unwrap_or_else(|| "AI Juggler backup.ajbackup".to_owned()))
                .blocking_save_file(),
            PickKind::RecoveryCodeDest => dialog
                .add_filter("Text", &["txt"])
                .set_file_name("AI Juggler recovery code.txt")
                .blocking_save_file(),
            PickKind::DiagnosticsDest => dialog
                .add_filter("Zip archive", &["zip"])
                .set_file_name(file_name.unwrap_or_else(|| "AI Juggler diagnostics.zip".to_owned()))
                .blocking_save_file(),
        };
        chosen.and_then(|path| path.into_path().ok())
    })
    .await
    .map_err(|error| AppError::Io {
        context: "open the picker",
        message: error.to_string(),
    })?;
    Ok(picked.map(|path| {
        remember(&path, Source::Picked(kind));
        path.to_string_lossy().into_owned()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANY: Allowed<'static> = Allowed {
        picked: &[PickKind::Folder],
        dropped: true,
    };

    #[test]
    fn a_picked_path_is_vouched_for_only_for_its_own_kind() {
        let path = "/tmp/aij-picked-test/backup.ajbackup";
        remember(Path::new(path), Source::Picked(PickKind::BackupDest));
        let dest = Allowed {
            picked: &[PickKind::BackupDest],
            dropped: false,
        };
        assert!(vouch(path, dest, |_| false).is_ok());
        assert!(vouch(path, ANY, |_| false).is_err(), "a save destination isn't a folder to work in");
    }

    #[test]
    fn a_dropped_folder_counts_where_drops_do() {
        record_dropped(&[PathBuf::from("/tmp/aij-dropped-test")]);
        assert!(vouch("/tmp/aij-dropped-test", ANY, |_| false).is_ok());
        let no_drops = Allowed {
            picked: &[PickKind::Folder],
            dropped: false,
        };
        assert!(vouch("/tmp/aij-dropped-test", no_drops, |_| false).is_err());
    }

    #[test]
    fn anything_else_needs_the_app_to_know_it() {
        assert!(vouch("/etc", ANY, |_| false).is_err());
        assert!(vouch("relative/path", ANY, |_| true).is_err(), "never a relative path");
        assert!(vouch("/Users/x/project/src", ANY, |p| within(p, Path::new("/Users/x/project"))).is_ok());
        assert!(
            vouch("/Users/x/project/../.ssh", ANY, |p| within(p, Path::new("/Users/x/project"))).is_err(),
            "`..` can't climb out of a known folder"
        );
    }
}
