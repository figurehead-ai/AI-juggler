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
//!
//! Two fallbacks keep a quit from ever doing nothing (the webview hung,
//! still loading, or never showing the sheet): a second ⌘Q or close
//! while a request waits quits anyway, as macOS apps do; and the window
//! acknowledges the request when its sheet is up — without that within
//! [`ACK_WINDOW`], AI Juggler quits as if Quit anyway was chosen.
//! Logging out and shutting down never ask: macOS sends those as
//! `terminate:`, past the menu, and nothing here prevents
//! `RunEvent::ExitRequested`.

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

/// How long the window has to show the sheet before the quit goes ahead.
pub const ACK_WINDOW: Duration = Duration::from_secs(3);

/// A quit request waiting on the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    /// Counts requests, so a stale timer never quits a later one.
    id: u64,
    acknowledged: bool,
}

/// The quit requests' state: none waiting, or one waiting on the sheet.
#[derive(Debug, Default)]
pub struct QuitGate {
    pending: Option<Pending>,
    next_id: u64,
}

/// What a request (⌘Q or close) does.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RequestAction {
    /// Quit now (nothing live, or a second request while one waits).
    Quit,
    /// Show the sheet; quit anyway if request `id` isn't acknowledged
    /// within [`ACK_WINDOW`].
    Ask {
        /// The request's number (its timer checks it).
        id: u64,
        /// What the sheet says.
        plan: QuitPlanInfo,
    },
}

impl QuitGate {
    /// A ⌘Q or close with this plan.
    #[must_use]
    pub fn request(&mut self, plan: QuitPlanInfo) -> RequestAction {
        if self.pending.is_some() {
            return RequestAction::Quit;
        }
        match decide(plan) {
            QuitDecision::Quit => RequestAction::Quit,
            QuitDecision::Ask(plan) => {
                self.next_id += 1;
                self.pending = Some(Pending {
                    id: self.next_id,
                    acknowledged: false,
                });
                RequestAction::Ask {
                    id: self.next_id,
                    plan,
                }
            }
        }
    }

    /// The window shows the sheet for the waiting request.
    pub fn acknowledge(&mut self) {
        if let Some(pending) = self.pending.as_mut() {
            pending.acknowledged = true;
        }
    }

    /// Cancel on the sheet: nothing waits any more.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// [`ACK_WINDOW`] after request `id`: true when it should quit now
    /// (still waiting, never acknowledged).
    #[must_use]
    pub fn timed_out(&self, id: u64) -> bool {
        matches!(self.pending, Some(Pending { id: waiting, acknowledged: false }) if waiting == id)
    }
}

/// The app's one gate.
static GATE: std::sync::Mutex<QuitGate> = std::sync::Mutex::new(QuitGate {
    pending: None,
    next_id: 0,
});

fn gate() -> std::sync::MutexGuard<'static, QuitGate> {
    GATE.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn quit_now(app: &tauri::AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    app.exit(0);
}

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
    /// Of those, the ones AI Juggler runs without a terminal: they end with
    /// it and come back when it opens again (AJ-E2310).
    pub come_back: u32,
}

/// One live session as quitting sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LiveSession {
    /// The occurrence's kind.
    pub kind: InstanceKind,
    /// Re-adopted after a crash: its terminal isn't ours any more.
    pub adopted: bool,
    /// Driven by AI Juggler (AJ-E2310): it comes back at the next start.
    pub driven: bool,
}

