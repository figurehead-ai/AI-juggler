//! Mouse, trackpad, keys and visibility for the Floor's window (ADR-0026;
//! AJ-E3501). Tauri hands a window without a webview no input events, so
//! this listens to the system directly, for that one window only.
//!
//! THE AUDIT: on macOS, `AppKit`'s local event monitor is the way to see a
//! window's events without owning its view. Its Rust bindings mark four
//! things `unsafe`, each for a reason the caller can uphold:
//!
//! 1. the `NSView` behind a window's raw handle (a pointer the window
//!    owns, valid while the window is: read on the main thread only, and
//!    retained at once);
//! 2. adding the monitor (its block must return a valid event or null:
//!    it returns the event it was handed, unchanged);
//! 3. reading the event the block is handed (`AppKit` passes a live event
//!    for the block's duration);
//! 4. removing the monitor (it must be the object adding returned: it is,
//!    kept in this crate's registry).
//!
//! What this crate is NOT: a general unsafe escape hatch. It exists so the
//! workspace law `unsafe_code = "forbid"` stays absolute everywhere else;
//! it doesn't inherit the workspace lint baseline (see `Cargo.toml`). Any
//! other `unsafe` anywhere needs a new recorded decision.
//!
//! Elsewhere (Windows until 0.6, Linux until 0.7) every call says
//! [`Error::Unsupported`], with why.
//!
//! Must not know: the Floor's scene, storage or anything but input and
//! visibility events: it is an audited shim.
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
#![cfg_attr(target_os = "macos", allow(unsafe_code))] // ADR-0026: the four audited calls

use raw_window_handle::HasWindowHandle;

/// A key the Floor answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A character, lower case.
    Char(char),
    /// ←.
    Left,
    /// →.
    Right,
    /// ↑.
    Up,
    /// ↓.
    Down,
    /// Tab.
    Tab,
    /// Shift-Tab.
    BackTab,
    /// Return.
    Enter,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Escape.
    Escape,
}

/// What happened in the window (points from its top left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A scroll: a trackpad's is `precise`; `zoom` while ⌘ is held.
    Scroll {
        /// Across.
        dx: f32,
        /// Down.
        dy: f32,
        /// From a trackpad.
        precise: bool,
        /// Where, across.
        x: f32,
        /// Where, down.
        y: f32,
        /// ⌘ held.
        zoom: bool,
    },
    /// A pinch.
    Magnify {
        /// How much.
        m: f32,
        /// Where, across.
        x: f32,
        /// Where, down.
        y: f32,
    },
    /// A two-finger twist, degrees.
    Twist(f32),
    /// A drag; `rotate` for a right-drag or an ⌥-drag.
    Drag {
        /// Across.
        dx: f32,
        /// Down.
        dy: f32,
        /// Turn instead of pan.
        rotate: bool,
    },
    /// The pointer moved.
    Move {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
    /// A click (a press and release without a drag).
    Click {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
    /// A key, without ⌘ (⌘ keys are the menu's).
    Key(Key),
}

/// Why the window's events can't be had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Not on this system yet: why, in plain words.
    Unsupported(&'static str),
    /// Called off the main thread.
    NotMainThread,
    /// The handle isn't a window this system draws.
    NoWindow,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(why) => write!(f, "not on this system yet: {why}"),
            Self::NotMainThread => f.write_str("the window's events are read on the main thread"),
            Self::NoWindow => f.write_str("not a window this system draws"),
        }
    }
}

impl std::error::Error for Error {}

/// A monitor's handle; [`remove`] ends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Monitor(u64);

/// Starts sending `window`'s events to `on` (main thread only).
///
/// # Errors
///
/// [`Error`] when this system isn't supported yet, off the main thread,
/// or for a handle that isn't a window.
pub fn install(
    window: &impl HasWindowHandle,
    on: impl Fn(Event) + 'static,
) -> Result<Monitor, Error> {
    imp::install(window, Box::new(on))
}

/// Ends a monitor (main thread only); unknown ones are ignored.
pub fn remove(monitor: Monitor) {
    imp::remove(monitor);
}

/// Whether `window` can be seen: on screen, not covered, not minimised
/// (main thread only).
///
/// # Errors
///
/// As [`install`].
pub fn visible(window: &impl HasWindowHandle) -> Result<bool, Error> {
    imp::visible(window)
}

