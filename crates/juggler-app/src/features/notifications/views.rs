//! AJ-E2501 — the views of the model outside the window's own lists: the
//! app badge (behind the platform seam), the menu-bar item, and the
//! window's bell (an event with the summary). Each change in the model
//! ([`super::changed`]) brings all three to the same count.

use std::sync::Arc;

use juggler_platform::system::{AppBadge, app_badge, badge_value};

use super::centre::{self, NoticeSummaryInfo};
use super::tray::{self, TrayClick};
use crate::state::AppState;

/// The window hears of every change: the summary.
pub const EVENT_CHANGED: &str = "notifications://changed";
/// The window is asked to open a notice (`id`) or the centre (`null`).
pub const EVENT_OPEN: &str = "notifications://open";
/// The window is asked to start a new session (the user chose it).
pub const EVENT_NEW_SESSION: &str = "app://new-session";

/// One app with sessions open in it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OpenApp {
    /// The app's name, as the window shows it ("Claude app", "Terminal").
    pub app: String,
    /// Sessions open in it.
    pub count: u32,
}

/// What is open now, as Home counts it (the menu-bar item lists it).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OpenNowInput {
    /// Sessions open now.
    pub total: u32,
    /// …of them working.
    pub working: u32,
    /// By app, most first.
    #[serde(default)]
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

/// Brings every view to the model's state now.
pub fn refresh<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &Arc<AppState>) {
    use tauri::Emitter;
    let summary = summary_now(state);
    let now = crate::commands::now_millis();
    let needs = state
        .storage
        .lock()
        .map(|storage| centre::open_needs(&storage, now, tray::NEEDS_LISTED))
        .unwrap_or_default();
    let menu = tray::build(summary.count, &needs, &state.notifier.open_now(), now);
    let count = summary.count;
    let _ = app.emit(EVENT_CHANGED, summary);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = show_badge(&handle, count);
        if let Err(error) = tray::apply(&handle, &menu) {
            eprintln!("ai-juggler: the menu-bar item couldn't be drawn: {error}");
        }
    });
}

/// A line of the menu-bar item was clicked: AI Juggler comes forward on
/// what it asked for. Nothing else happens until the user acts there.
pub fn tray_clicked<R: tauri::Runtime>(app: &tauri::AppHandle<R>, id: &str) {
    use tauri::{Emitter, Manager};
    let Some(click) = tray::click_of(id) else {
        return;
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    match click {
        TrayClick::Open => {}
        TrayClick::NewSession => {
            let _ = app.emit(EVENT_NEW_SESSION, ());
        }
        TrayClick::Centre => {
            let _ = app.emit(EVENT_OPEN, None::<i64>);
        }
        TrayClick::Notice(id) => {
            let _ = app.emit(EVENT_OPEN, Some(id));
        }
    }
}

/// Connects the views: every change in the model reaches them, and they
/// show the model's state from the start.
pub fn connect(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let handle = app.clone();
    state.notifier.set_on_change(Box::new(move || {
        use tauri::Manager;
        if let Some(state) = handle.try_state::<Arc<AppState>>() {
            refresh(&handle, state.inner());
        }
    }));
    refresh(app, state);
}
