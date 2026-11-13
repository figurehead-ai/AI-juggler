//! AI Juggler's own window: bringing it forward, in one place (§4.17),
//! and its close, which hides it (D-112).
//!
//! A second launch, a click on the Dock icon, a line of the menu-bar item,
//! a click on a system notification, the quit sheet and the global go-to
//! hotkey all mean "show me AI Juggler": the window is shown (it may be
//! hidden), un-minimised and focused, which also makes the app active.
//! Tauri's window calls are the platform seam, so this file has nothing
//! macOS-only in it; [`Raise`] and [`Hide`] let the order be tested with
//! no window system.
//!
//! D-112 (0.4.5, always-on): the close button and ⌘W (the Window menu's
//! Close Window, which arrives as a close request) hide the window.
//! Nothing quits, the quit sheet never shows, and every session keeps
//! running; ⌘Q quits (`quit.rs`). A window in full screen leaves it
//! first, and hides once it has (hidden in full screen, it would leave an
//! empty Space behind). The first close that hides it is told once, the
//! next time the window is on screen ([`CloseNote`]).

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri command signatures take State by value"
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use juggler_core::storage::Storage;
use tauri::Manager;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The main window's label.
pub const MAIN: &str = "main";

/// What raising a window takes (Tauri's `WebviewWindow`, or a test's
/// recorder).
pub trait Raise {
    /// Shows a hidden window.
    fn show(&self);
    /// Restores a minimised window.
    fn unminimize(&self);
    /// Focuses it, making the app active.
    fn focus(&self);
    /// On screen: shown and not minimised (a hidden app's windows aren't).
    fn on_screen(&self) -> bool;
    /// Runs `raise` with the window following to the Space and display
    /// the user is on (a door from another window: that window's); by
    /// default, just runs it.
    fn follow(&self, raise: &mut dyn FnMut()) {
        raise();
    }
}

impl<R: tauri::Runtime> Raise for tauri::WebviewWindow<R> {
    fn show(&self) {
        // Shown again: a close's hide still waiting on full screen to end
        // is called off.
        HIDE_ONCE_WINDOWED.store(false, Ordering::SeqCst);
        let _ = tauri::WebviewWindow::show(self);
    }
    fn unminimize(&self) {
        let _ = tauri::WebviewWindow::unminimize(self);
    }
    fn focus(&self) {
        let _ = self.set_focus();
    }
    fn on_screen(&self) -> bool {
        self.is_visible().unwrap_or(false) && !self.is_minimized().unwrap_or(true)
    }
}

/// Shows, restores and focuses `window`, in that order (a hidden window
/// can't take focus).
pub fn raise<W: Raise + ?Sized>(window: &W) {
    window.show();
    window.unminimize();
    window.focus();
}

/// The go-to hotkey (AJ-E414 Req.4, §4.17): pressed from another app,
/// AI Juggler comes forward first, then the palette opens in it — never
/// in a window left behind the app the user was in.
pub fn go_to_pressed<W: Raise + ?Sized>(window: Option<&W>, open_palette: impl FnOnce()) {
    if let Some(window) = window {
        raise(window);
    }
    open_palette();
}

/// A door from another window (the Floor's people, areas, rack, boards):
/// the main window goes to the page first, then comes forward, so the
/// page is what's seen whatever a window manager did to the window
/// meanwhile (AJ-E3514: with a stage manager the page opened behind).
///
/// It comes forward where the door was clicked: on that window's Space and
/// display ([`Raise::follow`]; 0.4.1, FL1: from the Floor, the page opened
/// in a main window left on another Space, never seen).
pub fn open_door<W: Raise + ?Sized>(window: Option<&W>, navigate: impl FnOnce()) {
    navigate();
    if let Some(window) = window {
        window.follow(&mut || raise(window));
    }
}

/// The main window, brought forward beside another of the app's windows
/// (the Floor's): on its Space and display.
pub struct Beside<'a, R: tauri::Runtime> {
    /// The main window.
    pub main: &'a tauri::WebviewWindow<R>,
    /// The window the door was in.
    pub beside: &'a tauri::Window<R>,
}

impl<R: tauri::Runtime> Raise for Beside<'_, R> {
    fn show(&self) {
        Raise::show(self.main);
    }
    fn unminimize(&self) {
        Raise::unminimize(self.main);
    }
    fn focus(&self) {
        Raise::focus(self.main);
    }
    fn on_screen(&self) -> bool {
        Raise::on_screen(self.main)
    }
    fn follow(&self, raise: &mut dyn FnMut()) {
        if let Err(e) = juggler_window_events::front_beside(self.main, self.beside, raise) {
            tracing::info!("the main window forward beside another: {e}");
        }
    }
}

