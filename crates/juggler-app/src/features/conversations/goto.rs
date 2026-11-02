//! AJ-E2102: Go to it. Brings the app a session is open in forward, by
//! that app's own process (`NSRunningApplication`), never by bundle id:
//! every Claude app instance shares one bundle id, and opening by it
//! brought the wrong profile forward. Then the step the app allows:
//! a Codex thread by its documented link, a folder in an editor, a
//! terminal's tab (only when the user allowed terminal tabs in Settings),
//! the window count (only when the user turned on window focus). A click
//! never raises a permission prompt, and the outcome names the step
//! reached. "Forward" is said only once the app is frontmost
//! ([`crate::activation`]); otherwise the outcome says that macOS kept it
//! behind.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::live::{HostKind, same_start};

use super::dto::GoToOutcomeInfo;
use super::places::{Host, HostRegistry};
use super::scan::Place;
use crate::activation::{self, Activation, Front, Request, Why};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// How long Go to it waits before saying it's still on it.
pub const LIMIT: Duration = Duration::from_millis(1500);

/// What Go to it needs from the Mac (a seam: the tests script it).
/// [`Front`] asks for an app to come forward and reads the frontmost one.
pub trait Desktop: Front {
    /// The process's start time now; `None` when it's gone.
    fn start_of(&self, pid: u32) -> Option<i64>;
    /// Opens a link with the app that handles it.
    fn open_link(&self, link: &str) -> bool;
    /// Runs `command <folder>` (an editor's own command).
    fn open_folder(&self, command: &str, folder: &Path) -> bool;
    /// Runs an `AppleScript`; only ever called behind a setting.
    ///
    /// # Errors
    ///
    /// The script's error text.
    fn osascript(&self, source: &str) -> Result<String, String>;
    /// The terminal device a process runs on (`/dev/ttys003`).
    fn tty(&self, pid: u32) -> Option<String>;
}

/// What the user turned on in Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gates {
    /// Window focus (Accessibility and System Events) is on.
    pub windows: bool,
    /// Terminal tabs (`AppleScript` to the terminal) are on.
    pub tabs: bool,
}

/// One Go to it, worked out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The tool's id for the session.
    pub session_id: String,
    /// Its folder.
    pub cwd: Option<PathBuf>,
    /// The place to go to.
    pub place: Place,
}

fn outcome(text: String, reached: &str) -> GoToOutcomeInfo {
    GoToOutcomeInfo {
        outcome: text,
        reached: reached.to_owned(),
    }
}

/// The place's name in the outcome: `Claude app · b-work`, `Terminal`.
fn label(host: &Host) -> String {
    match &host.instance {
        Some(instance) => format!("{} · {instance}", host.shown),
        None => host.shown.clone(),
    }
}

