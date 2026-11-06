//! D-62 (AJ-E3203): the desktop app's profiles that aren't running, so
//! every one of them is an App.
//!
//! A host app that keeps its profiles side by side says so as data
//! (`HostApp::siblings`, from its adapter). Beside a profile AI Juggler
//! saw running, the names of the folders there are the user's other
//! profiles: only the names, never anything inside them
//! (`juggler_adapter::siblings`). It's off until the user says yes once,
//! on an App's page ("Find this app's other profiles"); the answer is
//! kept under [`KEY`] (a `places.` key, so the window's setting writer
//! can't set it), and Settings › Desktop apps turns it off again.
//!
//! A profile found this way is remembered like one seen running, marked
//! [`KnownPlace::found`]: started with the bundle its sibling ran from and
//! only its own folder (`start` checks both every time), named by its
//! folder until its sessions name a config folder. It goes when its
//! folder does, or when finding is turned off.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::book::{self, Book, KnownPlace, MAX};
use crate::error::AppResult;
use crate::features::conversations::places::HostRegistry;
use crate::state::AppState;

/// The user's answer: `on` or `off`; none yet, ask.
pub const KEY: &str = "places.find_profiles";

/// Where finding stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Not asked yet.
    Ask,
    /// The user said yes.
    On,
    /// The user said no, or turned it off.
    Off,
}

impl Answer {
    /// The stored value's answer.
    #[must_use]
    pub fn of(stored: Option<&str>) -> Self {
        match stored {
            Some("on") => Self::On,
            Some("off") => Self::Off,
            _ => Self::Ask,
        }
    }

    /// The answer in one word (`ask`, `on`, `off`).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::On => "on",
            Self::Off => "off",
        }
    }
}

/// One folder whose names are read: the host app, and the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    /// The host registry's id for the app.
    pub app_id: String,
    /// The folder its profiles are kept in.
    pub folder: PathBuf,
}

/// The folders whose names finding reads, each with the remembered
/// profile beside it that was seen running most recently (its bundle
/// starts the ones found there).
fn beside(book: &Book, hosts: &HostRegistry, home: &Path) -> BTreeMap<PathBuf, KnownPlace> {
    let mut out: BTreeMap<PathBuf, KnownPlace> = BTreeMap::new();
    for place in book.places.values().filter(|place| !place.found) {
        let Some(dir) = place.instance_dir.as_deref() else {
            continue;
        };
        let Some(host) = hosts.by_id(&place.app_id) else {
            continue;
        };
        let Ok(folder) = host.sibling_folder(dir, home) else {
            continue;
        };
        let newer = out
            .get(&folder)
            .is_none_or(|known| known.seen_ms < place.seen_ms);
        if newer {
            out.insert(folder, place.clone());
        }
    }
    out
}

/// The folders finding reads (or would, once the user says yes).
#[must_use]
pub fn folders(book: &Book, hosts: &HostRegistry, home: &Path) -> Vec<Folder> {
    beside(book, hosts, home)
        .into_iter()
        .map(|(folder, place)| Folder {
            app_id: place.app_id,
            folder,
        })
        .collect()
}

/// Reads the folder names beside each remembered profile and remembers
/// the profiles found there; a found one whose folder went is forgotten.
/// Never makes room by forgetting a profile that ran. Returns whether the
/// book changed.
pub fn find(book: &mut Book, hosts: &HostRegistry, home: &Path, now_ms: i64) -> bool {
    let before = book.clone();
    for (folder, sibling) in beside(book, hosts, home) {
        let Some(host) = hosts.by_id(&sibling.app_id) else {
            continue;
        };
        // A folder that can't be read, or holds too many, offers none.
        let listed = juggler_adapter::siblings::folders_in(&folder).unwrap_or_default();
        // The profiles that ran, by folder (compared as paths: one given
        // as `b-work/` is `b-work`).
        let ran: Vec<PathBuf> = book
            .places
            .values()
            .filter(|place| !place.found && place.app_id == sibling.app_id)
            .filter_map(|place| place.instance_dir.clone())
            .collect();
        book.places.retain(|_, place| {
            let Some(dir) = place.instance_dir.as_deref() else {
                return true;
            };
            !place.found
                || place.app_id != sibling.app_id
                || dir.parent() != Some(folder.as_path())
                || (listed.iter().any(|one| one == dir) && !ran.iter().any(|one| one == dir))
        });
        for dir in listed {
            let known = book.places.values().any(|place| {
                place.app_id == sibling.app_id
                    && place.instance_dir.as_deref() == Some(dir.as_path())
            });
            if known || book.places.len() >= MAX {
                continue;
            }
            let id = book::place_id(&sibling.app_id, Some(&dir));
            let instance = dir
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            book.places.insert(
                id,
                KnownPlace {
                    app_id: sibling.app_id.clone(),
                    name: host.shown_name().to_owned(),
                    instance,
                    bundle: sibling.bundle.clone(),
                    exe: sibling.exe.clone(),
                    given: Some(dir.clone()),
                    instance_dir: Some(dir),
                    account_id: None,
                    conversations: Vec::new(),
                    seen_ms: now_ms,
                    keep_alive: false,
                    paused: None,
                    found: true,
                },
            );
        }
    }
    *book != before
}

/// Forgets every profile found and not seen running since.
pub fn forget_found(book: &mut Book) -> bool {
    let count = book.places.len();
    book.places.retain(|_, place| !place.found);
    book.places.len() != count
}

/// The user's answer, as stored.
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn answer(state: &Arc<AppState>) -> AppResult<Answer> {
    let stored = crate::commands::lock_storage_arc(state)?
        .app_settings()
        .get(KEY)?;
    Ok(Answer::of(stored.as_deref()))
}

/// After the live view's full scan: finds the profiles beside the ones
/// remembered, when the user said yes. Returns whether the book changed.
pub fn after_scan(state: &Arc<AppState>, hosts: &HostRegistry, home: &Path, now_ms: i64) -> bool {
    if answer(state).ok() != Some(Answer::On) {
        return false;
    }
    super::with_book(state, |book| find(book, hosts, home, now_ms)).unwrap_or(false)
}

/// Keeps the user's answer: yes finds the profiles now, no forgets the
/// ones found.
///
/// # Errors
///
/// A poisoned lock; storage errors.
pub fn set(state: &Arc<AppState>, on: bool, home: &Path, now_ms: i64) -> AppResult<()> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(KEY, if on { "on" } else { "off" })?;
    let hosts = HostRegistry::of(&state.registry);
    let changed = super::with_book(state, |book| {
        if on {
            find(book, &hosts, home, now_ms)
        } else {
            forget_found(book)
        }
    })?;
    if changed {
        super::changed(state);
    }
    Ok(())
}

/// `path` with the home folder as `~`, as the screens show a folder.
#[must_use]
pub fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if !rest.as_os_str().is_empty() => {
            format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display())
        }
        _ => path.display().to_string(),
    }
}

#[cfg(test)]
#[path = "profiles_tests.rs"]
mod tests;
