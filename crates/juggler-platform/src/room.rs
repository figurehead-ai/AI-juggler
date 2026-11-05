//! How much room this computer has (ADR-0023): free and total space on
//! the disk a folder is on, and its memory. The local cache sizes itself
//! from these, so a small Mac keeps a small cache and a nearly full disk
//! gets its room back.
//!
//! - [`disk_space`]: `statvfs` on Unix (one call, no walk); not on Windows
//!   yet, which says so, and the cache then keeps to its floor.
//! - [`total_memory`]: the system's own count, read once.

use std::path::Path;

/// Space on the disk a folder is on, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
    /// What this user may still write.
    pub free: u64,
    /// The disk's size.
    pub total: u64,
}

/// The free and total space of the disk `path` is on.
///
/// # Errors
///
/// The folder can't be asked about (it isn't there), or this system has
/// no implementation yet.
#[cfg(unix)]
pub fn disk_space(path: &Path) -> std::io::Result<DiskSpace> {
    let stat = nix::sys::statvfs::statvfs(path).map_err(std::io::Error::from)?;
    #[allow(clippy::useless_conversion)]
    let block = u64::from(stat.fragment_size());
    #[allow(clippy::useless_conversion)]
    let (available, blocks) = (u64::from(stat.blocks_available()), u64::from(stat.blocks()));
    Ok(DiskSpace {
        free: available.saturating_mul(block),
        total: blocks.saturating_mul(block),
    })
}

/// The free and total space of the disk `path` is on: not on this system
/// yet.
///
/// # Errors
///
/// Always: [`crate::unsupported`].
#[cfg(not(unix))]
pub fn disk_space(path: &Path) -> std::io::Result<DiskSpace> {
    let _ = path;
    Err(crate::unsupported("Reading a disk's free space"))
}

/// A test's stand-in for a disk's free space.
pub type FreeSpaceReader = fn(&Path) -> u64;

thread_local! {
    /// A test's stand-in for the disk's free space, on its own thread.
    static FREE_SPACE_FOR_TESTS: std::cell::Cell<Option<FreeSpaceReader>> =
        const { std::cell::Cell::new(None) };
}

/// What this user may still write on the disk `path` is on: the
/// system's answer ([`disk_space`]), or on a thread where a test set one
/// ([`set_free_space_on_this_thread`]), the test's.
///
/// # Errors
///
/// As [`disk_space`].
pub fn free_space(path: &Path) -> std::io::Result<u64> {
    if let Some(reader) = FREE_SPACE_FOR_TESTS.with(std::cell::Cell::get) {
        return Ok(reader(path));
    }
    disk_space(path).map(|space| space.free)
}

/// For tests: [`free_space`] on this thread reads `reader` instead of the
/// disk (a nearly full disk, or one filling up mid-copy); `None` puts the
/// disk back.
pub fn set_free_space_on_this_thread(reader: Option<FreeSpaceReader>) {
    FREE_SPACE_FOR_TESTS.with(|cell| cell.set(reader));
}

/// Whether copying `from` into the folder `to_dir` takes no room: the
/// copy is a clone (macOS, both on one APFS disk, which the startup disk
/// always is). Anywhere else a copy writes every byte.
///
/// # Errors
///
/// Either can't be read.
#[cfg(target_os = "macos")]
pub fn copy_is_a_clone(from: &Path, to_dir: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt as _;
    Ok(std::fs::metadata(from)?.dev() == std::fs::metadata(to_dir)?.dev())
}

/// Whether copying `from` into `to_dir` takes no room: never assumed off
/// macOS.
///
/// # Errors
///
/// Never.
#[cfg(not(target_os = "macos"))]
pub fn copy_is_a_clone(from: &Path, to_dir: &Path) -> std::io::Result<bool> {
    let _ = (from, to_dir);
    Ok(false)
}

/// This computer's memory in bytes, when the system says.
#[must_use]
pub fn total_memory() -> Option<u64> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    Some(system.total_memory()).filter(|bytes| *bytes > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn a_folders_disk_has_room_and_a_size() {
        let dir = tempfile::tempdir().unwrap();
        let space = disk_space(dir.path()).unwrap();
        assert!(space.total > 0, "a disk has a size");
        assert!(space.free <= space.total, "free fits in the disk");
        assert!(disk_space(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn memory_is_counted() {
        assert!(total_memory().is_some_and(|bytes| bytes > 256 * 1024 * 1024));
    }
}
