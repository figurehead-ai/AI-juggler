//! The alerts a launch shows before any window exists ("can't start",
//! "isn't answering"), in AI Juggler's own process.
//!
//! QA batch 9 (the maintainer's screen): rfd shows a dialog without a
//! parent window as a `CFUserNotification`, which the system owns. Killed
//! mid-alert, AI Juggler was gone and the alert stayed, with a Quit that
//! only dismissed it. On macOS these alerts are an `NSAlert` run on the
//! main thread instead: part of the process, gone with it. Every call is
//! a safe one (objc2's `MainThreadMarker` API), so there is no `unsafe`.
//! Elsewhere rfd's dialogs are already the process's own.

/// How serious the alert is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// It can't go on.
    Error,
    /// It may, after the user acts.
    Warning,
}

/// The test seams (SECURITY-GUIDELINES, test seams): in debug builds
/// only. A release build compiles none of this, so no environment
/// variable can silence its dialogs or answer them for the user
/// (`scripts/release_seams_check.sh` checks the release binary).
#[cfg(debug_assertions)]
mod seams {
    /// `AIJ_NO_DIALOGS`: dialogs go to stderr and answer their last
    /// button (headless runs, the tests that start the real binary).
    pub fn quiet() -> bool {
        std::env::var_os("AIJ_NO_DIALOGS").is_some()
    }

    /// `AIJ_DIALOG_ANSWER=<label>`: dialogs answer with that button (the
    /// tests that press Try again or Quit on the real binary).
    pub fn answer() -> Option<String> {
        std::env::var("AIJ_DIALOG_ANSWER").ok()
    }
}

#[cfg(not(debug_assertions))]
mod seams {
    pub fn quiet() -> bool {
        false
    }

    pub fn answer() -> Option<String> {
        None
    }
}

/// Dialogs silenced (a debug build's test seam): they go to stderr
/// instead. `true` then.
#[must_use]
pub fn quiet(title: &str, text: &str) -> bool {
    let quiet = seams::quiet();
    if quiet {
        eprintln!("ai-juggler: {title}\n{text}");
    }
    quiet
}

/// Shows the alert and returns which of `buttons` was pressed (its index;
/// the first is the default). A debug build's test seams can answer it
/// instead (see `seams`).
#[must_use]
pub fn show(level: Level, title: &str, text: &str, buttons: &[&str]) -> usize {
    let last = buttons.len().saturating_sub(1);
    if let Some(label) = seams::answer() {
        eprintln!("ai-juggler: {title}\n{text}\n(answered: {label})");
        return buttons
            .iter()
            .position(|button| *button == label)
            .unwrap_or(last);
    }
    if quiet(title, text) {
        return last;
    }
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        return native(mtm, level, title, text, buttons).min(last);
    }
    portable(level, title, text, buttons).min(last)
}

/// An `NSAlert` in this process (macOS, on the main thread).
#[cfg(target_os = "macos")]
fn native(
    mtm: objc2::MainThreadMarker,
    level: Level,
    title: &str,
    text: &str,
    buttons: &[&str],
) -> usize {
    use objc2_app_kit::{
        NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication,
        NSApplicationActivationPolicy,
    };
    use objc2_foundation::NSString;
    let app = NSApplication::sharedApplication(mtm);
    // Before any window: a regular app, in front, so the alert is seen.
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    // A process that never ran the app's loop hasn't finished launching:
    // without this its windows can draw blank (QA batch 9, a white alert).
    app.finishLaunching();
    #[allow(deprecated)] // `activate` needs macOS 14; this works on every release.
    app.activateIgnoringOtherApps(true);
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(match level {
        Level::Error => NSAlertStyle::Critical,
        Level::Warning => NSAlertStyle::Warning,
    });
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(text));
    for button in buttons {
        let _ = alert.addButtonWithTitle(&NSString::from_str(button));
    }
    let pressed = alert.runModal() - NSAlertFirstButtonReturn;
    usize::try_from(pressed).unwrap_or(usize::MAX)
}

/// rfd's dialog (Windows and Linux, where it is the process's own; and
/// macOS off the main thread, which the launch never is).
fn portable(level: Level, title: &str, text: &str, buttons: &[&str]) -> usize {
    let dialog = rfd::MessageDialog::new()
        .set_level(match level {
            Level::Error => rfd::MessageLevel::Error,
            Level::Warning => rfd::MessageLevel::Warning,
        })
        .set_title(title)
        .set_description(text);
    match buttons {
        [only] => {
            let _ = dialog
                .set_buttons(rfd::MessageButtons::OkCustom((*only).to_owned()))
                .show();
            0
        }
        [first, second, ..] => {
            match dialog
                .set_buttons(rfd::MessageButtons::OkCancelCustom(
                    (*first).to_owned(),
                    (*second).to_owned(),
                ))
                .show()
            {
                rfd::MessageDialogResult::Custom(label) if label == *first => 0,
                _ => 1,
            }
        }
        [] => 0,
    }
}

/// The launch that only shows an alert (see [`in_fresh_process`]): its
/// title and text, a newline apart.
const ALERT_ENV: &str = "AIJ_LAUNCH_ALERT";

/// Shows a "can't start" alert from a launch that has already built and
/// dropped a Tauri app (the hand-over to an older AI Juggler): tao's run
/// loop observers stay behind and abort an `NSAlert`'s modal loop there.
/// So the alert shows in a fresh launch of this app that does nothing
/// else, the app's own process, gone when it is; this one quits.
pub fn in_fresh_process(title: &str, text: &str) -> ! {
    if !quiet(title, text) {
        let _ = std::env::current_exe().and_then(|exe| {
            std::process::Command::new(exe)
                .env(ALERT_ENV, format!("{title}\n{text}"))
                .spawn()
        });
    }
    std::process::exit(1);
}

/// At the very start of a launch: the alert a launch asked for (see
/// [`in_fresh_process`]), then quit. Nothing else runs.
pub fn show_if_asked() {
    let Some(asked) = std::env::var_os(ALERT_ENV) else {
        return;
    };
    let asked = asked.to_string_lossy().into_owned();
    let (title, text) = asked.split_once('\n').unwrap_or((asked.as_str(), ""));
    let _ = show(Level::Error, title, text, &["Quit"]);
    std::process::exit(1);
}
