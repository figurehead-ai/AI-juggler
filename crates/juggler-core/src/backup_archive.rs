//! AJ-E1009 (ADR-0016): the `.ajbackup` archive format, and the reader
//! that stages one for a restore.
//!
//! The archive is a gzip tar: `manifest.json` first, then every file the
//! manifest lists with its size and sha256. Reading one is the part of a
//! restore that touches untrusted bytes (a backup someone sent), so it
//! lives here, below the app: the restore's check and the fuzz target
//! (AJ-E2008) run the very same code, and the fuzz build needs nothing
//! of the app. Writing a backup, and applying a restore, stay in the app.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::storage::Storage;

/// The manifest's `format` value.
pub const FORMAT: &str = "ai-juggler-backup";
/// The archive layout this build writes and reads.
pub const FORMAT_VERSION: u32 = 1;
/// AJ-E2006: what a backup may hold at most, so a crafted archive can't
/// fill the disk or the memory. A real one is far below each.
pub const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
/// At most this many files.
pub const MAX_FILES: usize = 200_000;
/// No one file larger than this.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
/// No more than this in all.
pub const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024 * 1024;

/// Why an archive could not be read or staged.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ArchiveError {
    /// The archive is refused: the message names the file and the reason.
    #[error("invalid restore: {0}")]
    Refused(String),
    /// Writing the staging folder failed.
    #[error("{context}: {message}")]
    Io {
        /// What was happening.
        context: &'static str,
        /// The failure.
        message: String,
    },
}

/// A refusal naming the archive and the reason.
pub fn refuse(path: &Path, reason: impl std::fmt::Display) -> ArchiveError {
    ArchiveError::Refused(format!("{} can't be restored: {reason}", path.display()))
}

fn io_error(context: &'static str, error: &impl std::fmt::Display) -> ArchiveError {
    ArchiveError::Io {
        context,
        message: error.to_string(),
    }
}

/// Lowercase hex of `bytes` (checksums, tokens).
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// One file in the archive.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export_to = "../../../ui/src/types/generated/")
)]
pub struct BackupFileInfo {
    /// Its path inside the archive.
    pub path: String,
    /// Its size in bytes.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub bytes: u64,
    /// Its sha256.
    pub sha256: String,
}

/// Where a backup came from.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export_to = "../../../ui/src/types/generated/")
)]
pub struct BackupOriginInfo {
    /// The writing Mac's Fleet fingerprint (to recognise it, not to be it).
    pub fingerprint: String,
    /// Its host name.
    pub label: String,
}

/// What a backup holds (ADR-0016).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export_to = "../../../ui/src/types/generated/")
)]
pub struct BackupManifestInfo {
    /// Always [`FORMAT`].
    pub format: String,
    /// The archive layout.
    pub format_version: u32,
    /// The database schema of the snapshot.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub schema_version: i64,
    /// The app version that wrote it.
    pub app_version: String,
    /// When, epoch ms.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at_ms: i64,
    /// The writing Mac.
    pub origin: BackupOriginInfo,
    /// Record counts (the restore is checked against them).
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number>"))]
    pub counts: BTreeMap<String, u64>,
    /// Every other entry, with its checksum.
    pub files: Vec<BackupFileInfo>,
    /// Left out on purpose; re-supply after a restore.
    pub withheld: Vec<String>,
    /// Never in a backup, and why.
    pub excluded: Vec<String>,
}

/// Staged entries: path → (bytes, sha256).
pub type Staged = BTreeMap<String, (u64, String)>;

/// A safe relative path inside the staging folder, or `None`.
fn staged_path(staging: &Path, name: &str) -> Option<PathBuf> {
    let relative = Path::new(name);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None; // absolute, `..` or `.` — never written
    }
    let allowed = name == "db/juggler.sqlite"
        || name == "settings/heuristics.json"
        || name.starts_with("parked/");
    allowed.then(|| staging.join(relative))
}

/// Stages the archive at `path` into `staging` exactly as a restore's
/// check does (every cap, every refusal), and nothing else: the fuzz
/// target's entry point (AJ-E2008).
///
/// # Errors
///
/// As a restore's check refuses the archive.
pub fn stage_archive(path: &Path, staging: &Path) -> Result<(), ArchiveError> {
    stage_entries(path, staging).map(|_| ())
}

