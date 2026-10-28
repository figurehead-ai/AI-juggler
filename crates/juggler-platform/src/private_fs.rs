//! Folders and files only this user can open (SECURITY-GUIDELINES C7):
//! keys, tokens and the files a launch reads.
//!
//! On Unix a private folder is `0700` and a private file `0600`. An
//! existing folder must already be ours and closed: it is never
//! `chmod`ed into shape, since a folder someone else made could already
//! hold their files. On Windows the same promise needs an owner-only ACL,
//! which needs Win32 calls this workspace doesn't make yet, so there both
//! say "not available" rather than write a file others might read.

use std::path::Path;

use crate::user::UserId;

/// Makes `dir` (and any missing folder above it) closed to everyone but
/// `me`; an existing one must already be `me`'s and closed.
///
/// # Errors
///
/// When it can't be made, when it is someone else's or open to others,
/// or `Unsupported` where owner-only folders aren't available (Windows).
pub fn ensure_private_dir(dir: &Path, me: UserId) -> std::io::Result<()> {
    if make_closed_dir(dir, me)? {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "{} belongs to someone else or others can open it",
                dir.display()
            ),
        ))
    }
}

/// Makes `dir` closed if it's missing; whether it is now `me`'s and
/// closed (`false`: it was there already and isn't, and is left as it
/// was).
pub(crate) fn make_closed_dir(dir: &Path, me: UserId) -> std::io::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        match std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(dir)
        {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        Ok(crate::file_trust::dir_is_ours(dir, me))
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        me.unreachable()
    }
}

/// [`ensure_private_dir`] for the user this process runs as.
///
/// # Errors
///
/// As [`ensure_private_dir`], and `Unsupported` where the system can't say
/// who that is.
pub fn ensure_own_private_dir(dir: &Path) -> std::io::Result<()> {
    ensure_private_dir(dir, crate::user::current()?)
}

/// Writes `contents` to `path`, readable by this user only: written in
/// full beside it first, then moved into place, so a reader never sees
/// half a file.
///
/// # Errors
///
/// The write's error, or `Unsupported` where owner-only files aren't
/// available (Windows).
pub fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let partial = path.with_extension("partial");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&partial, path)
    }
    #[cfg(not(unix))]
    {
        let _ = (path, contents);
        Err(crate::unsupported("a file only you can read"))
    }
}

/// Opens `path` to add to its end, readable by this user only: a new
/// file is made so, and an existing one of ours open to others is closed
/// (a log AI Juggler keeps, never a file someone else made: that can't be
/// changed and stays as it was).
///
/// # Errors
///
/// The open's error, or `Unsupported` where owner-only files aren't
/// available (Windows).
pub fn append_private(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)?;
        if file.metadata()?.permissions().mode() & 0o077 != 0 {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(crate::unsupported("a file only you can read"))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{append_private, ensure_own_private_dir, ensure_private_dir, write_private};
    use crate::user::{UserId, current};
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    #[test]
    fn a_new_folder_is_closed_and_a_file_in_it_is_ours_alone() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("a").join("b");
        ensure_own_private_dir(&dir).unwrap();
        assert_eq!(mode(&dir), 0o700);
        ensure_own_private_dir(&dir).unwrap();
        let file = dir.join("key.json");
        write_private(&file, b"{}").unwrap();
        assert_eq!(mode(&file), 0o600);
        assert_eq!(std::fs::read(&file).unwrap(), b"{}");
        assert!(!dir.join("key.partial").exists(), "nothing left beside it");
    }

    #[test]
    fn an_appended_file_is_ours_alone_and_keeps_what_it_had() {
        use std::io::Write;
        let root = tempfile::tempdir().unwrap();
        let log = root.path().join("errors.log");
        writeln!(append_private(&log).unwrap(), "one").unwrap();
        assert_eq!(mode(&log), 0o600);
        // One of ours left open to others is closed, and nothing is lost.
        std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o644)).unwrap();
        writeln!(append_private(&log).unwrap(), "two").unwrap();
        assert_eq!(mode(&log), 0o600);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "one\ntwo\n");
    }

    #[test]
    fn an_open_or_someone_elses_folder_is_refused_and_left_as_it_was() {
        let root = tempfile::tempdir().unwrap();
        let open = root.path().join("open");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(ensure_own_private_dir(&open).is_err());
        assert_eq!(mode(&open), 0o755, "never chmod'ed into shape");
        let closed = root.path().join("closed");
        ensure_own_private_dir(&closed).unwrap();
        let stranger = UserId::from_uid(current().unwrap().uid() + 1);
        assert!(ensure_private_dir(&closed, stranger).is_err());
    }
}