/// Brings the main window forward, if there is one: the one way back to
/// it from a second launch, the Dock icon and the menu-bar item (D-112),
/// a notification and the quit sheet. Call on the main thread.
pub fn bring_forward<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    // A test's launch stays hidden (src/test_launch.rs).
    if crate::test_launch::headless() {
        return;
    }
    if let Some(window) = app.get_webview_window(MAIN) {
        raise(&window);
    }
}

/// The main window is on screen: shown and not minimised.
#[must_use]
pub fn on_screen<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    app.get_webview_window(MAIN)
        .is_some_and(|window| Raise::on_screen(&window))
}

/// A click on the Dock icon (macOS's reopen; D-112): a main window that
/// isn't on screen (a close hid it, or it's minimised) comes forward
/// through `forward`; one on screen is left as it is (the click already
/// made the app active, and the Floor may be the window in use). True
/// when it comes forward.
pub fn reopen<W: Raise + ?Sized>(window: Option<&W>, forward: impl FnOnce()) -> bool {
    let hidden = window.is_some_and(|window| !window.on_screen());
    if hidden {
        forward();
    }
    hidden
}

/// [`reopen`] in the running app, through [`bring_forward`]. Call on the
/// main thread.
pub fn reopened<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let window = app.get_webview_window(MAIN);
    reopen(window.as_ref(), || bring_forward(app));
}

/// What a close that hides takes of a window (Tauri's, or a test's
/// recorder).
pub trait Hide {
    /// It fills its own full-screen Space.
    fn full_screen(&self) -> bool;
    /// Leaves full screen (a resize says when it has).
    fn leave_full_screen(&self);
    /// Hides it.
    fn hide(&self);
}

impl<R: tauri::Runtime> Hide for tauri::Window<R> {
    fn full_screen(&self) -> bool {
        self.is_fullscreen().unwrap_or(false)
    }
    fn leave_full_screen(&self) {
        let _ = self.set_fullscreen(false);
    }
    fn hide(&self) {
        let _ = tauri::Window::hide(self);
    }
}

/// A close asked while the main window was full screen: it hides once the
/// window has left full screen. Showing the window calls it off.
static HIDE_ONCE_WINDOWED: AtomicBool = AtomicBool::new(false);

/// A close hides `window` (D-112): at once, or, in full screen, once it
/// has left full screen (`waiting` holds that until [`resized`]).
pub fn hide_on_close<W: Hide + ?Sized>(window: &W, waiting: &AtomicBool) {
    if window.full_screen() {
        waiting.store(true, Ordering::SeqCst);
        window.leave_full_screen();
    } else {
        window.hide();
    }
}

/// The window was resized: a close waiting on full screen to end hides it
/// now that it has.
pub fn resized<W: Hide + ?Sized>(window: &W, waiting: &AtomicBool) {
    if waiting.load(Ordering::SeqCst) && !window.full_screen() {
        waiting.store(false, Ordering::SeqCst);
        window.hide();
    }
}

/// How long a close waits for full screen to end before giving up (the
/// exit animation takes under a second).
const LEAVE_FULL_SCREEN: Duration = Duration::from_secs(3);

/// A close from full screen hides the window once it is out of it. The
/// last resize of the way out can come while the window still says it is
/// full screen, and none comes after (the by-hand check of D-112: the
/// window stayed), so the same check [`resized`] makes runs every 100 ms
/// for up to [`LEAVE_FULL_SCREEN`], stopping once it has hidden the window
/// or the window was shown again.
fn hide_once_out_of_full_screen<R: tauri::Runtime>(window: tauri::Window<R>) {
    std::thread::spawn(move || {
        let step = Duration::from_millis(100);
        let mut waited = Duration::ZERO;
        while HIDE_ONCE_WINDOWED.load(Ordering::SeqCst) && waited < LEAVE_FULL_SCREEN {
            std::thread::sleep(step);
            waited += step;
            resized(&window, &HIDE_ONCE_WINDOWED);
        }
    });
}

