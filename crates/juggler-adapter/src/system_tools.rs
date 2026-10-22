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
/// `MacPorts`', then the system's (Apple's command-line tools shim on macOS,
/// the distribution's on Linux). Fixed places only, never a `PATH` search:
/// a git from nix, asdf or similar isn't used, the system's is.
pub const GIT_PLACES: &[&str] = &[
    "/opt/homebrew/bin/git",
    "/usr/local/bin/git",
    "/opt/local/bin/git",
    "/usr/bin/git",
];

/// git, by its resolved path: the first of [`GIT_PLACES`] that
/// [`crate::file_trust::trusted_executable`] accepts (it and every folder
/// above it root's or ours, none writable by others), found once and kept.
/// `None` when there is none: on macOS `/usr/bin/git` counts only when the
/// developer tools are installed, since without them it opens Apple's
/// "install the command line developer tools" dialog on every run.
#[must_use]
pub fn git() -> Option<&'static Path> {
    static GIT: OnceLock<Option<PathBuf>> = OnceLock::new();
    GIT.get_or_init(|| {
        #[cfg(unix)]
        let me = crate::file_trust::me();
        #[cfg(not(unix))]
        let me = 0;
        GIT_PLACES
            .iter()
            .map(Path::new)
            .filter(|place| *place != Path::new("/usr/bin/git") || developer_tools_installed())
            .find_map(|place| crate::file_trust::trusted_executable(place, me))
    })
    .as_deref()
}

/// What to tell someone whose Mac has no git AI Juggler can use.
pub const GIT_MISSING: &str = "git isn't installed. Install Apple's command line tools (run `xcode-select --install` in Terminal) or git from Homebrew, then try again.";

/// Whether `/usr/bin/git` has a git behind it: Apple's command line tools,
/// or the developer folder `xcode-select` chose. Read from the disk; never
/// by running the shim.
fn developer_tools_installed() -> bool {
    if !cfg!(target_os = "macos") {
        return true;
    }
    Path::new("/Library/Developer/CommandLineTools/usr/bin/git").is_file()
        || std::fs::read_link("/var/db/xcode_select_link")
            .is_ok_and(|developer| developer.join("usr/bin/git").is_file())
}

/// Windows' file browser, in the system folder (`%SystemRoot%`, normally
/// `C:\Windows`).
#[must_use]
pub fn explorer() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
        .join("explorer.exe")
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_tool_is_named_by_an_absolute_path() {
        for tool in [super::SECURITY, super::OPEN, super::XDG_OPEN]
            .iter()
            .chain(super::GIT_PLACES)
        {
            assert!(std::path::Path::new(tool).is_absolute(), "{tool}");
        }
        if let Some(git) = super::git() {
            assert!(git.is_absolute());
        }
    }
}
