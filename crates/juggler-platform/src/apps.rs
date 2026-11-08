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
//!
//! - [`detached`] (0.3.8): an app process that outlived its display. When
//!   macOS restarts its window server (the watchdog ended `WindowServer`
//!   on 2026-11-04 and the login session started again), an app can keep
//!   running with no window and no place in the apps the system knows: it
//!   can't be brought forward or sent a link, and starting a second one on
//!   the same profile folder risks its data. [`quit`] asks it to end.

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
    let out = crate::process::helper(program)
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

/// Why a link sent with [`send_link_quietly`] didn't go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkRefused {
    /// macOS would have asked the user first whether AI Juggler may send
    /// events to that app, and was told not to ask: nothing was shown.
    NeedsConsent,
    /// The user said no (System Settings › Privacy & Security ›
    /// Automation).
    Denied,
    /// Anything else (the process is gone; not built on this system).
    Failed(String),
}

/// As [`send_link`], but never lets macOS ask the user: when sending
/// would need the user's consent first, nothing is shown and the answer
/// is [`LinkRefused::NeedsConsent`] (`kAEDoNotPromptForUserConsent`).
/// For a click that must never raise a prompt (Go to it).
///
/// # Errors
///
/// Why it didn't go.
pub fn send_link_quietly(pid: u32, link: &str) -> Result<(), LinkRefused> {
    #[cfg(target_os = "macos")]
    {
        mac::send_link_with(pid, link, false).map_err(|code| match code {
            mac::NEEDS_CONSENT => LinkRefused::NeedsConsent,
            mac::NOT_PERMITTED => LinkRefused::Denied,
            other => LinkRefused::Failed(format!("Apple event error {other}")),
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pid, link);
        Err(LinkRefused::Failed(
            crate::unsupported("handing a link to an app").to_string(),
        ))
    }
}

/// Whether the app process `pid`, started at `start` (epoch seconds, as
/// the process table reports it), lost the window server of this login
/// session: it isn't one of the apps the system knows now
/// ([`registered`]), **and** it started before this login session did
/// ([`session_since`]). An app started in this session is always younger
/// than the session, so the two together never
/// take a working app for a lost one (an app still coming up isn't
/// registered yet, but it's younger). `false` whenever either can't be
/// read: never a restart on a guess.
#[must_use]
pub fn detached(pid: u32, start: Option<i64>) -> bool {
    // The cheap check first: an app the system knows is never lost, and
    // that's every app on a normal day.
    if registered(pid) != Some(false) {
        return false;
    }
    started_before(start, session_since())
}

/// `start` is earlier than `session`, both known (whole seconds: an app
/// started in the session's own second is never counted as older).
#[must_use]
pub fn started_before(start: Option<i64>, session: Option<i64>) -> bool {
    matches!((start, session), (Some(start), Some(session)) if start < session)
}

/// Whether `pid` is one of the apps this login session's window server
/// knows (macOS: `NSRunningApplication`, what `lsappinfo` lists); `None`
/// where this system can't tell yet.
#[must_use]
pub fn registered(pid: u32) -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        let pid = i32::try_from(pid).ok()?;
        Some(
            objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                .is_some(),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// When this login session started, epoch seconds: on macOS, this user's
/// own `loginwindow`, which starts with the session and starts again with
/// it when the window server is restarted (both at 01:59:05 on
/// 2026-11-04; `WindowServer` itself runs as its own user, and the
/// process table gives no start for it). `None` where it can't be read or
/// this system has none to read yet.
#[must_use]
pub fn session_since() -> Option<i64> {
    #[cfg(target_os = "macos")]
    {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let me = crate::user::current().ok()?.uid();
        let mut system = System::new();
        // Names, owners and start times only: never arguments or
        // environments.
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_user(UpdateKind::Always),
        );
        system
            .processes()
            .values()
            .filter(|process| process.name() == "loginwindow")
            // Its effective user is the person logged in (its real one is
            // the system's).
            .filter(|process| process.effective_user_id().is_some_and(|user| **user == me))
            .filter_map(|process| i64::try_from(process.start_time()).ok())
            .filter(|at| *at > 0)
            .max()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Asks the app process `pid` to quit (SIGTERM: it ends as it would on
/// a normal quit, saving what it saves then). Returns once the signal is
/// sent; the caller waits for it to end.
///
/// # Errors
///
/// The process is gone or isn't this user's, or this system can't yet.
pub fn quit(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;
        let pid = i32::try_from(pid)
            .ok()
            .filter(|pid| *pid > 1)
            .ok_or_else(|| std::io::Error::other("that isn't an app's process"))?;
        kill(Pid::from_raw(pid), Signal::SIGTERM).map_err(std::io::Error::from)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        Err(crate::unsupported("quitting an app"))
    }
}

/// The version an app bundle says it is (macOS: `CFBundleShortVersionString`
/// in its `Contents/Info.plist`, an XML property list; a binary one, or any
/// other system, answers `None`). Read only, at most 1 MB.
#[must_use]
pub fn bundle_version(bundle: &std::path::Path) -> Option<String> {
    use std::io::Read as _;
    let mut text = String::new();
    std::fs::File::open(bundle.join("Contents/Info.plist"))
        .ok()?
        .take(1 << 20)
        .read_to_string(&mut text)
        .ok()?;
    let key = text.find("<key>CFBundleShortVersionString</key>")?;
    let rest = &text[key..];
    let open = rest.find("<string>")? + "<string>".len();
    let close = rest[open..].find("</string>")?;
    let version = rest[open..open + close].trim();
    (!version.is_empty()
        && version.len() <= 64
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'))
    .then(|| version.to_owned())
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

    /// `errAEEventWouldRequireUserConsent`: it would have asked.
    pub(super) const NEEDS_CONSENT: isize = -1744;
    /// `errAEEventNotPermitted`: the user said no.
    pub(super) const NOT_PERMITTED: isize = -1743;
    /// `kAEDoNotPromptForUserConsent` (macOS 10.14): never ask.
    const NO_PROMPT: usize = 0x0002_0000;

    /// Sends it, letting macOS ask the user first only when `may_ask`;
    /// the Apple event error code when it didn't go (0: not an event
    /// error).
    pub(super) fn send_link_with(pid: u32, link: &str, may_ask: bool) -> Result<(), isize> {
        let pid = i32::try_from(pid).map_err(|_| 0_isize)?;
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
        let options = if may_ask {
            NSAppleEventSendOptions::NoReply
        } else {
            NSAppleEventSendOptions(
                NSAppleEventSendOptions::NoReply.0
                    | NSAppleEventSendOptions::NeverInteract.0
                    | NO_PROMPT,
            )
        };
        event
            .sendEventWithOptions_timeout_error(options, DELIVER)
            .map(|_| ())
            .map_err(|error| error.code())
    }

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
    fn a_bundles_version_is_read_from_its_xml_plist() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("Contents")).unwrap();
        std::fs::write(
            tmp.path().join("Contents/Info.plist"),
            "<plist><dict><key>CFBundleName</key><string>App</string>\n<key>CFBundleShortVersionString</key>\n    <string>4.16336.0</string></dict></plist>",
        )
        .unwrap();
        assert_eq!(bundle_version(tmp.path()).as_deref(), Some("4.16336.0"));
        std::fs::write(tmp.path().join("Contents/Info.plist"), "bplist00…").unwrap();
        assert_eq!(bundle_version(tmp.path()), None);
    }

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

    /// 0.3.8: only an app older than the login session counts as lost;
    /// anything unknown is never lost.
    #[test]
    fn only_an_app_older_than_its_session_counts_as_lost() {
        assert!(started_before(Some(100), Some(200)));
        assert!(!started_before(Some(200), Some(200)), "the same second");
        assert!(!started_before(Some(300), Some(200)), "started after it");
        assert!(!started_before(None, Some(200)));
        assert!(!started_before(Some(100), None));
    }

    /// This test process is no app: on a Mac it isn't registered, but it
    /// started in this login session, so it is never lost. Pid 1 and an
    /// unused pid are never asked to quit.
    #[test]
    fn a_process_started_in_this_session_is_never_lost() {
        let me = std::process::id();
        let start = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!detached(me, i64::try_from(start).ok()));
        // A runner with no one logged in has no session to read.
        if let Some(session) = session_since() {
            println!("this login session since {session}");
            assert!(session <= i64::try_from(start).unwrap());
        }
        assert!(quit(1).is_err());
        assert!(quit(0).is_err());
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
