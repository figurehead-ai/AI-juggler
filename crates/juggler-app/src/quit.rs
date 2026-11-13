//! AJ-E1503 Req.3 — quitting with live sessions says so first.
//!
//! ⌘Q (the app menu's own Quit item) asks the backend; with nothing live
//! AI Juggler quits at once, otherwise the window is told how many
//! sessions are live and what quitting does to them. D-112 (0.4.5): the
//! main window's close button and ⌘W never quit: they hide the window
//! ([`leaving`]; `main_window.rs` hides it), and every session keeps
//! running. A terminal session runs on a terminal this app holds, so it ends
//! when AI Juggler quits; a desktop session (its own app) or a session
//! re-adopted after a crash keeps running. The user picks Quit anyway,
//! Stop them first (a clean stop of each, then quit) or Cancel. Nothing is
//! stopped without that click (ADR-0003).
//!
//! Two fallbacks keep a quit from ever doing nothing (the webview hung,
//! still loading, or never showing the sheet): a second ⌘Q while a
//! request waits quits anyway, as macOS apps do; and the window
//! acknowledges the request when its sheet is up — without that within
//! [`ACK_WINDOW`] of the window showing, AI Juggler quits as if Quit
//! anyway was chosen.
//!
//! D-111 (0.4.5): the ack window starts only once the main window shows
//! (a request brings it forward). A request whose window never shows (a
//! test's hidden launch; before D-112, a close pressed on a hidden window
//! by something other than the person, an accessibility client, a
//! script) never quits by the timer: live sessions would end with nobody
//! having seen the sheet (ADR-0003). It waits, logged; the sheet comes up
//! when the window shows, and a second ⌘Q still quits. Every path is
//! logged, so a quit nobody chose can be told apart afterwards.
//! D-113 (0.4.5): the quit Apple event has a handler of our own
//! ([`answer_quit_events`]). One without a reason (the Dock's Quit,
//! Activity Monitor's, a script's `quit`) goes through [`request`], as ⌘Q
//! does; logging out, restarting and shutting down (the event says why),
//! or any while quitting, terminate at once and never ask. If the handler
//! can't be put in place, the system's own stays and quits without asking.

// Tauri command signatures take State by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use crate::events::{AppEvent, EmitEvent as _};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use juggler_core::model::InstanceKind;
use tauri::Manager;

use crate::error::AppResult;
use crate::state::AppState;

/// The event asking the window to show the quit sheet; payload
/// [`QuitPlanInfo`].
pub const EVENT_QUIT_REQUESTED: &str = AppEvent::QuitRequested.name();

/// The app menu's Quit item (replaces the predefined one, which quits
/// without asking anyone).
pub const QUIT_MENU_ID: &str = "aij-quit";
/// Help › Report a bug…'s id.
pub const REPORT_MENU_ID: &str = "aij-report-bug";
/// The main window's event: open Report a bug.
pub const EVENT_REPORT_BUG: &str = AppEvent::ReportBug.name();

/// Set once the user chose to quit: the window closing on the way out
/// must not ask again.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// How long a shown window has to show the sheet before the quit goes
/// ahead.
pub const ACK_WINDOW: Duration = Duration::from_secs(3);

/// How long a request waits for the main window to show (it brings it
/// forward) before the ack window would start.
pub const SHOW_WINDOW: Duration = Duration::from_secs(3);

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

/// What the ack window's end does to a waiting request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AfterWait {
    /// The window showed and its sheet never answered (it hung): quit as
    /// if Quit anyway was chosen.
    Quit,
    /// The window never showed: nobody saw the sheet, so nothing quits;
    /// the request keeps waiting (a second ⌘Q still quits).
    Wait,
    /// Answered, cancelled, or a later request: nothing to do.
    Done,
}

/// What a quit request (⌘Q) does.
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
    /// A ⌘Q with this plan.
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

    /// The ack window's end for request `id`, its window `shown` or not
    /// (D-111): quit only when a shown window never answered.
    #[must_use]
    pub fn after_ack_window(&self, id: u64, shown: bool) -> AfterWait {
        match self.pending {
            Some(Pending {
                id: waiting,
                acknowledged: false,
            }) if waiting == id => {
                if shown {
                    AfterWait::Quit
                } else {
                    AfterWait::Wait
                }
            }
            _ => AfterWait::Done,
        }
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

fn quit_now<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    QUITTING.store(true, Ordering::SeqCst);
    app.exit(0);
}

