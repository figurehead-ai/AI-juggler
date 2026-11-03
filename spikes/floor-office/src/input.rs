//! Mouse, trackpad and keys for the native Floor window on macOS.
//! Tauri passes a window without a webview no input events, so this listens to AppKit directly: a
//! local event monitor, filtered to the Floor's window, turned into messages for the render thread.
//! Windows and Linux need their own equivalents (a window procedure hook; GTK signals).

use crate::render::Msg;
use std::sync::mpsc::Sender;

#[cfg(target_os = "macos")]
pub fn install(window: &tauri::Window, tx: Sender<Msg>) {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSWindow};
    use std::cell::Cell;
    use std::ptr::NonNull;
    let Ok(ptr) = window.ns_window() else { return };
    let ptr = ptr as usize;
    let num = unsafe { &*(ptr as *const NSWindow) }.windowNumber();
    let last = Cell::new((0.0f32, 0.0f32));
    let dragged = Cell::new(false);
    let block = RcBlock::new(move |ev: NonNull<NSEvent>| -> *mut NSEvent {
        let e = unsafe { ev.as_ref() };
        if e.windowNumber() != num {
            return ev.as_ptr();
        }
        let nsw = unsafe { &*(ptr as *const NSWindow) };
        let content_h = nsw.contentLayoutRect().size.height;
        let loc = e.locationInWindow();
        let (x, y) = (loc.x as f32, (content_h - loc.y) as f32);
        let t = e.r#type();
        if t == NSEventType::ScrollWheel {
            let zoom = e.modifierFlags().contains(NSEventModifierFlags::Command);
            let _ = tx.send(Msg::Scroll { dx: e.scrollingDeltaX() as f32, dy: e.scrollingDeltaY() as f32, precise: e.hasPreciseScrollingDeltas(), x, y, zoom });
        } else if t == NSEventType::Magnify {
            let _ = tx.send(Msg::Magnify { m: e.magnification() as f32, x, y });
        } else if t == NSEventType::LeftMouseDown {
            last.set((x, y));
            dragged.set(false);
        } else if t == NSEventType::LeftMouseDragged {
            let (lx, ly) = last.get();
            if (x - lx).abs() + (y - ly).abs() > 0.0 {
                dragged.set(true);
                let _ = tx.send(Msg::Drag { dx: x - lx, dy: y - ly });
            }
            last.set((x, y));
        } else if t == NSEventType::LeftMouseUp {
            if !dragged.get() {
                let _ = tx.send(Msg::Click { x, y });
            }
        } else if t == NSEventType::KeyDown {
            if let Some(c) = e.characters().and_then(|s| s.to_string().chars().next()) {
                let _ = tx.send(Msg::Key(c));
            }
        }
        ev.as_ptr()
    });
    let mask = NSEventMask::ScrollWheel | NSEventMask::Magnify | NSEventMask::LeftMouseDown | NSEventMask::LeftMouseDragged | NSEventMask::LeftMouseUp | NSEventMask::KeyDown;
    let monitor = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) };
    // The monitor lives as long as the app.
    std::mem::forget(monitor);
    std::mem::forget(block);
}

#[cfg(not(target_os = "macos"))]
pub fn install(_window: &tauri::Window, _tx: Sender<Msg>) {}