/// `path` for a `file` link: each byte outside the plain set as `%XX`.
fn link_path(path: &Path) -> String {
    let mut out = String::new();
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The window count and how many are minimised, through System Events:
/// only called when the user turned window focus on.
fn windows(desktop: &dyn Desktop, pid: u32) -> Option<(usize, usize)> {
    let script = format!(
        "tell application \"System Events\" to tell (first application process whose unix id is {pid})\nset n to count of windows\nset m to 0\nrepeat with w in windows\ntry\nif value of attribute \"AXMinimized\" of w is true then set m to m + 1\nend try\nend repeat\nreturn (n as text) & \",\" & (m as text)\nend tell"
    );
    let raw = desktop.osascript(&script).ok()?;
    let (n, m) = raw.trim().split_once(',')?;
    Some((n.trim().parse().ok()?, m.trim().parse().ok()?))
}

/// Goes to `target`: the app forward by its process, then what the app
/// allows. Pure over the seam.
#[must_use]
pub fn go(
    target: &Target,
    hosts: &HostRegistry,
    desktop: &dyn Desktop,
    gates: Gates,
) -> GoToOutcomeInfo {
    let Some(host) = &target.place.host else {
        return outcome(
            "It doesn't run in an app AI Juggler can bring forward".to_owned(),
            "none",
        );
    };
    let name = label(host);
    // The app's process must still be the one seen (pids are reused).
    if !same_start(host.start, desktop.start_of(host.pid)) {
        return outcome(format!("{name} isn't open now"), "none");
    }
    let in_front = match desktop.request(host.pid) {
        Request::Gone => return outcome(format!("{name} isn't open now"), "none"),
        Request::Refused => return outcome(activation::behind_text(&name, Why::Refused), "none"),
        Request::Asked { in_front } => in_front,
    };
    // The step the app allows, then whether it really came forward.
    let done = match step(target, host, &name, hosts, desktop, gates) {
        Step::Final(said) => return said,
        Step::Done(done) => done,
    };
    match activation::settle(desktop, host.pid, in_front) {
        Activation::Forward => outcome(done.forward, done.reached),
        Activation::Gone => outcome(format!("{name} isn't open now"), "none"),
        Activation::Behind(why) => match done.kept {
            // The step itself held (the link opened, the tab selected).
            Some(kept) => outcome(
                format!("{kept}, but {}", activation::behind_text("it", why)),
                done.reached,
            ),
            None => outcome(activation::behind_text(&name, why), "none"),
        },
    }
}

/// A step's words: when the app is in front, and what still holds when
/// it isn't (`None`: nothing beyond the request).
struct Done {
    forward: String,
    kept: Option<String>,
    reached: &'static str,
}

/// What the step after the request came to.
enum Step {
    /// Its words; whether the app came forward is checked next.
    Done(Done),
    /// The answer, whatever is in front (every window minimised).
    Final(GoToOutcomeInfo),
}

/// The words for coming forward and nothing more.
fn app_only(forward: String) -> Step {
    Step::Done(Done {
        forward,
        kept: None,
        reached: "app",
    })
}

/// The step the app allows after the request: a thread by its link, a
/// folder in an editor, a terminal's tab, the window count.
fn step(
    target: &Target,
    host: &Host,
    name: &str,
    hosts: &HostRegistry,
    desktop: &dyn Desktop,
    gates: Gates,
) -> Step {
    let app = host.app_id.as_deref().and_then(|id| hosts.by_id(id));
    if let Some(link) = app.and_then(|app| app.thread_link_for(&target.session_id))
        && desktop.open_link(&link)
    {
        let opened = format!("Opened the thread in {}", host.shown);
        return Step::Done(Done {
            forward: opened.clone(),
            kept: Some(opened),
            reached: "thread",
        });
    }
    match host.kind {
        Some(HostKind::Editor) => {
            if let (Some(app), Some(folder)) = (app, target.cwd.as_deref()) {
                let opened = match (&app.folder_link, &app.folder_command) {
                    (Some(link), _) => {
                        desktop.open_link(&link.replace("{folder}", &link_path(folder)))
                    }
                    (None, Some(command)) => desktop.open_folder(command, folder),
                    (None, None) => false,
                };
                if opened {
                    let shown = folder.file_name().map_or_else(
                        || folder.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    );
                    let opened = format!("Opened {shown} in {}", host.shown);
                    return Step::Done(Done {
                        forward: opened.clone(),
                        kept: Some(opened),
                        reached: "folder",
                    });
                }
            }
            app_only(format!("Brought {name} forward"))
        }
        Some(HostKind::Terminal) => {
            if gates.tabs
                && let Some(script) = app.and_then(|app| app.tab_script.as_ref())
                && let Some(tty) = desktop.tty(target.place.pid)
                && desktop
                    .osascript(&script.replace("{tty}", &tty))
                    .is_ok_and(|said| said.trim() == "ok")
            {
                return Step::Done(Done {
                    forward: format!("Brought {name} forward and selected its tab"),
                    kept: Some(format!("Selected its tab in {name}")),
                    reached: "tab",
                });
            }
            app_only(format!(
                "Brought {name} forward; the session is in one of its tabs"
            ))
        }
        Some(HostKind::Desktop) | None => {
            let mut text = format!("Brought {name} forward");
            if gates.windows
                && let Some((all, minimised)) = windows(desktop, host.pid)
            {
                if all > 0 && minimised == all {
                    return Step::Final(outcome(
                        format!("{name} is minimised — click it in the Dock"),
                        "app",
                    ));
                }
                if all > 1 {
                    let _ = write!(text, " ({all} windows)");
                }
            }
            if host.kind == Some(HostKind::Desktop) {
                text.push_str("; pick the session in its sidebar");
            }
            app_only(text)
        }
    }
}

/// Runs `body` with a time limit: its answer, or `None` when it's still
/// running (it finishes on its own thread).
pub fn within<T: Send + 'static>(
    limit: Duration,
    body: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("go-to".to_owned())
        .spawn(move || {
            let _ = send.send(body());
        })
        .ok()?;
    receive.recv_timeout(limit).ok()
}

