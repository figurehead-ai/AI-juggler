//! Who this process runs as: the identity every ownership and peer check
//! compares against.
//!
//! On Unix it is the effective user id. On Windows it would be the
//! process token's SID, which needs Win32 calls this workspace doesn't
//! make yet; there [`current`] says so, and [`UserId`] has no values, so
//! nothing that needs to know who we are can run as if it knew.

/// A user of this computer, as the system names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserId(Inner);

#[cfg(unix)]
type Inner = u32;
#[cfg(not(unix))]
type Inner = crate::Never;

#[cfg(unix)]
impl UserId {
    /// The user with this Unix user id.
    #[must_use]
    pub const fn from_uid(uid: u32) -> Self {
        Self(uid)
    }

    /// Their Unix user id.
    #[must_use]
    pub const fn uid(self) -> u32 {
        self.0
    }
}

#[cfg(not(unix))]
impl UserId {
    /// There are no users to name on this system yet, so code holding
    /// one can't run.
    pub(crate) fn unreachable(self) -> ! {
        match self.0 {}
    }
}

/// The user this process runs as.
///
/// # Errors
///
/// `Unsupported` where this system's user identity isn't read yet
/// (Windows); every check that needs it then refuses.
#[cfg(unix)]
pub fn current() -> std::io::Result<UserId> {
    Ok(UserId(nix::unistd::geteuid().as_raw()))
}

/// The user this process runs as.
///
/// # Errors
///
/// `Unsupported` where this system's user identity isn't read yet
/// (Windows); every check that needs it then refuses.
#[cfg(not(unix))]
pub fn current() -> std::io::Result<UserId> {
    Err(crate::unsupported(
        "telling which user this program runs as",
    ))
}

/// The home folder every part of the app resolves `~` with: `$HOME`
/// when it is set (on Windows `%USERPROFILE%`), else the user record's
/// ([`record_home`]). The one way to it (a test checks no other code
/// asks): a lab or replica run on `HOME=<lab>` sees the lab, never this
/// Mac's real folders. An app started from Finder or at login gets `HOME`
/// from launchd, set to the user record's home, so there it is the same.
#[must_use]
pub fn home() -> Option<std::path::PathBuf> {
    home_from(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        record_home,
    )
}

/// [`home`] for these values (pure, testable): an empty or relative one
/// counts as unset.
#[must_use]
pub fn home_from(
    home: Option<std::ffi::OsString>,
    profile: Option<std::ffi::OsString>,
    record: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    let usable = |value: Option<std::ffi::OsString>| {
        value
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    usable(home).or_else(|| usable(profile)).or_else(record)
}

/// Whether this run's home ([`home`]) is another folder than the user
/// record's: a lab or a replica (`HOME=<lab>`). Its folder then, else
/// `None` (the user's own home, or one that can't be told).
#[must_use]
pub fn other_home() -> Option<std::path::PathBuf> {
    other_home_of(home().as_deref(), record_home().as_deref())
}

/// [`other_home`] for these folders (pure, testable): compared as the
/// file system resolves them (`/tmp` is `/private/tmp`).
#[must_use]
pub fn other_home_of(
    home: Option<&std::path::Path>,
    record: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match (home, record) {
        (Some(home), Some(record)) if canonical(home) != canonical(record) => {
            Some(home.to_path_buf())
        }
        _ => None,
    }
}

/// This user's home folder as the system's user record keeps it — not
/// `$HOME`, which a test lab points elsewhere. `None` where it can't be
/// read (or on Windows, not read yet).
#[cfg(unix)]
#[must_use]
pub fn record_home() -> Option<std::path::PathBuf> {
    nix::unistd::User::from_uid(nix::unistd::geteuid())
        .ok()
        .flatten()
        .map(|user| user.dir)
}

/// This user's home folder as the system's user record keeps it.
#[cfg(not(unix))]
#[must_use]
pub fn record_home() -> Option<std::path::PathBuf> {
    None
}

/// Whether a connection from `peer` may be answered: only when the system
/// named the peer, and it is `me`.
#[must_use]
pub fn peer_allowed(peer: Option<UserId>, me: UserId) -> bool {
    peer == Some(me)
}

#[cfg(all(test, unix))]
mod tests {
    use super::{UserId, current, peer_allowed};

    #[test]
    fn this_process_is_someone() {
        let me = current().unwrap();
        assert_eq!(UserId::from_uid(me.uid()), me);
    }

    #[test]
    fn only_this_user_is_allowed() {
        let me = UserId::from_uid(501);
        assert!(peer_allowed(Some(me), me));
        assert!(!peer_allowed(Some(UserId::from_uid(502)), me));
        assert!(
            !peer_allowed(None, me),
            "a peer the system can't name isn't answered"
        );
    }
}

#[cfg(test)]
mod home_tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn home_is_home_when_set_and_the_user_record_only_when_not() {
        let record = || Some(PathBuf::from("/Users/real"));
        let lab = Some(OsString::from("/tmp/aij-lab-x"));
        assert_eq!(
            home_from(lab, None, record),
            Some(PathBuf::from("/tmp/aij-lab-x"))
        );
        assert_eq!(home_from(None, None, record), record());
        // Empty or relative: unset.
        assert_eq!(home_from(Some(OsString::new()), None, record), record());
        assert_eq!(home_from(Some("lab".into()), None, record), record());
        assert_eq!(
            home_from(None, Some("/Users/win".into()), record),
            Some(PathBuf::from("/Users/win"))
        );
    }

    #[test]
    fn another_home_is_one_that_isnt_the_user_records() {
        let lab = tempfile::tempdir().unwrap();
        let record = tempfile::tempdir().unwrap();
        assert_eq!(
            other_home_of(Some(lab.path()), Some(record.path())),
            Some(lab.path().to_path_buf())
        );
        assert_eq!(
            other_home_of(Some(record.path()), Some(record.path())),
            None
        );
        // The same folder spelt through a link is the same home.
        let link = lab.path().join("me");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(record.path(), &link).unwrap();
            assert_eq!(other_home_of(Some(&link), Some(record.path())), None);
        }
        assert_eq!(other_home_of(None, Some(Path::new("/Users/real"))), None);
        assert_eq!(other_home_of(Some(Path::new("/Users/real")), None), None);
    }
}
