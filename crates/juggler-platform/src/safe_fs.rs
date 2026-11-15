//! AJ-E3013 (security review L5): files inside a folder, read and written
//! without a check-then-use race. The one helper every connector writer
//! and every attestation reader goes through.
//!
//! - A walk starts at a folder opened once ([`Dir::root`]: a real folder,
//!   never itself a link) and goes down one name at a time with
//!   `openat(O_DIRECTORY | O_NOFOLLOW)`: a link anywhere on the way stops
//!   it, and a folder swapped for a link after a check can't redirect it
//!   (there is no separate check).
//! - A file is opened with `O_NOFOLLOW`, and everything about it (a regular
//!   file, its size, its owner and mode when asked) is checked on the open
//!   file itself (`fstat`), then read from that same descriptor.
//! - Writes create with `O_CREAT | O_EXCL | O_NOFOLLOW`, replace through a
//!   temporary file renamed over the old one in the same folder, and
//!   remove with `unlinkat` in the folder already opened.
//!
//! Where the system has none of this (Windows), every call says
//! "unsupported" and nothing is read or written.

use std::ffi::OsStr;
use std::io;
use std::path::{Component, Path, PathBuf};

/// What a file must be, checked on the opened file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expect {
    /// The most bytes it may hold.
    pub max: u64,
    /// Owned by whoever owns the walk's root folder, and no one else can
    /// write it (`mode & 0o022 == 0`): a record another tool keeps in its
    /// own folder, fit to be trusted for an attestation.
    pub owned: bool,
}

impl Expect {
    /// Any regular file of at most `max` bytes.
    #[must_use]
    pub const fn small(max: u64) -> Self {
        Self { max, owned: false }
    }

    /// A regular file of at most `max` bytes, the root owner's, that no
    /// one else can write.
    #[must_use]
    pub const fn owned(max: u64) -> Self {
        Self { max, owned: true }
    }
}

/// The name of the temporary file a replacement of `name` writes first.
#[must_use]
pub fn temp_name(name: &OsStr) -> std::ffi::OsString {
    let mut temp = std::ffi::OsString::from(".");
    temp.push(name);
    temp.push(".ai-juggler-new");
    temp
}

/// L6 (round 2): what [`Dir::replace`] refuses with when something that
/// isn't its own leftover sits at the temporary name: a link, a folder,
/// anything not a plain file, or a file not the folder owner's. It's never
/// removed; the caller names it ([`temp_in_the_way`]).
#[derive(Debug)]
pub struct TempInTheWay(pub std::ffi::OsString);

impl std::fmt::Display for TempInTheWay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is in the way of the temporary file",
            self.0.to_string_lossy()
        )
    }
}

impl std::error::Error for TempInTheWay {}

/// The name in the way, when `error` is a [`TempInTheWay`].
#[must_use]
pub fn temp_in_the_way(error: &io::Error) -> Option<&OsStr> {
    error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<TempInTheWay>())
        .map(|found| found.0.as_os_str())
}

