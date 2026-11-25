//! How much room this computer has (ADR-0023): free and total space on
//! the disk a folder is on, and its memory. The local cache sizes itself
//! from these, so a small Mac keeps a small cache and a nearly full disk
//! gets its room back.
//!
//! - [`disk_space`]: `statvfs` on Unix (one call, no walk); not on Windows
//!   yet, which says so, and the cache then keeps to its floor.
//! - [`total_memory`]: the system's own count, read once.
//! - [`reclaimable`] (AJ-E2325): what deleting a file or folder gives
//!   back. On APFS each file's private (unshared) bytes, read by D-146's
//!   one audited `getattrlist` call (`private_size`); elsewhere, or where
//!   that call can't answer, allocated blocks (a ceiling on a disk that
//!   makes clones, said so).

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
    #[allow(
        clippy::useless_conversion,
        reason = "statvfs field widths differ by target (u32 block counts on macOS, u64 on Linux): useless on some only"
    )]
    let block = u64::from(stat.fragment_size());
    #[allow(
        clippy::useless_conversion,
        reason = "statvfs field widths differ by target (u32 block counts on macOS, u64 on Linux): useless on some only"
    )]
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
/// it) would give back to the disk, read from metadata only, links never
/// followed; a file with another name elsewhere (a hard link) gives
/// nothing back, since that name keeps it. On APFS (D-146) each file
/// counts its private bytes, the blocks no clone shares: an untouched
/// clone frees 0, so the answer is exact. Where that can't be read (not
/// APFS, or the call fails), a file counts its allocated blocks, never
/// its apparent length, and on a disk that makes clones the answer is a
/// ceiling ([`Reclaim::exact`] false), never a `du` total said as if it
/// were freed.
///
/// # Errors
///
/// `path` can't be read, or this system can't tell yet.
pub fn reclaimable(path: &Path) -> std::io::Result<Reclaim> {
    #[cfg(unix)]
    {
        measure(path, private_size)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(crate::unsupported("Measuring what deleting frees"))
    }
}

/// [`reclaimable`], each file's private bytes read by `private` (a test
/// passes one that always fails).
#[cfg(unix)]
fn measure(path: &Path, private: fn(&Path) -> Option<u64>) -> std::io::Result<Reclaim> {
    let clones = crate::transfer::clones_supported(path).unwrap_or(true);
    let mut exact = true;
    let bytes = allocated(path, clones.then_some(private), &mut exact)?;
    Ok(Reclaim { bytes, exact })
}

/// What deleting `path` and everything under it frees, in bytes: a
/// file's private bytes where `private` reads them, else its blocks
/// (clearing `exact` where clones may share them).
#[cfg(unix)]
fn allocated(
    path: &Path,
    private: Option<fn(&Path) -> Option<u64>>,
    exact: &mut bool,
) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt as _;
    let meta = std::fs::symlink_metadata(path)?;
    let blocks = meta.blocks().saturating_mul(512);
    let own = if !meta.is_file() {
        blocks
    } else if meta.nlink() > 1 {
        0
    } else if let Some(private) = private {
        private(path).unwrap_or_else(|| {
            *exact = false;
            blocks
        })
    } else {
        blocks
    };
    if !meta.is_dir() {
        return Ok(own);
    }
    let mut total = own;
    for entry in std::fs::read_dir(path)? {
        total = total.saturating_add(allocated(&entry?.path(), private, exact)?);
    }
    Ok(total)
}

/// D-146: the bytes of the file `path` that no other file shares (APFS's
/// `ATTR_CMNEXT_PRIVATESIZE`): what deleting it gives back, a clone's
/// shared blocks left out. `None` where the disk doesn't report it (not
/// APFS) or the call fails; the caller then counts blocks, as a ceiling.
/// Never follows a link.
///
/// D-146's carve-out, one of this crate's TWO `unsafe` calls (with
/// D-131's `thread::background_io`; a source test,
/// `the_only_unsafe_is_background_io_and_private_size`, keeps it so).
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "D-146: getattrlist(ATTR_CMNEXT_PRIVATESIZE) has no safe binding (libc has only the raw call, nix none); one audited read-only call, SAFETY below"
)]
fn private_size(path: &Path) -> Option<u64> {
    use nix::libc;
    use std::os::unix::ffi::OsStrExt as _;

    /// The answer, as <sys/attr.h> packs it: a `u32` length, the
    /// `attribute_set_t` of what was returned (five `u32`s, the fork
    /// group's at byte 20), then the `off_t` at byte 24. Room to spare,
    /// and 8-aligned so the `off_t` sits on its own alignment.
    #[repr(C, align(8))]
    struct Answer([u8; 64]);
    const RETURNED_FORK: usize = 20;
    const SIZE_AT: usize = 24;

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut list = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: libc::ATTR_CMN_RETURNED_ATTRS,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: libc::ATTR_CMNEXT_PRIVATESIZE,
    };
    let mut answer = Answer([0; 64]);

    // SAFETY: `getattrlist` is libSystem's, with libc's exact C signature
    // (<sys/attr.h>, macOS 10.4+; ATTR_CMNEXT_PRIVATESIZE since 10.13).
    // - Path: a NUL-terminated `CString` that outlives the call.
    // - Attribute list: a whole `attrlist` on the stack, `bitmapcount`
    //   ATTR_BIT_MAP_COUNT (5), asking only ATTR_CMN_RETURNED_ATTRS and,
    //   in the fork group read as the extended common group
    //   (FSOPT_ATTR_CMN_EXTENDED), ATTR_CMNEXT_PRIVATESIZE. The kernel
    //   only reads it.
    // - Buffer: `answer`, 64 bytes, 8-aligned, its true size passed as
    //   `attrBufSize`; the kernel writes at most that many bytes into it
    //   and nothing outside it, and keeps no pointer after returning.
    //   What it needs here is 32 (length, returned set, `off_t`).
    // - Errors: -1 with errno (the path is gone, not allowed, the disk
    //   rejects the options) returns `None`; a disk without the attribute
    //   leaves its bit out of the returned set, and a short length, both
    //   `None` too. The bytes are parsed below with bounds-checked safe
    //   reads, never cast to a struct.
    // - Read-only: it reads one file's metadata and writes nothing to the
    //   disk; FSOPT_NOFOLLOW keeps it off a link's target.
    // Nothing else here is unsafe.
    let status = unsafe {
        libc::getattrlist(
            c_path.as_ptr(),
            (&raw mut list).cast(),
            (&raw mut answer).cast(),
            std::mem::size_of::<Answer>(),
            libc::FSOPT_NOFOLLOW | libc::FSOPT_ATTR_CMN_EXTENDED,
        )
    };
    if status != 0 {
        return None;
    }
    let bytes = &answer.0;
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_ne_bytes)
    };
    let length = word(0)?;
    let returned = word(RETURNED_FORK)?;
    if returned & libc::ATTR_CMNEXT_PRIVATESIZE == 0 || length < 32 {
        return None;
    }
    let size = bytes.get(SIZE_AT..SIZE_AT + 8)?.try_into().ok()?;
    u64::try_from(i64::from_ne_bytes(size)).ok()
}

