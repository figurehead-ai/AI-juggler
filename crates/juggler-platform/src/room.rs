//! How much room this computer has (ADR-0023): free and total space on
//! the disk a folder is on, and its memory. The local cache sizes itself
//! from these, so a small Mac keeps a small cache and a nearly full disk
//! gets its room back.
//!
//! - [`disk_space`]: `statvfs` on Unix (one call, no walk); not on Windows
//!   yet, which says so, and the cache then keeps to its floor.
//! - [`total_memory`]: the system's own count, read once.
//! - [`reclaimable`] (AJ-E2325): what deleting a file or folder gives
//!   back, from allocated blocks (a ceiling on a disk that makes clones).

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

/// What deleting a file or folder would give back to its disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reclaim {
    /// Bytes the disk gets back, at most (see `exact`).
    pub bytes: u64,
    /// `bytes` is what deleting frees. `false`: it is the most it frees,
    /// because the disk makes clones (APFS), whose blocks a file may share
    /// with its original until either is written, and only the blocks it
    /// alone holds come back.
    pub exact: bool,
}

/// AJ-E2325: what deleting `path` (a file, or a folder and everything in
/// it) would give back to the disk, read from metadata only: each item's
/// allocated blocks, never its apparent length; a file with another name
/// elsewhere (a hard link) gives nothing back, since that name keeps it;
/// links are never followed. On a disk that makes clones the answer is a
/// ceiling ([`Reclaim::exact`] false), never a `du` total said as if it
/// were freed.
///
/// # Errors
///
/// `path` can't be read, or this system can't tell yet.
pub fn reclaimable(path: &Path) -> std::io::Result<Reclaim> {
    #[cfg(unix)]
    {
        let bytes = allocated(path)?;
        let clones = crate::transfer::clones_supported(path).unwrap_or(true);
        Ok(Reclaim {
            bytes,
            exact: !clones,
        })
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(crate::unsupported("Measuring what deleting frees"))
    }
}

/// The blocks `path` and everything under it alone hold, in bytes.
#[cfg(unix)]
fn allocated(path: &Path) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt as _;
    let meta = std::fs::symlink_metadata(path)?;
    let own = if meta.is_file() && meta.nlink() > 1 {
        0
    } else {
        meta.blocks().saturating_mul(512)
    };
    if !meta.is_dir() {
        return Ok(own);
    }
    let mut total = own;
    for entry in std::fs::read_dir(path)? {
        total = total.saturating_add(allocated(&entry?.path())?);
    }
    Ok(total)
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

    /// AJ-E2325 (R6): a file counts its blocks, not its length; a hard
    /// link's file and a link's target count nothing; a folder counts
    /// what is in it.
    #[test]
    #[cfg(unix)]
    fn deleting_frees_allocated_blocks_never_a_linked_files_nor_a_links_target() {
        use std::os::unix::fs::MetadataExt as _;
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        std::fs::create_dir(&backups).unwrap();
        let file = backups.join("a.jsonl");
        std::fs::write(&file, vec![7_u8; 200_000]).unwrap();
        let blocks = std::fs::metadata(&file).unwrap().blocks() * 512;
        assert!(blocks >= 200_000);
        let alone = reclaimable(&file).unwrap();
        assert_eq!(alone.bytes, blocks);

        // A sparse file: long, but nearly no blocks.
        let sparse = backups.join("sparse");
        std::fs::File::create(&sparse)
            .unwrap()
            .set_len(50_000_000)
            .unwrap();
        assert!(
            reclaimable(&sparse).unwrap().bytes < 1_000_000,
            "blocks, not length"
        );

        // A file with a second name elsewhere: deleting this one frees nothing.
        let shared = backups.join("shared");
        std::fs::write(&shared, vec![1_u8; 300_000]).unwrap();
        std::fs::hard_link(&shared, dir.path().join("other-name")).unwrap();
        assert_eq!(reclaimable(&shared).unwrap().bytes, 0);

        // A link to a big file outside: the target isn't counted.
        let outside = dir.path().join("big");
        std::fs::write(&outside, vec![2_u8; 2_000_000]).unwrap();
        std::os::unix::fs::symlink(&outside, backups.join("link")).unwrap();

        let folder = reclaimable(&backups).unwrap();
        assert!(folder.bytes >= blocks, "the folder counts its file");
        assert!(
            folder.bytes < blocks + 1_000_000,
            "never the link's target nor the linked file"
        );
        assert!(reclaimable(&dir.path().join("missing")).is_err());
    }

    /// On a disk that makes clones the answer is a ceiling, said so; a
    /// clone of a file still there is counted at most its own blocks.
    #[test]
    #[cfg(unix)]
    fn on_a_cloning_disk_the_size_is_a_ceiling() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a");
        std::fs::write(&file, vec![3_u8; 100_000]).unwrap();
        let clones = crate::transfer::clones_supported(dir.path()).unwrap();
        let reclaim = reclaimable(dir.path()).unwrap();
        assert_eq!(
            reclaim.exact, !clones,
            "exact only where nothing shares blocks"
        );
    }

    #[test]
    fn memory_is_counted() {
        assert!(total_memory().is_some_and(|bytes| bytes > 256 * 1024 * 1024));
    }
}