/// Waits up to [`SHOW_WINDOW`] for the main window to show; true once it
/// has. A short bounded look (the window answers no event when shown by
/// another app's hand).
fn wait_until_shown<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    loop {
        if crate::main_window::on_screen(app) {
            return true;
        }
        if waited >= SHOW_WINDOW {
            return false;
        }
        std::thread::sleep(step);
        waited += step;
    }
}

/// The system's quit signals (TERM at log out, restart and shut down, or
/// a `kill`; INT; HUP) quit as asked: the run is marked clean on the way
/// out, so the watchdog (AJ-E1509) never reopens AI Juggler after one.
/// Only a crash (SEGV, BUS, ABRT, ILL, KILL) leaves the run unclean. No
/// sheet: nobody may be there to answer it.
pub fn on_quit_signals(app: &tauri::AppHandle) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        for kind in [
            SignalKind::terminate(),
            SignalKind::interrupt(),
            SignalKind::hangup(),
        ] {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let Ok(mut heard) = signal(kind) else {
                    return;
                };
                if heard.recv().await.is_some() {
                    tracing::info!("a quit signal: quitting as asked");
                    quit_now(&app);
                }
            });
        }
    }
    #[cfg(not(unix))]
    let _ = app;
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
    /// rc30: each one, by its app, with what quitting does to it (the
    /// sheet said "They end when AI Juggler quits" and a driven Codex
    /// session came back).
    pub sessions: Vec<QuitSessionInfo>,
}

/// One live session on the quit sheet: its app, and what quitting does
/// to it.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QuitSessionInfo {
    /// Its occurrence's kind (`terminal`, `desktop`).
    pub kind: String,
    /// Its tool.
    pub provider: String,
    /// Its account's label.
    pub account_label: String,
    /// `ends` | `keeps_running` | `comes_back`.
    pub fate: String,
}

/// What quitting does to one live session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fate {
    /// It ends with AI Juggler, and nothing brings it back.
    Ends,
    /// It keeps running on its own.
    KeepsRunning,
    /// It ends with AI Juggler and comes back when it opens again.
    ComesBack,
}

impl Fate {
    /// Its wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ends => "ends",
            Self::KeepsRunning => "keeps_running",
            Self::ComesBack => "comes_back",
        }
    }
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

impl LiveSession {
    /// What quitting does to it.
    #[must_use]
    pub fn fate(&self) -> Fate {
        if self.kind == InstanceKind::Desktop || self.adopted {
            Fate::KeepsRunning
        } else if self.driven {
            Fate::ComesBack
        } else {
            Fate::Ends
        }
    }
}

/// The plan for these live sessions.
#[must_use]
pub fn plan_of(live: &[LiveSession]) -> QuitPlanInfo {
    let count = |fate: Fate| {
        u32::try_from(live.iter().filter(|session| session.fate() == fate).count())
            .unwrap_or(u32::MAX)
    };
    QuitPlanInfo {
        live: u32::try_from(live.len()).unwrap_or(u32::MAX),
        end_on_quit: count(Fate::Ends),
        keep_running: count(Fate::KeepsRunning),
        come_back: count(Fate::ComesBack),
        sessions: Vec::new(),
    }
}

/// The sessions this app supervises that are still running, with what
/// the next start `restores`.
fn live_sessions(
    state: &Arc<AppState>,
    restores: crate::features::app::crash::Restores,
) -> Vec<(juggler_core::model::SessionId, LiveSession)> {
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
            if crate::features::sessions::read_model::is_terminal(record.status) {
                return None;
            }
            // It comes back only when its conversation is kept to bring
            // back (its mark) and the next start brings it back (not a
            // test run): the rc29 walk's promise must hold.
            let session = LiveSession {
                kind: record.kind,
                adopted: adopted.contains(&id),
                driven: state.runtime.is_driven(&id)
                    && crate::features::runtime::pickup::is_marked(&storage, &id)
                    && restores.driven,
            };
            Some((id, session))
        })
        .collect()
}

/// The quit plan for what runs now, each session by its app.
#[must_use]
pub fn quit_plan_impl(state: &Arc<AppState>) -> QuitPlanInfo {
    quit_plan_with(state, crate::features::app::crash::Restores::this_run())
}