/// Reads the whole archive into `staging`: its manifest (checked for
/// format and schema) and every entry's size and sha256.
///
/// # Errors
///
/// [`ArchiveError::Refused`] for a damaged, foreign, too-new or
/// over-large archive, or one with an entry it doesn't list;
/// [`ArchiveError::Io`] when the staging folder can't be written.
pub fn stage_entries(
    path: &Path,
    staging: &Path,
) -> Result<(BackupManifestInfo, Staged), ArchiveError> {
    let file = std::fs::File::open(path).map_err(|e| refuse(path, e))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut manifest: Option<BackupManifestInfo> = None;
    let mut seen: Staged = BTreeMap::new();
    for entry in archive.entries().map_err(|e| refuse(path, e))? {
        let mut entry = entry.map_err(|e| refuse(path, format!("damaged archive ({e})")))?;
        if !entry.header().entry_type().is_file() {
            return Err(refuse(path, "it holds something other than plain files"));
        }
        let name = entry
            .path()
            .map_err(|e| refuse(path, e))?
            .to_string_lossy()
            .into_owned();
        if manifest.is_none() {
            if name != "manifest.json" {
                return Err(refuse(path, "it is not an AI Juggler backup (no manifest)"));
            }
            let parsed = manifest_of(path, &mut entry)?;
            if parsed.format != FORMAT {
                return Err(refuse(path, "it is not an AI Juggler backup"));
            }
            if parsed.format_version != FORMAT_VERSION {
                return Err(refuse(
                    path,
                    format!(
                        "it uses backup format {} and this build reads format {FORMAT_VERSION} — update AI Juggler",
                        parsed.format_version
                    ),
                ));
            }
            if parsed.schema_version > Storage::schema_version() {
                return Err(refuse(
                    path,
                    format!(
                        "it was made by a newer AI Juggler (database schema {} > {}) — update first",
                        parsed.schema_version,
                        Storage::schema_version()
                    ),
                ));
            }
            limits(path, &parsed)?;
            manifest = Some(parsed);
            continue;
        }
        let target = staged_path(staging, &name)
            .ok_or_else(|| refuse(path, format!("unexpected entry {name}")))?;
        // Each file only as large as its manifest says (and listed there).
        let declared = manifest
            .as_ref()
            .and_then(|m| m.files.iter().find(|file| file.path == name))
            .map(|file| file.bytes)
            .ok_or_else(|| refuse(path, format!("{name} is not listed in its manifest")))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_error("restore staging", &e))?;
        }
        // Never over a file already staged: a name listed twice is refused.
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::AlreadyExists => refuse(path, format!("{name} is in it twice")),
                _ => io_error("restore staging", &e),
            })?;
        let mut entry = std::io::Read::take(&mut entry, declared.saturating_add(1));
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1024 * 1024];
        let mut bytes = 0u64;
        loop {
            let read = entry
                .read(&mut buf)
                .map_err(|e| refuse(path, format!("damaged archive ({e})")))?;
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
            out.write_all(&buf[..read])
                .map_err(|e| io_error("restore staging", &e))?;
            bytes += read as u64;
        }
        if bytes > declared {
            return Err(refuse(
                path,
                format!("{name} is larger than its manifest says"),
            ));
        }
        seen.insert(name, (bytes, hex(&hasher.finalize())));
    }
    let manifest = manifest.ok_or_else(|| refuse(path, "it is empty"))?;
    Ok((manifest, seen))
}

/// Reads only the manifest of the archive at `path`: what it holds and
/// which version and schema wrote it — without checking the rest.
///
/// # Errors
///
/// [`ArchiveError::Refused`] naming the file when it can't be read or
/// has no manifest first.
pub fn read_manifest(path: &Path) -> Result<BackupManifestInfo, ArchiveError> {
    let file = std::fs::File::open(path).map_err(|e| refuse(path, e))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut first = archive
        .entries()
        .map_err(|e| refuse(path, e))?
        .next()
        .ok_or_else(|| refuse(path, "it is empty"))?
        .map_err(|e| refuse(path, e))?;
    manifest_of(path, &mut first)
}

/// The manifest entry read and parsed, at most [`MAX_MANIFEST_BYTES`].
fn manifest_of(path: &Path, entry: &mut impl Read) -> Result<BackupManifestInfo, ArchiveError> {
    let mut raw = Vec::new();
    entry
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|e| refuse(path, e))?;
    if raw.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(refuse(path, "its manifest is far too large"));
    }
    serde_json::from_slice(&raw)
        .map_err(|e| refuse(path, format!("its manifest can't be read ({e})")))
}

/// AJ-E2006: the manifest's own claims within the caps.
fn limits(path: &Path, manifest: &BackupManifestInfo) -> Result<(), ArchiveError> {
    if manifest.files.len() > MAX_FILES {
        return Err(refuse(path, "it lists far too many files"));
    }
    let mut total = 0u64;
    for file in &manifest.files {
        if file.bytes > MAX_FILE_BYTES {
            return Err(refuse(path, format!("{} is far too large", file.path)));
        }
        total = total.saturating_add(file.bytes);
    }
    if total > MAX_TOTAL_BYTES {
        return Err(refuse(path, "it is far too large"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_two_digits_a_byte() {
        assert_eq!(hex(&[0x00, 0x0f, 0xab, 0xff]), "000fabff");
        assert_eq!(hex(&[]), "");
    }
}
