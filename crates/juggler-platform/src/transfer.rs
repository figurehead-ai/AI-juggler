//! Moving and cloning files within one disk (ADR-0031, a switch by move):
//! what a file *is* on disk (its device and inode), whether two places are
//! on one disk, whether that disk makes clones, a clone that is a clone or
//! nothing, and a rename that never crosses a disk or replaces a file.
//!
//! The rule this seam keeps: **no silent full copy.** A move of a
//! conversation renames its files, so they keep their identity and take
//! no time or room; its backups are clones, independent of the original
//! from the first write on. Where either isn't possible (two disks, a disk
//! that doesn't clone, a system with no implementation yet) the seam says
//! [`std::io::ErrorKind::Unsupported`], and the caller declines the move in
//! words instead of copying gigabytes behind the user's back.
//!
//! - macOS: APFS clones (`clonefile`, which `std::fs::copy` uses first on
//!   macOS), allowed only after the disk is checked to be APFS and both
//!   places are on it, so the copy can't fall back to writing every byte.
//! - Linux and Windows: no clone implementation yet: every call that needs
//!   one says so. Identity and same-disk checks work on Unix.

use std::io;
use std::path::{Path, PathBuf};

/// What a file is on disk, beyond its name: a rename keeps it, a copy
/// doesn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileId {
    /// The disk (device) it is on.
    pub device: u64,
    /// Its inode on that disk.
    pub inode: u64,
}

/// What `path` itself is on disk (a link is the link, never followed).
///
/// # Errors
///
/// It can't be read, or this system can't tell yet.
pub fn file_id(path: &Path) -> io::Result<FileId> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::symlink_metadata(path)?;
        Ok(FileId {
            device: meta.dev(),
            inode: meta.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(crate::unsupported("telling a file's identity"))
    }
}

/// How many names `path` has (a hard-linked file has more than one).
///
/// # Errors
///
/// It can't be read, or this system can't tell yet.
pub fn link_count(path: &Path) -> io::Result<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        Ok(std::fs::symlink_metadata(path)?.nlink())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(crate::unsupported("counting a file's names"))
    }
}

/// The disk `path` is on: its own, or (for a place that doesn't exist
/// yet) that of the nearest folder above it that does.
///
/// # Errors
///
/// Nothing above it can be read, or this system can't tell yet.
pub fn device_of(path: &Path) -> io::Result<u64> {
    let existing = existing_ancestor(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no folder above it exists"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        Ok(std::fs::metadata(existing)?.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = existing;
        Err(crate::unsupported("telling which disk a folder is on"))
    }
}

/// Whether `a` and `b` (or the folders above them) are on one disk.
///
/// # Errors
///
/// As [`device_of`].
pub fn same_device(a: &Path, b: &Path) -> io::Result<bool> {
    Ok(device_of(a)? == device_of(b)?)
}

/// Whether the disk `dir` (or the nearest folder above it) is on makes
/// clones: a copy there shares the original's blocks until either is
/// written. macOS: the disk is APFS. Elsewhere: not yet.
///
/// # Errors
///
/// The disk can't be read.
pub fn clones_supported(dir: &Path) -> io::Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let existing = existing_ancestor(dir)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no folder above it exists"))?;
        let stats = nix::sys::statfs::statfs(existing).map_err(io::Error::from)?;
        Ok(stats.filesystem_type_name() == "apfs")
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = dir;
        Ok(false)
    }
}

/// Clones the file `from` to the new file `to`: a clone or nothing. It
/// is made only when `from` is a plain file (not a link), `to` doesn't
/// exist, and both are on one disk that makes clones; otherwise
/// [`io::ErrorKind::Unsupported`] (or `AlreadyExists`), and nothing is
/// written. The folder `to` goes in must exist.
///
/// # Errors
///
/// As above; the clone's own failure.
pub fn clone_file(from: &Path, to: &Path) -> io::Result<()> {
    let meta = std::fs::symlink_metadata(from)?;
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only a plain file is cloned (never a link or a folder)",
        ));
    }
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the clone's place is taken",
        ));
    }
    let dir = to.parent().unwrap_or_else(|| Path::new("."));
    if !same_device(from, dir)? || !clones_supported(dir)? {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this disk can't make a clone there, and a full copy isn't made in its place",
        ));
    }
    clone_checked(from, to)
}

#[cfg(target_os = "macos")]
fn clone_checked(from: &Path, to: &Path) -> io::Result<()> {
    // `std::fs::copy` on macOS is `fclonefileat` first; on one APFS disk
    // (checked above) that is the call that succeeds.
    std::fs::copy(from, to).map(|_| ())
}

#[cfg(not(target_os = "macos"))]
fn clone_checked(from: &Path, to: &Path) -> io::Result<()> {
    let _ = (from, to);
    Err(crate::unsupported("cloning a file"))
}

/// Clones the folder `from` (every plain file and folder below it) to the
/// new folder `to`, keeping its layout. A link anywhere inside refuses the
/// whole clone before anything is written (a move never follows or
/// recreates links).
///
/// # Errors
///
/// A link inside, `to` exists, or a clone fails (what was made stays for
/// the caller to remove: it is in a place the caller made).
pub fn clone_tree(from: &Path, to: &Path) -> io::Result<()> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    walk(from, Path::new(""), &mut files, &mut dirs)?;
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the clone's place is taken",
        ));
    }
    std::fs::create_dir(to)?;
    for dir in &dirs {
        std::fs::create_dir(to.join(dir))?;
    }
    for file in &files {
        clone_file(&from.join(file), &to.join(file))?;
    }
    Ok(())
}