/// The plain names of `rel` (no root, no `..`, no `.`), or an error.
fn names(rel: &Path) -> io::Result<Vec<&OsStr>> {
    let mut out = Vec::new();
    for part in rel.components() {
        match part {
            Component::Normal(name) => out.push(name),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a path that leaves its folder",
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(unix)]
mod imp {
    use std::ffi::OsStr;
    use std::io::{self, Read, Write};
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::{Path, PathBuf};

    use nix::errno::Errno;
    use nix::fcntl::{AtFlags, OFlag, openat, renameat};
    use nix::sys::stat::{FileStat, Mode, SFlag, fstat, fstatat, mkdirat};
    use nix::unistd::{UnlinkatFlags, unlinkat};

    use super::Expect;

    fn io_of(errno: Errno) -> io::Error {
        io::Error::from_raw_os_error(errno as i32)
    }

    fn kind(stat: &FileStat) -> SFlag {
        SFlag::from_bits_truncate(stat.st_mode & SFlag::S_IFMT.bits())
    }

    /// A folder opened for walking.
    #[derive(Debug)]
    pub struct Dir {
        fd: OwnedFd,
        owner: u32,
    }

    const DIR_FLAGS: OFlag = OFlag::O_RDONLY
        .union(OFlag::O_DIRECTORY)
        .union(OFlag::O_NOFOLLOW)
        .union(OFlag::O_CLOEXEC);

    impl Dir {
        pub fn root(path: &Path) -> io::Result<Self> {
            let fd = nix::fcntl::open(path, DIR_FLAGS, Mode::empty()).map_err(io_of)?;
            let owner = fstat(&fd).map_err(io_of)?.st_uid;
            Ok(Self { fd, owner })
        }

        pub fn walk(
            &self,
            names: &[&OsStr],
            create_missing: bool,
            made: &mut Vec<PathBuf>,
        ) -> io::Result<Self> {
            let mut at: Option<OwnedFd> = None;
            let mut rel = PathBuf::new();
            for name in names {
                rel.push(name);
                let parent = at.as_ref().map_or_else(|| self.fd.as_fd(), AsFd::as_fd);
                let opened = match openat(parent, *name, DIR_FLAGS, Mode::empty()) {
                    Err(Errno::ENOENT) if create_missing => {
                        mkdirat(parent, *name, Mode::from_bits_truncate(0o755)).map_err(io_of)?;
                        made.push(rel.clone());
                        openat(parent, *name, DIR_FLAGS, Mode::empty())
                    }
                    other => other,
                }
                .map_err(io_of)?;
                at = Some(opened);
            }
            let fd = match at {
                Some(fd) => fd,
                None => self.fd.try_clone()?,
            };
            Ok(Self {
                fd,
                owner: self.owner,
            })
        }

        /// The entry `name` here, as it is (`lstat`): `None` when absent.
        fn entry(&self, name: &OsStr) -> io::Result<Option<FileStat>> {
            match fstatat(&self.fd, name, AtFlags::AT_SYMLINK_NOFOLLOW) {
                Ok(stat) => Ok(Some(stat)),
                Err(Errno::ENOENT) => Ok(None),
                Err(errno) => Err(io_of(errno)),
            }
        }

        pub fn read(&self, name: &OsStr, expect: Expect) -> io::Result<Option<Vec<u8>>> {
            let flags = OFlag::O_RDONLY
                .union(OFlag::O_NOFOLLOW)
                .union(OFlag::O_NONBLOCK)
                .union(OFlag::O_CLOEXEC);
            let fd = match openat(&self.fd, name, flags, Mode::empty()) {
                Ok(fd) => fd,
                Err(Errno::ENOENT) => return Ok(None),
                Err(errno) => return Err(io_of(errno)),
            };
            let stat = fstat(&fd).map_err(io_of)?;
            if kind(&stat) != SFlag::S_IFREG {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not a regular file",
                ));
            }
            let size = u64::try_from(stat.st_size).unwrap_or(u64::MAX);
            if size > expect.max {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "too big"));
            }
            if expect.owned && (stat.st_uid != self.owner || stat.st_mode & 0o022 != 0) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "not the folder owner's alone",
                ));
            }
            let mut bytes = Vec::new();
            std::fs::File::from(fd)
                .take(expect.max.saturating_add(1))
                .read_to_end(&mut bytes)?;
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > expect.max {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "too big"));
            }
            Ok(Some(bytes))
        }

        fn create(&self, name: &OsStr, bytes: &[u8], mode: nix::libc::mode_t) -> io::Result<()> {
            let flags = OFlag::O_WRONLY
                .union(OFlag::O_CREAT)
                .union(OFlag::O_EXCL)
                .union(OFlag::O_NOFOLLOW)
                .union(OFlag::O_CLOEXEC);
            let fd =
                openat(&self.fd, name, flags, Mode::from_bits_truncate(mode)).map_err(io_of)?;
            let mut file = std::fs::File::from(fd);
            let wrote = file.write_all(bytes).and_then(|()| file.sync_all());
            if wrote.is_err() {
                // L6: a half-written new file never stays.
                let _ = unlinkat(&self.fd, name, UnlinkatFlags::NoRemoveDir);
            }
            wrote
        }

        pub fn create_new(&self, name: &OsStr, bytes: &[u8]) -> io::Result<()> {
            self.create(name, bytes, 0o644)
        }

        pub fn replace(&self, name: &OsStr, bytes: &[u8]) -> io::Result<()> {
            let now = self
                .entry(name)?
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "nothing to replace"))?;
            if kind(&now) != SFlag::S_IFREG {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not a regular file",
                ));
            }
            let temp = super::temp_name(name);
            // L6 (round 2): whatever is at the temporary name is stale. A
            // plain file of the folder owner's is a write of ours that
            // stopped half-way (the name's `.ai-juggler-new` is our marker,
            // and only this writer makes one): taken away. Anything else
            // (a link, a folder, another's file) is never removed: the
            // replacement is refused, naming it.
            if let Some(left) = self.entry(&temp)? {
                if kind(&left) != SFlag::S_IFREG || left.st_uid != self.owner {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        super::TempInTheWay(temp),
                    ));
                }
                unlinkat(&self.fd, temp.as_os_str(), UnlinkatFlags::NoRemoveDir).map_err(io_of)?;
            }
            let mode = now.st_mode & 0o777;
            if let Err(error) = self.create(&temp, bytes, mode) {
                let _ = unlinkat(&self.fd, temp.as_os_str(), UnlinkatFlags::NoRemoveDir);
                return Err(error);
            }
            renameat(&self.fd, temp.as_os_str(), &self.fd, name).map_err(|errno| {
                let _ = unlinkat(&self.fd, temp.as_os_str(), UnlinkatFlags::NoRemoveDir);
                io_of(errno)
            })
        }

        pub fn remove_file(&self, name: &OsStr) -> io::Result<bool> {
            match self.entry(name)? {
                None => Ok(false),
                Some(stat) if kind(&stat) == SFlag::S_IFDIR => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a folder, not a file",
                )),
                Some(_) => unlinkat(&self.fd, name, UnlinkatFlags::NoRemoveDir)
                    .map(|()| true)
                    .map_err(io_of),
            }
        }

        pub fn remove_dir(&self, name: &OsStr) -> io::Result<bool> {
            match unlinkat(&self.fd, name, UnlinkatFlags::RemoveDir) {
                Ok(()) => Ok(true),
                Err(Errno::ENOENT | Errno::ENOTEMPTY | Errno::EEXIST) => Ok(false),
                Err(errno) => Err(io_of(errno)),
            }
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::Expect;

    fn unsupported() -> io::Error {
        io::Error::new(io::ErrorKind::Unsupported, "not on this system yet")
    }

    #[derive(Debug)]
    pub struct Dir;

    impl Dir {
        pub fn root(_path: &Path) -> io::Result<Self> {
            Err(unsupported())
        }
        pub fn walk(
            &self,
            _names: &[&OsStr],
            _create_missing: bool,
            _made: &mut Vec<PathBuf>,
        ) -> io::Result<Self> {
            Err(unsupported())
        }
        pub fn read(&self, _name: &OsStr, _expect: Expect) -> io::Result<Option<Vec<u8>>> {
            Err(unsupported())
        }
        pub fn create_new(&self, _name: &OsStr, _bytes: &[u8]) -> io::Result<()> {
            Err(unsupported())
        }
        pub fn replace(&self, _name: &OsStr, _bytes: &[u8]) -> io::Result<()> {
            Err(unsupported())
        }
        pub fn remove_file(&self, _name: &OsStr) -> io::Result<bool> {
            Err(unsupported())
        }
        pub fn remove_dir(&self, _name: &OsStr) -> io::Result<bool> {
            Err(unsupported())
        }
    }
}

