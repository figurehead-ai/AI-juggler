//! The main window as it opens (AJ-E1549; 0.5.1 F21): in the colour of
//! the theme the user chose and at the size and place it was left, from
//! its first frame.
//!
//! The walk saw two things at a cold launch. With AI Juggler's theme on
//! Light and the Mac in dark mode, the first frame was dark (the window's
//! fixed colour) before the light page painted; and the window opened at
//! the config's 1360×860, then was put back to the user's 1701×1348 by
//! the tiling window manager within 0.7 s. So the page tells the backend
//! the theme it chose ([`THEME_KEY`], `ui/src/theme.ts`), the backend
//! keeps the window's last frame as it is moved and resized
//! ([`FRAME_KEY`]), and the window is made from both. Both live in the
//! data folder's settings: a lab's Dev keeps its own.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::window::Color;
use tauri::{Manager, Runtime};

use crate::state::AppState;

/// The app setting holding the theme the user chose: `dark` | `light` |
/// `system` (the page's `aij.theme`; `ui/src/theme.ts` writes it).
pub const THEME_KEY: &str = "ui.window.theme";

/// The app setting holding the main window's last frame: `x,y,w,h` in
/// points (its outer top-left, its content's size).
pub const FRAME_KEY: &str = "ui.window.frame";

/// The light theme's canvas (`--color-canvas` under `[data-theme="light"]`
/// in `ui/src/styles.css`; `ui/tests/bundle/theme-base.test.ts` keeps it in
/// step). The dark one is [`crate::webview_guard::BASE_COLOUR`].
pub const LIGHT_COLOUR: Color = Color(0xec, 0xee, 0xf3, 0xff);

/// The theme the user chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Dark (the default, as `theme-boot.js` has it).
    Dark,
    /// Light.
    Light,
    /// Whatever macOS shows.
    System,
}

impl Choice {
    /// The choice from its stored word; dark when none or unknown.
    #[must_use]
    pub fn from_stored(raw: Option<&str>) -> Self {
        match raw {
            Some("light") => Self::Light,
            Some("system") => Self::System,
            _ => Self::Dark,
        }
    }

    /// The colour the window paints for this choice, `system` being what
    /// the system shows now.
    #[must_use]
    pub fn colour(self, system: tauri::Theme) -> Color {
        match (self, system) {
            (Self::Light, _) | (Self::System, tauri::Theme::Light) => LIGHT_COLOUR,
            _ => crate::webview_guard::BASE_COLOUR,
        }
    }
}

/// A rectangle in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Frame {
    /// A frame from its setting (`x,y,w,h`); `None` unless four finite
    /// numbers with a size.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let numbers: Vec<f64> = raw
            .split(',')
            .map(|part| part.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<_>>()?;
        match numbers[..] {
            [x, y, width, height] if width > 0.0 && height > 0.0 => Some(Self {
                x,
                y,
                width,
                height,
            }),
            _ => None,
        }
    }

    /// The frame as its setting keeps it, in whole points.
    #[must_use]
    pub fn setting(self) -> String {
        format!(
            "{},{},{},{}",
            self.x.round(),
            self.y.round(),
            self.width.round(),
            self.height.round()
        )
    }

    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Where the window opens (pure): the saved size, never under `min` nor
/// over the largest screen's work area; and the saved place when the
/// window's title bar would land on one of `screens` (work areas, in
/// points), else `None` (centred).
#[must_use]
pub fn placed(saved: Frame, min: (f64, f64), screens: &[Frame]) -> (f64, f64, Option<(f64, f64)>) {
    let room = screens.iter().fold((0.0_f64, 0.0_f64), |(w, h), s| {
        (w.max(s.width), h.max(s.height))
    });
    let fit = |want: f64, least: f64, most: f64| {
        let want = if most > 0.0 { want.min(most) } else { want };
        want.max(least)
    };
    let width = fit(saved.width, min.0, room.0);
    let height = fit(saved.height, min.1, room.1);
    // A grip on the title bar: a point a little in from its left end.
    let grip = (saved.x + 80.0_f64.min(width / 2.0), saved.y + 14.0);
    let at = screens
        .iter()
        .any(|screen| screen.contains(grip.0, grip.1))
        .then_some((saved.x, saved.y));
    (width, height, at)
}

/// The theme and last frame kept in `state`'s settings.
#[must_use]
pub fn kept(state: &AppState) -> (Choice, Option<Frame>) {
    let Ok(storage) = state.storage.lock() else {
        return (Choice::Dark, None);
    };
    let settings = storage.app_settings();
    let read = |key| settings.get(key).ok().flatten();
    (
        Choice::from_stored(read(THEME_KEY).as_deref()),
        read(FRAME_KEY).as_deref().and_then(Frame::parse),
    )
}

/// The screens' work areas, in points.
#[must_use]
pub fn screens<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<Frame> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let scale = monitor.scale_factor().max(0.1);
            let area = monitor.work_area();
            Frame {
                x: f64::from(area.position.x) / scale,
                y: f64::from(area.position.y) / scale,
                width: f64::from(area.size.width) / scale,
                height: f64::from(area.size.height) / scale,
            }
        })
        .collect()
}

