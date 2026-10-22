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
/// Where git is when nothing else is found (Apple's command-line tools
/// shim on macOS, the distribution's on Linux).
pub const GIT_FALLBACK: &str = "/usr/bin/git";

/// git, by absolute path: the one the user's own terminal would run
/// (their resolved `PATH`, e.g. Homebrew's), found once and kept; else
/// [`GIT_FALLBACK`].
#[must_use]
pub fn git() -> &'static Path {
    static GIT: OnceLock<PathBuf> = OnceLock::new();
    GIT.get_or_init(|| crate::user_path::find("git").unwrap_or_else(|| PathBuf::from(GIT_FALLBACK)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_tool_is_named_by_an_absolute_path() {
        for tool in [
            super::SECURITY,
            super::OPEN,
            super::XDG_OPEN,
            super::GIT_FALLBACK,
        ] {
            assert!(std::path::Path::new(tool).is_absolute(), "{tool}");
        }
        assert!(super::git().is_absolute());
    }
}
