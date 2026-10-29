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