/// The plan for these live sessions.
#[must_use]
pub fn plan_of(live: &[LiveSession]) -> QuitPlanInfo {
    let keeps = |session: &&LiveSession| session.kind == InstanceKind::Desktop || session.adopted;
    let keep = live.iter().filter(keeps).count();
    let back = live
        .iter()
        .filter(|session| !keeps(session) && session.driven)
        .count();
    let total = live.len();
    QuitPlanInfo {
        live: u32::try_from(total).unwrap_or(u32::MAX),
        end_on_quit: u32::try_from(total - keep - back).unwrap_or(u32::MAX),
        keep_running: u32::try_from(keep).unwrap_or(u32::MAX),
        come_back: u32::try_from(back).unwrap_or(u32::MAX),
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
                driven: state.runtime.is_driven(&id),
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
#[must_use]
pub fn request(app: &tauri::AppHandle) -> bool {
    if quitting() {
        return false;
    }
    let state = app.state::<Arc<AppState>>();
    let plan = quit_plan_impl(state.inner());
    let action = gate().request(plan);
    match action {
        RequestAction::Quit => {
            quit_now(app);
            false
        }
        RequestAction::Ask { id, plan } => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            let _ = app.emit(EVENT_QUIT_REQUESTED, plan);
            let app = app.clone();
            std::thread::spawn(move || {
                // One wait for the sheet's acknowledgement, not a poll.
                std::thread::sleep(ACK_WINDOW);
                let timed_out = gate().timed_out(id);
                if timed_out {
                    quit_now(&app);
                }
            });
            true
        }
    }
}

/// The window shows the quit sheet (the request is in a person's hands).
// ipc: main-thread (pure)
#[tauri::command]
pub fn app_quit_ack() {
    gate().acknowledge();
}

/// Cancel on the quit sheet: everything keeps running, and the next ⌘Q
/// asks again.
// ipc: main-thread (pure)
#[tauri::command]
pub fn app_quit_cancel() {
    gate().cancel();
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
///
/// # Errors
///
/// [`AppError::Io`](crate::error::AppError::Io) when the read could not
/// finish.
#[tauri::command]
pub async fn app_quit_plan(state: tauri::State<'_, Arc<AppState>>) -> AppResult<QuitPlanInfo> {
    crate::ipc::blocking(state.inner(), "quit", |state| Ok(quit_plan_impl(state))).await
}

/// The user chose to quit: with `stop_first`, each live session is
/// stopped cleanly (in parallel, the usual 5 s grace each) before AI
/// Juggler quits.
///
/// # Errors
///
/// Never fails; the result type matches the other commands.
// ipc: main-thread (pure: the stops and the exit run on their own thread)
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
        driven: false,
    };

    /// Each kind is pinned to what quitting does to it: a terminal
    /// session ends (its terminal lives in this app); a desktop session
    /// and a re-adopted one keep running.
    #[test]
    fn each_kind_is_pinned_to_what_quitting_does() {
        let alone = |kind, adopted| {
            plan_of(&[LiveSession {
                kind,
                adopted,
                driven: false,
            }])
        };
        let ends = QuitPlanInfo {
            live: 1,
            end_on_quit: 1,
            keep_running: 0,
            come_back: 0,
        };
        let keeps = QuitPlanInfo {
            live: 1,
            end_on_quit: 0,
            keep_running: 1,
            come_back: 0,
        };
        assert_eq!(alone(InstanceKind::Terminal, false), ends);
        assert_eq!(alone(InstanceKind::Desktop, false), keeps);
        assert_eq!(alone(InstanceKind::Terminal, true), keeps);
        assert_eq!(alone(InstanceKind::Desktop, true), keeps);
    }

    fn asked(action: RequestAction) -> u64 {
        match action {
            RequestAction::Ask { id, .. } => Some(id),
            RequestAction::Quit => None,
        }
        .expect("asks")
    }

    const ONE: QuitPlanInfo = QuitPlanInfo {
        live: 1,
        end_on_quit: 1,
        keep_running: 0,
        come_back: 0,
    };

    /// Fallback (a): a second ⌘Q or close while the sheet is pending
    /// quits anyway; after Cancel the next one asks again.
    #[test]
    fn a_second_request_while_one_waits_quits_anyway() {
        let mut gate = QuitGate::default();
        assert!(matches!(gate.request(ONE), RequestAction::Ask { .. }));
        gate.acknowledge();
        assert_eq!(gate.request(ONE), RequestAction::Quit);
        let mut gate = QuitGate::default();
        let _ = gate.request(ONE);
        gate.cancel();
        assert!(
            matches!(gate.request(ONE), RequestAction::Ask { .. }),
            "asks again after Cancel"
        );
    }

    /// Fallback (b): unacknowledged after the window, it quits; once the
    /// sheet acknowledges, the timer leaves it to the person; a stale
    /// timer never quits a later request.
    #[test]
    fn an_unacknowledged_request_quits_after_the_window() {
        let mut gate = QuitGate::default();
        let id = asked(gate.request(ONE));
        assert!(gate.timed_out(id), "no sheet: quit as if Quit anyway");

        let mut gate = QuitGate::default();
        let id = asked(gate.request(ONE));
        gate.acknowledge();
        assert!(!gate.timed_out(id), "the sheet is up: the person decides");

        let mut gate = QuitGate::default();
        let first = asked(gate.request(ONE));
        gate.cancel();
        let second = asked(gate.request(ONE));
        assert!(
            !gate.timed_out(first),
            "the cancelled request's timer is stale"
        );
        assert!(gate.timed_out(second));
        assert_eq!(
            QuitGate::default().request(QuitPlanInfo::default()),
            RequestAction::Quit,
            "nothing live"
        );
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
                come_back: 0,
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
                driven: false,
            },
            LiveSession {
                kind: InstanceKind::Terminal,
                adopted: true,
                driven: false,
            },
        ];
        assert_eq!(
            decide(plan_of(&live)),
            QuitDecision::Ask(QuitPlanInfo {
                live: 4,
                end_on_quit: 2,
                keep_running: 2,
                come_back: 0,
            })
        );
    }

    /// AJ-E2310: a driven session ends with AI Juggler and comes back at
    /// the next start; the sheet says so, apart from the ones that end.
    #[test]
    fn a_driven_session_comes_back_and_the_plan_says_so() {
        let driven = LiveSession {
            kind: InstanceKind::Terminal,
            adopted: false,
            driven: true,
        };
        assert_eq!(
            plan_of(&[TERMINAL, driven]),
            QuitPlanInfo {
                live: 2,
                end_on_quit: 1,
                keep_running: 0,
                come_back: 1,
            }
        );
    }
}
