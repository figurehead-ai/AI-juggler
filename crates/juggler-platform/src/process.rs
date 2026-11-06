//! The processes AI Juggler starts for itself (AJ-E2010): its own helpers
//! — the keychain tool, `open`, `lsof`, `ioreg`, `footprint`, its own crash
//! watcher — get only what they need. Their environment is cleared, then
//! [`ENV_ALLOWLIST`] is kept and `PATH` is the system's folders; the
//! program is named by absolute path (`juggler_adapter::system_tools`).
//!
//! The user's own tools (an agent session, their editor, `gh`, a vendor
//! CLI) are not helpers: they run in the user's login environment, which
//! is theirs to set (`scripts/spawn_lint.py` keeps the two apart).

use std::ffi::OsStr;
use std::process::Command;

/// The variables a helper keeps from the app's own environment: who the
/// user is, their locale and their temporary folder.
pub const ENV_ALLOWLIST: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "__CF_USER_TEXT_ENCODING",
];

/// The system's own program folders, a helper's whole `PATH`.
pub const SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

/// Clears `command`'s environment, then keeps the [`ENV_ALLOWLIST`].
pub fn allowlisted_env(command: &mut Command) {
    command.env_clear();
    for key in ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

/// A command for one of AI Juggler's own helpers: `program` (an absolute
/// path) with the [`ENV_ALLOWLIST`] and the [`SYSTEM_PATH`].
#[must_use]
pub fn helper(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    allowlisted_env(&mut command);
    command.env("PATH", SYSTEM_PATH);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_sees_only_the_allowlist_and_the_system_path() {
        let command = helper("/usr/bin/env");
        let names: Vec<String> = command
            .get_envs()
            .filter(|(_, value)| value.is_some())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert!(
            names
                .iter()
                .all(|name| name == "PATH" || ENV_ALLOWLIST.contains(&name.as_str())),
            "{names:?}"
        );
        assert!(
            command
                .get_envs()
                .any(|(name, value)| name == "PATH" && value == Some(OsStr::new(SYSTEM_PATH)))
        );
        #[cfg(unix)]
        {
            let out = helper("/usr/bin/env").output().unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                !text.contains("CARGO"),
                "nothing of the parent's beyond the allowlist: {text}"
            );
        }
    }
}
