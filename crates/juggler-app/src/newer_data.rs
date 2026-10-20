//! AJ-E1010 Req.2: going back to an older AI Juggler.
//!
//! Database migrations only go forward, so an older build refuses data a
//! newer one saved ([`juggler_core::CoreError::SchemaTooNew`]) rather than
//! risk it. Two ways back exist, and this module drives both:
//!
//! - **From the newer build** — Settings › About & updates › "Go back to
//!   an earlier version": the copy storage kept before it upgraded the data
//!   (`juggler.sqlite.before-v<N>`) is put in place as it is, the data as it
//!   is now is kept beside it, and the app quits so the older version can
//!   be installed ([`going_back_prepare_impl`]).
//! - **From the older build**, at launch: before this, the refusal was a
//!   line on stderr and an app that vanished. Now the launch says so in
//!   plain words and, when there is something it can read — a
//!   `before-v<N>` copy or a backup in `backups/` (every update writes one
//!   first) — offers to go back to it ([`at_launch`], [`go_back`]): the
//!   newer database is moved aside (`juggler.sqlite.from-newer-<ms>`, never
//!   deleted), the copy is put in place or the backup verified and
//!   restored, and the app starts on it.
//!
//! Account folders are not part of the data being swapped: they stay
//! where they are, so the accounts find their own folders. Any failure on
//! the way puts the newer database back as it was.

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_core::storage::Storage;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

const DB: &str = "juggler.sqlite";
const DB_FILES: [&str; 3] = ["", "-wal", "-shm"];

/// The last released version that reads each database schema (kept with
/// `docs/RELEASING.md` § Database changes). A schema not listed is named
/// by number.
const SCHEMA_RELEASES: &[(i64, &str)] = &[(27, "0.0.7"), (28, "0.0.8")];

/// The released version that reads `schema`, when it is known.
#[must_use]
pub fn version_reading(schema: i64) -> Option<&'static str> {
    SCHEMA_RELEASES
        .iter()
        .find(|(known, _)| *known == schema)
        .map(|(_, version)| *version)
}

/// What a way back is made of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// A backup file (`backups/*.ajbackup`): verified, then restored.
    Backup,
    /// The copy storage kept before upgrading (`juggler.sqlite.before-v<N>`):
    /// put in place as it is.
    Copy,
}

/// Something this build can go back to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GoBack {
    /// The file.
    pub source: PathBuf,
    /// A backup or the pre-upgrade copy.
    pub kind: Source,
    /// When it was written, epoch ms.
    pub made_at_ms: i64,
    /// The AI Juggler version that wrote it, when known.
    pub app_version: Option<String>,
}

fn modified_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}

