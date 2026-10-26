//! Bringing another app forward, and checking that it came (0.1.5 review).
//!
//! Since macOS 14, activation is cooperative: an app can't take the front
//! by asking. The app in front hands it over first. So AI Juggler, the app
//! the user just clicked, yields to the target's process
//! (`yieldActivationToApplication:`, on the main thread, where
//! `NSApplication` lives) and then activates it from itself
//! (`activateFromApplication:options:`). Without the yield, macOS accepted
//! the request and ignored it. Go to it then said "Brought … forward"
//! while the app stayed where it was.
//!
//! By process, never by bundle id: all of the maintainer's Claude
//! profiles share one bundle id, and one of them must come forward. Only
//! safe objc2 calls, and no permission prompt. The outcome is checked: the
//! frontmost app's process (`NSWorkspace`) is read for up to
//! [`SETTLE`]. Success is never assumed.

use std::sync::OnceLock;
use std::time::Duration;

/// How long the frontmost app is watched after a request.
pub const SETTLE: Duration = Duration::from_millis(500);
/// How often it is read meanwhile.
const STEP: Duration = Duration::from_millis(25);
/// How long a request waits for the main thread.
const MAIN_WAIT: Duration = Duration::from_millis(400);

/// What macOS said to one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// macOS took the request. `in_front`: AI Juggler was the active app,
    /// so it had activation to hand over.
    Asked {
        /// AI Juggler was the active app when it asked.
        in_front: bool,
    },
    /// No such app process.
    Gone,
    /// macOS turned the request down.
    Refused,
}

/// Why an app didn't come forward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// AI Juggler wasn't the active app, so it had nothing to hand over.
    NotInFront,
    /// macOS turned the request down.
    Refused,
    /// macOS took the request, and another app stayed in front.
    StayedBehind,
}

/// What bringing an app forward did, checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    /// It's the frontmost app now.
    Forward,
    /// No such app process.
    Gone,
    /// It isn't in front.
    Behind(Why),
}

/// The honest words when `subject` didn't come forward, with the reason
/// when it's known: `macOS didn't bring Claude · Work forward. Click it in
/// the Dock`.
#[must_use]
pub fn behind_text(subject: &str, why: Why) -> String {
    match why {
        Why::NotInFront => format!(
            "macOS didn't bring {subject} forward: AI Juggler wasn't in front. Click it in the Dock"
        ),
        Why::Refused => format!("macOS refused to bring {subject} forward. Click it in the Dock"),
        Why::StayedBehind => format!("macOS didn't bring {subject} forward. Click it in the Dock"),
    }
}

/// What bringing an app forward needs from the Mac (a seam: tests script
/// it, and the integration test runs the main thread's loop while it waits).
pub trait Front: Send + Sync {
    /// Asks macOS to bring the app whose process is `pid` forward.
    fn request(&self, pid: u32) -> Request;
    /// The frontmost app's process now.
    fn frontmost(&self) -> Option<u32>;
    /// Waits `span` before the next look.
    fn pause(&self, span: Duration) {
        std::thread::sleep(span);
    }
}

/// Asks for `pid` to come forward, then checks that it did.
pub fn bring<F: Front + ?Sized>(front: &F, pid: u32) -> Activation {
    match front.request(pid) {
        Request::Gone => Activation::Gone,
        Request::Refused => Activation::Behind(Why::Refused),
        Request::Asked { in_front } => settle(front, pid, in_front),
    }
}

/// After a request was taken: `pid` is frontmost within [`SETTLE`], or
/// the reason it isn't (`in_front`: AI Juggler was the active app).
pub fn settle<F: Front + ?Sized>(front: &F, pid: u32, in_front: bool) -> Activation {
    let mut waited = Duration::ZERO;
    loop {
        if front.frontmost() == Some(pid) {
            return Activation::Forward;
        }
        if waited >= SETTLE {
            return Activation::Behind(if in_front {
                Why::StayedBehind
            } else {
                Why::NotInFront
            });
        }
        front.pause(STEP);
        waited += STEP;
    }
}

/// Runs a job on the main thread (the app installs Tauri's).
type MainRunner = Box<dyn Fn(Box<dyn FnOnce() + Send>) -> bool + Send + Sync>;

static MAIN: OnceLock<MainRunner> = OnceLock::new();

/// Installs how a job reaches the main thread (the app passes
/// `AppHandle::run_on_main_thread`). Once; later calls are ignored.
pub fn install_main_thread(run: impl Fn(Box<dyn FnOnce() + Send>) -> bool + Send + Sync + 'static) {
    let _ = MAIN.set(Box::new(run));
}

/// The real Mac: `NSRunningApplication` by process, `NSWorkspace` for
/// the frontmost app.
#[derive(Clone, Copy, Debug, Default)]
pub struct Mac;