/// Every plain file below `root`, by its path below it (folders listed
/// first, in `dirs`, parents before children); a link anywhere refuses.
///
/// # Errors
///
/// A link inside; a folder can't be read.
pub fn tree(root: &Path) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    walk(root, Path::new(""), &mut files, &mut dirs)?;
    Ok((files, dirs))
}

fn walk(
    root: &Path,
    below: &Path,
    files: &mut Vec<PathBuf>,
    dirs: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(root.join(below))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.file_name())
        .collect();
    entries.sort();
    for name in entries {
        let rel = below.join(&name);
        let meta = std::fs::symlink_metadata(root.join(&rel))?;
        if meta.is_dir() {
            dirs.push(rel.clone());
            walk(root, &rel, files, dirs)?;
        } else if meta.is_file() {
            files.push(rel);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is a link or special file", rel.display()),
            ));
        }
    }
    Ok(())
}

/// Renames `from` (a file or a folder) to `to` on the same disk, never
/// over anything: `to` must not exist, its folder must, and both must be
/// on one disk (a rename across disks is a copy, which this never makes).
/// Returns what was moved, checked to be the same file at `to`.
///
/// Callers hold the only writer of both places (the move's stopped
/// phase), so nothing can take `to` between the check and the rename.
///
/// # Errors
///
/// `to` exists ([`io::ErrorKind::AlreadyExists`]), the disks differ
/// ([`io::ErrorKind::Unsupported`]), the rename fails, or what arrived is
/// another file.
pub fn rename_on_device(from: &Path, to: &Path) -> io::Result<FileId> {
    let before = file_id(from)?;
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is taken", to.display()),
        ));
    }
    let dir = to.parent().unwrap_or_else(|| Path::new("."));
    if device_of(dir)? != before.device {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the two places are on different disks, and a move never copies",
        ));
    }
    std::fs::rename(from, to)?;
    let after = file_id(to)?;
    if after != before {
        return Err(io::Error::other(
            "the file that arrived isn't the one that left",
        ));
    }
    Ok(after)
}

/// Makes what was written in `dir` (names created, renamed, removed)
/// durable: the folder itself is synced. Where a system can't, nothing.
///
/// # Errors
///
/// The folder can't be opened or synced.
pub fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

/// Writes `bytes` to `path` atomically: a new file beside it, synced,
/// renamed over it, the folder synced. The file is readable by this user
/// only. Never leaves a half-written file at `path`.
///
/// # Errors
///
/// The write, sync or rename fails (the temporary file is removed).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    let temp = dir.join(format!(
        ".{}.aij-{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = crate::private_fs::create_new_private(&temp, bytes)
        .and_then(|()| std::fs::rename(&temp, path))
        .and_then(|()| sync_dir(dir));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|dir| std::fs::symlink_metadata(dir).is_ok() && !dir.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_keeps_the_file_and_never_replaces_one() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("a.jsonl");
        std::fs::write(&from, b"one\n").unwrap();
        let id = file_id(&from).unwrap();
        std::fs::create_dir(tmp.path().join("to")).unwrap();
        let to = tmp.path().join("to/a.jsonl");
        assert_eq!(rename_on_device(&from, &to).unwrap(), id);
        assert!(!from.exists());
        std::fs::write(&from, b"two\n").unwrap();
        let error = rename_on_device(&from, &to).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&to).unwrap(), b"one\n");
    }

    #[test]
    fn a_folder_renames_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("s");
        std::fs::create_dir_all(from.join("subagents")).unwrap();
        std::fs::write(from.join("subagents/a.jsonl"), b"x").unwrap();
        let to = tmp.path().join("t");
        rename_on_device(&from, &to).unwrap();
        assert_eq!(std::fs::read(to.join("subagents/a.jsonl")).unwrap(), b"x");
    }

    #[test]
    fn a_clone_is_a_clone_or_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("a");
        std::fs::write(&from, b"bytes").unwrap();
        let to = tmp.path().join("b");
        match clone_file(&from, &to) {
            Ok(()) => {
                assert!(clones_supported(tmp.path()).unwrap());
                assert_eq!(std::fs::read(&to).unwrap(), b"bytes");
                assert_ne!(file_id(&to).unwrap(), file_id(&from).unwrap());
                // Independent: writing the clone leaves the original.
                std::fs::write(&to, b"other").unwrap();
                assert_eq!(std::fs::read(&from).unwrap(), b"bytes");
            }
            Err(error) => {
                assert_eq!(error.kind(), io::ErrorKind::Unsupported);
                assert!(!to.exists(), "nothing written in its place");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_tree_with_a_link_inside_is_refused_before_anything_is_cloned() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("s");
        std::fs::create_dir_all(from.join("d")).unwrap();
        std::fs::write(from.join("d/a"), b"x").unwrap();
        std::os::unix::fs::symlink("/etc/hosts", from.join("d/link")).unwrap();
        let to = tmp.path().join("copy");
        let error = clone_tree(&from, &to).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(!to.exists());
    }

    #[test]
    fn an_atomic_write_replaces_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("r.json");
        std::fs::write(&path, b"{\"a\":1}").unwrap();
        write_atomic(&path, b"{\"a\":2}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\":2}");
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_place_not_made_yet_is_on_its_folders_disk() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(same_device(tmp.path(), &tmp.path().join("not/yet/here")).unwrap());
    }
}
