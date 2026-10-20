//! AJ-E1010 Req.2: going back to an older AI Juggler.
//!
//! Database migrations only go forward, so an older build refuses data a
//! newer one saved ([`juggler_core::CoreError::SchemaTooNew`]) rather than
//! risk it. Before this, that refusal was a line on stderr and an app
//! that vanished at launch. Now the launch says so in plain words and —
//! when `backups/` holds a backup this build can read (every update
//! writes one first) — offers to go back to it:
//!
//! 1. the newer database is moved aside, next to it, never deleted
//!    (`juggler.sqlite.from-newer-<ms>`, with its `-wal`/`-shm`);
//! 2. a fresh database is opened and the backup is verified and restored
//!    into it, exactly as Settings › Backup & restore does;
//! 3. the app starts on the restored data.
//!
//! Account folders are not part of the data being swapped: they stay
//! where they are, so the restored accounts find their own folders.
//! Any failure on the way puts the newer database back as it was.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::storage::Storage;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

const DB: &str = "juggler.sqlite";
const DB_FILES: [&str; 3] = ["", "-wal", "-shm"];

/// A backup this build can go back to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GoBack {
    /// The `.ajbackup` file.
    pub backup: PathBuf,
    /// When it was written, epoch ms.
    pub made_at_ms: i64,
    /// The AI Juggler version that wrote it.
    pub app_version: String,
}