/// A folder opened for walking: every file under it is reached through it.
#[derive(Debug)]
pub struct Dir(imp::Dir);

impl Dir {
    /// Opens `path` as a walk's root: a real folder, never itself a link.
    ///
    /// # Errors
    ///
    /// When it isn't there, is a link or a file, or can't be opened.
    pub fn root(path: &Path) -> io::Result<Self> {
        imp::Dir::root(path).map(Self)
    }

    /// The folder `rel` under this one, never through a link; each missing
    /// one made when `create_missing`, and recorded in `made` (relative to this one).
    ///
    /// # Errors
    ///
    /// A path that leaves the folder, a link or a file on the way, or the
    /// system's error.
    pub fn walk(
        &self,
        rel: &Path,
        create_missing: bool,
        made: &mut Vec<PathBuf>,
    ) -> io::Result<Self> {
        self.0.walk(&names(rel)?, create_missing, made).map(Self)
    }

    /// The file `name` here, checked as `expect` on the opened file;
    /// `None` when it isn't there.
    ///
    /// # Errors
    ///
    /// A link, not a regular file, too big, not the owner's alone (when
    /// asked), or the system's error.
    pub fn read(&self, name: &OsStr, expect: Expect) -> io::Result<Option<Vec<u8>>> {
        self.0.read(name, expect)
    }

