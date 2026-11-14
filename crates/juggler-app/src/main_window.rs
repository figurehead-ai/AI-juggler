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
//! first, and hides once the exit is over (hidden in full screen, it would
//! leave an empty Space behind; [`HideAfterExit`]). The first close that
//! hides it is told once, the next time the window is on screen
//! ([`CloseNote`]).

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri command signatures take State by value"
)]

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
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
        if HIDE_AFTER_EXIT.call_off() {
            tracing::info!("a close from full screen: called off, the window was shown again");
        }
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
    /// Leaves full screen (the exit event says when it has).
    fn leave_full_screen(&self);
    /// Hides it.
    fn hide(&self);
    /// It is on screen.
    fn visible(&self) -> bool;
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
    fn visible(&self) -> bool {
        self.is_visible().unwrap_or(false)
    }
}

/// What a close from full screen waits on before it hides the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The window's own exit event, sent once the exit animation is over
    /// (`juggler_window_events::watch_full_screen_exit`).
    Event,
    /// Looks every 200 ms ([`HideAfterExit::settle`]): the event can't be
    /// watched, or didn't come within 5 s.
    Fallback,
}

/// No close waiting.
const IDLE: u8 = 0;
/// A close waits on the exit event.
const ON_EVENT: u8 = 1;
/// A close's fallback is at it.
const ON_FALLBACK: u8 = 2;

/// A close from full screen, until the window is hidden (D-112). tao says
/// a window has left full screen as soon as it is asked to, so a hide then
/// lands mid-way out and the exit's end puts the window back: the hide
/// waits for the exit's own end, the window's exit event, and hides once.
/// Where that event can't be watched, or doesn't come in time, the
/// fallback looks instead, hiding the window whenever it is out of full
/// screen and on screen, until it stays hidden. Showing the window calls
/// either off.
#[derive(Debug, Default)]
pub struct HideAfterExit(AtomicU8);

impl HideAfterExit {
    /// Nothing waiting.
    #[must_use]
    pub const fn new() -> Self {
        Self(AtomicU8::new(IDLE))
    }

    /// A close hides `window`: at once, or, in full screen, once it has
    /// left it, on its exit event when that is `watched`, else by the
    /// fallback. What it waits on; `None` when it hid at once.
    #[must_use]
    pub fn close<W: Hide + ?Sized>(&self, window: &W, watched: bool) -> Option<Exit> {
        if !window.full_screen() {
            window.hide();
            return None;
        }
        let (exit, state) = if watched {
            (Exit::Event, ON_EVENT)
        } else {
            (Exit::Fallback, ON_FALLBACK)
        };
        self.0.store(state, Ordering::SeqCst);
        window.leave_full_screen();
        Some(exit)
    }

