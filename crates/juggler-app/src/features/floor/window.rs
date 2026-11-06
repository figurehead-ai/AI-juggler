//! The Floor's window (AJ-E3501): a native window from Tauri's `unstable`
//! builder, made on first open and gone on close, which ends the render
//! thread and lets the GPU go. Its surface and its event monitor are made
//! on the main thread; everything else runs on the render thread.
//!
//! Entry points: the header's Floor button (`floor_open`), Window › The
//! Floor and the tray (⌘⇧M, which also brings the main window back from
//! the Floor). A click on a door goes through the app's own `app://open`. Where the window's input can't be read yet, the entry says
//! so, and why.

use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::time::Instant;

use juggler_floor::driver::{Input, Key};
use juggler_window_events::{Event, Monitor};
use tauri::{Emitter, Manager};

use super::render::{self, Msg};
use super::store::Settings;
use crate::error::{AppError, AppResult};
use crate::features::notifications::{EVENT_OPEN as APP_OPEN, OpenRequest};
use crate::state::AppState;

/// The window's label.
pub const LABEL: &str = "floor";
/// The menu item's (and the tray's) id.
pub const MENU_ID: &str = "floor:open";

/// The open window: the render thread's channel and the event monitor.
pub struct Open {
    tx: Sender<Msg>,
    monitor: Option<Monitor>,
}

/// The window side of the render thread.
struct Side {
    app: tauri::AppHandle,
    window: tauri::Window,
}

impl render::Window for Side {
    fn title(&self, summary: String) {
        let window = self.window.clone();
        let _ = self.app.run_on_main_thread(move || {
            let _ = window.set_title(&format!("The Floor — {summary}"));
        });
    }

    fn ask_visible(&self, tx: Sender<Msg>) {
        let window = self.window.clone();
        let _ = self.app.run_on_main_thread(move || {
            let seen = juggler_window_events::visible(&window).unwrap_or(true);
            let _ = tx.send(Msg::Input(Input::Visible(seen)));
        });
    }

    fn cursor(&self, hand: bool) {
        let window = self.window.clone();
        let _ = self.app.run_on_main_thread(move || {
            let _ = window.set_cursor_icon(if hand {
                tauri::CursorIcon::Hand
            } else {
                tauri::CursorIcon::Default
            });
        });
    }

    fn open(&self, door: OpenRequest) {
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            crate::main_window::bring_forward(&app);
            let _ = app.emit(APP_OPEN, door);
        });
    }
}

/// An event of the window, as the driver takes it.
#[must_use]
pub fn input_of(event: Event) -> Input {
    match event {
        Event::Scroll {
            dx,
            dy,
            precise,
            x,
            y,
            zoom,
        } => Input::Scroll {
            dx,
            dy,
            precise,
            x,
            y,
            zoom,
        },
        Event::Magnify { m, x, y } => Input::Magnify { m, x, y },
        Event::Twist(deg) => Input::Twist(deg),
        Event::Drag { dx, dy, rotate } => Input::Drag { dx, dy, rotate },
        Event::Move { x, y } => Input::Move { x, y },
        Event::Click { x, y } => Input::Click { x, y },
        Event::Key(key) => Input::Key(match key {
            juggler_window_events::Key::Char(c) => Key::Char(c),
            juggler_window_events::Key::Left => Key::Left,
            juggler_window_events::Key::Right => Key::Right,
            juggler_window_events::Key::Up => Key::Up,
            juggler_window_events::Key::Down => Key::Down,
            juggler_window_events::Key::Tab => Key::Tab,
            juggler_window_events::Key::BackTab => Key::BackTab,
            juggler_window_events::Key::Enter => Key::Enter,
            juggler_window_events::Key::PageUp => Key::PageUp,
            juggler_window_events::Key::PageDown => Key::PageDown,
        }),
    }
}