/// The main window's own events (the Floor's window has its own, in
/// `features::floor::window`).
pub fn on_window_event<R: tauri::Runtime>(window: &tauri::Window<R>, event: &tauri::WindowEvent) {
    match event {
        // D-112: the close button and ⌘W hide the window; on the way out
        // (quitting) the close goes through.
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if crate::quit::close_hides() {
                api.prevent_close();
                hide_on_close(window, &HIDE_ONCE_WINDOWED);
                if HIDE_ONCE_WINDOWED.load(Ordering::SeqCst) {
                    hide_once_out_of_full_screen(window.clone());
                }
                CLOSE_NOTE.closed();
                tracing::info!("the main window's close: hidden, nothing quits (D-112)");
            }
        }
        tauri::WindowEvent::Resized(_) => resized(window, &HIDE_ONCE_WINDOWED),
        // AJ-E1510: no system notification while AI Juggler is in front.
        tauri::WindowEvent::Focused(focused) => window
            .state::<Arc<AppState>>()
            .notifier
            .set_in_front(*focused),
        _ => {}
    }
}

/// The app setting that records, once per install, that the close note
/// was shown (`1`).
pub const CLOSE_NOTE_TOLD_KEY: &str = "close_note_told";

/// D-112: the first close that hides the window says so once, as a toast,
/// the next time the window is on screen (a hidden window can't show
/// one): AI Juggler kept running, it stays in the menu bar, and ⌘Q quits.
/// Due from a close that hid the window in this run until the window
/// says it; once said, recorded so it never shows again on this install.
#[derive(Debug, Default)]
pub struct CloseNote {
    /// A close hid the window in this run (never read from storage: the
    /// close is handled on the main thread).
    closed: AtomicBool,
}

