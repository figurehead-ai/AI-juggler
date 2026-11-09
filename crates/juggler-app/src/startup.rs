//! The first open after an update (0.4.1): opening the data can take
//! seconds (a safety copy, the upgrade, a tidy), and the app has no window
//! yet, so the app's bouncing icon looked like a hang. The state is built on
//! a worker thread; if a long step has started and 300 ms have passed, a
//! small window says what it is doing until it is done. Small databases
//! never see it. Quitting meanwhile is safe (the safety copy, and each
//! step's own transaction), and the window says so.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_core::storage::{OpenStep, OpenWatch};

use crate::error::AppError;
use crate::state::AppState;

/// What the work answers.
type Done = Result<AppState, AppError>;

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

/// The window's step line: which of the three, what it does, and how
/// long it has run ("Step 2 of 3: Updating your data… 0:32").
#[must_use]
pub fn step_line(step: OpenStep, secs: u64) -> String {
    format!(
        "Step {} of 3: {}… {}:{:02}",
        step_number(step),
        step_words(step),
        secs / 60,
        secs % 60
    )
}

/// The window's title line and its note (one place, for its tests).
pub const HEADLINE: &str = "Updating AI Juggler's data. This happens once after an update.";
/// What quitting does now.
pub const QUIT_IS_SAFE: &str =
    "You can quit safely: a copy of your data was kept before the update.";

/// Builds the app's state at `data_dir`, with the window while a long
/// step runs: shown once one has started and `SHOW_AFTER` (300 ms) has passed
/// since `launched` (the app's own start, so a cold launch's earlier
/// seconds count too and the app's icon never bounces long with no
/// window). Returns the state and the notes for the debug log (a tidy
/// skipped and why; when the window showed), written once that log exists.
///
/// # Errors
///
/// As [`AppState::init`].
pub fn init_state(
    data_dir: PathBuf,
    launched: Instant,
) -> Result<(AppState, Vec<String>), AppError> {
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
    let mut shown = None;
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
        if long && launched.elapsed() >= SHOW_AFTER {
            shown = Some((launched.elapsed(), started.duration_since(launched)));
            break while_working(&said, &finished)?;
        }
    };
    let mut notes = said
        .lock()
        .map(|mut said| std::mem::take(&mut said.notes))
        .unwrap_or_default();
    if let Some((at, opening)) = shown {
        notes.push(format!(
            "startup window: shown {} ms after launch (the data began opening at {} ms); ready at {} ms",
            at.as_millis(),
            opening.as_millis(),
            launched.elapsed().as_millis()
        ));
    }
    Ok((state?, notes))
}

/// The environment variable that makes a launch only the window (see
/// [`window_if_asked`]).
const WINDOW_ENV: &str = "AIJ_STARTUP_WINDOW";