/// Opens the Floor, or brings it forward. Call off the main thread (it
/// waits for work it hands the main thread).
///
/// # Errors
///
/// [`AppError::Refused`] where the Floor can't run yet (why, in words),
/// or when its window can't be made or drawn into.
pub fn open(app: &tauri::AppHandle) -> AppResult<()> {
    juggler_window_events::supported()
        .map_err(|e| AppError::Refused(format!("The Floor isn't available here yet: {e}.")))?;
    // Held until the window and its render thread exist: a second open
    // meanwhile finds the window below instead of making another.
    let _one = OPENING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(window) = app.get_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let state = Arc::clone(app.state::<Arc<AppState>>().inner());
    let settings = state
        .storage
        .lock()
        .ok()
        .and_then(|s| Settings::read(&s).ok())
        .unwrap_or_default();
    let opened = Instant::now();
    render::open_step("asked", opened);
    let refused =
        |e: &dyn std::fmt::Display| AppError::Refused(format!("The Floor couldn't open: {e}."));
    let window = tauri::window::WindowBuilder::new(app, LABEL)
        .title("The Floor")
        .inner_size(1278.0, 798.0)
        .min_inner_size(640.0, 420.0)
        .always_on_top(settings.on_top)
        .focused(true)
        .build()
        .map_err(|e| refused(&e))?;
    render::open_step("window", opened);
    let size = window.inner_size().map_err(|e| refused(&e))?;
    #[allow(clippy::cast_possible_truncation)] // a display's scale: 1–3
    let scale = window.scale_factor().map_err(|e| refused(&e))? as f32;
    let instance = juggler_floor_render::instance_for(Box::new(window.clone()));
    let (tx, rx) = channel::<Msg>();
    let (made_tx, made_rx) = channel();
    {
        let (window, instance, tx) = (window.clone(), instance.clone(), tx.clone());
        app.run_on_main_thread(move || {
            let surface = juggler_floor_render::surface_for(&instance, window.clone());
            let events = tx.clone();
            let monitor = juggler_window_events::install(&window, move |event| {
                let _ = events.send(Msg::Input(input_of(event)));
            })
            .ok();
            let _ = made_tx.send((surface, monitor));
        })
        .map_err(|e| refused(&e))?;
    }
    let (surface, monitor) = made_rx.recv().map_err(|e| refused(&e))?;
    let surface = surface.map_err(|e| refused(&e))?;
    render::open_step("surface", opened);
    if let Ok(mut open) = state.floor.window.lock() {
        *open = Some(Open {
            tx: tx.clone(),
            monitor,
        });
    }
    let snapshots = tx.clone();
    state.floor.subscribe(Box::new(move |snapshot| {
        snapshots.send(Msg::Snapshot(snapshot)).is_ok()
    }));
    let start = render::Start {
        state,
        instance,
        surface,
        size: (size.width, size.height, scale),
        opened,
    };
    let window_side = Side {
        app: app.clone(),
        window,
    };
    std::thread::Builder::new()
        .name("floor-render".into())
        .spawn(move || render::run(start, &rx, &tx, &window_side))
        .map_err(|e| refused(&e))?;
    Ok(())
}

/// The Floor's settings or saved office changed: an open Floor reads them
/// again (and keeps itself on top or not, as asked).
pub fn reload(app: &tauri::AppHandle) {
    let state = app.state::<Arc<AppState>>();
    if let Ok(open) = state.floor.window.lock()
        && let Some(open) = open.as_ref()
    {
        let _ = open.tx.send(Msg::Reload);
    }
    let on_top = state
        .storage
        .lock()
        .ok()
        .and_then(|s| Settings::read(&s).ok())
        .is_some_and(|s| s.on_top);
    if let Some(window) = app.get_window(LABEL) {
        let _ = window.set_always_on_top(on_top);
    }
}

/// Sends an open Floor a message (nothing when it isn't open).
pub(crate) fn send(app: &tauri::AppHandle, msg: super::render::Msg) {
    let state = app.state::<Arc<AppState>>();
    if let Ok(open) = state.floor.window.lock()
        && let Some(open) = open.as_ref()
    {
        let _ = open.tx.send(msg);
    }
}

/// One press arrives twice: Tauri hands a menu event to the app menu's
/// handler and to the tray's, and both carry the Floor's id. A second
/// toggle this soon after the first is the same press (0.4.1: one
/// Window › The Floor made two Floor windows, and a second toggle could
/// send the user straight back to the main window).
const SAME_PRESS: std::time::Duration = std::time::Duration::from_millis(400);