impl Front for Mac {
    fn request(&self, pid: u32) -> Request {
        #[cfg(target_os = "macos")]
        {
            use objc2::MainThreadMarker;
            if let Some(mtm) = MainThreadMarker::new() {
                return mac::ask(pid, Some(mtm));
            }
            let Some(run) = MAIN.get() else {
                // No main thread to reach (a test, or before the app set
                // up): asked without the hand-over; the check tells.
                return mac::ask(pid, None);
            };
            let (send, receive) = std::sync::mpsc::channel();
            let queued = run(Box::new(move || {
                let _ = send.send(mac::ask(pid, MainThreadMarker::new()));
            }));
            if !queued {
                return mac::ask(pid, None);
            }
            // A busy main thread may still ask later; the check tells.
            receive
                .recv_timeout(MAIN_WAIT)
                .unwrap_or(Request::Asked { in_front: true })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (pid, &MAIN, MAIN_WAIT);
            Request::Refused
        }
    }

    fn frontmost(&self) -> Option<u32> {
        #[cfg(target_os = "macos")]
        {
            objc2_app_kit::NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .and_then(|app| u32::try_from(app.processIdentifier()).ok())
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication};

    use super::Request;

    /// One request. `mtm`: on the main thread, where AI Juggler can hand
    /// activation over first.
    pub fn ask(pid: u32, mtm: Option<MainThreadMarker>) -> Request {
        let Ok(pid) = i32::try_from(pid) else {
            return Request::Gone;
        };
        let Some(target) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        else {
            return Request::Gone;
        };
        if target.isTerminated() {
            return Request::Gone;
        }
        if target.isHidden() {
            let _ = target.unhide();
        }
        let me = NSRunningApplication::currentApplication();
        let in_front = mtm.map_or_else(
            || me.isActive(),
            |mtm| NSApplication::sharedApplication(mtm).isActive(),
        );
        let options = NSApplicationActivationOptions::ActivateAllWindows;
        let taken = if objc2::available!(macos = 14.0) {
            // Cooperative activation: hand over, then activate from here.
            if let Some(mtm) = mtm {
                NSApplication::sharedApplication(mtm).yieldActivationToApplication(&target);
            }
            target.activateFromApplication_options(&me, options)
        } else {
            target.activateWithOptions(options)
        };
        if taken {
            Request::Asked { in_front }
        } else {
            Request::Refused
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// A scripted Mac: what the request says, and from which look on the
    /// target is frontmost (`None`: never).
    struct Script {
        request: Request,
        front_from: Option<u32>,
        looks: AtomicU32,
        waited: Mutex<Duration>,
    }

    impl Script {
        fn new(request: Request, front_from: Option<u32>) -> Self {
            Self {
                request,
                front_from,
                looks: AtomicU32::new(0),
                waited: Mutex::new(Duration::ZERO),
            }
        }
    }

    impl Front for Script {
        fn request(&self, _pid: u32) -> Request {
            self.request
        }

        fn frontmost(&self) -> Option<u32> {
            let look = self.looks.fetch_add(1, Ordering::SeqCst);
            match self.front_from {
                Some(from) if look >= from => Some(7),
                _ => Some(1),
            }
        }

        fn pause(&self, span: Duration) {
            *self.waited.lock().unwrap() += span;
        }
    }

    #[test]
    fn forward_only_once_the_app_is_frontmost() {
        let at_once = Script::new(Request::Asked { in_front: true }, Some(0));
        assert_eq!(bring(&at_once, 7), Activation::Forward);
        assert_eq!(*at_once.waited.lock().unwrap(), Duration::ZERO);
        // macOS takes a moment: still forward, after a few looks.
        let later = Script::new(Request::Asked { in_front: true }, Some(4));
        assert_eq!(bring(&later, 7), Activation::Forward);
        assert_eq!(*later.waited.lock().unwrap(), STEP * 4);
    }

    #[test]
    fn a_taken_request_is_not_success_when_another_app_stays_in_front() {
        let stays = Script::new(Request::Asked { in_front: true }, None);
        assert_eq!(bring(&stays, 7), Activation::Behind(Why::StayedBehind));
        // It watched for the whole settle time, and no longer.
        assert_eq!(*stays.waited.lock().unwrap(), SETTLE);
        let not_in_front = Script::new(Request::Asked { in_front: false }, None);
        assert_eq!(bring(&not_in_front, 7), Activation::Behind(Why::NotInFront));
    }

    #[test]
    fn a_gone_or_refused_app_is_never_watched() {
        let gone = Script::new(Request::Gone, Some(0));
        assert_eq!(bring(&gone, 7), Activation::Gone);
        let refused = Script::new(Request::Refused, Some(0));
        assert_eq!(bring(&refused, 7), Activation::Behind(Why::Refused));
        assert_eq!(gone.looks.load(Ordering::SeqCst), 0);
        assert_eq!(refused.looks.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_words_say_what_happened_and_what_to_do() {
        assert_eq!(
            behind_text("Claude · Work", Why::StayedBehind),
            "macOS didn't bring Claude · Work forward. Click it in the Dock"
        );
        assert_eq!(
            behind_text("Claude · Work", Why::NotInFront),
            "macOS didn't bring Claude · Work forward: AI Juggler wasn't in front. Click it in the Dock"
        );
        assert_eq!(
            behind_text("ChatGPT", Why::Refused),
            "macOS refused to bring ChatGPT forward. Click it in the Dock"
        );
    }
}
