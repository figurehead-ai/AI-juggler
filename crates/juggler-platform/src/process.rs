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

/// Whether process `pid` is there (it may be another user's): a signal-0
/// probe on Unix. `false` for pid 0 and anything a pid can't be. Other
/// systems can't tell yet and answer `false`.
#[must_use]
pub fn alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::errno::Errno;
        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        let Some(pid) = i32::try_from(pid).ok().filter(|pid| *pid > 0) else {
            return false;
        };
        match kill(Pid::from_raw(pid), None) {
            Ok(()) | Err(Errno::EPERM) => true,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Asks a child AI Juggler started to end (SIGTERM on Unix, so it can let
/// go of what it holds); the caller waits, then kills it if it hasn't.
///
/// # Errors
///
/// It is gone already, or this system has no such request (the caller
/// kills it instead).
pub fn terminate(child: &std::process::Child) -> std::io::Result<()> {
    ask_to_end(
        child.id(),
        "not a child's process",
        "asking a process to end",
    )
}

/// SIGTERM to `pid` on Unix, never to 0 or 1 (the caller's group or
/// init): `not_one` refuses those, `unsupported` names the request where
/// the system has none.
pub(crate) fn ask_to_end(pid: u32, not_one: &str, unsupported: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;
        let _ = unsupported;
        let pid = i32::try_from(pid)
            .ok()
            .filter(|pid| *pid > 1)
            .ok_or_else(|| std::io::Error::other(not_one.to_owned()))?;
        kill(Pid::from_raw(pid), Signal::SIGTERM).map_err(std::io::Error::from)
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, not_one);
        Err(crate::unsupported(unsupported))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_asks_the_group_or_init_to_end() {
        for pid in [0, 1, u32::MAX] {
            let error = ask_to_end(pid, "not that one", "ending").expect_err("refused");
            if cfg!(unix) {
                assert_eq!(error.to_string(), "not that one");
            }
        }
    }

    #[test]
    fn this_process_is_alive_and_no_pid_isnt() {
        assert!(alive(std::process::id()));
        assert!(!alive(0));
        assert!(!alive(u32::MAX));
    }

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
