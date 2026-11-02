//! Starting an app the user has installed (0.3.3): by its bundle, with
//! only the arguments that pick one of its instances (a profile's data
//! folder) and, for a profile paired with a tool's folder, the one
//! variable that names it (0.3.5). Never a shell.
//!
//! - [`send_link`] (0.3.5): an "open this link" event to one running
//!   process, so an app that runs one instance per profile gets it in the
//!   profile meant, never the instance macOS would pick for the app.
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
pub fn open_command(
    bundle: &Path,
    start: Start,
    env: &[(String, String)],
    args: &[String],
) -> (String, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    if start.new_instance {
        out.push("-n".to_owned());
    }
    if start.background {
        out.push("-g".to_owned());
    }
    for (name, value) in env {
        out.push("--env".to_owned());
        out.push(format!("{name}={value}"));
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
pub fn open_app(
    bundle: &Path,
    start: Start,
    env: &[(String, String)],
    args: &[String],
) -> std::io::Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(crate::unsupported("starting an app"));
    }
    let (program, words) = open_command(bundle, start, env, args);
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

/// Sends `link` to the running process `pid` as the system's "open this
/// link" event (`GURL`), addressed to that process alone, and returns once
/// the system delivered it (no reply is awaited). Under the hardened
/// runtime this needs the Apple Events entitlement, and macOS asks the
/// user once whether AI Juggler may send it to that app.
///
/// # Errors
///
/// The process is gone, the user didn't allow it, or this isn't built on
/// this system yet.
pub fn send_link(pid: u32, link: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        mac::send_link(pid, link)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pid, link);
        Err(crate::unsupported("handing a link to an app"))
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventSendOptions, NSString};

    /// `kInternetEventClass` / `kAEGetURL`: "open this link".
    const GURL: u32 = u32::from_be_bytes(*b"GURL");
    /// `keyDirectObject`: the link itself.
    const DIRECT: u32 = u32::from_be_bytes(*b"----");
    /// `kAutoGenerateReturnID` and `kAnyTransactionID`.
    const RETURN_ID: i16 = -1;
    const TRANSACTION: i32 = 0;
    /// How long the system may take to deliver it, in seconds.
    const DELIVER: f64 = 10.0;

    pub(super) fn send_link(pid: u32, link: &str) -> std::io::Result<()> {
        let pid = i32::try_from(pid).map_err(|_| std::io::Error::other("that isn't a process"))?;
        let target = NSAppleEventDescriptor::descriptorWithProcessIdentifier(pid);
        let event = NSAppleEventDescriptor::appleEventWithEventClass_eventID_targetDescriptor_returnID_transactionID(
            GURL,
            GURL,
            Some(&target),
            RETURN_ID,
            TRANSACTION,
        );
        let words = NSAppleEventDescriptor::descriptorWithString(&NSString::from_str(link));
        event.setParamDescriptor_forKeyword(&words, DIRECT);
        event
            .sendEventWithOptions_timeout_error(NSAppleEventSendOptions::NoReply, DELIVER)
            .map(|_| ())
            .map_err(|error| std::io::Error::other(error.localizedDescription().to_string()))
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
            &[],
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
            &[],
        );
        assert_eq!(args, ["-g", "-a", "/Applications/Codex.app"]);
    }

    /// 0.3.5: a profile paired with a tool's folder starts with it, as
    /// `open --env` passes it to the app (never through a shell).
    #[test]
    fn a_paired_profile_starts_with_its_folder_variable() {
        let (_, args) = open_command(
            &PathBuf::from("/Applications/Claude.app"),
            Start {
                new_instance: true,
                background: false,
            },
            &[(
                "CLAUDE_CONFIG_DIR".to_owned(),
                "/Users/you/Code Profiles/work".to_owned(),
            )],
            &["--user-data-dir=/Users/you/Profiles/work".to_owned()],
        );
        assert_eq!(
            args,
            [
                "-n",
                "--env",
                "CLAUDE_CONFIG_DIR=/Users/you/Code Profiles/work",
                "-a",
                "/Applications/Claude.app",
                "--args",
                "--user-data-dir=/Users/you/Profiles/work"
            ]
        );
    }
}
