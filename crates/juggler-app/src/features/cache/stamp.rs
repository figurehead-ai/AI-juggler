//! ADR-0023: one cache entry on disk. Its first line says what it was
//! made from — which cache, the format, a print of the settings that
//! shaped it and its source's identity — and the rest is the entry. A
//! read that finds any of them different, or can't read the line, drops
//! the file and says "not cached": the caller reads the source again, so
//! stale words are never shown as current.
//!
//! Written whole through a temporary file and moved into place, readable
//! by this user only ([`juggler_platform::private_fs::write_private`]),
//! in a folder only this user can open. A read sets the file's time, so
//! trimming lets the least recently used go first.

use std::io::Read as _;
use std::path::Path;
use std::time::SystemTime;

use super::Gate;

/// What an entry was made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp<'a> {
    /// The cache it belongs to.
    pub cache: &'a str,
    /// The entry's format: another is not read.
    pub format: u32,
    /// A print of the settings that shape it ([`settings_print`]).
    pub settings: u64,
    /// Its source's identity: a version, or a path with its size and
    /// time, or a hash of them.
    pub source: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Head<'a> {
    c: &'a str,
    f: u32,
    s: String,
    src: &'a str,
}

/// The most a head line may be.
const HEAD_MAX: usize = 4096;

/// A print of the settings an entry is shaped by (FNV-1a over the
/// words, each ended).
#[must_use]
pub fn settings_print(parts: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in parts.iter().flat_map(|part| part.bytes().chain([0])) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// `payload` with `stamp`'s head line before it.
///
/// # Errors
///
/// The head can't be written (never, in practice).
pub fn encode(stamp: &Stamp<'_>, payload: &[u8]) -> std::io::Result<Vec<u8>> {
    let head = serde_json::to_vec(&Head {
        c: stamp.cache,
        f: stamp.format,
        s: format!("{:016x}", stamp.settings),
        src: stamp.source,
    })
    .map_err(std::io::Error::other)?;
    let mut bytes = Vec::with_capacity(head.len() + 1 + payload.len());
    bytes.extend_from_slice(&head);
    bytes.push(b'\n');
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

/// Why an entry wasn't used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stale {
    /// Its head can't be read.
    Corrupt,
    /// Made from something else: another format, settings or source.
    Changed,
}

/// The cache, format and settings print an entry's head names.
#[must_use]
pub fn head_of(bytes: &[u8]) -> Option<(String, u32, u64)> {
    let end = bytes.iter().take(HEAD_MAX).position(|b| *b == b'\n')?;
    let head: Head<'_> = serde_json::from_slice(&bytes[..end]).ok()?;
    let settings = u64::from_str_radix(&head.s, 16).ok()?;
    Some((head.c.to_owned(), head.f, settings))
}

/// The payload of `bytes`, when its head is `stamp`'s.
///
/// # Errors
///
/// Why it can't be used.
pub fn decode<'b>(bytes: &'b [u8], stamp: &Stamp<'_>) -> Result<&'b [u8], Stale> {
    let end = bytes
        .iter()
        .take(HEAD_MAX)
        .position(|b| *b == b'\n')
        .ok_or(Stale::Corrupt)?;
    let head: Head<'_> = serde_json::from_slice(&bytes[..end]).map_err(|_| Stale::Corrupt)?;
    let same = head.c == stamp.cache
        && head.f == stamp.format
        && head.s == format!("{:016x}", stamp.settings)
        && head.src == stamp.source;
    if same {
        Ok(&bytes[end + 1..])
    } else {
        Err(Stale::Changed)
    }
}

/// The entry at `path` made as `stamp` says, at most `max` bytes. A file
/// made otherwise, unreadable, too big or not a plain file is removed
/// (the source is read again); nothing is read while the cache is off.
#[must_use]
pub fn read(gate: &Gate, path: &Path, stamp: &Stamp<'_>, max: u64) -> Option<Vec<u8>> {
    if !gate.is_on() {
        return None;
    }
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_file() || meta.len() > max {
        drop_entry(path);
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(max + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if let Ok(payload) = decode(&bytes, stamp) {
        touch(path);
        Some(payload.to_vec())
    } else {
        drop_entry(path);
        None
    }
}

/// Writes `payload` at `path` as `stamp` says, whole or not at all and
/// readable by this user only; false (nothing written) while the cache is
/// off or the disk is nearly full.
///
/// # Errors
///
/// The folder or file can't be written.
pub fn write(gate: &Gate, path: &Path, stamp: &Stamp<'_>, payload: &[u8]) -> std::io::Result<bool> {
    if !gate.may_write() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        private_dir(dir)?;
    }
    let bytes = encode(stamp, payload)?;
    juggler_platform::private_fs::write_private(path, &bytes)?;
    gate.wrote(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
    Ok(true)
}

/// Makes `dir` closed to everyone but this user when it is missing. One
/// made before (by 0.3's first nightlies, open to others) stays; its
/// files are owner-only, and clearing the cache removes it.
///
/// # Errors
///
/// It can't be made.
pub fn private_dir(dir: &Path) -> std::io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    juggler_platform::private_fs::ensure_own_private_dir(dir)
}

/// Marks the entry at `path` used now (trimming lets the oldest go).
pub fn touch(path: &Path) {
    if let Ok(file) = std::fs::OpenOptions::new().append(true).open(path) {
        let _ = file.set_modified(SystemTime::now());
    }
}

fn drop_entry(path: &Path) {
    // Only a plain file or a link: never a folder someone put there.
    if std::fs::symlink_metadata(path).is_ok_and(|meta| !meta.is_dir()) {
        let _ = std::fs::remove_file(path);
    }
}
