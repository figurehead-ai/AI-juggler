//! AJ-E2320 (the review's A3.4, A7.1–A7.3, B5.1, E10, E11): the move area
//! every AI Juggler on this Mac shares (the Dev build and the installed one
//! keep different data folders): the locks on the folders a move touches,
//! the moves' journals, and a hold marker per profile a move keeps closed.
//! Before, a move's hold lived in one process's memory, so the other AI
//! Juggler's keep alive started a profile again mid-move.
//!
//! A marker (`held/<place key>`) is written before the move quits anything
//! and names the move and its owner (pid and start). It goes only once the
//! move is committed or rolled back (undone, or never past its plan). Keep
//! alive, Open, Restart and the automation watch of every AI Juggler leave
//! a held profile alone; a marker whose owner is gone is a move that
//! crashed, not a free profile.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use juggler_handover::moving::{MoveError, Phase, Transaction};
use sha2::{Digest as _, Sha256};

use crate::features::ownership::{ProcId, signal};

/// The shared area's folder under the OS data directory (Application
/// Support on macOS): one name for every edition.
pub const SHARED: &str = "AI Juggler Shared";

/// The move area of the AI Juggler whose data folder is `data_dir`: the
/// Mac's shared one for an edition's own data folder; a folder inside any
/// other (a test's or a lab's), which never touches the Mac's moves.
#[must_use]
pub fn area_at(data_dir: &Path) -> PathBuf {
    let own = crate::state::resolve_data_dir().is_ok_and(|own| own == data_dir);
    match dirs::data_dir() {
        Some(base) if own => base.join(SHARED).join("moves"),
        _ => data_dir.join("shared-moves"),
    }
}

/// Where the moves' journals are written.
#[must_use]
pub fn journals(area: &Path) -> PathBuf {
    area.join("journals")
}

/// Where the locks on the folders a move touches are.
#[must_use]
pub fn locks(area: &Path) -> PathBuf {
    area.join("locks")
}

fn held_dir(area: &Path) -> PathBuf {
    area.join("held")
}

/// Every folder holding moves' journals: the shared one, then this AI
/// Juggler's own from before the shared area (still read: its moves can
/// be undone).
#[must_use]
pub fn journal_folders(data_dir: &Path) -> [PathBuf; 2] {
    [
        journals(&area_at(data_dir)),
        data_dir.join(juggler_handover::moving::FOLDER),
    ]
}

/// Every move in [`journal_folders`], oldest first (one per id).
#[must_use]
pub fn all_moves(data_dir: &Path) -> Vec<Transaction> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Transaction> = journal_folders(data_dir)
        .iter()
        .flat_map(|folder| Transaction::all(folder))
        .filter(|txn| seen.insert(txn.manifest.id.clone()))
        .collect();
    out.sort_by_key(|txn| txn.manifest.created_ms);
    out
}

/// The folder of move `move_id`, wherever its journal is.
#[must_use]
pub fn move_folder(data_dir: &Path, move_id: &str) -> Option<PathBuf> {
    journal_folders(data_dir)
        .into_iter()
        .map(|folder| folder.join(move_id))
        .find(|dir| dir.is_dir())
}

/// Opens move `move_id`, wherever its journal is.
///
/// # Errors
///
/// No such move, or its manifest can't be read.
pub fn open_move(data_dir: &Path, move_id: &str) -> Result<Transaction, MoveError> {
    let dir = move_folder(data_dir, move_id)
        .unwrap_or_else(|| journals(&area_at(data_dir)).join(move_id));
    Transaction::open(&dir)
}

/// A profile's marker's file name: its place id, hashed (any spelling
/// is a safe file name).
#[must_use]
pub fn key(place_id: &str) -> String {
    use std::fmt::Write as _;
    Sha256::digest(place_id.as_bytes()).iter().take(16).fold(
        String::with_capacity(32),
        |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        },
    )
}

/// What a hold marker says.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Marker {
    /// The move holding the profile.
    pub move_id: String,
    /// The profile's place id.
    pub place: String,
    /// The AI Juggler running the move: its pid…
    pub owner_pid: u32,
    /// …and when it started, epoch microseconds.
    pub owner_start_micros: u64,
}

impl Marker {
    fn owner(&self) -> ProcId {
        ProcId {
            pid: self.owner_pid,
            start_micros: u128::from(self.owner_start_micros),
        }
    }

    /// Whether the move's owner still runs (the very process, by pid and
    /// start).
    #[must_use]
    pub fn owner_alive(&self) -> bool {
        signal::identify(&[self.owner_pid]).get(&self.owner_pid) == Some(&self.owner())
    }

    /// Whether this AI Juggler is the move's owner.
    #[must_use]
    pub fn mine(&self) -> bool {
        me() == Some(self.owner())
    }
}