/// Go to it for one of the user's conversations, open now: at the place
/// with process `pid`, else its first place.
///
/// # Errors
///
/// [`AppError::Refused`] when the conversation isn't in the index; a
/// poisoned lock.
pub fn go_to_impl(
    state: &Arc<AppState>,
    account_id: &str,
    conversation_id: &str,
    pid: Option<u32>,
    desktop: Arc<dyn Desktop>,
) -> AppResult<GoToOutcomeInfo> {
    let conversation = state
        .conversations
        .all()?
        .into_iter()
        .find(|c| c.account_id == account_id && c.summary.id == conversation_id)
        .ok_or_else(|| AppError::Refused("that session isn't there any more".to_owned()))?;
    let Some(entry) =
        state
            .conversations
            .live_entry(account_id, &conversation.tool, conversation_id)
    else {
        return Ok(outcome("It isn't open now".to_owned(), "none"));
    };
    let place = pid
        .and_then(|pid| entry.places.iter().find(|place| place.pid == pid))
        .or_else(|| entry.places.iter().find(|place| place.host.is_some()))
        .or_else(|| entry.places.first())
        .cloned()
        .ok_or_else(|| AppError::Refused("it isn't open now".to_owned()))?;
    let target = Target {
        session_id: conversation_id.to_owned(),
        cwd: conversation
            .folder
            .clone()
            .or(conversation.summary.cwd.clone()),
        place,
    };
    let gates = Gates {
        windows: crate::os_access::exact_windows_allowed(state),
        tabs: crate::os_access::terminal_tabs_allowed(state),
    };
    let hosts = HostRegistry::of(&state.registry);
    let still = target.place.host.as_ref().map_or_else(String::new, label);
    Ok(
        within(LIMIT, move || go(&target, &hosts, desktop.as_ref(), gates))
            .unwrap_or_else(|| outcome(format!("Still bringing {still} forward…"), "pending")),
    )
}

/// AJ-E2104: a place's Open. Brings forward the app whose process is
/// `app_pid`, only when an open session runs in it now (never another
/// process, never a launch), within the same limit, and says what it did.
///
/// # Errors
///
/// A poisoned lock.
pub fn bring_forward_impl(
    state: &Arc<AppState>,
    app_pid: u32,
    desktop: Arc<dyn Desktop>,
) -> AppResult<GoToOutcomeInfo> {
    let host = state
        .conversations
        .model
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .entries
        .iter()
        .flat_map(|entry| entry.places.iter())
        .filter_map(|place| place.host.as_ref())
        .find(|host| host.pid == app_pid)
        .cloned();
    let Some(host) = host else {
        return Ok(outcome("That app isn't open now".to_owned(), "none"));
    };
    let still = label(&host);
    Ok(
        within(LIMIT, move || bring_forward(&host, desktop.as_ref()))
            .unwrap_or_else(|| outcome(format!("Still bringing {still} forward…"), "pending")),
    )
}

/// Brings `host` forward by its process, when it's still the one seen,
/// and says so only once it's in front.
#[must_use]
pub fn bring_forward(host: &Host, desktop: &dyn Desktop) -> GoToOutcomeInfo {
    let name = label(host);
    if !same_start(host.start, desktop.start_of(host.pid)) {
        return outcome(format!("{name} isn't open now"), "none");
    }
    match activation::bring(desktop, host.pid) {
        Activation::Forward => outcome(format!("Brought {name} forward"), "app"),
        Activation::Gone => outcome(format!("{name} isn't open now"), "none"),
        Activation::Behind(why) => outcome(activation::behind_text(&name, why), "none"),
    }
}

/// The real Mac: [`activation::Mac`] to come forward, `open` for links.
#[derive(Clone, Copy, Debug, Default)]
pub struct MacDesktop;

impl Front for MacDesktop {
    fn request(&self, pid: u32) -> Request {
        activation::Mac.request(pid)
    }

    fn frontmost(&self) -> Option<u32> {
        activation::Mac.frontmost()
    }
}

impl Desktop for MacDesktop {
    fn start_of(&self, pid: u32) -> Option<i64> {
        super::procs::some(&[pid])
            .get(&pid)
            .and_then(|process| process.start)
    }

    fn open_link(&self, link: &str) -> bool {
        std::process::Command::new("/usr/bin/open")
            .arg(link)
            .status()
            .is_ok_and(|status| status.success())
    }

    fn open_folder(&self, command: &str, folder: &Path) -> bool {
        let Some(program) = juggler_adapter::user_path::login_shell_path()
            .and_then(|path| juggler_adapter::user_path::find_in(command, path))
        else {
            return false;
        };
        std::process::Command::new(program)
            .arg(folder)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
    }

    fn osascript(&self, source: &str) -> Result<String, String> {
        let out = std::process::Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(source)
            .output()
            .map_err(|error| error.to_string())?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
        }
    }

    fn tty(&self, pid: u32) -> Option<String> {
        let out = std::process::Command::new("/bin/ps")
            .args(["-o", "tty=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let tty = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        (tty.starts_with("ttys") && tty[4..].bytes().all(|b| b.is_ascii_digit()))
            .then(|| format!("/dev/{tty}"))
    }
}

#[cfg(test)]
mod tests;
