//! AJ-E1511: reading a saved pack back. A pack is checked whole before
//! anything is done with it: its manifest names every file, each file is
//! a plain file inside the pack whose bytes match the manifest's hash, its
//! `state.json` is this format's, and no file holds credential material
//! (the same always-on guard [`crate::pack::write_pack`] runs). Only then
//! is it described, or a session primed with it.

use std::path::{Component, Path};

use crate::manifest::{ManifestRow, credential_scan, sha256_hex};
use crate::pack::{PACK_FORMAT_VERSION, PackState};

/// The most one file in a pack may weigh (AJ-E1513: also the largest
/// transcript staged in the background).
pub const MAX_FILE: u64 = 16 * 1024 * 1024;
/// The most a whole pack may weigh.
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
/// The files every pack has.
const REQUIRED: [&str; 3] = ["PACK.md", "conversation.md", "state.json"];

/// A pack, read and checked.
#[derive(Clone, Debug)]
pub struct OpenedPack {
    /// Its `state.json`.
    pub state: PackState,
    /// Its brief (`PACK.md`).
    pub pack_md: String,
    /// Its conversation (`conversation.md`), redacted when it was made.
    pub conversation_md: String,
    /// Its manifest, as read.
    pub rows: Vec<ManifestRow>,
    /// The manifest's hash: names exactly these bytes (every file's hash
    /// is in it), so a pack can be checked unchanged between looking at
    /// it and using it.
    pub digest: String,
}

/// Why a folder can't be opened as a pack (plain words reach the user).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PackReadError {
    /// Not a pack folder at all.
    #[error("this isn't a saved handover: {0}")]
    NotAPack(String),
    /// A file is missing, a link, or outside the pack.
    #[error("this handover is incomplete or unsafe: {0}")]
    Unsafe(String),
    /// A file's bytes don't match the manifest.
    #[error("this handover was changed after it was made: {0} doesn't match")]
    Changed(String),
    /// Made by a newer AI Juggler.
    #[error("this handover was made by a newer AI Juggler (format {0}); update to open it")]
    Newer(u32),
    /// Too big to be one of ours.
    #[error("this handover is too big to open ({0})")]
    TooBig(String),
    /// Holds what looks like a credential.
    #[error(
        "this handover holds what looks like a key or password ({0} found), so it isn't opened"
    )]
    Credentials(usize),
}

/// Whether `rel` is a plain relative path inside the pack.
fn inside(rel: &str) -> bool {
    !rel.is_empty()
        && Path::new(rel)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

/// Reads one file of the pack: a plain file (never a link), inside it,
/// within the size limit.
fn read(dir: &Path, rel: &str) -> Result<Vec<u8>, PackReadError> {
    if !inside(rel) {
        return Err(PackReadError::Unsafe(format!(
            "{rel} is outside the handover"
        )));
    }
    let path = dir.join(rel);
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|_| PackReadError::Unsafe(format!("{rel} is missing")))?;
    if !meta.is_file() {
        return Err(PackReadError::Unsafe(format!("{rel} isn't a plain file")));
    }
    if meta.len() > MAX_FILE {
        return Err(PackReadError::TooBig(format!(
            "{rel} is {} MB",
            meta.len() / (1024 * 1024)
        )));
    }
    std::fs::read(&path).map_err(|_| PackReadError::Unsafe(format!("{rel} can't be read")))
}

/// Reads and checks the pack in `dir`.
///
/// # Errors
///
/// See [`PackReadError`]: nothing is returned unless every check passes.
pub fn read_pack(dir: &Path) -> Result<OpenedPack, PackReadError> {
    let folder = std::fs::symlink_metadata(dir)
        .map_err(|_| PackReadError::NotAPack("the folder can't be read".into()))?;
    if !folder.is_dir() {
        return Err(PackReadError::NotAPack("pick the handover's folder".into()));
    }
    let manifest = read(dir, "manifest.json")
        .map_err(|_| PackReadError::NotAPack("it has no manifest.json".into()))?;
    let rows: Vec<ManifestRow> = serde_json::from_slice(&manifest)
        .map_err(|_| PackReadError::NotAPack("its manifest.json can't be read".into()))?;
    for required in REQUIRED {
        if !rows
            .iter()
            .any(|row| row.rel_path == required && row.disposition != "drop")
        {
            return Err(PackReadError::NotAPack(format!(
                "its manifest doesn't list {required}"
            )));
        }
    }
    let mut total = 0u64;
    let mut found = 0usize;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for row in &rows {
        if !inside(&row.rel_path) {
            return Err(PackReadError::Unsafe(format!(
                "{} is outside the handover",
                row.rel_path
            )));
        }
        if row.disposition == "drop" {
            continue; // a drop row names what didn't travel; it has no bytes
        }
        let bytes = read(dir, &row.rel_path)?;
        total += bytes.len() as u64;
        if total > MAX_TOTAL {
            return Err(PackReadError::TooBig(format!(
                "over {} MB",
                MAX_TOTAL / (1024 * 1024)
            )));
        }
        if sha256_hex(&bytes) != row.sha256 {
            return Err(PackReadError::Changed(row.rel_path.clone()));
        }
        found += credential_scan(&bytes).len();
        files.push((row.rel_path.clone(), bytes));
    }
    if found > 0 {
        return Err(PackReadError::Credentials(found));
    }
    let text = |name: &str| {
        files
            .iter()
            .find(|(rel, _)| rel == name)
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default()
    };
    let state: PackState = serde_json::from_str(&text("state.json"))
        .map_err(|_| PackReadError::NotAPack("its state.json can't be read".into()))?;
    if state.format_version > PACK_FORMAT_VERSION {
        return Err(PackReadError::Newer(state.format_version));
    }
    if state.format_version == 0 {
        return Err(PackReadError::NotAPack(
            "its state.json has no format version".into(),
        ));
    }
    if !matches!(
        state.fidelity.level.as_str(),
        "F0" | "F1" | "F2" | "F3" | "F4"
    ) {
        return Err(PackReadError::NotAPack(format!(
            "its fidelity {:?} isn't one AI Juggler gives",
            state.fidelity.level
        )));
    }
    Ok(OpenedPack {
        pack_md: text("PACK.md"),
        conversation_md: text("conversation.md"),
        state,
        rows,
        digest: sha256_hex(&manifest),
    })
}