/// [`quit_plan_impl`] for a start that `restores` this.
#[must_use]
pub fn quit_plan_with(
    state: &Arc<AppState>,
    restores: crate::features::app::crash::Restores,
) -> QuitPlanInfo {
    let live = live_sessions(state, restores);
    let mut plan = plan_of(&live.iter().map(|(_, session)| *session).collect::<Vec<_>>());
    let Ok(storage) = crate::commands::lock_storage_arc(state) else {
        return plan;
    };
    plan.sessions = live
        .iter()
        .map(|(id, session)| {
            let account = storage
                .sessions()
                .get(id)
                .ok()
                .and_then(|record| storage.profiles().get(&record.profile).ok())
                .and_then(|profile| storage.accounts().get(&profile.account).ok());
            QuitSessionInfo {
                kind: session.kind.as_str().to_owned(),
                provider: account
                    .as_ref()
                    .map(|account| account.harness.to_string())
                    .unwrap_or_default(),
                account_label: account.map(|account| account.label).unwrap_or_default(),
                fate: session.fate().as_str().to_owned(),
            }
        })
        .collect();
    plan
}

/// Quit anyway keeps the sheet's word: a session it said ends is never
/// brought back by a later start (rc30: a test run's sheet said "They end
/// when AI Juggler quits", and a plain reopen relaunched its driven
/// Codex session from the mark it left).
pub fn keep_the_sheets_word(state: &Arc<AppState>) {
    keep_the_sheets_word_with(state, crate::features::app::crash::Restores::this_run());
}

/// [`keep_the_sheets_word`] for a start that `restores` this.
pub fn keep_the_sheets_word_with(
    state: &Arc<AppState>,
    restores: crate::features::app::crash::Restores,
) {
    let ending: Vec<juggler_core::model::SessionId> = live_sessions(state, restores)
        .into_iter()
        .filter(|(_, session)| session.fate() == Fate::Ends)
        .map(|(id, _)| id)
        .collect();
    let Ok(storage) = crate::commands::lock_storage_arc(state) else {
        return;
    };
    for id in &ending {
        crate::features::runtime::pickup::note(&storage, id, None);
    }
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

/// How the person asked the main window to go (D-112).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leave {
    /// Its close button, or ⌘W (the Window menu's Close Window).
    Close,
    /// ⌘Q (the app menu's Quit).
    Quit,
}

/// What a [`Leave`] does.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Leaving {
    /// AI Juggler is on its way out: the close goes through, nothing asks.
    Through,
    /// The main window hides; nothing quits and every session keeps
    /// running, live or not (D-112).
    Hide,
    /// Quit now: nothing live, or a second ⌘Q while one waits.
    Quit,
    /// Show the sheet; quit anyway if request `id` isn't acknowledged
    /// within [`ACK_WINDOW`] of the window showing.
    Ask {
        /// The request's number (its timer checks it).
        id: u64,
        /// What the sheet says.
        plan: QuitPlanInfo,
    },
}

/// What `how` does while `quitting` or not: a close hides the window and
/// never reaches the quit request, whatever is live; a Quit asks `gate`
/// with the `plan` (read only for a Quit).
pub fn leaving(
    how: Leave,
    quitting: bool,
    gate: &mut QuitGate,
    plan: impl FnOnce() -> QuitPlanInfo,
) -> Leaving {
    if quitting {
        return Leaving::Through;
    }
    match how {
        Leave::Close => Leaving::Hide,
        Leave::Quit => match gate.request(plan()) {
            RequestAction::Quit => Leaving::Quit,
            RequestAction::Ask { id, plan } => Leaving::Ask { id, plan },
        },
    }
}

/// The main window's close button or ⌘W: true when it hides the window
/// (D-112), false on the way out (the close goes through).
#[must_use]
pub fn close_hides() -> bool {
    leaving(Leave::Close, quitting(), &mut gate(), QuitPlanInfo::default) == Leaving::Hide
}

/// ⌘Q or the menu-bar item's Quit: quit now, or ask the window. Returns
/// true when the request was turned into a question.
#[must_use]
pub fn request<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    if quitting() {
        return false;
    }
    let state = app.state::<Arc<AppState>>();
    let plan = quit_plan_impl(state.inner());
    let live = plan.live;
    // Not on the way out (above); the gate's lock ends with this line.
    let answer = leaving(Leave::Quit, false, &mut gate(), || plan);
    match answer {
        Leaving::Through | Leaving::Hide => false,
        Leaving::Quit => {
            tracing::info!(
                live,
                "quit requested: quitting now (nothing live, or asked twice)"
            );
            quit_now(app);
            false
        }
        Leaving::Ask { id, plan } => {
            tracing::info!(live, "quit requested: asking first");
            crate::main_window::bring_forward(app);
            app.emit_event(AppEvent::QuitRequested, plan);
            let app = app.clone();
            std::thread::spawn(move || {
                // D-111: the ack window starts once the window shows.
                let shown = wait_until_shown(&app);
                if shown {
                    std::thread::sleep(ACK_WINDOW);
                }
                match gate().after_ack_window(id, shown) {
                    AfterWait::Quit => {
                        tracing::warn!(
                            "quit: the shown window never answered its sheet; quitting as if Quit anyway"
                        );
                        quit_now(&app);
                    }
                    AfterWait::Wait => tracing::warn!(
                        "quit: the window didn't show, so nobody saw the sheet; nothing quits (a second Quit still quits)"
                    ),
                    AfterWait::Done => {}
                }
            });
            true
        }
    }
}

