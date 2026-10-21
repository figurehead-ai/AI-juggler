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
const SCHEMA_RELEASES: &[(i64, &str)] = &[(27, "0.0.7"), (28, "0.0.8"), (29, "0.0.9")];

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

/// What going back did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WentBack {
    /// Where the newer database was kept.
    pub kept: PathBuf,
    /// What the user still has to do (a backup leaves sign-ins and the
    /// values it withheld to re-enter); empty for a pre-upgrade copy.
    pub next_steps: Vec<String>,
}

/// Besides the database, what a backup restore replaces in the data
/// folder (`backup_restore_impl`).
const RESTORED_BESIDE: [&str; 2] = ["heuristics.json", "parked"];

/// Goes back to `offer`, all or nothing: moves the newer database aside
/// (kept), then puts the pre-upgrade copy in place, or opens a fresh
/// database with `open` and restores the backup into it; either way the
/// result must open. For a backup, what it also replaces
/// (`heuristics.json`, `parked/`) is moved aside first, with the newer
/// data. On any failure every path is put back as it was.
///
/// # Errors
///
/// The typed error of the move, the copy, the open, or the verify/restore
/// step.
pub fn go_back(
    data_dir: &Path,
    offer: &GoBack,
    open: &dyn Fn(PathBuf) -> AppResult<AppState>,
) -> AppResult<WentBack> {
    let stamp = juggler_core::model::Timestamp::now().millis();
    let kept = data_dir.join(kept_name(stamp, ""));
    std::fs::rename(data_dir.join(DB), &kept).map_err(|e| AppError::io("keep newer data", &e))?;
    move_files(data_dir, &|suffix| format!("{DB}{suffix}"), &|suffix| {
        kept_name(stamp, suffix)
    });
    let beside = |name: &str, tag: &str| data_dir.join(format!("{name}.{tag}-{stamp}"));
    if offer.kind == Source::Backup {
        for name in RESTORED_BESIDE {
            if data_dir.join(name).exists() {
                let _ = std::fs::rename(data_dir.join(name), beside(name, "from-newer"));
            }
        }
    }
    let restored: AppResult<Vec<String>> = match offer.kind {
        Source::Copy => std::fs::copy(&offer.source, data_dir.join(DB))
            .map_err(|e| AppError::io("put the copy in place", &e))
            .and_then(|_| open(data_dir.to_path_buf()).map(|_| Vec::new())),
        Source::Backup => open(data_dir.to_path_buf())
            .map(Arc::new)
            .and_then(|state| {
                let plan = crate::backup::backup_verify_impl(&state, &offer.source)?;
                let done = crate::backup::backup_restore_impl(&state, &offer.source, &plan.token)?;
                // It starts on the restored data right away: no restart.
                Ok(done
                    .next_steps
                    .into_iter()
                    .filter(|step| !step.starts_with("Restart"))
                    .collect())
            }),
    };
    match restored {
        Ok(next_steps) => Ok(WentBack { kept, next_steps }),
        Err(error) => {
            // Put everything back as it was: what was tried aside, the
            // newer data where it was.
            let _ = std::fs::remove_dir_all(data_dir.join("restore-staging"));
            move_files(data_dir, &|suffix| format!("{DB}{suffix}"), &|suffix| {
                format!("{DB}.unused-{stamp}{suffix}")
            });
            move_files(data_dir, &|suffix| kept_name(stamp, suffix), &|suffix| {
                format!("{DB}{suffix}")
            });
            if offer.kind == Source::Backup {
                for name in RESTORED_BESIDE {
                    if data_dir.join(name).exists() {
                        let _ = std::fs::rename(data_dir.join(name), beside(name, "unused"));
                    }
                    if beside(name, "from-newer").exists() {
                        let _ = std::fs::rename(beside(name, "from-newer"), data_dir.join(name));
                    }
                }
            }
            Err(error)
        }
    }
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
        &mut |_| {},
    )
}