    /// The window's exit event: a close waiting on it hides the window,
    /// once. True when it did.
    #[must_use]
    pub fn exited<W: Hide + ?Sized>(&self, window: &W) -> bool {
        let waited = self
            .0
            .compare_exchange(ON_EVENT, IDLE, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        if waited {
            window.hide();
        }
        waited
    }

    /// The window was shown again: a close still waiting is called off.
    /// True when one was.
    #[must_use]
    pub fn call_off(&self) -> bool {
        self.0.swap(IDLE, Ordering::SeqCst) != IDLE
    }

    /// A close waits on the exit event still.
    #[must_use]
    pub fn on_event(&self) -> bool {
        self.0.load(Ordering::SeqCst) == ON_EVENT
    }

    /// The exit event didn't come in time: the fallback takes over. True
    /// when it does (a close still waited on the event).
    #[must_use]
    pub fn fall_back(&self) -> bool {
        self.0
            .compare_exchange(ON_EVENT, ON_FALLBACK, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// The window was resized: in the fallback, it is hidden if it is out
    /// of full screen and still on screen (the window says it has left
    /// full screen as soon as it is asked to, so a hide can land mid-way
    /// out and the exit's end puts the window back; the exit's own last
    /// resize then hides it again). Waiting on the event, a resize does
    /// nothing: the event hides it once the exit is over.
    pub fn resized<W: Hide + ?Sized>(&self, window: &W) {
        if self.0.load(Ordering::SeqCst) == ON_FALLBACK && !window.full_screen() && window.visible()
        {
            window.hide();
        }
    }

    /// One look of the fallback; true when it is over: called off (the
    /// window was shown again), or the window has stayed hidden for six
    /// looks in a row (`hidden` counts them). Out of full screen and on
    /// screen, it hides the window again.
    #[must_use]
    pub fn settle<W: Hide + ?Sized>(&self, window: &W, hidden: &mut u8) -> bool {
        if self.0.load(Ordering::SeqCst) != ON_FALLBACK {
            return true;
        }
        if window.full_screen() || window.visible() {
            *hidden = 0;
            self.resized(window);
            return false;
        }
        *hidden += 1;
        if *hidden >= HIDDEN_LOOKS {
            self.give_up();
            return true;
        }
        false
    }

    /// The fallback is over, done or not.
    pub fn give_up(&self) {
        let _ = self
            .0
            .compare_exchange(ON_FALLBACK, IDLE, Ordering::SeqCst, Ordering::SeqCst);
    }
}

/// Waits for the exit event a look at a time (`sleep` between looks), up
/// to `looks` looks. True when the fallback takes over: the event didn't
/// come, and nothing showed the window meanwhile.
#[must_use]
pub fn event_or_fallback(pending: &HideAfterExit, looks: u32, mut sleep: impl FnMut()) -> bool {
    for _ in 0..looks {
        sleep();
        if !pending.on_event() {
            return false;
        }
    }
    pending.fall_back()
}

/// The main window's close from full screen.
static HIDE_AFTER_EXIT: HideAfterExit = HideAfterExit::new();

/// Whether the main window's exit event is watched: tried once, at the
/// first close from full screen (most closes never need it).
static EXIT_WATCHED: OnceLock<bool> = OnceLock::new();

/// Watches the main window's exit event, once (main thread only); false
/// when it can't be, said in the log: closes from full screen then hide by
/// the fallback.
fn watch_exit<R: tauri::Runtime>(window: &tauri::Window<R>) -> bool {
    *EXIT_WATCHED.get_or_init(|| {
        let hide = window.clone();
        let watched = juggler_window_events::watch_full_screen_exit(window, move || {
            if HIDE_AFTER_EXIT.exited(&hide) {
                tracing::info!("a close from full screen: hidden on the exit event");
            }
        });
        if let Err(e) = &watched {
            tracing::warn!(
                "a close from full screen: the exit event can't be watched ({e}); the fallback hides the window"
            );
        }
        watched.is_ok()
    })
}

/// A look, waiting on full screen to end.
const LOOK: Duration = Duration::from_millis(200);

/// How long a close from full screen waits for the exit event before the
/// fallback takes over (the exit animation takes under a second).
const EXIT_EVENT_WAIT: Duration = Duration::from_secs(5);

/// Looks in a row the window must stay hidden for the fallback to be done
/// (at 200 ms a look: longer than the exit animation, which can put a
/// window hidden mid-way back on screen).
const HIDDEN_LOOKS: u8 = 6;

/// How long the fallback keeps at it.
const LEAVE_FULL_SCREEN: Duration = Duration::from_secs(5);

/// Looks in `wait`.
fn looks(wait: Duration) -> u32 {
    u32::try_from(wait.as_millis() / LOOK.as_millis()).unwrap_or(u32::MAX)
}

/// A close from full screen, after the window was asked to leave it:
/// waiting on the exit event, nothing happens here unless the event hasn't
/// come within [`EXIT_EVENT_WAIT`]; then, or at once when the event can't
/// be watched, the fallback: [`HideAfterExit::settle`] every 200 ms for up
/// to [`LEAVE_FULL_SCREEN`] (the by-hand check of D-112: a hide that landed
/// during the exit animation was undone by its end).
fn hide_after_exit<R: tauri::Runtime>(window: tauri::Window<R>, exit: Exit) {
    std::thread::spawn(move || {
        if exit == Exit::Event {
            if !event_or_fallback(&HIDE_AFTER_EXIT, looks(EXIT_EVENT_WAIT), || {
                std::thread::sleep(LOOK);
            }) {
                return;
            }
            tracing::warn!(
                "a close from full screen: no exit event within 5 s; the fallback hides the window"
            );
        }
        let mut waited = Duration::ZERO;
        let mut hidden = 0;
        while waited < LEAVE_FULL_SCREEN {
            std::thread::sleep(LOOK);
            waited += LOOK;
            let (full, shown) = (window.full_screen(), window.visible());
            if HIDE_AFTER_EXIT.settle(&window, &mut hidden) {
                tracing::info!(
                    ms = waited.as_millis(),
                    "a close from full screen: done by the fallback (the window stayed hidden, or was shown again)"
                );
                return;
            }
            tracing::debug!(
                ms = waited.as_millis(),
                full,
                shown,
                hidden,
                "a close from full screen: the fallback is still at it"
            );
        }
        HIDE_AFTER_EXIT.give_up();
        tracing::warn!(
            "a close from full screen: the fallback gave up after 5 s; the window may still be on screen"
        );
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
                // Watched from the first close in full screen on (most
                // closes never need it).
                let watched = Hide::full_screen(window) && watch_exit(window);
                if let Some(exit) = HIDE_AFTER_EXIT.close(window, watched) {
                    tracing::info!(
                        "the main window's close in full screen: leaving it first; it hides {}",
                        match exit {
                            Exit::Event => "on the exit event",
                            Exit::Fallback => "by the fallback",
                        }
                    );
                    hide_after_exit(window.clone(), exit);
                }
                CLOSE_NOTE.closed();
                tracing::info!("the main window's close: hidden, nothing quits (D-112)");
            }
        }
        tauri::WindowEvent::Resized(_) => HIDE_AFTER_EXIT.resized(window),
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
        fn visible(&self) -> bool {
            self.1.get()
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
        assert_eq!(HideAfterExit::new().close(&window, true), None, "at once");
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

    /// A window in full screen, closed: the hide waits on `watched`'s
    /// path, and the window says it is out of full screen at once, as tao
    /// does mid-exit.
    fn closed_in_full_screen(watched: bool) -> (Recorder, HideAfterExit, Option<Exit>) {
        let window = shown();
        window.2.set(true);
        let pending = HideAfterExit::new();
        let exit = pending.close(&window, watched);
        assert_eq!(*window.0.borrow(), ["leave full screen"]);
        window.2.set(false);
        (window, pending, exit)
    }

    /// The by-hand check of D-112: a hide that lands mid-exit is undone by
    /// the exit's end. With the exit event watched, nothing hides the
    /// window until the event, which hides it once; the fallback's wait
    /// then ends without a look, and nothing hides it again.
    #[test]
    fn the_exit_event_hides_the_window_once_and_the_fallback_does_not() {
        let (window, pending, exit) = closed_in_full_screen(true);
        assert_eq!(exit, Some(Exit::Event));
        pending.resized(&window);
        assert_eq!(
            *window.0.borrow(),
            ["leave full screen"],
            "mid-exit: left on screen"
        );
        let mut look = 0;
        let fallback = event_or_fallback(&pending, looks(EXIT_EVENT_WAIT), || {
            look += 1;
            if look == 3 {
                assert!(pending.exited(&window), "the exit's end");
            }
        });
        assert!(!fallback, "the event came: no fallback");
        assert_eq!(look, 3, "the wait ends at the event");
        assert_eq!(*window.0.borrow(), ["leave full screen", "hide"]);
        window.1.set(true); // whatever puts it back later is not this close's
        pending.resized(&window);
        assert!(pending.settle(&window, &mut 0), "nothing left to do");
        assert!(!pending.exited(&window), "a second event hides nothing");
        assert_eq!(
            *window.0.borrow(),
            ["leave full screen", "hide"],
            "hidden once"
        );
    }

    /// No exit event within the wait: the fallback takes over and hides
    /// the window; an event that comes after that does nothing.
    #[test]
    fn with_no_exit_event_in_time_the_fallback_hides_the_window() {
        let (window, pending, _) = closed_in_full_screen(true);
        let mut waited = 0;
        assert!(
            event_or_fallback(&pending, looks(EXIT_EVENT_WAIT), || waited += 1),
            "the fallback takes over"
        );
        assert_eq!(waited, 25, "5 s at 200 ms a look");
        assert!(!pending.settle(&window, &mut 0));
        assert_eq!(*window.0.borrow(), ["leave full screen", "hide"]);
        assert!(
            !pending.exited(&window),
            "a late event: the fallback has it"
        );
        assert_eq!(*window.0.borrow(), ["leave full screen", "hide"]);
    }

    /// The window shown again before the exit event: the close is called
    /// off; the event hides nothing, and no fallback follows.
    #[test]
    fn a_show_before_the_exit_event_calls_the_close_off() {
        let (window, pending, _) = closed_in_full_screen(true);
        assert!(pending.call_off(), "a close was waiting");
        assert!(!pending.exited(&window));
        assert!(!event_or_fallback(&pending, looks(EXIT_EVENT_WAIT), || ()));
        assert!(pending.settle(&window, &mut 0), "over");
        assert_eq!(*window.0.borrow(), ["leave full screen"], "never hidden");
        assert!(!pending.call_off(), "nothing left to call off");
    }

    /// The exit event can't be watched: the fallback alone hides the
    /// window, on a resize once it is out of full screen (never while in
    /// it), and keeps it hidden: a hide undone by the exit's end is done
    /// again; it is over once the window has stayed hidden for six looks
    /// in a row, and a show calls it off.
    #[test]
    fn unwatched_the_fallback_hides_the_window_and_keeps_it_hidden() {
        let window = shown();
        window.2.set(true);
        let pending = HideAfterExit::new();
        assert_eq!(pending.close(&window, false), Some(Exit::Fallback));
        let mut hidden = 0;
        pending.resized(&window);
        assert!(!pending.settle(&window, &mut hidden), "still full screen");
        assert_eq!(*window.0.borrow(), ["leave full screen"]);
        window.2.set(false);
        pending.resized(&window);
        assert_eq!(*window.0.borrow(), ["leave full screen", "hide"]);
        window.1.set(true); // the animation's end put it back
        assert!(!pending.settle(&window, &mut hidden));
        assert_eq!(
            *window.0.borrow(),
            ["leave full screen", "hide", "hide"],
            "hidden again"
        );
        for look in 1..HIDDEN_LOOKS {
            assert!(!pending.settle(&window, &mut hidden), "{look} looks hidden");
        }
        assert!(pending.settle(&window, &mut hidden), "stayed hidden: done");
        window.1.set(true);
        pending.resized(&window);
        assert_eq!(window.0.borrow().len(), 3, "done: a resize leaves it alone");

        let (window, pending, _) = closed_in_full_screen(false);
        assert!(pending.call_off(), "shown again");
        assert!(pending.settle(&window, &mut 0), "called off");
        assert_eq!(*window.0.borrow(), ["leave full screen"]);
    }

    /// A resize with no close waiting never hides the window.
    #[test]
    fn a_resize_alone_never_hides() {
        let window = shown();
        HideAfterExit::new().resized(&window);
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