/// The quit Apple event as AI Juggler answers it (D-113).
struct QuitEventsOf<R: tauri::Runtime>(tauri::AppHandle<R>);

impl<R: tauri::Runtime> juggler_window_events::QuitEvents for QuitEventsOf<R> {
    fn quitting(&self) -> bool {
        quitting()
    }

    fn ask(&self) {
        tracing::info!(
            "a quit Apple event without a reason (the Dock's Quit, Activity Monitor, a script): asking as ⌘Q does"
        );
        let _ = request(&self.0);
    }

    fn terminating(&self, reason: Option<u32>) {
        let reason = reason.map(|code| String::from_utf8_lossy(&code.to_be_bytes()).into_owned());
        tracing::info!(
            ?reason,
            "a quit Apple event with a reason (log out, restart, shut down), or while quitting: quitting as asked"
        );
    }
}

/// D-113: the quit Apple event from now on asks as ⌘Q does when it gives
/// no reason, and quits at once when it does (log out, restart, shut
/// down). Call once, on the main thread, once the app has launched
/// (`RunEvent::Ready`: AppKit puts its own handler in place as it
/// launches). If it can't be put in place, the system's own stays.
pub fn answer_quit_events<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    match juggler_window_events::answer_quit_events(QuitEventsOf(app.clone())) {
        Ok(()) => tracing::info!("the quit Apple event: asks first without a reason (D-113)"),
        Err(error) => tracing::warn!(
            "the quit Apple event keeps the system's handler, which quits without asking: {error}"
        ),
    }
}

/// The window shows the quit sheet (the request is in a person's hands).
#[tauri::command]
pub async fn app_quit_ack() {
    gate().acknowledge();
}

/// Cancel on the quit sheet: everything keeps running, and the next ⌘Q
/// asks again.
#[tauri::command]
pub async fn app_quit_cancel() {
    gate().cancel();
}