/// Whether a toggle at `now` is a new press, given the last one's time;
/// records it when it is.
fn new_press(last: &std::sync::Mutex<Option<Instant>>, now: Instant) -> bool {
    let Ok(mut last) = last.lock() else {
        return true;
    };
    if last.is_some_and(|at| now.saturating_duration_since(at) < SAME_PRESS) {
        return false;
    }
    *last = Some(now);
    true
}

static LAST_TOGGLE: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);

/// One Floor at a time: set while a window is being made, so a second
/// open meanwhile waits for it rather than making another.
static OPENING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// ⌘⇧M: from the Floor, back to the main window; otherwise the Floor.
pub fn toggle(app: &tauri::AppHandle) {
    if !new_press(&LAST_TOGGLE, Instant::now()) {
        return;
    }
    let floor_in_front = app
        .get_window(LABEL)
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    if floor_in_front {
        crate::main_window::bring_forward(app);
        return;
    }
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("floor-open".into())
        .spawn(move || {
            if let Err(e) = open(&app) {
                let _ = app.emit_to("main", EVENT_UNAVAILABLE, e.to_string());
            }
        });
}

/// The main window's event: the Floor couldn't open (its words).
pub const EVENT_UNAVAILABLE: &str = "floor://unavailable";

/// Asks the backend to open the Floor (the tray, which is generic over the
/// runtime, sends it; [`listen`] answers).
pub const OPEN_REQUEST: &str = "floor://open-request";

/// Answers [`OPEN_REQUEST`], and opens the Floor at start when the user
/// asked for that (Settings › The Floor). Nothing else of the Floor runs
/// until it's first opened.
pub fn listen(app: &tauri::AppHandle, state: &AppState) {
    use tauri::Listener;
    let handle = app.clone();
    app.listen_any(OPEN_REQUEST, move |_| toggle(&handle));
    let at_start = state
        .storage
        .lock()
        .ok()
        .and_then(|s| Settings::read(&s).ok())
        .is_some_and(|s| s.at_start);
    if at_start {
        toggle(app);
    }
}

/// The Floor window's own events (on the main thread): a resize goes to
/// the render thread; closing ends it and the monitor.
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let state = window.state::<Arc<AppState>>();
    match event {
        tauri::WindowEvent::Resized(size) => {
            #[allow(clippy::cast_possible_truncation)] // a display's scale: 1–3
            let scale = window.scale_factor().unwrap_or(2.0) as f32;
            if let Ok(open) = state.floor.window.lock()
                && let Some(open) = open.as_ref()
            {
                let _ = open.tx.send(Msg::Input(Input::Resize {
                    w: size.width,
                    h: size.height,
                    scale,
                }));
            }
        }
        tauri::WindowEvent::Destroyed => {
            let open = state
                .floor
                .window
                .lock()
                .ok()
                .and_then(|mut open| open.take());
            if let Some(open) = open {
                if let Some(monitor) = open.monitor {
                    juggler_window_events::remove(monitor);
                }
                let _ = open.tx.send(Msg::Close);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use juggler_floor::driver::{Input, Key};
    use juggler_window_events::Event;

    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::{SAME_PRESS, input_of, new_press};

    #[test]
    fn window_events_become_the_drivers_inputs() {
        assert_eq!(
            input_of(Event::Click { x: 1.0, y: 2.0 }),
            Input::Click { x: 1.0, y: 2.0 }
        );
        assert_eq!(
            input_of(Event::Key(juggler_window_events::Key::Tab)),
            Input::Key(Key::Tab)
        );
        assert_eq!(input_of(Event::Twist(5.0)), Input::Twist(5.0));
    }

    /// 0.4.1: the app menu and the tray both get one press; it toggles once.
    #[test]
    fn one_press_seen_twice_toggles_once() {
        let last = Mutex::new(None);
        let t = Instant::now();
        assert!(new_press(&last, t));
        assert!(!new_press(&last, t + Duration::from_millis(5)));
        assert!(new_press(&last, t + SAME_PRESS + Duration::from_millis(1)));
    }
}
