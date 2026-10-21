//! AJ-E1503 Req.3 — quitting with live sessions says so first.
//!
//! ⌘Q (the app menu's own Quit item) and closing the window ask the
//! backend; with nothing live AI Juggler quits at once, otherwise the
//! window is told how many sessions are live and what quitting does to
//! them. A terminal session runs on a terminal this app holds, so it ends
//! when AI Juggler quits; a desktop session (its own app) or a session
//! re-adopted after a crash keeps running. The user picks Quit anyway,
//! Stop them first (a clean stop of each, then quit) or Cancel. Nothing is
//! stopped without that click (ADR-0003).

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use juggler_core::model::InstanceKind;
use tauri::{Emitter, Manager};

use crate::error::AppResult;
use crate::state::AppState;

/// The event asking the window to show the quit sheet; payload
/// [`QuitPlanInfo`].
pub const EVENT_QUIT_REQUESTED: &str = "app://quit-requested";

/// The app menu's Quit item (replaces the predefined one, which quits
/// without asking anyone).
pub const QUIT_MENU_ID: &str = "aij-quit";

/// Set once the user chose to quit: the window closing on the way out
/// must not ask again.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// What quitting now would do.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QuitPlanInfo {
    /// Live sessions on this Mac.
    pub live: u32,
    /// Of those, the ones that end when AI Juggler quits (terminal
    /// sessions on a terminal this app holds).
    pub end_on_quit: u32,
    /// Of those, the ones that keep running (desktop sessions, sessions
    /// re-adopted after a crash).
    pub keep_running: u32,
}

/// One live session as quitting sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LiveSession {
    /// The occurrence's kind.
    pub kind: InstanceKind,
    /// Re-adopted after a crash: its terminal isn't ours any more.
    pub adopted: bool,
}

/// The plan for these live sessions.
#[must_use]
pub fn plan_of(live: &[LiveSession]) -> QuitPlanInfo {
    let keep = live
        .iter()
        .filter(|session| session.kind == InstanceKind::Desktop || session.adopted)
        .count();
    let total = live.len();
    QuitPlanInfo {
        live: u32::try_from(total).unwrap_or(u32::MAX),
        end_on_quit: u32::try_from(total - keep).unwrap_or(u32::MAX),
        keep_running: u32::try_from(keep).unwrap_or(u32::MAX),
    }
}

/// The sessions this app supervises that are still running.
fn live_sessions(state: &Arc<AppState>) -> Vec<(juggler_core::model::SessionId, LiveSession)> {
    let adopted = state.supervisor.alive_adopted_sessions();
    let roots = state.supervisor.live_roots();
    let Ok(storage) = crate::commands::lock_storage_arc(state) else {
        return Vec::new();
    };
    roots
        .into_iter()
        .filter(|(_, _, finished)| !finished)
        .filter_map(|(id, _, _)| {
            let record = storage.sessions().get(&id).ok()?;
            if crate::commands::is_terminal(record.status) {
                return None;
            }
            let session = LiveSession {
                kind: record.kind,
                adopted: adopted.contains(&id),
            };
            Some((id, session))
        })
        .collect()
}

/// The quit plan for what runs now.
#[must_use]
pub fn quit_plan_impl(state: &Arc<AppState>) -> QuitPlanInfo {
    let live: Vec<LiveSession> = live_sessions(state).into_iter().map(|(_, s)| s).collect();
    plan_of(&live)
}

/// What a quit request does: quit now, or ask with this plan.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum QuitDecision {
    /// Nothing is live: quit at once.
    Quit,
    /// Sessions are live: show the sheet.
    Ask(QuitPlanInfo),
}

/// The decision for a plan (0 live → quit; otherwise ask).
#[must_use]
pub fn decide(plan: QuitPlanInfo) -> QuitDecision {
    if plan.live == 0 {
        QuitDecision::Quit
    } else {
        QuitDecision::Ask(plan)
    }
}

/// True once the user chose to quit (the closing window passes through).
#[must_use]
pub fn quitting() -> bool {
    QUITTING.load(Ordering::SeqCst)
}