/// A file's private bytes: not read off macOS (blocks are counted).
#[cfg(all(unix, not(target_os = "macos")))]
fn private_size(path: &Path) -> Option<u64> {
    let _ = path;
    None
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

    /// `bytes` written to a new file `path` and on the disk (blocks are
    /// allocated by then, so a measure reads them).
    fn written(path: &Path, bytes: &[u8]) {
        use std::io::Write as _;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }

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
        written(&file, &[7_u8; 200_000]);
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

    /// A plain file's answer is exact: on APFS its private bytes (all of
    /// it, nothing shares them), elsewhere its blocks.
    #[test]
    #[cfg(unix)]
    fn a_plain_files_size_is_exact() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a");
        written(&file, &[3_u8; 100_000]);
        let reclaim = reclaimable(dir.path()).unwrap();
        assert!(reclaim.exact, "{reclaim:?}");
        assert!(reclaim.bytes >= 100_000, "{reclaim:?}");
    }

    /// D-146: on APFS an untouched clone frees 0, exactly; after an
    /// append it frees what was appended, rounded to blocks.
    #[test]
    #[cfg(target_os = "macos")]
    fn a_clone_frees_only_what_it_alone_holds() {
        use std::io::Write as _;
        let dir = tempfile::tempdir().unwrap();
        if !crate::transfer::clones_supported(dir.path()).unwrap() {
            return;
        }
        let original = dir.path().join("original");
        written(&original, &vec![5_u8; 1_000_000]);
        let backups = dir.path().join("backups");
        std::fs::create_dir(&backups).unwrap();
        let clone = backups.join("clone");
        crate::transfer::clone_file(&original, &clone).unwrap();
        assert_eq!(
            reclaimable(&clone).unwrap(),
            Reclaim {
                bytes: 0,
                exact: true
            },
            "an untouched clone frees nothing"
        );
        assert_eq!(reclaimable(&backups).unwrap().bytes, 0);

        let appended = 40_000_u64;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&clone)
            .unwrap();
        file.write_all(&vec![6_u8; 40_000]).unwrap();
        file.sync_all().unwrap();
        #[allow(
            clippy::useless_conversion,
            reason = "statvfs field widths differ by target (u32 block counts on macOS, u64 on Linux): useless on some only"
        )]
        let block = u64::from(
            nix::sys::statvfs::statvfs(dir.path())
                .unwrap()
                .fragment_size(),
        );
        let freed = reclaimable(&clone).unwrap();
        assert!(freed.exact, "{freed:?}");
        assert_eq!(freed.bytes % block, 0, "whole blocks: {freed:?}");
        assert!(
            freed.bytes >= appended && freed.bytes <= appended.div_ceil(block) * block + block,
            "the appended bytes, rounded to blocks (the shared last block rewritten): {freed:?}"
        );
        // The original gone, the clone holds every block alone.
        std::fs::remove_file(&original).unwrap();
        assert!(reclaimable(&clone).unwrap().bytes >= 1_000_000 + appended);
    }

    /// D-146: where the private size can't be read (a failing call, a disk
    /// that isn't APFS) the answer falls back to blocks, said as a ceiling
    /// on a cloning disk; never an error, never a wrong exact number.
    #[test]
    #[cfg(unix)]
    fn a_failing_or_non_apfs_read_falls_back_to_the_ceiling() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a");
        written(&file, &[3_u8; 100_000]);
        let clones = crate::transfer::clones_supported(dir.path()).unwrap();
        let failing = measure(dir.path(), |_| None).unwrap();
        assert_eq!(
            failing.exact, !clones,
            "a ceiling where clones may share blocks"
        );
        assert!(failing.bytes >= 100_000, "blocks counted: {failing:?}");
        // Not APFS (devfs), and a path that isn't there: no private size.
        assert_eq!(private_size(Path::new("/dev/null")), None);
        assert_eq!(private_size(&dir.path().join("missing")), None);
    }

    #[test]
    fn memory_is_counted() {
        assert!(total_memory().is_some_and(|bytes| bytes > 256 * 1024 * 1024));
    }
}
