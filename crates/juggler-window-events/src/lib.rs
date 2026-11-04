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
    /// Return.
    Enter,
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

/// What the system calls the keys that move the Floor's view.
#[must_use]
pub const fn hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "Drag: pan  ·  Right-drag or Option-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset"
    } else {
        "Drag: pan  ·  Right-drag or Alt-drag: turn and tilt  ·  Ctrl-scroll: zoom  ·  H: reset"
    }
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
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::{Error, Event, Key, Monitor};

    type Handler = RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent>;

    thread_local! {
        static MONITORS: RefCell<HashMap<u64, (Retained<AnyObject>, Handler)>> = RefCell::new(HashMap::new());
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }

    fn window_of(window: &impl HasWindowHandle) -> Result<Retained<NSWindow>, Error> {
        MainThreadMarker::new().ok_or(Error::NotMainThread)?;
        let handle = window.window_handle().map_err(|_| Error::NoWindow)?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return Err(Error::NoWindow);
        };
        // SAFETY (1): `ns_view` is the window's own `NSView`, valid while
        // the window is; we're on the main thread (checked above), where
        // AppKit objects live, and retain it at once.
        let view: Retained<NSView> =
            unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }
                .ok_or(Error::NoWindow)?;
        view.window().ok_or(Error::NoWindow)
    }

    /// One event of `window` (numbered `number`), as the Floor reads it.
    #[allow(clippy::cast_possible_truncation)] // points and deltas: small numbers
    fn read(
        e: &NSEvent,
        window: &NSWindow,
        number: isize,
        last: &Cell<(f32, f32)>,
        dragged: &Cell<bool>,
    ) -> Option<Event> {
        if e.windowNumber() != number {
            return None;
        }
        let content_h = window.contentLayoutRect().size.height;
        let at = e.locationInWindow();
        let (x, y) = (at.x as f32, (content_h - at.y) as f32);
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
                48 => Key::Tab,
                36 | 76 => Key::Enter,
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
        let window = window_of(window)?;
        window.setAcceptsMouseMovedEvents(true);
        let number = window.windowNumber();
        let (last, dragged) = (Cell::new((0.0f32, 0.0f32)), Cell::new(false));
        let block: Handler = RcBlock::new(move |ev: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY (3): AppKit hands the monitor a live event, valid for
            // the block's duration; it is only read here.
            let e = unsafe { ev.as_ref() };
            if let Some(event) = read(e, &window, number, &last, &dragged) {
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
}

#[cfg(test)]
mod tests {
    use super::{Error, hint};

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
}