/// The newest thing in the data folder that this build can open — a
/// pre-upgrade copy or a backup whose schema it reads — if any. At equal
/// times the copy wins (it is the data exactly as it was).
#[must_use]
pub fn newest_restorable(data_dir: &Path) -> Option<GoBack> {
    let supported = Storage::schema_version();
    let copies = juggler_core::storage::pre_migration_copies(&data_dir.join(DB))
        .into_iter()
        .filter(|(schema, _)| *schema <= supported)
        .map(|(schema, path)| GoBack {
            made_at_ms: modified_ms(&path),
            source: path,
            kind: Source::Copy,
            app_version: version_reading(schema).map(str::to_owned),
        });
    let backups = std::fs::read_dir(data_dir.join("backups"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ajbackup"))
        .filter_map(|path| {
            let manifest = crate::backup::read_manifest(&path).ok()?;
            (manifest.schema_version <= supported).then_some(GoBack {
                source: path,
                kind: Source::Backup,
                made_at_ms: manifest.created_at_ms,
                app_version: Some(manifest.app_version),
            })
        });
    copies
        .chain(backups)
        .max_by_key(|offer| (offer.made_at_ms, offer.kind == Source::Copy))
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

/// Goes back to `offer`: moves the newer database aside (kept), then puts
/// the pre-upgrade copy in place, or opens a fresh database with `open`
/// and restores the backup into it; either way the result must open.
/// Returns where the newer database was kept. On any failure the newer
/// database is put back as it was.
///
/// # Errors
///
/// The typed error of the move, the copy, the open, or the verify/restore
/// step.
pub fn go_back(
    data_dir: &Path,
    offer: &GoBack,
    open: &dyn Fn(PathBuf) -> AppResult<AppState>,
) -> AppResult<PathBuf> {
    let stamp = juggler_core::model::Timestamp::now().millis();
    let kept = data_dir.join(kept_name(stamp, ""));
    std::fs::rename(data_dir.join(DB), &kept).map_err(|e| AppError::io("keep newer data", &e))?;
    move_files(data_dir, &|suffix| format!("{DB}{suffix}"), &|suffix| {
        kept_name(stamp, suffix)
    });
    let restored = match offer.kind {
        Source::Copy => std::fs::copy(&offer.source, data_dir.join(DB))
            .map_err(|e| AppError::io("put the copy in place", &e))
            .and_then(|_| open(data_dir.to_path_buf()).map(drop)),
        Source::Backup => open(data_dir.to_path_buf())
            .map(Arc::new)
            .and_then(|state| {
                let plan = crate::backup::backup_verify_impl(&state, &offer.source)?;
                crate::backup::backup_restore_impl(&state, &offer.source, &plan.token)?;
                Ok(())
            }),
    };
    if let Err(error) = restored {
        // Put things back as they were: what was tried aside, the newer
        // database where it was.
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

// --- from the newer build: "Go back to an earlier version" ---------------

/// A version this install can go back to (Settings › About & updates).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GoingBackInfo {
    /// The database schema of the kept copy.
    #[ts(type = "number")]
    pub schema: i64,
    /// The released version that reads it, when known (`0.0.7`).
    pub version: Option<String>,
    /// When the copy was kept (the update), epoch ms.
    #[ts(type = "number")]
    pub kept_at_ms: i64,
}

/// Core: the earlier versions this install can go back to — the copies
/// kept before upgrading, older than the data now.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn going_back_options_impl(state: &Arc<AppState>) -> AppResult<Vec<GoingBackInfo>> {
    let current = crate::commands::lock_storage_arc(state)?.user_version()?;
    Ok(
        juggler_core::storage::pre_migration_copies(&state.data_dir.join(DB))
            .into_iter()
            .filter(|(schema, _)| *schema < current)
            .map(|(schema, path)| GoingBackInfo {
                schema,
                version: version_reading(schema).map(str::to_owned),
                kept_at_ms: modified_ms(&path),
            })
            .collect(),
    )
}

/// Core: prepares going back to the version that reads `schema` — refused
/// while a switch, a bring-in or a session would be cut short. The data
/// as it is now is kept beside it (`juggler.sqlite.from-v<now>-<ms>`), then
/// the copy kept before the upgrade replaces it as it is. The caller quits
/// the app at once (it can't run on the older data). Returns where the
/// data as it was is kept.
///
/// # Errors
///
/// [`AppError::Validation`] when something would be cut short or no copy
/// for `schema` is kept; the typed error of the keep or replace otherwise
/// (nothing is replaced if keeping fails).
pub fn going_back_prepare_impl(state: &Arc<AppState>, schema: i64) -> AppResult<PathBuf> {
    going_back_prepare_at(
        state,
        schema,
        juggler_core::model::Timestamp::now().millis(),
    )
}

/// [`going_back_prepare_impl`] as of `now_ms` (tests step past the
/// bring-in gap).
///
/// # Errors
///
/// Per [`going_back_prepare_impl`].
pub fn going_back_prepare_at(
    state: &Arc<AppState>,
    schema: i64,
    now_ms: i64,
) -> AppResult<PathBuf> {
    if let Some(busy) = crate::updates::busy_at(state, now_ms)? {
        return Err(AppError::Validation {
            subject: "going back",
            message: busy.words("Going back is possible"),
        });
    }
    let db = state.data_dir.join(DB);
    let mut storage = crate::commands::lock_storage_arc(state)?;
    let current = storage.user_version()?;
    let Some((_, copy)) = juggler_core::storage::pre_migration_copies(&db)
        .into_iter()
        .find(|(kept, _)| *kept == schema && *kept < current)
    else {
        return Err(AppError::Validation {
            subject: "going back",
            message: "there is no copy of the data from that version any more".to_owned(),
        });
    };
    let kept = state.data_dir.join(format!(
        "{DB}.from-v{current}-{}",
        juggler_core::model::Timestamp::now().millis()
    ));
    storage.snapshot_to(&kept)?;
    storage.replace_with_older(&copy)?;
    Ok(kept)
}

/// AJ-E1010: the earlier versions this install can go back to.
///
/// # Errors
///
/// Per [`going_back_options_impl`].
#[tauri::command]
pub fn going_back_options(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<GoingBackInfo>> {
    going_back_options_impl(state.inner())
}

/// AJ-E1010: prepares going back to an earlier version, then quits.
///
/// # Errors
///
/// Per [`going_back_prepare_impl`]; on success the app quits.
#[tauri::command]
pub fn going_back_prepare(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    schema: i64,
) -> AppResult<()> {
    let kept = going_back_prepare_impl(state.inner(), schema)?;
    eprintln!(
        "ai-juggler: ready to go back (schema {schema}); the data as it was is kept at {}",
        kept.display()
    );
    app.exit(0);
    Ok(())
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
            let what = match (offer.kind, offer.app_version.as_deref()) {
                (Source::Copy, _) => {
                    format!("the copy of your data kept on {when}, just before it was upgraded")
                }
                (Source::Backup, Some(version)) => {
                    format!("the backup from {when} (made by {app_name} {version})")
                }
                (Source::Backup, None) => format!("the backup from {when}"),
            };
            (
                title,
                format!(
                    "{opening}\n\nYou can go back to {what}. Your accounts, sessions and settings come back as they were then. What the newer version saved is kept beside it in the data folder; nothing is deleted.\n\nOr quit, and install the newer version again to keep working with its data."
                ),
                Some(match offer.kind {
                    Source::Copy => format!("Use the data from {when}"),
                    Source::Backup => format!("Use the backup from {when}"),
                }),
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
            match go_back(data_dir, &offer, &|dir| AppState::init(dir)) {
                Ok(kept) => {
                    eprintln!(
                        "ai-juggler: went back to {}; newer data kept at {}",
                        offer.source.display(),
                        kept.display()
                    );
                    AppState::init(data_dir.to_path_buf()).unwrap_or_else(|error| {
                        cannot_start(&error.to_string());
                        std::process::exit(1);
                    })
                }
                Err(error) => {
                    cannot_start(&format!(
                        "Going back didn't work, so the data is as it was: {error}"
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