/// [`going_back_prepare_impl`] as of `now_ms`, calling `finish` with the
/// kept path **while the storage lock is still held** — the app quits
/// there, so nothing can start between the check and the swap, and
/// nothing goes through the app's storage after it. The process ledger
/// and the cleanup repositories keep their own connections outside that
/// lock: in the instant before the exit, a reconciler tick could still
/// record a row in their tables, which the earlier version has too.
///
/// # Errors
///
/// Per [`going_back_prepare_impl`].
pub fn going_back_prepare_at(
    state: &Arc<AppState>,
    schema: i64,
    now_ms: i64,
    finish: &mut dyn FnMut(&Path),
) -> AppResult<PathBuf> {
    let db = state.data_dir.join(DB);
    let mut storage = crate::commands::lock_storage_arc(state)?;
    if let Some(busy) = crate::updates::busy_with(&storage, &state.updates, now_ms)? {
        return Err(AppError::Validation {
            subject: "going back",
            message: busy.words("Going back is possible"),
        });
    }
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
    // The older build finds it all in the database file itself.
    let _ = storage
        .raw_conn()
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
    finish(&kept);
    drop(storage);
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
pub fn going_back_prepare(state: tauri::State<'_, Arc<AppState>>, schema: i64) -> AppResult<()> {
    going_back_prepare_at(
        state.inner(),
        schema,
        juggler_core::model::Timestamp::now().millis(),
        &mut |kept| {
            eprintln!(
                "ai-juggler: ready to go back (schema {schema}); the data as it was is kept at {}",
                kept.display()
            );
            // Quit with the storage lock still held: nothing goes through
            // the app's storage (or is uploaded from it) after the swap.
            std::process::exit(0);
        },
    )?;
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
            let comes_back = match offer.kind {
                Source::Copy => {
                    "Your accounts, sessions and settings come back exactly as they were then."
                }
                Source::Backup => {
                    "Your accounts, sessions and settings come back as they were then; you may need to sign in to each account again and re-enter values the backup left out."
                }
            };
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
                    "{opening}\n\nYou can go back to {what}. {comes_back} What the newer version saved is kept beside it in the data folder; nothing is deleted.\n\nOr quit, and install the newer version again to keep working with its data."
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

/// The lock that says "this edition's data is in use": held by a running
/// AI Juggler from before it opens the data until it quits. Going back
/// moves database files, so it needs this lock (another instance would
/// otherwise lose what it hasn't written to the database file yet).
static INSTANCE_LOCK: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();

/// Takes the data folder's instance lock (`juggler.lock`); `false` when
/// another AI Juggler of this edition holds it.
pub fn hold_instance_lock(data_dir: &Path) -> bool {
    if INSTANCE_LOCK.get().is_some() {
        return true;
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data_dir.join("juggler.lock"))
    else {
        return true; // can't tell: don't block a normal launch
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = INSTANCE_LOCK.set(file);
            true
        }
        Err(std::fs::TryLockError::WouldBlock) => false,
        Err(std::fs::TryLockError::Error(_)) => true,
    }
}

/// At launch: shows the alert and, if the user chooses to go back, goes
/// back and returns the state to start with (after saying what is left to
/// do). Quitting exits. Only reached with the instance lock held (a second
/// launch hands over before it opens any data), so moving the database
/// files can't pull them from under another instance.
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
                Ok(went) => {
                    eprintln!(
                        "ai-juggler: went back to {}; newer data kept at {}",
                        offer.source.display(),
                        went.kept.display()
                    );
                    if !went.next_steps.is_empty() {
                        tell(
                            &format!(
                                "{app_name} is back to the data from {}",
                                day(offer.made_at_ms)
                            ),
                            &format!("Still to do:\n\n{}", went.next_steps.join("\n")),
                        );
                    }
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
            show_folder(data_dir);
            std::process::exit(1);
        }
        _ => std::process::exit(1),
    }
}

fn show_folder(dir: &Path) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(dir).status();
}

enum Choice {
    GoBack,
    ShowFolder,
    Quit,
}

/// `AIJ_NO_DIALOGS` set: the launch's dialogs (before any window exists)
/// are written to stderr instead, and every question is answered Quit —
/// for headless runs and the tests that start the real binary.
fn no_dialogs(title: &str, text: &str) -> bool {
    let quiet = std::env::var_os("AIJ_NO_DIALOGS").is_some();
    if quiet {
        eprintln!("ai-juggler: {title}\n{text}");
    }
    quiet
}

/// The launch alert (a native dialog on every desktop platform; it runs
/// before any window exists).
fn ask(title: &str, text: &str, go_back: Option<&str>) -> Choice {
    if no_dialogs(title, text) {
        return Choice::Quit;
    }
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

/// A plain notice with one button (before any window exists).
fn tell(title: &str, text: &str) {
    if no_dialogs(title, text) {
        return;
    }
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title(title)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::OkCustom("Continue".to_owned()))
        .show();
}

/// Says that the app can't start, and why (before any window exists).
pub fn cannot_start(reason: &str) {
    let app_name = crate::edition::current().app_name();
    if no_dialogs(&format!("{app_name} can't start"), reason) {
        return;
    }
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(format!("{app_name} can't start"))
        .set_description(reason)
        .set_buttons(rfd::MessageButtons::OkCustom("Quit".to_owned()))
        .show();
}
