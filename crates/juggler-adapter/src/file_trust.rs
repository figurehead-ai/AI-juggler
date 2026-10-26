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

/// Why an executable isn't run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Untrusted {
    /// Nothing is there.
    Missing,
    /// It isn't a file that can run.
    NotRunnable,
    /// This entry on the way belongs to another user.
    Owner(PathBuf),
    /// Other users (or a group beyond the Mac's administrators) can change
    /// this entry on the way.
    Writable(PathBuf),
}

impl Untrusted {
    /// The reason, in plain words, for the user.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::Missing => "it isn't there".to_owned(),
            Self::NotRunnable => "it isn't a program that can run".to_owned(),
            Self::Owner(entry) => format!("{} belongs to another user", entry.display()),
            Self::Writable(entry) => format!("other users can change {}", entry.display()),
        }
    }
}

/// The executable at `path` when it is safe to run: the path itself and
/// the file it resolves to, and every folder above either, belong to root
/// or `me`, and no folder or file among them can be written by other users
/// (a link's own mode doesn't count; see [`entry_trusted`] for groups).
/// Returns the resolved path, which is what should be run.
#[cfg(unix)]
#[must_use]
pub fn trusted_executable(path: &Path, me: u32) -> Option<PathBuf> {
    executable_trust(path, me).ok()
}

/// [`trusted_executable`], with the reason when it isn't run: the first
/// entry on the way that fails, so the words can name it.
///
/// # Errors
///
/// Why it isn't run ([`Untrusted`]).
#[cfg(unix)]
pub fn executable_trust(path: &Path, me: u32) -> Result<PathBuf, Untrusted> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if std::fs::symlink_metadata(path).is_err() {
        return Err(Untrusted::Missing);
    }
    let resolved = std::fs::canonicalize(path).map_err(|_| Untrusted::Missing)?;
    let file = std::fs::symlink_metadata(&resolved).map_err(|_| Untrusted::Missing)?;
    if !file.file_type().is_file() || file.permissions().mode() & 0o111 == 0 {
        return Err(Untrusted::NotRunnable);
    }
    for entry in path
        .ancestors()
        .chain(resolved.ancestors())
        .filter(|entry| !entry.as_os_str().is_empty())
    {
        let meta = std::fs::symlink_metadata(entry).map_err(|_| Untrusted::Missing)?;
        if meta.uid() != 0 && meta.uid() != me {
            return Err(Untrusted::Owner(entry.to_path_buf()));
        }
        let is_link = meta.file_type().is_symlink();
        if !entry_trusted(meta.uid(), meta.gid(), meta.permissions().mode(), is_link, me) {
            return Err(Untrusted::Writable(entry.to_path_buf()));
        }
    }
    Ok(resolved)
}

/// The groups whose write access adds no one: their members can already
/// act as root. On a Mac that is `wheel` (0) and `admin` (80); `admin` is
/// the group Homebrew's standard layout is writable by (`/opt/homebrew/bin`
/// is `drwxrwxr-x`, the user's, group `admin`). Elsewhere only root's group.
#[cfg(target_os = "macos")]
pub const ADMIN_GROUPS: &[u32] = &[0, 80];
/// The groups whose write access adds no one (root's, off a Mac).
#[cfg(not(target_os = "macos"))]
pub const ADMIN_GROUPS: &[u32] = &[0];

/// One entry on the way: root's or `me`'s, never writable by other users,
/// and writable by its group only when that group is the Mac's
/// administrators ([`ADMIN_GROUPS`]). A link's own mode means nothing; its
/// owner does.
///
/// Why the administrators' group may write: whoever is in it can already
/// become root and change any root-owned program, so its write access lets
/// no new person plant a program. Homebrew installs that way by default,
/// and the user's own shell already runs everything in it. Any other group
/// (`staff`, which every user is in, or a shared group) could be someone
/// else, so a folder it can write is refused, as is one anyone can write.
#[must_use]
pub fn entry_trusted(owner: u32, group: u32, mode: u32, is_link: bool, me: u32) -> bool {
    let owner_ok = owner == 0 || owner == me;
    let others_ok = mode & 0o002 == 0;
    let group_ok = mode & 0o020 == 0 || ADMIN_GROUPS.contains(&group);
    owner_ok && (is_link || (others_ok && group_ok))
}

/// Windows: no owner checks behind this seam yet (Unix only).
#[cfg(not(unix))]
#[must_use]
pub fn trusted_executable(path: &Path, _me: u32) -> Option<PathBuf> {
    path.is_file().then(|| path.to_path_buf())
}

/// Windows: [`trusted_executable`] with its one reason (Unix only).
///
/// # Errors
///
/// [`Untrusted::Missing`] when there's no file.
#[cfg(not(unix))]
pub fn executable_trust(path: &Path, me: u32) -> Result<PathBuf, Untrusted> {
    trusted_executable(path, me).ok_or(Untrusted::Missing)
}

#[cfg(test)]
mod rule {
    use super::entry_trusted;

    const STAFF: u32 = 20;

    /// M-a (#126 re-review): the owner counts, not just the mode bits.
    #[test]
    fn root_or_me_and_nobody_else_can_write() {
        let me = 501;
        assert!(entry_trusted(0, 0, 0o755, false, me));
        assert!(entry_trusted(me, STAFF, 0o755, false, me));
        assert!(entry_trusted(me, STAFF, 0o700, false, me));
        assert!(!entry_trusted(502, STAFF, 0o755, false, me), "another user's");
        assert!(!entry_trusted(0, STAFF, 0o775, false, me), "staff can write");
        assert!(!entry_trusted(0, 0, 0o757, false, me), "anyone can write");
        assert!(!entry_trusted(0, 0, 0o1777, false, me), "a shared /tmp");
        assert!(
            entry_trusted(0, STAFF, 0o777, true, me),
            "a link's own mode means nothing"
        );
        assert!(!entry_trusted(502, 0, 0o777, true, me), "but its owner does");
    }

    /// Lab 2026-10-26 (tasks, bug 3): Homebrew's folders are the user's,
    /// `drwxrwxr-x`, group `admin`; administrators can already act as
    /// root, so that adds no one. Any other group, or anyone, still can't.
    #[cfg(target_os = "macos")]
    #[test]
    fn homebrews_admin_group_folders_are_trusted() {
        const ADMIN: u32 = 80;
        let me = 501;
        assert!(entry_trusted(me, ADMIN, 0o775, false, me), "/opt/homebrew/bin");
        assert!(entry_trusted(0, 0, 0o775, false, me), "wheel");
        assert!(!entry_trusted(me, STAFF, 0o775, false, me), "staff is everyone");
        assert!(!entry_trusted(me, 12, 0o775, false, me), "another group");
        assert!(!entry_trusted(me, ADMIN, 0o777, false, me), "anyone can write");
        assert!(!entry_trusted(502, ADMIN, 0o775, false, me), "another user's");
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
        assert!(
            matches!(executable_trust(&tool, me()), Err(Untrusted::Writable(_))),
            "the reason is who can change it"
        );
        assert_eq!(
            executable_trust(&dir.path().join("nothing-here"), me()),
            Err(Untrusted::Missing)
        );
        assert_eq!(
            executable_trust(Path::new("/etc/hosts"), me()),
            Err(Untrusted::NotRunnable),
            "not runnable"
        );
        assert_eq!(
            trusted_executable(Path::new("/bin"), me()),
            None,
            "not a file"
        );
    }
}