/// Brings `window` (another of this app's windows) forward where the user
/// is, beside `beside`: on another display, it first moves to `beside`'s,
/// its size kept; not shown on the active Space (a window manager took it
/// off, or left it on another Space), it's ordered out, so `raise` (show,
/// restore, focus) orders it in afresh, on the active Space, as a window
/// that just appeared. Its collection behaviour is never touched.
///
/// 0.4.1, FL1: a Floor door opened the page in a main window kept off
/// screen. A tabbing window manager (the walk Mac's BetterStage) holds the
/// Floor and the main window as tabs of one pane and shows only the Floor;
/// raising the hidden tab, by the app, the Window menu or `AXRaise`, didn't
/// show it (and rc32's `MoveToActiveSpace` acts only when the app becomes
/// active, which it already was). A window ordered in afresh is what such
/// a manager shows, as it showed the Floor when it opened.
/// Main thread only; a safe call, not one of the audited four. `raise`
/// runs once, whatever happens.
///
/// # Errors
///
/// As [`install`]; `raise` has still run.
pub fn front_beside(
    window: &impl HasWindowHandle,
    beside: &impl HasWindowHandle,
    raise: impl FnOnce(),
) -> Result<(), Error> {
    imp::front_beside(window, beside, raise)
}

/// Whether the user asked the system for less motion (macOS: Reduce
/// motion in Accessibility › Display). Any thread; a safe call, not one of
/// the audited four. `false` where it can't be read.
#[must_use]
pub fn reduce_motion() -> bool {
    imp::reduce_motion()
}

/// Whether the Floor's window can read its events on this system.
///
/// # Errors
///
/// [`Error::Unsupported`], with why, where it can't yet.
pub const fn supported() -> Result<(), Error> {
    imp::SUPPORTED
}

/// What the system calls the keys that move the Floor's view.
#[must_use]
pub const fn hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "Drag: pan  ·  Right-drag or Option-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset"
    } else {
        "Drag: pan  ·  Right-drag or Alt-drag: turn and tilt  ·  Ctrl-scroll: zoom  ·  H: reset"
    }
}

/// `frame` (x, y, w, h) moved into `room`, centred, cut to fit it.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn centred(frame: [f64; 4], room: [f64; 4]) -> [f64; 4] {
    let (w, h) = (frame[2].min(room[2]), frame[3].min(room[3]));
    [
        (room[0] + (room[2] - w) / 2.0).round(),
        (room[1] + (room[3] - h) / 2.0).round(),
        w,
        h,
    ]
}

/// What [`front_beside`] asks of a window: AppKit's, or a test's.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
trait Placed {
    /// Ordered in, on the active Space, some of it seen (or minimised:
    /// restoring it brings it back where it was).
    fn shown_here(&self) -> bool;
    /// Its frame (x, y, w, h), in the system's screen coordinates.
    fn frame(&self) -> [f64; 4];
    /// Its display's usable frame, when it's on one.
    fn display(&self) -> Option<[f64; 4]>;
    fn set_frame(&self, frame: [f64; 4]);
    /// Takes it off screen, so the next raise orders it in afresh.
    fn order_out(&self);
}

/// [`front_beside`], over [`Placed`] windows.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn front(window: &impl Placed, beside: &impl Placed, raise: impl FnOnce()) {
    if let (Some(theirs), Some(ours)) = (beside.display(), window.display())
        && theirs != ours
    {
        window.set_frame(centred(window.frame(), theirs));
    }
    if !window.shown_here() {
        window.order_out();
    }
    raise();
}

/// How far (points) a press may wander and still be a click.
pub const CLICK_SLOP: f32 = 4.0;

/// Whether the pointer at `to` has left a press at `from` (points) far
/// enough to be a drag rather than a click.
#[must_use]
pub fn past_slop(from: (f32, f32), to: (f32, f32)) -> bool {
    (to.0 - from.0).hypot(to.1 - from.1) > CLICK_SLOP
}

#[cfg(target_os = "macos")]
mod imp {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{
        NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSView, NSWindow,
        NSWindowOcclusionState,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::{Error, Event, Key, Monitor};

    type Handler = RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent>;

    pub(super) const SUPPORTED: Result<(), Error> = Ok(());

    thread_local! {
        static MONITORS: RefCell<HashMap<u64, (Retained<AnyObject>, Handler)>> = RefCell::new(HashMap::new());
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }

    /// The view the Floor draws into (its surface's layer is this view's).
    fn view_of(window: &impl HasWindowHandle) -> Result<Retained<NSView>, Error> {
        MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let handle = window.window_handle().map_err(|_| Error::NoWindow)?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return Err(Error::NoWindow);
        };
        // SAFETY (1): `ns_view` is the window's own `NSView`, valid while
        // the window is; we're on the main thread (checked above), where
        // AppKit objects live, and retain it at once.
        unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }.ok_or(Error::NoWindow)
    }

    fn window_of(window: &impl HasWindowHandle) -> Result<Retained<NSWindow>, Error> {
        view_of(window)?.window().ok_or(Error::NoWindow)
    }

    /// One event of `window` (numbered `number`), as the Floor reads it.
    #[allow(clippy::cast_possible_truncation)] // points and deltas: small numbers
    fn read(
        e: &NSEvent,
        view: &NSView,
        number: isize,
        last: &Cell<(f32, f32)>,
        dragged: &Cell<bool>,
    ) -> Option<Event> {
        if e.windowNumber() != number {
            return None;
        }
        // In the drawing view's own points, from its top left: the frame is
        // drawn into this view, so a click lands where it was drawn (0.4.1:
        // the window's content-layout rect left clicks a title bar's height
        // below the floor tabs they were aimed at).
        let at = view.convertPoint_fromView(e.locationInWindow(), None);
        let y = if view.isFlipped() {
            at.y
        } else {
            view.bounds().size.height - at.y
        };
        let (x, y) = (at.x as f32, y as f32);
        let flags = e.modifierFlags();
        let t = e.r#type();
        Some(if t == NSEventType::ScrollWheel {
            Event::Scroll {
                dx: e.scrollingDeltaX() as f32,
                dy: e.scrollingDeltaY() as f32,
                precise: e.hasPreciseScrollingDeltas(),
                x,
                y,
                zoom: flags.contains(NSEventModifierFlags::Command),
            }
        } else if t == NSEventType::Magnify {
            Event::Magnify {
                m: e.magnification() as f32,
                x,
                y,
            }
        } else if t == NSEventType::Rotate {
            Event::Twist(e.rotation())
        } else if t == NSEventType::LeftMouseDown || t == NSEventType::RightMouseDown {
            last.set((x, y));
            dragged.set(false);
            return None;
        } else if t == NSEventType::LeftMouseDragged || t == NSEventType::RightMouseDragged {
            // A press wobbles a point or two on a trackpad; until the
            // pointer has moved past the slop it is still a click (0.4.1:
            // the floor tabs read every such press as a drag and ignored
            // it). Past it, the drag starts where the press did.
            if !dragged.get() && !super::past_slop(last.get(), (x, y)) {
                return None;
            }
            let (lx, ly) = last.replace((x, y));
            if (x - lx).abs() + (y - ly).abs() <= 0.0 {
                return None;
            }
            dragged.set(true);
            Event::Drag {
                dx: x - lx,
                dy: y - ly,
                rotate: flags.contains(NSEventModifierFlags::Option)
                    || t == NSEventType::RightMouseDragged,
            }
        } else if t == NSEventType::LeftMouseUp {
            if dragged.get() {
                return None;
            }
            Event::Click { x, y }
        } else if t == NSEventType::MouseMoved {
            Event::Move { x, y }
        } else if t == NSEventType::KeyDown && !flags.contains(NSEventModifierFlags::Command) {
            Event::Key(match e.keyCode() {
                123 => Key::Left,
                124 => Key::Right,
                125 => Key::Down,
                126 => Key::Up,
                48 if flags.contains(NSEventModifierFlags::Shift) => Key::BackTab,
                48 => Key::Tab,
                36 | 76 => Key::Enter,
                53 => Key::Escape,
                116 => Key::PageUp,
                121 => Key::PageDown,
                _ => Key::Char(
                    e.charactersIgnoringModifiers()?
                        .to_string()
                        .chars()
                        .next()?
                        .to_ascii_lowercase(),
                ),
            })
        } else {
            return None;
        })
    }

    pub(super) fn install(
        window: &impl HasWindowHandle,
        on: Box<dyn Fn(Event)>,
    ) -> Result<Monitor, Error> {
        let view = view_of(window)?;
        let window = view.window().ok_or(Error::NoWindow)?;
        window.setAcceptsMouseMovedEvents(true);
        let number = window.windowNumber();
        let (last, dragged) = (Cell::new((0.0f32, 0.0f32)), Cell::new(false));
        let block: Handler = RcBlock::new(move |ev: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY (3): AppKit hands the monitor a live event, valid for
            // the block's duration; it is only read here.
            let e = unsafe { ev.as_ref() };
            if let Some(event) = read(e, &view, number, &last, &dragged) {
                on(event);
            }
            ev.as_ptr()
        });
        let mask = NSEventMask::ScrollWheel
            | NSEventMask::Magnify
            | NSEventMask::Rotate
            | NSEventMask::LeftMouseDown
            | NSEventMask::LeftMouseDragged
            | NSEventMask::LeftMouseUp
            | NSEventMask::RightMouseDown
            | NSEventMask::RightMouseDragged
            | NSEventMask::MouseMoved
            | NSEventMask::KeyDown;
        // SAFETY (2): the block returns the event it was handed, unchanged
        // (a valid pointer), as the method requires.
        let monitor =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) }
                .ok_or(Error::NoWindow)?;
        let id = NEXT.with(|n| n.replace(n.get() + 1));
        MONITORS.with(|m| m.borrow_mut().insert(id, (monitor, block)));
        Ok(Monitor(id))
    }

    pub(super) fn remove(monitor: Monitor) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        if let Some((object, _block)) = MONITORS.with(|m| m.borrow_mut().remove(&monitor.0)) {
            // SAFETY (4): `object` is exactly what adding the monitor
            // returned, kept in the registry until now.
            unsafe { NSEvent::removeMonitor(&object) };
        }
    }

    pub(super) fn visible(window: &impl HasWindowHandle) -> Result<bool, Error> {
        let window = window_of(window)?;
        Ok(window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible)
            && !window.isMiniaturized())
    }

    pub(super) fn front_beside(
        window: &impl HasWindowHandle,
        beside: &impl HasWindowHandle,
        raise: impl FnOnce(),
    ) -> Result<(), Error> {
        match (window_of(window), window_of(beside)) {
            (Ok(window), Ok(beside)) => {
                super::front(&Ns(window), &Ns(beside), raise);
                Ok(())
            }
            (Err(e), _) | (_, Err(e)) => {
                raise();
                Err(e)
            }
        }
    }

    /// An AppKit window, as [`super::front`] places it.
    struct Ns(Retained<NSWindow>);

    impl super::Placed for Ns {
        fn shown_here(&self) -> bool {
            let w = &self.0;
            w.isMiniaturized()
                || (w.isVisible()
                    && w.isOnActiveSpace()
                    && w.occlusionState().contains(NSWindowOcclusionState::Visible))
        }
        fn frame(&self) -> [f64; 4] {
            rect(self.0.frame())
        }
        fn display(&self) -> Option<[f64; 4]> {
            self.0.screen().map(|s| rect(s.visibleFrame()))
        }
        fn set_frame(&self, to: [f64; 4]) {
            self.0.setFrame_display(
                NSRect::new(NSPoint::new(to[0], to[1]), NSSize::new(to[2], to[3])),
                true,
            );
        }
        fn order_out(&self) {
            self.0.orderOut(None);
        }
    }

    fn rect(r: NSRect) -> [f64; 4] {
        [r.origin.x, r.origin.y, r.size.width, r.size.height]
    }

    pub(super) fn reduce_motion() -> bool {
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use raw_window_handle::HasWindowHandle;

    use super::{Error, Event, Monitor};

    const WHY: &str = if cfg!(target_os = "windows") {
        "the Floor's window reads its input with a window-procedure hook, coming in the Windows release"
    } else {
        "the Floor's window reads its input with GTK signals, coming in the Linux release"
    };

    pub(super) const SUPPORTED: Result<(), Error> = Err(Error::Unsupported(WHY));

    pub(super) fn install(
        _window: &impl HasWindowHandle,
        _on: Box<dyn Fn(Event)>,
    ) -> Result<Monitor, Error> {
        Err(Error::Unsupported(WHY))
    }

    pub(super) fn remove(_monitor: Monitor) {}

    pub(super) fn visible(_window: &impl HasWindowHandle) -> Result<bool, Error> {
        Err(Error::Unsupported(WHY))
    }

    pub(super) fn front_beside(
        _window: &impl HasWindowHandle,
        _beside: &impl HasWindowHandle,
        raise: impl FnOnce(),
    ) -> Result<(), Error> {
        raise();
        Err(Error::Unsupported(WHY))
    }

    pub(super) const fn reduce_motion() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, hint};

    #[test]
    fn a_press_that_wobbles_is_still_a_click() {
        assert!(!super::past_slop((10.0, 10.0), (12.0, 11.5)));
        assert!(!super::past_slop((10.0, 10.0), (10.0, 14.0)));
        assert!(super::past_slop((10.0, 10.0), (15.0, 10.0)));
    }

    #[test]
    fn errors_say_why_in_plain_words() {
        assert!(
            Error::Unsupported("coming soon")
                .to_string()
                .contains("coming soon")
        );
        assert!(Error::NotMainThread.to_string().contains("main thread"));
        assert!(hint().contains("H: reset"));
    }

    #[test]
    fn off_the_main_thread_it_refuses() {
        struct NoHandle;
        impl raw_window_handle::HasWindowHandle for NoHandle {
            fn window_handle(
                &self,
            ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
            {
                Err(raw_window_handle::HandleError::Unavailable)
            }
        }
        // Test threads aren't the main thread.
        let err = std::thread::spawn(|| super::visible(&NoHandle))
            .join()
            .map_err(|_| "joined");
        assert!(
            matches!(err, Ok(Err(Error::NotMainThread | Error::Unsupported(_)))),
            "{err:?}"
        );
    }

    #[test]
    fn a_window_brought_beside_another_lands_inside_its_display() {
        // a 1200×800 window on a display at the left, brought to a
        // 1440×875 one at its right: centred there, its size kept
        assert_eq!(
            super::centred([100.0, 50.0, 1200.0, 800.0], [1512.0, 0.0, 1440.0, 875.0]),
            [1632.0, 38.0, 1200.0, 800.0]
        );
        // too big for it: cut to fit
        assert_eq!(
            super::centred([0.0, 0.0, 2000.0, 1200.0], [0.0, 25.0, 1440.0, 875.0]),
            [0.0, 25.0, 1440.0, 875.0]
        );
    }

    /// A window as a tabbing window manager holds it (the walk Mac's
    /// BetterStage): a tab it hides stays hidden when raised; a window
    /// ordered in afresh is the tab it shows.
    #[derive(Default)]
    struct Tab {
        hidden: std::cell::Cell<bool>,
        ordered_out: std::cell::Cell<bool>,
        frame: std::cell::Cell<[f64; 4]>,
        display: std::cell::Cell<Option<[f64; 4]>>,
        log: std::cell::RefCell<Vec<&'static str>>,
    }

    impl Tab {
        /// What a raise (show, restore, focus) does to it.
        fn raised(&self) {
            self.log.borrow_mut().push("raise");
            if self.ordered_out.replace(false) {
                self.hidden.set(false);
            }
        }
    }

    impl super::Placed for Tab {
        fn shown_here(&self) -> bool {
            !self.hidden.get() && !self.ordered_out.get()
        }
        fn frame(&self) -> [f64; 4] {
            self.frame.get()
        }
        fn display(&self) -> Option<[f64; 4]> {
            self.display.get()
        }
        fn set_frame(&self, frame: [f64; 4]) {
            self.log.borrow_mut().push("move");
            self.frame.set(frame);
        }
        fn order_out(&self) {
            self.log.borrow_mut().push("order out");
            self.ordered_out.set(true);
        }
    }

    /// 0.4.1, FL1 (rc32): a Floor door left the main window a hidden tab
    /// beside the Floor; it's shown, and a window already shown isn't
    /// taken off screen on the way.
    #[test]
    fn a_window_a_manager_hid_is_shown_and_one_shown_stays_put() {
        let room = Some([0.0, 0.0, 1440.0, 875.0]);
        let floor = Tab::default();
        floor.display.set(room);
        let main = Tab::default();
        main.display.set(room);
        main.hidden.set(true);
        super::front(&main, &floor, || main.raised());
        assert!(super::Placed::shown_here(&main), "{:?}", main.log.borrow());
        assert_eq!(*main.log.borrow(), ["order out", "raise"]);
        // already shown: just raised, no flicker
        main.log.borrow_mut().clear();
        super::front(&main, &floor, || main.raised());
        assert_eq!(*main.log.borrow(), ["raise"]);
        // on another display: moved to the Floor's first
        main.display.set(Some([-1440.0, 0.0, 1440.0, 875.0]));
        main.frame.set([-1300.0, 50.0, 1200.0, 800.0]);
        main.log.borrow_mut().clear();
        super::front(&main, &floor, || main.raised());
        assert_eq!(*main.log.borrow(), ["move", "raise"]);
        assert_eq!(main.frame.get(), [120.0, 38.0, 1200.0, 800.0]);
    }

    #[test]
    fn bringing_a_window_forward_raises_it_once_even_when_it_cant_follow() {
        struct NoHandle;
        impl raw_window_handle::HasWindowHandle for NoHandle {
            fn window_handle(
                &self,
            ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
            {
                Err(raw_window_handle::HandleError::Unavailable)
            }
        }
        let raised = std::thread::spawn(|| {
            let mut n = 0;
            let out = super::front_beside(&NoHandle, &NoHandle, || n += 1);
            (n, out.is_err())
        })
        .join()
        .map_err(|_| "joined");
        assert_eq!(raised, Ok((1, true)));
    }
}