/// Shows the window while the work runs, then answers with the work's
/// result. rc8 walk: drawn in this process, the window set the app's
/// `NSApplication` up before Tauri could, and Tauri's event loop never
/// heard that launching had finished: after an update no main window
/// ever came. So the window is a launch of this app that does nothing
/// else (as the "can't start" alert is): it gets the step on its input
/// and ends when that closes. Closing the window quits this launch too
/// (safe at any step, and it says so).
fn while_working(said: &Arc<Mutex<Said>>, finished: &Receiver<Done>) -> Result<Done, AppError> {
    use std::io::Write as _;
    let Some(mut window) = window::spawn() else {
        return finished.recv().map_err(|_| stopped());
    };
    let mut input = window.stdin.take();
    let mut shown = None;
    loop {
        match finished.recv_timeout(Duration::from_millis(30)) {
            Ok(done) => {
                drop(input.take());
                end_window(&mut window);
                return Ok(done);
            }
            Err(RecvTimeoutError::Disconnected) => {
                end_window(&mut window);
                return Err(stopped());
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        let now = said.lock().ok().and_then(|said| said.step);
        if now != shown
            && let (Some(step), Some(pipe)) = (now, input.as_mut())
        {
            let _ = writeln!(pipe, "{}", step_number(step)).and_then(|()| pipe.flush());
            shown = now;
        }
        // Its window closed: the user quit.
        if window.try_wait().ok().flatten().is_some() {
            std::process::exit(0);
        }
    }
}

/// The window's launch, told the work is done: given a moment to go,
/// then ended.
fn end_window(window: &mut std::process::Child) {
    let until = Instant::now() + Duration::from_secs(1);
    while Instant::now() < until {
        if window.try_wait().ok().flatten().is_some() {
            return;
        }
        // poll: the window's exit, bounded by the 1 s deadline above.
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = window.kill();
    let _ = window.wait();
}

fn stopped() -> AppError {
    AppError::Io {
        context: "open the data",
        message: "the work stopped before it finished".into(),
    }
}

/// A step as the window's input says it (1 to 3).
const fn step_number(step: OpenStep) -> u8 {
    match step {
        OpenStep::SafetyCopy => 1,
        OpenStep::Updating => 2,
        OpenStep::Tidying => 3,
    }
}

/// At the very start of a launch: the startup window, when this launch
/// is only that (asked by `AIJ_STARTUP_WINDOW=1`); it ends when its input closes
/// or its window does. Nothing else runs.
pub fn window_if_asked() {
    if std::env::var_os(WINDOW_ENV).is_some_and(|value| value == "1") {
        window::run();
        std::process::exit(0);
    }
}

#[cfg(target_os = "macos")]
mod window {
    //! The window's own launch: `AppKit`, on its main thread, safe calls
    //! only. It has no Dock icon of its own (the app's is there already).
    use std::io::BufRead as _;
    use std::sync::{Arc, Mutex};

    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSEventMask, NSProgressIndicator,
        NSProgressIndicatorStyle, NSStackView, NSTextField, NSUserInterfaceLayoutOrientation,
        NSView, NSViewController, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSArray, NSDate, NSEdgeInsets, NSSize, NSString};

    use super::{HEADLINE, QUIT_IS_SAFE, WINDOW_ENV};

    /// Starts the window's launch, its input piped.
    pub(super) fn spawn() -> Option<std::process::Child> {
        let exe = std::env::current_exe().ok()?;
        // Only the app starts its window: a test binary that opened data
        // slowly started itself again (and ran its tests).
        if !exe.to_string_lossy().contains(".app/Contents/MacOS/") {
            return None;
        }
        std::process::Command::new(exe)
            .env(WINDOW_ENV, "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .ok()
    }

    /// What the input has said: the step (1 to 3), and whether it closed.
    #[derive(Default)]
    struct Heard {
        step: Option<u8>,
        closed: bool,
    }

    pub(super) fn run() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let heard = Arc::new(Mutex::new(Heard::default()));
        {
            let heard = Arc::clone(&heard);
            std::thread::spawn(move || {
                for line in std::io::stdin().lock().lines() {
                    let Ok(line) = line else { break };
                    if let (Ok(step), Ok(mut heard)) = (line.trim().parse::<u8>(), heard.lock()) {
                        heard.step = Some(step);
                    }
                }
                if let Ok(mut heard) = heard.lock() {
                    heard.closed = true;
                }
            });
        }
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
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
        #[allow(
            deprecated,
            reason = "`activate` needs system version 14; this works on every release"
        )]
        app.activateIgnoringOtherApps(true);
        // The default run loop mode by its value (modes compare by string;
        // the framework's constant is an extern static, not safe to read).
        let default_mode = NSString::from_str("kCFRunLoopDefaultMode");
        let mut shown = None;
        let mut since = std::time::Instant::now();
        let mut said_secs = None;
        loop {
            let (now, closed) = heard
                .lock()
                .map_or((None, true), |heard| (heard.step, heard.closed));
            if closed {
                return;
            }
            if now != shown {
                shown = now;
                since = std::time::Instant::now();
                said_secs = None;
                bar.setDoubleValue(f64::from(now.unwrap_or(0)));
            }
            // The step and how long it has run, every second (the rc12
            // walk: a bar that moves only between three steps looked
            // stuck for most of a minute). A time, never a guessed share.
            let secs = since.elapsed().as_secs();
            if let Some(current) = now.and_then(super::step_of)
                && said_secs != Some(secs)
            {
                step.setStringValue(&NSString::from_str(&super::step_line(current, secs)));
                said_secs = Some(secs);
            }
            // Its events: drawing, moving it, and its close button; a
            // short wait so it doesn't spin.
            let until = NSDate::dateWithTimeIntervalSinceNow(0.03);
            while let Some(event) = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(&until),
                &default_mode,
                true,
            ) {
                app.sendEvent(&event);
            }
            if !window.isVisible() {
                // Closed: the launch doing the work hears it and quits
                // (safe at any step).
                return;
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod window {
    //! Windows and Linux get the window with their own first release
    //! (D-52); until then the open runs without one.
    pub(super) fn spawn() -> Option<std::process::Child> {
        None
    }

    pub(super) fn run() {}
}

/// The step a window input number names.
#[cfg_attr(
    not(target_os = "macos"),
    allow(
        dead_code,
        reason = "the window comes to each system with its own release (D-52)"
    )
)]
const fn step_of(number: u8) -> Option<OpenStep> {
    match number {
        1 => Some(OpenStep::SafetyCopy),
        2 => Some(OpenStep::Updating),
        3 => Some(OpenStep::Tidying),
        _ => None,
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
        assert_eq!(
            step_line(OpenStep::Updating, 92),
            "Step 2 of 3: Updating your data… 1:32"
        );
        assert!(HEADLINE.contains("This happens once after an update"));
        assert!(QUIT_IS_SAFE.starts_with("You can quit safely"));
    }

    /// A new data folder and one already up to date open without a step to
    /// show and with nothing to note (headless here: no main thread).
    #[test]
    fn an_up_to_date_open_has_nothing_to_show() {
        let dir = tempfile::tempdir().unwrap();
        let (state, notes) = init_state(dir.path().join("data"), Instant::now()).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        drop(state);
        let (_, notes) = init_state(dir.path().join("data"), Instant::now()).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
    }
}
