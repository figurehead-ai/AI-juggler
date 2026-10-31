//! Starting an app the user has installed (0.3.3): by its bundle, with
//! only the arguments that pick one of its instances (a profile's data
//! folder). Never an environment, never a shell.
//!
//! - [`open_app`]: macOS asks Launch Services (`/usr/bin/open -a`), the
//!   way the Dock and Finder start an app; `-n` starts a further instance
//!   when the app runs several (a Claude profile beside another), `-g`
//!   leaves it behind the app in front (a restart nobody clicked never
//!   takes the keyboard). Windows and Linux: not built yet, said plainly.

use std::path::Path;

/// How an app starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Start {
    /// A further instance, beside any already running (an app that runs
    /// one per profile).
    pub new_instance: bool,
    /// Behind the app in front, not brought forward.
    pub background: bool,
}

/// The command line that starts `bundle` (a `.app` folder) with `args`,
/// as [`open_app`] runs it: the program, then its arguments.
#[must_use]
pub fn open_command(bundle: &Path, start: Start, args: &[String]) -> (String, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    if start.new_instance {
        out.push("-n".to_owned());
    }
    if start.background {
        out.push("-g".to_owned());
    }
    out.push("-a".to_owned());
    out.push(bundle.to_string_lossy().into_owned());
    if !args.is_empty() {
        out.push("--args".to_owned());
        out.extend(args.iter().cloned());
    }
    ("/usr/bin/open".to_owned(), out)
}

/// Starts the app at `bundle` with `args`, and returns once the system
/// took the request (the app comes up on its own after that).
///
/// # Errors
///
/// The system refused (no such app, or it couldn't start), or starting
/// apps isn't built on this system yet.
pub fn open_app(bundle: &Path, start: Start, args: &[String]) -> std::io::Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(crate::unsupported("starting an app"));
    }
    let (program, words) = open_command(bundle, start, args);
    let out = std::process::Command::new(program)
        .args(words)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(
            String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_profile_starts_as_a_further_instance_with_only_its_folder() {
        let (program, args) = open_command(
            &PathBuf::from("/Applications/Claude.app"),
            Start {
                new_instance: true,
                background: false,
            },
            &["--user-data-dir=/Users/you/Profiles/work".to_owned()],
        );
        assert_eq!(program, "/usr/bin/open");
        assert_eq!(
            args,
            [
                "-n",
                "-a",
                "/Applications/Claude.app",
                "--args",
                "--user-data-dir=/Users/you/Profiles/work"
            ]
        );
    }

    #[test]
    fn a_single_instance_app_starts_plainly_and_a_restart_stays_behind() {
        let (_, args) = open_command(
            &PathBuf::from("/Applications/Codex.app"),
            Start {
                new_instance: false,
                background: true,
            },
            &[],
        );
        assert_eq!(args, ["-g", "-a", "/Applications/Codex.app"]);
    }
}
