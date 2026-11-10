//! Bringing AI Juggler's own window forward, in one place (§4.17).
//!
//! A second launch, a click on a system notification, the quit sheet and
//! the global go-to hotkey all mean "show me AI Juggler": the window is
//! shown (it may be hidden), un-minimised and focused, which also makes
//! the app active. Tauri's window calls are the platform seam, so this
//! file has nothing macOS-only in it; [`Raise`] lets the order be tested
//! with no window system.

use tauri::Manager;

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
}

impl<R: tauri::Runtime> Raise for tauri::WebviewWindow<R> {
    fn show(&self) {
        let _ = tauri::WebviewWindow::show(self);
    }
    fn unminimize(&self) {
        let _ = tauri::WebviewWindow::unminimize(self);
    }
    fn focus(&self) {
        let _ = self.set_focus();
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
pub fn open_door<W: Raise + ?Sized>(window: Option<&W>, navigate: impl FnOnce()) {
    navigate();
    if let Some(window) = window {
        raise(window);
    }
}

/// Brings the main window forward, if there is one. Call on the main
/// thread.
pub fn bring_forward<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    // A test's launch stays hidden (src/test_launch.rs).
    if crate::test_launch::headless() {
        return;
    }
    if let Some(window) = app.get_webview_window(MAIN) {
        raise(&window);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct Recorder(RefCell<Vec<&'static str>>);

    impl Raise for Recorder {
        fn show(&self) {
            self.0.borrow_mut().push("show");
        }
        fn unminimize(&self) {
            self.0.borrow_mut().push("unminimize");
        }
        fn focus(&self) {
            self.0.borrow_mut().push("focus");
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
        assert_eq!(
            *window.0.borrow(),
            ["navigate", "show", "unminimize", "focus"],
            "the page first, then the window forward"
        );
    }

    #[test]
    fn with_no_window_the_palette_event_still_goes_out() {
        let opened = RefCell::new(false);
        go_to_pressed(None::<&Recorder>, || *opened.borrow_mut() = true);
        assert!(*opened.borrow(), "the palette event is sent");
    }
}