    /// A new file `name` here (never over anything, never through a link).
    ///
    /// # Errors
    ///
    /// When something is already there, or the write fails.
    pub fn create_new(&self, name: &OsStr, bytes: &[u8]) -> io::Result<()> {
        self.0.create_new(name, bytes)
    }

    /// Replaces the regular file `name` here whole: a temporary file in the
    /// same folder (a leftover one of its name taken away first), renamed
    /// over it, with its permissions.
    ///
    /// # Errors
    ///
    /// When it isn't a regular file, or the write or the rename fails (the
    /// old file is unchanged then).
    pub fn replace(&self, name: &OsStr, bytes: &[u8]) -> io::Result<()> {
        self.0.replace(name, bytes)
    }

    /// Removes the file (or link) `name` here, never a folder; `false`
    /// when nothing was there.
    ///
    /// # Errors
    ///
    /// A folder, or the system's error.
    pub fn remove_file(&self, name: &OsStr) -> io::Result<bool> {
        self.0.remove_file(name)
    }

    /// Removes the folder `name` here when it's empty; `false` when it
    /// isn't there or isn't empty.
    ///
    /// # Errors
    ///
    /// The system's error.
    pub fn remove_dir(&self, name: &OsStr) -> io::Result<bool> {
        self.0.remove_dir(name)
    }
}

/// `rel` under `root`, the whole way without a link, checked as `expect`
/// on the opened file; `None` when it isn't there.
///
/// # Errors
///
/// As [`Dir::root`], [`Dir::walk`] and [`Dir::read`].
pub fn read_in(root: &Path, rel: &Path, expect: Expect) -> io::Result<Option<Vec<u8>>> {
    let parts = names(rel)?;
    let Some((name, folders)) = parts.split_last() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file named"));
    };
    let dir = Dir::root(root)?;
    let folder = dir.0.walk(folders, false, &mut Vec::new())?;
    folder.read(name, expect)
}