impl CloseNote {
    /// A close hid the window.
    pub fn closed(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// The window, on screen again, says it now.
    #[must_use]
    pub fn due(&self, storage: &Storage) -> bool {
        self.closed.load(Ordering::SeqCst) && !Self::was_told(storage)
    }

    /// The window said it: never again on this install.
    ///
    /// # Errors
    ///
    /// The storage's error when the mark can't be written.
    pub fn told(&self, storage: &Storage) -> AppResult<()> {
        storage
            .app_settings()
            .set(CLOSE_NOTE_TOLD_KEY, "1")
            .map_err(AppError::from)?;
        self.closed.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn was_told(storage: &Storage) -> bool {
        storage
            .app_settings()
            .get(CLOSE_NOTE_TOLD_KEY)
            .ok()
            .flatten()
            .is_some()
    }
}

/// The app's one close note.
static CLOSE_NOTE: CloseNote = CloseNote {
    closed: AtomicBool::new(false),
};

/// Whether the window, now on screen, says that a close hid it and AI
/// Juggler kept running (D-112).
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when storage can't be read.
#[tauri::command]
pub async fn app_close_note_due(state: tauri::State<'_, Arc<AppState>>) -> AppResult<bool> {
    crate::ipc::blocking(state.inner(), "close note", |state| {
        Ok(CLOSE_NOTE.due(&*crate::commands::lock_storage_arc(state)?))
    })
    .await
}

/// The window said it: it never shows again on this install.
///
/// # Errors
///
/// The storage's error when the mark can't be written.
#[tauri::command]
pub async fn app_close_note_told(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "close note told", |state| {
        CLOSE_NOTE.told(&*crate::commands::lock_storage_arc(state)?)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// A window's calls, in order; then whether it's on screen and in full
    /// screen.
    #[derive(Default)]
    struct Recorder(RefCell<Vec<&'static str>>, Cell<bool>, Cell<bool>);

    impl Raise for Recorder {
        fn show(&self) {
            self.0.borrow_mut().push("show");
            self.1.set(true);
        }
        fn unminimize(&self) {
            self.0.borrow_mut().push("unminimize");
        }
        fn focus(&self) {
            self.0.borrow_mut().push("focus");
        }
        fn on_screen(&self) -> bool {
            self.1.get()
        }
        fn follow(&self, raise: &mut dyn FnMut()) {
            self.0.borrow_mut().push("follow");
            raise();
            self.0.borrow_mut().push("settled");
        }
    }

    #[test]
    fn the_hotkey_brings_the_window_forward_before_the_palette_opens() {
        let window = Recorder::default();
        go_to_pressed(Some(&window), || window.0.borrow_mut().push("palette"));
        assert_eq!(
            *window.0.borrow(),
            ["show", "unminimize", "focus", "palette"],
            "the window comes forward, then the palette opens"
        );
    }

    #[test]
    fn a_door_navigates_then_brings_the_window_forward() {
        let window = Recorder::default();
        open_door(Some(&window), || window.0.borrow_mut().push("navigate"));
        // FL1 (rc31): forward on the Space and display of the window the
        // door was in, never left on another one
        assert_eq!(
            *window.0.borrow(),
            [
                "navigate",
                "follow",
                "show",
                "unminimize",
                "focus",
                "settled"
            ],
            "the page first, then the window forward where the door was"
        );
    }

    #[test]
    fn with_no_window_the_palette_event_still_goes_out() {
        let opened = RefCell::new(false);
        go_to_pressed(None::<&Recorder>, || *opened.borrow_mut() = true);
        assert!(*opened.borrow(), "the palette event is sent");
    }

    impl Hide for Recorder {
        fn full_screen(&self) -> bool {
            self.2.get()
        }
        fn leave_full_screen(&self) {
            self.0.borrow_mut().push("leave full screen");
        }
        fn hide(&self) {
            self.0.borrow_mut().push("hide");
            self.1.set(false);
        }
    }

    fn shown() -> Recorder {
        let window = Recorder::default();
        window.1.set(true);
        window
    }

    /// D-112: a close hides the window (`quit.rs` decides it never
    /// quits), and a click on the Dock icon brings it back the way every
    /// door does: shown, restored, focused.
    #[test]
    fn a_close_hides_the_window_and_the_dock_icon_brings_it_back() {
        let window = shown();
        let waiting = AtomicBool::new(false);
        hide_on_close(&window, &waiting);
        assert_eq!(*window.0.borrow(), ["hide"]);
        assert!(!window.on_screen());
        assert!(reopen(Some(&window), || raise(&window)), "it comes back");
        assert_eq!(
            *window.0.borrow(),
            ["hide", "show", "unminimize", "focus"],
            "shown first (a hidden window can't take focus)"
        );
        assert!(window.on_screen());
    }

    /// A click on the Dock icon with the window already on screen leaves
    /// it as it is (the Floor may be the window in use).
    #[test]
    fn the_dock_icon_leaves_a_window_on_screen_as_it_is() {
        let window = shown();
        assert!(!reopen(Some(&window), || raise(&window)));
        assert!(window.0.borrow().is_empty());
        assert!(!reopen(None::<&Recorder>, || unreachable!("no window")));
    }

    /// In full screen a close leaves full screen first and hides once the
    /// window is out of it (hidden in full screen, it would leave an empty
    /// Space); a resize still in full screen doesn't hide it.
    #[test]
    fn a_close_in_full_screen_hides_once_out_of_it() {
        let window = shown();
        window.2.set(true);
        let waiting = AtomicBool::new(false);
        hide_on_close(&window, &waiting);
        assert_eq!(*window.0.borrow(), ["leave full screen"]);
        resized(&window, &waiting);
        assert_eq!(
            *window.0.borrow(),
            ["leave full screen"],
            "still full screen"
        );
        window.2.set(false);
        resized(&window, &waiting);
        assert_eq!(*window.0.borrow(), ["leave full screen", "hide"]);
        resized(&window, &waiting);
        assert_eq!(
            *window.0.borrow(),
            ["leave full screen", "hide"],
            "one hide; a later resize leaves it alone"
        );
    }

    /// A resize with no close waiting never hides the window.
    #[test]
    fn a_resize_alone_never_hides() {
        let window = shown();
        resized(&window, &AtomicBool::new(false));
        assert!(window.0.borrow().is_empty());
    }

    /// D-112: the toast is due after the first close that hid the window,
    /// until the window says it; once said, it is recorded, and no later
    /// close (in this run or the next) makes it due again.
    #[test]
    fn the_close_note_shows_once_after_the_first_close_and_never_again() {
        let storage = Storage::open_in_memory().expect("storage");
        let note = CloseNote::default();
        assert!(!note.due(&storage), "no close yet");
        note.closed();
        assert!(note.due(&storage), "due the next time the window shows");
        assert!(note.due(&storage), "still due until it is said");
        note.told(&storage).expect("told");
        assert!(!note.due(&storage), "said once");
        note.closed();
        assert!(!note.due(&storage), "a later close says nothing");
        let next_run = CloseNote::default();
        next_run.closed();
        assert!(!next_run.due(&storage), "nor does one in a later run");
        assert_eq!(
            storage
                .app_settings()
                .get(CLOSE_NOTE_TOLD_KEY)
                .expect("read"),
            Some("1".to_owned())
        );
    }

    /// A close in one run that was never followed by the window showing
    /// (quit from the Dock) doesn't tell at the next start: the words
    /// say it kept running, which only holds in the same run.
    #[test]
    fn the_close_note_is_due_only_in_the_run_its_close_hid_the_window() {
        let storage = Storage::open_in_memory().expect("storage");
        CloseNote::default().closed();
        assert!(!CloseNote::default().due(&storage));
    }
}
