//! The first open after an update (0.4.1): opening the data can take
//! seconds (a safety copy, the upgrade, a tidy), and the app has no window
//! yet, so a bouncing Dock icon looked like a hang. The state is built on
//! a worker thread; if a long step has started and 300 ms have passed, a
//! small window says what it is doing until it is done. Small databases
//! never see it. Quitting meanwhile is safe (the safety copy, and each
//! step's own transaction), and the window says so.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_core::storage::{OpenStep, OpenWatch};

use crate::error::AppError;
use crate::state::AppState;

/// How long the work runs before the window shows.
const SHOW_AFTER: Duration = Duration::from_millis(300);

/// What the worker has said so far.
#[derive(Default)]
struct Said {
    step: Option<OpenStep>,
    notes: Vec<String>,
}

/// The worker's watcher: the step, the free room, the notes.
struct Watch(Arc<Mutex<Said>>);

impl OpenWatch for Watch {
    fn step(&mut self, step: OpenStep) {
        if let Ok(mut said) = self.0.lock() {
            said.step = Some(step);
        }
    }
    fn free_bytes(&self, folder: &std::path::Path) -> Option<u64> {
        juggler_platform::room::free_space(folder).ok()
    }
    fn note(&mut self, words: &str) {
        if let Ok(mut said) = self.0.lock() {
            said.notes.push(words.to_owned());
        }
    }
}

/// A step in the window's words.
#[must_use]
pub fn step_words(step: OpenStep) -> &'static str {
    match step {
        OpenStep::SafetyCopy => "Making a safety copy",
        OpenStep::Updating => "Updating your data",
        OpenStep::Tidying => "Tidying up",
    }
}

/// The window's title line and its note (one place, for its tests).
pub const HEADLINE: &str = "Updating AI Juggler's data. This happens once after an update.";
/// What quitting does now.
pub const QUIT_IS_SAFE: &str =
    "You can quit safely: a copy of your data was kept before the update.";