/// The Quit item's words, in the app menu and the menu-bar item's menu:
/// "Quit" and this edition's name.
#[must_use]
pub fn quit_words() -> String {
    format!("Quit {}", crate::edition::current().app_name())
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
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, quit_words(), true, Some("CmdOrCtrl+Q"))?;
    // ADR-0026: Window › The Floor (⌘⇧M, which also brings the main
    // window back from the Floor).
    let floor = MenuItem::with_id(
        app,
        crate::features::floor::window::MENU_ID,
        "The Floor",
        true,
        Some("CmdOrCtrl+Shift+M"),
    )?;
    // AJ-E2911: Help › Report a bug… (the window opens its sheet).
    let report = MenuItem::with_id(app, REPORT_MENU_ID, "Report a bug…", true, None::<&str>)?;
    for item in menu.items()? {
        if let MenuItemKind::Submenu(submenu) = item {
            match submenu.text()?.as_str() {
                "Window" => submenu.append(&floor)?,
                "Help" => submenu.append(&report)?,
                _ => {}
            }
        }
    }
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
#[tauri::command]
pub async fn app_quit(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    stop_first: bool,
) -> AppResult<()> {
    QUITTING.store(true, Ordering::SeqCst);
    let state = Arc::clone(state.inner());
    std::thread::spawn(move || {
        if stop_first {
            let stops: Vec<_> =
                live_sessions(&state, crate::features::app::crash::Restores::this_run())
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
        } else {
            keep_the_sheets_word(&state);
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
            sessions: Vec::new(),
        };
        let keeps = QuitPlanInfo {
            live: 1,
            end_on_quit: 0,
            keep_running: 1,
            come_back: 0,
            sessions: Vec::new(),
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
        sessions: Vec::new(),
    };

    /// Fallback (a): a second ⌘Q while the sheet is pending quits
    /// anyway; after Cancel the next one asks again.
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

    /// Fallback (b): a shown window whose sheet never answered (a hung
    /// webview) quits after the window; once the sheet acknowledges, the
    /// timer leaves it to the person; a stale timer never quits a later
    /// request.
    #[test]
    fn a_shown_window_that_never_answers_quits_after_the_window() {
        let mut gate = QuitGate::default();
        let id = asked(gate.request(ONE));
        assert_eq!(
            gate.after_ack_window(id, true),
            AfterWait::Quit,
            "shown, no sheet: quit as if Quit anyway"
        );

        let mut gate = QuitGate::default();
        let id = asked(gate.request(ONE));
        gate.acknowledge();
        assert_eq!(
            gate.after_ack_window(id, true),
            AfterWait::Done,
            "the sheet is up: the person decides"
        );

        let mut gate = QuitGate::default();
        let first = asked(gate.request(ONE));
        gate.cancel();
        let second = asked(gate.request(ONE));
        assert_eq!(
            gate.after_ack_window(first, true),
            AfterWait::Done,
            "the cancelled request's timer is stale"
        );
        assert_eq!(gate.after_ack_window(second, true), AfterWait::Quit);
        assert_eq!(
            QuitGate::default().request(QuitPlanInfo::default()),
            RequestAction::Quit,
            "nothing live"
        );
    }

    /// D-111: a request whose window never showed (a test's hidden
    /// launch; before D-112, a close pressed on a hidden window by an
    /// accessibility client or a script) with sessions live, its sheet
    /// never answered: nothing quits, and the request still waits, so a
    /// second ⌘Q quits as before.
    #[test]
    fn a_hidden_window_with_live_sessions_never_quits_unanswered() {
        let mut gate = QuitGate::default();
        let id = asked(gate.request(ONE));
        assert_eq!(gate.after_ack_window(id, false), AfterWait::Wait);
        assert_eq!(
            gate.request(ONE),
            RequestAction::Quit,
            "a second request while it waits still quits"
        );
    }

    /// D-111: with nothing live, ⌘Q quits at once, shown or hidden (the
    /// gate never asks the window).
    #[test]
    fn a_hidden_window_with_nothing_live_quits_at_once() {
        assert_eq!(
            QuitGate::default().request(plan_of(&[])),
            RequestAction::Quit
        );
    }

    /// D-112: the close button and ⌘W hide the window, with sessions live
    /// or none: nothing quits, no sheet, and no quit request is left
    /// waiting (a later ⌘Q asks, never "asked twice: quit").
    #[test]
    fn a_close_hides_and_never_quits_even_with_nothing_live() {
        for plan in [plan_of(&[]), ONE] {
            let mut gate = QuitGate::default();
            assert_eq!(
                leaving(Leave::Close, false, &mut gate, || plan.clone()),
                Leaving::Hide
            );
            assert_eq!(
                leaving(Leave::Close, false, &mut gate, || plan.clone()),
                Leaving::Hide,
                "a second close hides too"
            );
            assert!(
                matches!(
                    leaving(Leave::Quit, false, &mut gate, || ONE),
                    Leaving::Ask { .. }
                ),
                "the closes left no request waiting"
            );
        }
    }

    /// On the way out the closing window goes through, and a ⌘Q then
    /// asks nothing.
    #[test]
    fn on_the_way_out_a_close_goes_through() {
        let mut gate = QuitGate::default();
        assert_eq!(
            leaving(Leave::Close, true, &mut gate, || ONE),
            Leaving::Through
        );
        assert_eq!(
            leaving(Leave::Quit, true, &mut gate, || ONE),
            Leaving::Through
        );
    }

    /// ⌘Q with nothing live quits at once (D-112 keeps D-111's rules).
    #[test]
    fn cmd_q_with_nothing_live_quits() {
        assert_eq!(
            leaving(
                Leave::Quit,
                false,
                &mut QuitGate::default(),
                || plan_of(&[])
            ),
            Leaving::Quit
        );
    }

    /// ⌘Q with live sessions shows the sheet, with what quitting does to
    /// them; a second ⌘Q while it waits quits anyway.
    #[test]
    fn cmd_q_with_live_sessions_asks() {
        let mut gate = QuitGate::default();
        let asked = leaving(Leave::Quit, false, &mut gate, || ONE);
        assert!(
            matches!(&asked, Leaving::Ask { plan, .. } if *plan == ONE),
            "asks, with what quitting does: {asked:?}"
        );
        assert_eq!(
            leaving(Leave::Quit, false, &mut gate, || ONE),
            Leaving::Quit
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
                sessions: Vec::new(),
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
                sessions: Vec::new(),
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
                sessions: Vec::new(),
            }
        );
    }
}