/// ⌘Q or the window's close button: quit now, or ask the window.
/// Returns true when the request was turned into a question.
pub fn request(app: &tauri::AppHandle) -> bool {
    if quitting() {
        return false;
    }
    let state = app.state::<Arc<AppState>>();
    match decide(quit_plan_impl(state.inner())) {
        QuitDecision::Quit => {
            QUITTING.store(true, Ordering::SeqCst);
            app.exit(0);
            false
        }
        QuitDecision::Ask(plan) => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            let _ = app.emit(EVENT_QUIT_REQUESTED, plan);
            true
        }
    }
}

/// The app menu with its Quit item asking first (the rest is Tauri's
/// default menu).
///
/// # Errors
///
/// Returns the menu builder's error.
pub fn app_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind};
    let menu = Menu::default(app)?;
    let quit = MenuItem::with_id(
        app,
        QUIT_MENU_ID,
        format!("Quit {}", crate::edition::current().app_name()),
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    // macOS: the app submenu comes first and ends with Quit.
    if let Some(MenuItemKind::Submenu(app_submenu)) = menu.items()?.into_iter().next() {
        let items = app_submenu.items()?;
        if let Some(last) = items.len().checked_sub(1) {
            app_submenu.remove_at(last)?;
        }
        app_submenu.append(&quit)?;
    }
    Ok(menu)
}

/// The window's live sessions and what quitting does to them.
#[tauri::command]
#[must_use]
pub fn app_quit_plan(state: tauri::State<'_, Arc<AppState>>) -> QuitPlanInfo {
    quit_plan_impl(state.inner())
}

/// The user chose to quit: with `stop_first`, each live session is
/// stopped cleanly (in parallel, the usual 5 s grace each) before AI
/// Juggler quits.
///
/// # Errors
///
/// Never fails; the result type matches the other commands.
#[tauri::command]
pub fn app_quit(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    stop_first: bool,
) -> AppResult<()> {
    QUITTING.store(true, Ordering::SeqCst);
    let state = Arc::clone(state.inner());
    std::thread::spawn(move || {
        if stop_first {
            let stops: Vec<_> = live_sessions(&state)
                .into_iter()
                .map(|(id, _)| {
                    let state = Arc::clone(&state);
                    std::thread::spawn(move || {
                        let _ = state.supervisor.stop(&id, Duration::from_secs(5));
                    })
                })
                .collect();
            for stop in stops {
                let _ = stop.join();
            }
        }
        app.exit(0);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERMINAL: LiveSession = LiveSession {
        kind: InstanceKind::Terminal,
        adopted: false,
    };

    /// Each kind is pinned to what quitting does to it: a terminal
    /// session ends (its terminal lives in this app); a desktop session
    /// and a re-adopted one keep running.
    #[test]
    fn each_kind_is_pinned_to_what_quitting_does() {
        let alone = |kind, adopted| plan_of(&[LiveSession { kind, adopted }]);
        let ends = QuitPlanInfo {
            live: 1,
            end_on_quit: 1,
            keep_running: 0,
        };
        let keeps = QuitPlanInfo {
            live: 1,
            end_on_quit: 0,
            keep_running: 1,
        };
        assert_eq!(alone(InstanceKind::Terminal, false), ends);
        assert_eq!(alone(InstanceKind::Desktop, false), keeps);
        assert_eq!(alone(InstanceKind::Terminal, true), keeps);
        assert_eq!(alone(InstanceKind::Desktop, true), keeps);
    }

    #[test]
    fn nothing_live_quits_at_once() {
        assert_eq!(decide(plan_of(&[])), QuitDecision::Quit);
    }

    #[test]
    fn one_live_session_asks() {
        assert_eq!(
            decide(plan_of(&[TERMINAL])),
            QuitDecision::Ask(QuitPlanInfo {
                live: 1,
                end_on_quit: 1,
                keep_running: 0,
            })
        );
    }

    #[test]
    fn many_live_sessions_ask_and_say_which_keep_running() {
        let live = [
            TERMINAL,
            TERMINAL,
            LiveSession {
                kind: InstanceKind::Desktop,
                adopted: false,
            },
            LiveSession {
                kind: InstanceKind::Terminal,
                adopted: true,
            },
        ];
        assert_eq!(
            decide(plan_of(&live)),
            QuitDecision::Ask(QuitPlanInfo {
                live: 4,
                end_on_quit: 2,
                keep_running: 2,
            })
        );
    }
}