/// The newest backup in `<data>/backups/` that this build can restore
/// (its database schema is one this build reads), if any.
#[must_use]
pub fn newest_restorable(data_dir: &Path) -> Option<GoBack> {
    let supported = Storage::schema_version();
    std::fs::read_dir(data_dir.join("backups"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ajbackup"))
        .filter_map(|path| {
            let manifest = crate::backup::read_manifest(&path).ok()?;
            (manifest.schema_version <= supported).then_some(GoBack {
                backup: path,
                made_at_ms: manifest.created_at_ms,
                app_version: manifest.app_version,
            })
        })
        .max_by_key(|offer| offer.made_at_ms)
}

/// Where the newer database is kept when going back.
fn kept_name(stamp: i64, suffix: &str) -> String {
    format!("{DB}.from-newer-{stamp}{suffix}")
}

fn move_files(data_dir: &Path, from: &dyn Fn(&str) -> String, to: &dyn Fn(&str) -> String) {
    for suffix in DB_FILES {
        let source = data_dir.join(from(suffix));
        if source.exists() {
            let _ = std::fs::rename(&source, data_dir.join(to(suffix)));
        }
    }
}

/// Goes back to `backup`: moves the newer database aside (kept), opens a
/// fresh one with `open`, and restores the backup into it. Returns where
/// the newer database was kept. On any failure the newer database is put
/// back as it was.
///
/// # Errors
///
/// The typed error of the move, the open, or the verify/restore step.
pub fn go_back(
    data_dir: &Path,
    backup: &Path,
    open: &dyn Fn(PathBuf) -> AppResult<AppState>,
) -> AppResult<PathBuf> {
    let stamp = juggler_core::model::Timestamp::now().millis();
    let kept = data_dir.join(kept_name(stamp, ""));
    std::fs::rename(data_dir.join(DB), &kept).map_err(|e| AppError::io("keep newer data", &e))?;
    move_files(data_dir, &|suffix| format!("{DB}{suffix}"), &|suffix| {
        kept_name(stamp, suffix)
    });
    let restored = open(data_dir.to_path_buf())
        .map(Arc::new)
        .and_then(|state| {
            let plan = crate::backup::backup_verify_impl(&state, backup)?;
            crate::backup::backup_restore_impl(&state, backup, &plan.token)?;
            Ok(())
        });
    if let Err(error) = restored {
        // Put things back as they were: the fresh database aside, the
        // newer one where it was.
        move_files(data_dir, &|suffix| format!("{DB}{suffix}"), &|suffix| {
            format!("{DB}.unused-{stamp}{suffix}")
        });
        move_files(data_dir, &|suffix| kept_name(stamp, suffix), &|suffix| {
            format!("{DB}{suffix}")
        });
        return Err(error);
    }
    Ok(kept)
}

/// "20 Oct 2026" for an epoch-ms time (UTC day).
fn day(ms: i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let date = crate::commands::epoch_today(ms);
    let month = usize::try_from(date.month.saturating_sub(1)).unwrap_or(0);
    format!("{} {} {}", date.day, MONTHS[month.min(11)], date.year)
}

/// What the launch says when this build meets newer data: title, text,
/// and the label of the go-back button (`None`: there is no backup to go
/// back to).
#[must_use]
pub fn launch_message(
    app_name: &str,
    this_version: &str,
    offer: Option<&GoBack>,
) -> (String, String, Option<String>) {
    let title = format!("This data is from a newer {app_name}");
    let opening = format!(
        "{app_name} {this_version} can't open the data that a newer version saved, so it can't start with it. Nothing has been changed."
    );
    match offer {
        Some(offer) => {
            let when = day(offer.made_at_ms);
            (
                title,
                format!(
                    "{opening}\n\nYou can go back to the backup from {when} (made by {app_name} {}). Your accounts, sessions and settings come back as they were then. What the newer version saved is kept beside it in the data folder; nothing is deleted.\n\nOr quit, and install the newer version again to keep working with its data.",
                    offer.app_version
                ),
                Some(format!("Use the backup from {when}")),
            )
        }
        None => (
            title,
            format!(
                "{opening}\n\nThere is no backup this version can read. To keep working, install the newer version again from the releases page. To start this version afresh instead, quit it and move the data folder somewhere else first."
            ),
            None,
        ),
    }
}

/// At launch: shows the alert and, if the user chooses the backup, goes
/// back to it and returns the state to start with. Quitting (or no
/// window system) exits.
#[must_use]
pub fn at_launch(data_dir: &Path, found: i64, supported: i64) -> AppState {
    eprintln!(
        "ai-juggler: the data in {} is schema {found}; this build reads up to {supported}",
        data_dir.display()
    );
    let app_name = crate::edition::current().app_name();
    let offer = newest_restorable(data_dir);
    let (title, text, go_back_label) =
        launch_message(app_name, env!("CARGO_PKG_VERSION"), offer.as_ref());
    let chosen = ask(&title, &text, go_back_label.as_deref());
    match (chosen, offer) {
        (Choice::GoBack, Some(offer)) => {
            match go_back(data_dir, &offer.backup, &|dir| AppState::init(dir)) {
                Ok(kept) => {
                    eprintln!(
                        "ai-juggler: restored {}; newer data kept at {}",
                        offer.backup.display(),
                        kept.display()
                    );
                    AppState::init(data_dir.to_path_buf()).unwrap_or_else(|error| {
                        cannot_start(&error.to_string());
                        std::process::exit(1);
                    })
                }
                Err(error) => {
                    cannot_start(&format!(
                        "Going back to the backup didn't work, so the data is as it was: {error}"
                    ));
                    std::process::exit(1);
                }
            }
        }
        (Choice::ShowFolder, _) => {
            let _ = std::process::Command::new("open").arg(data_dir).status();
            std::process::exit(1);
        }
        _ => std::process::exit(1),
    }
}

enum Choice {
    GoBack,
    ShowFolder,
    Quit,
}

#[cfg(target_os = "macos")]
fn ask(title: &str, text: &str, go_back: Option<&str>) -> Choice {
    let first = go_back.unwrap_or("Show the data folder").to_owned();
    let answer = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(title)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            first.clone(),
            "Quit".to_owned(),
        ))
        .show();
    match answer {
        rfd::MessageDialogResult::Custom(label) if label == first => {
            if go_back.is_some() {
                Choice::GoBack
            } else {
                Choice::ShowFolder
            }
        }
        _ => Choice::Quit,
    }
}

#[cfg(not(target_os = "macos"))]
fn ask(title: &str, text: &str, _go_back: Option<&str>) -> Choice {
    eprintln!("{title}\n\n{text}");
    Choice::Quit
}

/// Says that the app can't start, and why (before any window exists).
pub fn cannot_start(reason: &str) {
    #[cfg(target_os = "macos")]
    {
        let app_name = crate::edition::current().app_name();
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title(format!("{app_name} can't start"))
            .set_description(reason)
            .set_buttons(rfd::MessageButtons::OkCustom("Quit".to_owned()))
            .show();
    }
    #[cfg(not(target_os = "macos"))]
    eprintln!("{reason}");
}
