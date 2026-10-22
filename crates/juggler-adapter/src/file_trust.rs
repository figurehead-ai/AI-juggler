//! Whose a file is, and whether it is safe to run or trust: the one place
//! these checks live (the handover socket's folder, the system tools).
//! Every check reads the entry itself (`lstat`), so a link is judged as a
//! link and never followed by accident.

use std::path::{Path, PathBuf};

/// This process's effective user id.
#[cfg(unix)]
#[must_use]
pub fn me() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// The owner of `path` (following links), when it can be read.
#[cfg(unix)]
#[must_use]
pub fn owner_of(path: &Path) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|meta| meta.uid())
}

/// The directory is a real directory (not a link), `me`'s, and `0700`:
/// closed to everyone else.
#[cfg(unix)]
#[must_use]
pub fn dir_is_ours(dir: &Path, me: u32) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    std::fs::symlink_metadata(dir).is_ok_and(|meta| {
        meta.file_type().is_dir() && meta.uid() == me && meta.permissions().mode() & 0o777 == 0o700
    })
}

/// The executable at `path` when it is safe to run: the path itself and
/// the file it resolves to, and every folder above either, belong to root
/// or `me`, and no folder or file among them can be written by a group or
/// other users (a link's own mode doesn't count). Returns the resolved
/// path, which is what should be run.
#[cfg(unix)]
#[must_use]
pub fn trusted_executable(path: &Path, me: u32) -> Option<PathBuf> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let entry_ok = |entry: &Path| {
        std::fs::symlink_metadata(entry).is_ok_and(|meta| {
            entry_trusted(
                meta.uid(),
                meta.permissions().mode(),
                meta.file_type().is_symlink(),
                me,
            )
        })
    };
    let resolved = std::fs::canonicalize(path).ok()?;
    let file = std::fs::symlink_metadata(&resolved).ok()?;
    let runnable = file.file_type().is_file() && file.permissions().mode() & 0o111 != 0;
    let all_ok = path
        .ancestors()
        .chain(resolved.ancestors())
        .filter(|entry| !entry.as_os_str().is_empty())
        .all(entry_ok);
    (runnable && all_ok).then_some(resolved)
}

/// One entry on the way: root's or `me`'s, and (unless it is a link, whose
/// own mode means nothing) not writable by a group or other users.
#[must_use]
pub const fn entry_trusted(owner: u32, mode: u32, is_link: bool, me: u32) -> bool {
    (owner == 0 || owner == me) && (is_link || mode & 0o022 == 0)
}

/// Windows: no owner checks behind this seam yet (Unix only).
#[cfg(not(unix))]
#[must_use]
pub fn trusted_executable(path: &Path, _me: u32) -> Option<PathBuf> {
    path.is_file().then(|| path.to_path_buf())
}

#[cfg(test)]
mod rule {
    use super::entry_trusted;

    /// M-a (#126 re-review): the owner counts, not just the mode bits.
    #[test]
    fn root_or_me_and_nobody_else_can_write() {
        let me = 501;
        assert!(entry_trusted(0, 0o755, false, me));
        assert!(entry_trusted(me, 0o755, false, me));
        assert!(entry_trusted(me, 0o700, false, me));
        assert!(!entry_trusted(502, 0o755, false, me), "another user's");
        assert!(!entry_trusted(0, 0o775, false, me), "group-writable");
        assert!(!entry_trusted(0, 0o757, false, me), "anyone can write");
        assert!(!entry_trusted(0, 0o1777, false, me), "a shared /tmp");
        assert!(
            entry_trusted(0, 0o777, true, me),
            "a link's own mode means nothing"
        );
        assert!(!entry_trusted(502, 0o777, true, me), "but its owner does");
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_system_tool_is_trusted_and_resolves() {
        assert_eq!(
            trusted_executable(Path::new("/bin/sh"), me()),
            std::fs::canonicalize("/bin/sh").ok()
        );
        assert!(
            trusted_executable(Path::new("/bin/sh"), me() + 1).is_some(),
            "root's is fine for anyone"
        );
    }

    #[test]
    fn what_cant_run_or_sits_under_a_shared_folder_is_refused() {
        // A temporary folder sits under a folder anyone can write (or
        // another's); either way, nothing in it is run.
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join("git");
        std::fs::write(&tool, "").unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&tool, &link).unwrap();
        if dir.path().ancestors().any(|up| {
            std::fs::symlink_metadata(up).is_ok_and(|meta| meta.permissions().mode() & 0o002 != 0)
        }) {
            assert_eq!(trusted_executable(&tool, me()), None);
            assert_eq!(trusted_executable(&link, me()), None);
        }
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o757)).unwrap();
        assert_eq!(trusted_executable(&tool, me()), None);
        assert_eq!(
            trusted_executable(Path::new("/etc/hosts"), me()),
            None,
            "not runnable"
        );
        assert_eq!(
            trusted_executable(Path::new("/bin"), me()),
            None,
            "not a file"
        );
    }
}