/// This AI Juggler's own identity.
#[must_use]
pub fn me() -> Option<ProcId> {
    static ME: OnceLock<Option<ProcId>> = OnceLock::new();
    *ME.get_or_init(|| {
        let pid = std::process::id();
        signal::identify(&[pid]).get(&pid).copied()
    })
}

/// Whether, and how, a profile is held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Held {
    /// No move holds it.
    Free,
    /// A running move holds it (its marker, when it reads).
    Moving(Option<Marker>),
    /// A move whose owner is gone holds it: a crashed move.
    Crashed(Marker),
}

impl Held {
    /// Whether a move holds it.
    #[must_use]
    pub fn is_held(&self) -> bool {
        !matches!(self, Self::Free)
    }
}

fn read(path: &Path) -> Option<Marker> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Whether a move, of any AI Juggler, holds `place_id` in `area`.
#[must_use]
pub fn held(area: &Path, place_id: &str) -> Held {
    let path = held_dir(area).join(key(place_id));
    if !path.exists() {
        return Held::Free;
    }
    match read(&path) {
        Some(marker) if !marker.owner_alive() => Held::Crashed(marker),
        // One being written reads empty a moment: held all the same.
        marker => Held::Moving(marker),
    }
}

/// Whether any move holds any profile in `area`.
#[must_use]
pub fn any_held(area: &Path) -> bool {
    std::fs::read_dir(held_dir(area)).is_ok_and(|mut entries| {
        entries.any(|entry| {
            entry.is_ok_and(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        })
    })
}

/// Words: another move holds one of the profiles.
pub const BUSY: &str = "another move of these profiles is running";
/// Words: a move that didn't finish holds one of the profiles.
pub const UNFINISHED: &str =
    "a move of these profiles didn't finish (AI Juggler quit part way); undo that move first";

/// Writes the markers of `ids` for move `move_id`, owned by this AI
/// Juggler, or none: refused when another move holds one (running, or
/// crashed). The same move (its Undo) takes its own markers over once
/// their owner is gone.
///
/// # Errors
///
/// [`BUSY`], [`UNFINISHED`], or the folder can't be written.
pub fn take(area: &Path, ids: &[String], move_id: &str) -> Result<(), String> {
    let owner = me().ok_or("AI Juggler couldn't read its own process")?;
    let dir = held_dir(area);
    std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't hold the profiles: {e}"))?;
    let mut made = Vec::new();
    for id in ids {
        let marker = Marker {
            move_id: move_id.to_owned(),
            place: id.clone(),
            owner_pid: owner.pid,
            owner_start_micros: u64::try_from(owner.start_micros).unwrap_or(u64::MAX),
        };
        let path = dir.join(key(id));
        match claim(&path, &marker) {
            Ok(true) => made.push(path),
            Ok(false) => {}
            Err(why) => {
                for path in &made {
                    let _ = std::fs::remove_file(path);
                }
                return Err(why);
            }
        }
    }
    Ok(())
}

/// Writes `marker` at `path`: `true` when made new, `false` when this
/// move's own marker was taken over.
fn claim(path: &Path, marker: &Marker) -> Result<bool, String> {
    let bytes = serde_json::to_vec(marker).map_err(|e| e.to_string())?;
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path);
    match written {
        Ok(mut file) => file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map(|()| true)
            .map_err(|e| format!("couldn't hold the profiles: {e}")),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => match read(path) {
            Some(theirs)
                if theirs.move_id == marker.move_id
                    && (theirs.owner() == marker.owner() || !theirs.owner_alive()) =>
            {
                let next = path.with_extension("next");
                std::fs::write(&next, &bytes)
                    .and_then(|()| std::fs::rename(&next, path))
                    .map(|()| false)
                    .map_err(|e| format!("couldn't hold the profiles: {e}"))
            }
            Some(theirs) if !theirs.owner_alive() => Err(UNFINISHED.to_owned()),
            _ => Err(BUSY.to_owned()),
        },
        Err(error) => Err(format!("couldn't hold the profiles: {error}")),
    }
}

/// Removes the markers of `ids` that move `move_id` wrote (never another
/// move's).
pub fn release(area: &Path, ids: &[String], move_id: &str) {
    for id in ids {
        let path = held_dir(area).join(key(id));
        if read(&path).is_some_and(|marker| marker.move_id == move_id) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Whether move `move_id` is settled, so its markers may go: committed,
/// rolled back (undone, or never past its plan: nothing changed), or never
/// begun (no journal). A move part way, or one whose journal can't be
/// read, keeps its profiles held.
#[must_use]
pub fn settled(data_dir: &Path, move_id: &str) -> bool {
    let Some(dir) = move_folder(data_dir, move_id) else {
        return true;
    };
    Transaction::open(&dir).is_ok_and(|txn| {
        matches!(
            txn.manifest.phase,
            Phase::Committed | Phase::Undone | Phase::Planned
        )
    })
}

#[cfg(test)]
#[path = "hold_tests.rs"]
mod tests;