/// The frame waiting to be kept, and whether a write is on its way.
static LATEST: Mutex<Option<Frame>> = Mutex::new(None);
static PENDING: AtomicBool = AtomicBool::new(false);

/// How long the frame must hold before it is kept (a drag writes once).
const SETTLE: Duration = Duration::from_millis(600);

/// The main window was moved or resized: keeps its frame, once it holds
/// still. Never in full screen, minimised or hidden (a test launch's
/// window never shows).
pub fn changed<R: Runtime>(window: &tauri::Window<R>) {
    let shown = window.is_visible().unwrap_or(false)
        && !window.is_fullscreen().unwrap_or(true)
        && !window.is_minimized().unwrap_or(true);
    if !shown {
        return;
    }
    let (Ok(scale), Ok(at), Ok(size)) = (
        window.scale_factor(),
        window.outer_position(),
        window.inner_size(),
    ) else {
        return;
    };
    let at = at.to_logical::<f64>(scale);
    let size = size.to_logical::<f64>(scale);
    if let Ok(mut latest) = LATEST.lock() {
        *latest = Some(Frame {
            x: at.x,
            y: at.y,
            width: size.width,
            height: size.height,
        });
    }
    if PENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = window.app_handle().clone();
    let spawned = std::thread::Builder::new()
        .name("window-frame".to_owned())
        .spawn(move || {
            std::thread::sleep(SETTLE);
            PENDING.store(false, Ordering::SeqCst);
            let frame = LATEST.lock().ok().and_then(|mut latest| latest.take());
            let (Some(frame), Some(state)) = (frame, app.try_state::<Arc<AppState>>()) else {
                return;
            };
            if let Ok(storage) = state.storage.lock()
                && let Err(error) = storage.app_settings().set(FRAME_KEY, &frame.setting())
            {
                tracing::debug!("the window's frame wasn't kept: {error}");
            }
        });
    if spawned.is_err() {
        PENDING.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_paints_the_chosen_theme_not_the_systems() {
        let dark = crate::webview_guard::BASE_COLOUR;
        // F21: Light chosen, the Mac in dark: light from the first frame.
        assert_eq!(Choice::Light.colour(tauri::Theme::Dark), LIGHT_COLOUR);
        assert_eq!(Choice::Dark.colour(tauri::Theme::Light), dark);
        assert_eq!(Choice::System.colour(tauri::Theme::Light), LIGHT_COLOUR);
        assert_eq!(Choice::System.colour(tauri::Theme::Dark), dark);
        // Nothing chosen yet: dark, as the page's theme-boot.js has it.
        assert_eq!(Choice::from_stored(None), Choice::Dark);
        assert_eq!(Choice::from_stored(Some("hot-pink")), Choice::Dark);
        assert_eq!(Choice::from_stored(Some("light")), Choice::Light);
        assert_eq!(Choice::from_stored(Some("system")), Choice::System);
        assert_eq!(LIGHT_COLOUR.3, 0xff, "opaque");
    }

    #[test]
    fn a_frame_reads_back_what_it_kept_and_nothing_else() {
        let frame = Frame {
            x: 12.4,
            y: 40.0,
            width: 1701.2,
            height: 1348.0,
        };
        assert_eq!(frame.setting(), "12,40,1701,1348");
        assert_eq!(
            Frame::parse(&frame.setting()),
            Some(Frame {
                x: 12.0,
                y: 40.0,
                width: 1701.0,
                height: 1348.0
            })
        );
        for bad in [
            "",
            "1,2,3",
            "1,2,3,4,5",
            "a,b,c,d",
            "0,0,0,10",
            "0,0,NaN,10",
            "0,0,inf,10",
        ] {
            assert_eq!(Frame::parse(bad), None, "{bad:?}");
        }
        assert!(
            Frame::parse("-1700,25,1200,800").is_some(),
            "a screen left of the main one"
        );
    }

    #[test]
    fn the_window_opens_where_it_was_left_while_that_is_on_a_screen() {
        let main = Frame {
            x: 0.0,
            y: 25.0,
            width: 1728.0,
            height: 1080.0,
        };
        let left = Frame {
            x: -1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1415.0,
        };
        let min = (1024.0, 680.0);
        let saved = |x, y, width, height| Frame {
            x,
            y,
            width,
            height,
        };
        // F21: its saved size and place from the first frame.
        assert_eq!(
            placed(saved(-1900.0, 10.0, 1701.0, 1348.0), min, &[main, left]),
            (1701.0, 1348.0, Some((-1900.0, 10.0)))
        );
        // The screen it was on is gone: its size, centred.
        assert_eq!(
            placed(saved(-1900.0, 10.0, 1701.0, 1348.0), min, &[main]),
            (1701.0, 1080.0, None)
        );
        // Never under the minimum.
        assert_eq!(
            placed(saved(100.0, 100.0, 300.0, 200.0), min, &[main]),
            (1024.0, 680.0, Some((100.0, 100.0)))
        );
        // No screen known: as kept.
        assert_eq!(
            placed(saved(5.0, 5.0, 1400.0, 900.0), min, &[]),
            (1400.0, 900.0, None)
        );
    }
}