/// Builds the app's state at `data_dir`, with the window while a long
/// step runs. Returns the state and the notes for the debug log (a tidy
/// skipped, and why), written once that log exists.
///
/// # Errors
///
/// As [`AppState::init`].
pub fn init_state(data_dir: PathBuf) -> Result<(AppState, Vec<String>), AppError> {
    let said = Arc::new(Mutex::new(Said::default()));
    let (done, finished) = std::sync::mpsc::channel();
    let watch = Watch(Arc::clone(&said));
    std::thread::Builder::new()
        .name("open-data".into())
        .spawn(move || {
            let mut watch = watch;
            let _ = done.send(AppState::init_watched(data_dir, &mut watch));
        })
        .map_err(|error| AppError::io("start opening the data", &error))?;
    let started = Instant::now();
    let state = loop {
        match finished.recv_timeout(Duration::from_millis(20)) {
            Ok(state) => break state,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(AppError::Io {
                    context: "open the data",
                    message: "the work stopped before it finished".into(),
                });
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        let long = said.lock().is_ok_and(|said| said.step.is_some());
        if long && started.elapsed() >= SHOW_AFTER {
            break window::while_working(&said, &finished)?;
        }
    };
    let notes = said
        .lock()
        .map(|mut said| std::mem::take(&mut said.notes))
        .unwrap_or_default();
    Ok((state?, notes))
}

#[cfg(target_os = "macos")]
mod window {
    //! The window itself: `AppKit`, on the main thread, safe calls only.
    use std::sync::mpsc::{Receiver, RecvTimeoutError};
    use std::sync::{Arc, Mutex};

    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSEventMask, NSProgressIndicator,
        NSProgressIndicatorStyle, NSStackView, NSTextField, NSUserInterfaceLayoutOrientation,
        NSView, NSViewController, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSArray, NSDate, NSEdgeInsets, NSSize, NSString};

    use super::{HEADLINE, QUIT_IS_SAFE, Said, step_words};
    use crate::error::AppError;
    use crate::state::AppState;

    type Done = Result<AppState, AppError>;

    /// Shows the window and keeps it answering until the work is done;
    /// closing it quits (safe, it says so).
    pub(super) fn while_working(
        said: &Arc<Mutex<Said>>,
        finished: &Receiver<Done>,
    ) -> Result<Done, AppError> {
        let Some(mtm) = MainThreadMarker::new() else {
            return finished.recv().map_err(|_| stopped());
        };
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();
        let headline = NSTextField::labelWithString(&NSString::from_str(HEADLINE), mtm);
        let step = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        let note = NSTextField::labelWithString(&NSString::from_str(QUIT_IS_SAFE), mtm);
        let bar = NSProgressIndicator::new(mtm);
        bar.setStyle(NSProgressIndicatorStyle::Bar);
        bar.setIndeterminate(false);
        bar.setMaxValue(3.0);
        let views: Vec<Retained<NSView>> = vec![
            Retained::into_super(Retained::into_super(headline)),
            Retained::into_super(Retained::into_super(step.clone())),
            Retained::into_super(bar.clone()),
            Retained::into_super(Retained::into_super(note)),
        ];
        let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(&views), mtm);
        stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        stack.setSpacing(12.0);
        stack.setEdgeInsets(NSEdgeInsets {
            top: 24.0,
            left: 28.0,
            bottom: 24.0,
            right: 28.0,
        });
        let controller = NSViewController::new(mtm);
        controller.setView(&stack);
        let window = NSWindow::windowWithContentViewController(&controller);
        window.setTitle(&NSString::from_str("AI Juggler"));
        window.setStyleMask(NSWindowStyleMask::Titled | NSWindowStyleMask::Closable);
        window.setContentSize(NSSize {
            width: 440.0,
            height: 160.0,
        });
        window.center();
        window.makeKeyAndOrderFront(None);
        #[allow(deprecated, reason = "`activate` needs macOS 14; this works on every release")]
        app.activateIgnoringOtherApps(true);
        // The default run loop mode by its value (modes compare by string;
        // the framework's constant is an extern static, not safe to read).
        let default_mode = NSString::from_str("kCFRunLoopDefaultMode");
        let mut shown = None;
        loop {
            match finished.recv_timeout(std::time::Duration::from_millis(16)) {
                Ok(done) => {
                    window.close();
                    return Ok(done);
                }
                Err(RecvTimeoutError::Disconnected) => return Err(stopped()),
                Err(RecvTimeoutError::Timeout) => {}
            }
            let now = said.lock().ok().and_then(|said| said.step);
            if now != shown
                && let Some(current) = now
            {
                step.setStringValue(&NSString::from_str(&format!("{}…", step_words(current))));
                bar.setDoubleValue(match current {
                    juggler_core::storage::OpenStep::SafetyCopy => 1.0,
                    juggler_core::storage::OpenStep::Updating => 2.0,
                    juggler_core::storage::OpenStep::Tidying => 3.0,
                });
                shown = now;
            }
            // Its events: drawing, moving it, and its close button.
            while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(&NSDate::distantPast()),
                &default_mode,
                true,
            ) {
                app.sendEvent(&event);
            }
            if !window.isVisible() {
                // Closed: quitting is safe at any step (each is its own
                // transaction, and the safety copy is kept).
                std::process::exit(0);
            }
        }
    }

    fn stopped() -> AppError {
        AppError::Io {
            context: "open the data",
            message: "the work stopped before it finished".into(),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod window {
    //! Windows and Linux get the window with their own first release
    //! (D-52); until then the open runs without one.
    use std::sync::mpsc::Receiver;
    use std::sync::{Arc, Mutex};

    use super::Said;
    use crate::error::AppError;
    use crate::state::AppState;

    pub(super) fn while_working(
        _: &Arc<Mutex<Said>>,
        finished: &Receiver<Result<AppState, AppError>>,
    ) -> Result<Result<AppState, AppError>, AppError> {
        finished.recv().map_err(|_| AppError::Io {
            context: "open the data",
            message: "the work stopped before it finished".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window's words, plainly (0.4.1's ruling).
    #[test]
    fn the_steps_say_what_they_do_in_plain_words() {
        assert_eq!(step_words(OpenStep::SafetyCopy), "Making a safety copy");
        assert_eq!(step_words(OpenStep::Updating), "Updating your data");
        assert_eq!(step_words(OpenStep::Tidying), "Tidying up");
        assert!(HEADLINE.contains("This happens once after an update"));
        assert!(QUIT_IS_SAFE.starts_with("You can quit safely"));
    }

    /// A new data folder and one already up to date open without a step to
    /// show and with nothing to note (headless here: no main thread).
    #[test]
    fn an_up_to_date_open_has_nothing_to_show() {
        let dir = tempfile::tempdir().unwrap();
        let (state, notes) = init_state(dir.path().join("data")).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        drop(state);
        let (_, notes) = init_state(dir.path().join("data")).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
    }
}
