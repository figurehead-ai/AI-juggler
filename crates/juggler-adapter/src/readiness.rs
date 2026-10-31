//! 0.3.2 — before a switch changes anything, can the target launch? The
//! maintainer's walk: a switch into another account ran half way, macOS
//! said "Data Access Blocked", and the new session opened on its tool's
//! first-run setup instead of the conversation, while AI Juggler said
//! "switched". Each tool says what its launch reads and whether it's set
//! up; the app checks it, and the files a switch reads, first.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Why a launch can't go ahead yet.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NotReady {
    /// The OS refused AI Juggler this file or folder (macOS keeps another
    /// app's data to itself until the user allows it).
    Blocked(PathBuf),
    /// The tool there isn't set up yet: what to do, in plain words.
    NotSetUp(String),
}

/// Opens `path` to read it, as the switch will: `Blocked` when the OS
/// refuses it. Missing is fine here (the caller decides what missing
/// means). Nothing is written.
///
/// # Errors
///
/// [`NotReady::Blocked`] when the OS refuses access.
pub fn probe_read(path: &Path) -> Result<(), NotReady> {
    let refused = |error: &std::io::Error| error.kind() == ErrorKind::PermissionDenied;
    let result = if path.is_dir() {
        std::fs::read_dir(path).map(|_| ())
    } else {
        std::fs::File::open(path).map(|_| ())
    };
    match result {
        Err(error) if refused(&error) => Err(NotReady::Blocked(path.to_path_buf())),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_readable_or_missing_path_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(probe_read(&file), Ok(()));
        assert_eq!(probe_read(dir.path()), Ok(()));
        assert_eq!(probe_read(&dir.path().join("missing")), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn a_refused_path_is_blocked() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        std::fs::write(&file, "x").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
        // root reads anything: the refusal can't be staged there.
        if std::fs::File::open(&file).is_err() {
            assert_eq!(probe_read(&file), Err(NotReady::Blocked(file.clone())));
        }
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}
