//! The system tools AI Juggler runs, by absolute path (SECURITY-GUIDELINES
//! C2, AJ-E2010). A tool run by name goes through `PATH`, and since the
//! app runs with the user's `PATH` in front (the 0.0.11 fix), a stand-in
//! earlier on it would run instead. Every system tool is named here, once;
//! `tests/no_bare_tools.rs` in juggler-app fails on a bare name anywhere
//! else.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// macOS's keychain tool (existence probes only; never `-w`).
pub const SECURITY: &str = "/usr/bin/security";
/// macOS's "open with the default app".
pub const OPEN: &str = "/usr/bin/open";
/// The freedesktop "open with the default app".
pub const XDG_OPEN: &str = "/usr/bin/xdg-open";
/// Where git may be, in order: Homebrew's (Apple silicon, then Intel),
/// then the system's (Apple's command-line tools shim on macOS, the
/// distribution's on Linux). Fixed places only, never a `PATH` search.
pub const GIT_PLACES: &[&str] = &[
    "/opt/homebrew/bin/git",
    "/usr/local/bin/git",
    "/usr/bin/git",
];

/// git, by absolute path: the first of [`GIT_PLACES`] that is a file no
/// one but its owner can write, found once and kept; `/usr/bin/git` when
/// none is.
#[must_use]
pub fn git() -> &'static Path {
    static GIT: OnceLock<PathBuf> = OnceLock::new();
    GIT.get_or_init(|| {
        GIT_PLACES
            .iter()
            .map(PathBuf::from)
            .find(|place| owner_only_file(place))
            .unwrap_or_else(|| PathBuf::from("/usr/bin/git"))
    })
}

/// Windows' file browser, in the system folder (`%SystemRoot%`, normally
/// `C:\Windows`).
#[must_use]
pub fn explorer() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
        .join("explorer.exe")
}

/// A file that no group or other user can write (followed through links).
fn owner_only_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o022 == 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

#[cfg(test)]
mod tests {
    /// M5 (#126 review): a git that others can write is passed over.
    #[cfg(unix)]
    #[test]
    fn a_file_others_can_write_is_not_run() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join("git");
        std::fs::write(&tool, "").unwrap();
        for (mode, ok) in [(0o755, true), (0o775, false), (0o757, false)] {
            std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(super::owner_only_file(&tool), ok, "{mode:o}");
        }
        assert!(!super::owner_only_file(dir.path()), "a folder isn't a tool");
    }

    #[test]
    fn every_tool_is_named_by_an_absolute_path() {
        for tool in [super::SECURITY, super::OPEN, super::XDG_OPEN]
            .iter()
            .chain(super::GIT_PLACES)
        {
            assert!(std::path::Path::new(tool).is_absolute(), "{tool}");
        }
        assert!(super::git().is_absolute());
        assert!(super::GIT_PLACES.contains(&super::git().to_str().unwrap()));
    }
}
