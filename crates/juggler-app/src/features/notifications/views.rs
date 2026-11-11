//! AJ-E2501 — the views of the model outside the window's own lists: the
//! app badge (behind the platform seam), the menu-bar item, and the
//! window's bell (an event with the summary). Each change in the model
//! ([`super::changed`]) brings all three to the same count.

use crate::events::{AppEvent, EmitEvent as _};
use std::sync::Arc;

use juggler_platform::system::{AppBadge, app_badge, badge_value};

use super::centre::{self, NoticeSummaryInfo};
use super::tray::{self, TrayClick, TrayMenu};
use crate::state::AppState;

/// The window hears of every change: the summary.
pub const EVENT_CHANGED: &str = AppEvent::NotificationsChanged.name();
/// The window is asked to open a notice (`id`) or the centre (`null`).
pub const EVENT_OPEN: &str = AppEvent::NotificationsOpen.name();
/// The window is asked to start a new session (the user chose it).
pub const EVENT_NEW_SESSION: &str = AppEvent::NewSession.name();

/// One app with sessions open in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenApp {
    /// The app's name, as the rail shows it ("Claude app", "Terminal").
    pub app: String,
    /// Sessions open in it.
    pub count: u32,
}

/// What is open now, as the backend works it out ([`super::needs`]; the
/// menu-bar item lists it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenNowInput {
    /// Sessions open now.
    pub total: u32,
    /// …of them working.
    pub working: u32,
    /// By app, most first.
    pub apps: Vec<OpenApp>,
}

/// How the app badge went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeShown {
    /// The icon shows the count (or no badge at zero).
    Shown,
    /// Not on this system yet (the platform seam says why).
    NotYet,
    /// The window wasn't there to carry it.
    NoWindow,
}

/// Puts `count` on the app's icon, the way this system does it.
pub fn show_badge<R: tauri::Runtime>(app: &tauri::AppHandle<R>, count: u32) -> BadgeShown {
    use tauri::Manager;
    match app_badge() {
        AppBadge::Dock => app
            .get_webview_window("main")
            .map_or(BadgeShown::NoWindow, |window| {
                let _ = window.set_badge_count(badge_value(count));
                BadgeShown::Shown
            }),
        AppBadge::NotYet(_) => BadgeShown::NotYet,
    }
}

/// The summary now (an empty one when storage can't be read).
#[must_use]
pub fn summary_now(state: &Arc<AppState>) -> NoticeSummaryInfo {
    let clock = state.notifier.clock();
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| centre::summary(&storage, clock).ok())
        .unwrap_or_default()
}

/// Where the views are drawn: the running app's window, badge and
/// menu-bar item ([`AppViews`]), or a test's record of what it was given.
pub trait Surface: Send + Sync {
    /// The window's bell hears the summary.
    fn tell_window(&self, summary: NoticeSummaryInfo);
    /// The app badge shows `count`, and the menu-bar item draws `menu`
    /// (made the first time).
    fn draw(&self, count: u32, menu: TrayMenu);
}

/// The running app's views.
pub struct AppViews<R: tauri::Runtime>(pub tauri::AppHandle<R>);

impl<R: tauri::Runtime> Surface for AppViews<R> {
    fn tell_window(&self, summary: NoticeSummaryInfo) {
        self.0.emit_event(AppEvent::NotificationsChanged, summary);
    }

    fn draw(&self, count: u32, menu: TrayMenu) {
        let handle = self.0.clone();
        let _ = self.0.run_on_main_thread(move || {
            let _ = show_badge(&handle, count);
            if let Err(error) = tray::apply(&handle, &menu) {
                tracing::warn!("the menu-bar item couldn't be drawn: {error}");
            }
        });
    }
}

/// Brings every view to the model's state now.
pub fn refresh<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &Arc<AppState>) {
    refresh_on(&AppViews(app.clone()), state);
}

/// [`refresh`] on `surface`.
pub fn refresh_on(surface: &dyn Surface, state: &Arc<AppState>) {
    let summary = summary_now(state);
    let now = crate::commands::now_millis();
    let needs = state
        .storage
        .lock()
        .map(|storage| centre::open_needs(&storage, now, tray::NEEDS_LISTED))
        .unwrap_or_default();
    let menu = tray::build(summary.count, &needs, &state.notifier.open_now(), now);
    let count = summary.count;
    surface.tell_window(summary);
    surface.draw(count, menu);
}

/// A line of the menu-bar item was clicked: AI Juggler comes forward on
/// what it asked for. Nothing else happens until the user acts there.
pub fn tray_clicked<R: tauri::Runtime>(app: &tauri::AppHandle<R>, id: &str) {
    use tauri::Manager;
    let Some(click) = tray::click_of(id) else {
        return;
    };
    if click == TrayClick::Floor {
        app.emit_event(AppEvent::FloorOpenRequest, ());
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    match click {
        TrayClick::NewSession => {
            app.emit_event(AppEvent::NewSession, ());
        }
        TrayClick::Centre => {
            app.emit_event(AppEvent::NotificationsOpen, None::<i64>);
        }
        TrayClick::Notice(id) => {
            app.emit_event(AppEvent::NotificationsOpen, Some(id));
        }
        TrayClick::Open | TrayClick::Floor => {}
    }
}

/// Connects the views: every change in the model reaches them, and they
/// show the model's state from the start (the menu-bar item is made
/// then).
pub fn connect(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let surface: Arc<dyn Surface> = Arc::new(AppViews(app.clone()));
    connect_on(&surface, state);
}

/// [`connect`] on `surface`. The notifier holds the state weakly (the
/// state holds the notifier).
pub fn connect_on(surface: &Arc<dyn Surface>, state: &Arc<AppState>) {
    let weak = Arc::downgrade(state);
    let on_change = Arc::clone(surface);
    state.notifier.set_on_change(Box::new(move || {
        if let Some(state) = weak.upgrade() {
            refresh_on(on_change.as_ref(), &state);
        }
    }));
    refresh_on(surface.as_ref(), state);
}