/// The file at `path` (its folder reached as the system resolves it, the
/// file itself never through a link), a regular file of at most `max`
/// bytes, checked on the opened file; `None` when it isn't there.
///
/// # Errors
///
/// As [`Dir::read`].
pub fn read_file(path: &Path, max: u64) -> io::Result<Option<Vec<u8>>> {
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file named"));
    };
    let folder = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    let resolved = std::fs::canonicalize(folder)?;
    Dir::root(&resolved)?.read(name, Expect::small(max))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_file_is_read_through_its_folder_never_through_a_link() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("a/b")).unwrap();
        std::fs::write(root.path().join("a/b/f.json"), "{}").unwrap();
        assert_eq!(
            read_in(root.path(), Path::new("a/b/f.json"), Expect::small(10)).unwrap(),
            Some(b"{}".to_vec())
        );
        assert_eq!(
            read_in(root.path(), Path::new("a/b/none"), Expect::small(10)).unwrap(),
            None
        );
        // A link at the file, on the way, or as the root: refused.
        std::os::unix::fs::symlink(root.path().join("a/b/f.json"), root.path().join("a/l.json"))
            .unwrap();
        assert!(read_in(root.path(), Path::new("a/l.json"), Expect::small(10)).is_err());
        std::os::unix::fs::symlink(root.path().join("a/b"), root.path().join("a/lb")).unwrap();
        assert!(read_in(root.path(), Path::new("a/lb/f.json"), Expect::small(10)).is_err());
        let linked_root = root.path().join("root-link");
        std::os::unix::fs::symlink(root.path().join("a"), &linked_root).unwrap();
        assert!(read_in(&linked_root, Path::new("b/f.json"), Expect::small(10)).is_err());
        // Leaving the folder, too big, or writable by others (when owned).
        assert!(read_in(root.path(), Path::new("../x"), Expect::small(10)).is_err());
        assert!(read_in(root.path(), Path::new("a/b/f.json"), Expect::small(1)).is_err());
        std::fs::set_permissions(
            root.path().join("a/b/f.json"),
            std::fs::Permissions::from_mode(0o666),
        )
        .unwrap();
        assert!(read_in(root.path(), Path::new("a/b/f.json"), Expect::owned(10)).is_err());
        assert!(read_in(root.path(), Path::new("a/b/f.json"), Expect::small(10)).is_ok());
        assert!(
            read_in(root.path(), Path::new("a/b"), Expect::small(10)).is_err(),
            "a folder"
        );
        assert_eq!(
            read_file(&root.path().join("a/b/f.json"), 10).unwrap(),
            Some(b"{}".to_vec())
        );
        assert!(read_file(&root.path().join("a/l.json"), 10).is_err());
    }

    #[test]
    fn writes_create_new_replace_whole_and_remove_in_place() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::root(root.path()).unwrap();
        let mut made = Vec::new();
        let deep = dir.walk(Path::new("x/.y"), true, &mut made).unwrap();
        assert_eq!(made, [PathBuf::from("x"), PathBuf::from("x/.y")]);
        deep.create_new(OsStr::new("f"), b"one").unwrap();
        assert!(
            deep.create_new(OsStr::new("f"), b"two").is_err(),
            "never over a file"
        );
        std::fs::set_permissions(
            root.path().join("x/.y/f"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        // L6: something at the temporary name that isn't our leftover (a
        // link, a folder) is never removed: the replacement is refused,
        // naming it, and the file is as it was.
        let temp = root.path().join("x/.y/.f.ai-juggler-new");
        for in_the_way in ["link", "folder"] {
            if in_the_way == "link" {
                std::os::unix::fs::symlink(root.path().join("x"), &temp).unwrap();
            } else {
                std::fs::create_dir(&temp).unwrap();
            }
            let refused = deep.replace(OsStr::new("f"), b"two").unwrap_err();
            assert_eq!(
                temp_in_the_way(&refused),
                Some(OsStr::new(".f.ai-juggler-new")),
                "{in_the_way}"
            );
            assert!(
                std::fs::symlink_metadata(&temp).is_ok(),
                "{in_the_way} kept"
            );
            assert_eq!(std::fs::read(root.path().join("x/.y/f")).unwrap(), b"one");
            std::fs::remove_dir(&temp)
                .or_else(|_| std::fs::remove_file(&temp))
                .unwrap();
        }
        // A plain file there is a write of ours that stopped half-way:
        // it doesn't block a replacement.
        std::fs::write(&temp, "left").unwrap();
        deep.replace(OsStr::new("f"), b"two").unwrap();
        assert_eq!(std::fs::read(root.path().join("x/.y/f")).unwrap(), b"two");
        assert!(!root.path().join("x/.y/.f.ai-juggler-new").exists());
        let mode = std::fs::metadata(root.path().join("x/.y/f"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "its permissions kept");
        // A link where the file is: never replaced through.
        let elsewhere = root.path().join("elsewhere");
        std::fs::write(&elsewhere, "keep").unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.path().join("x/.y/l")).unwrap();
        assert!(deep.replace(OsStr::new("l"), b"no").is_err());
        assert!(deep.create_new(OsStr::new("l"), b"no").is_err());
        assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "keep");
        // A link on the way: never walked through.
        std::os::unix::fs::symlink(root.path().join("x"), root.path().join("lx")).unwrap();
        assert!(dir.walk(Path::new("lx/.y"), true, &mut Vec::new()).is_err());
        assert!(deep.remove_file(OsStr::new("f")).unwrap());
        assert!(!deep.remove_file(OsStr::new("f")).unwrap());
        assert!(
            deep.remove_file(OsStr::new("l")).unwrap(),
            "the link itself, not its target"
        );
        assert_eq!(std::fs::read_to_string(&elsewhere).unwrap(), "keep");
        let x = dir.walk(Path::new("x"), false, &mut Vec::new()).unwrap();
        assert!(x.remove_dir(OsStr::new(".y")).unwrap());
    }
}
