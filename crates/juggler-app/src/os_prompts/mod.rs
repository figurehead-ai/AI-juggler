//! AJ-E1547 (PAT-OSP): the app's half of the one gate for system prompts
//! (the seam is [`juggler_platform::os_prompt`]).
//!
//! - [`registry`]: every prompt AI Juggler can cause, with its id, why,
//!   what happens when declined and every code path that triggers it.
//! - The **ledger**: whether each entry is primed, one app setting per
//!   entry (`os_prompt_primed:<id>`), with the marks earlier versions left
//!   (an entry the user already went through isn't primed twice).
//! - The **primer**: the window's sheet. Asked for an entry not primed
//!   yet, it sends `os-prompt://ask` and waits for `os_prompt_answer`
//!   (Not now after [`ASK_WAIT`]); one sheet per entry at a time, however
//!   many calls wait on it. The outcome follows as `os-prompt://outcome`.
//! - **IPC** for setup's "Let AI Juggler work with…" step:
//!   [`os_prompts_list`], [`os_prompt_prime`] (the card already showed the
//!   dialog, so it primes and makes the call), [`os_prompt_status`].
//!
//! Every call that can make the system ask is in here or in the seam
//! (`scripts/os_prompt_lint.py`).

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri command signatures take State and values by value (as in commands.rs)"
)]

pub mod needs;
pub mod notify;
pub mod registry;

use std::collections::HashMap;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_platform::os_prompt::{
    self, Answer, Gated, Kind, Ledger, Outcome, Presence, Primer, Prompt, Status,
};

use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EmitEvent};
use crate::state::AppState;
use registry::Entry;

/// One system prompt, as the window shows it (setup's cards, the sheet).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OsPromptInfo {
    /// The registry's short, stable id.
    pub id: String,
    /// What it makes possible, as the user would say it.
    pub title: String,
    /// Why AI Juggler needs it.
    pub why: String,
    /// What the system's dialog says, in its words (title, then text).
    pub system_says: String,
    /// What to click in it.
    pub click: String,
    /// What happens if the user declines.
    pub if_declined: String,
    /// This setup can use it (D-127: only these are offered).
    pub needed: bool,
    /// `allowed` | `not_allowed` | `not_asked` | `asked` (the system
    /// won't say what was answered).
    pub status: String,
    /// The user was told and said go.
    pub primed: bool,
    /// Where the answer can be changed later, in the system's words.
    pub change_later: Option<String>,
    /// What background work does until it's primed.
    pub background: String,
    /// Built never to be shown (listed so a walk knows it's a bug).
    pub never_shown: bool,
    /// The system's dialog, for the picture the sheet and cards draw.
    pub dialog: Option<OsPromptDialogInfo>,
}

/// The system's dialog in its own words: what the picture is drawn from.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OsPromptDialogInfo {
    /// `alert` (waits for an answer) | `banner` (a notice in the corner).
    pub style: String,
    /// Its title line.
    pub title: String,
    /// Its text under the title (may be empty).
    pub body: String,
    /// Its buttons, left to right.
    pub buttons: Vec<String>,
    /// The button that says yes (empty for a notice that asks nothing).
    pub click: String,
}

/// What came of priming an entry.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OsPromptOutcomeInfo {
    /// The entry.
    pub id: String,
    /// `called` (the system may have asked) | `not_now` (nothing was
    /// asked; the next click asks again) | `waiting` (nobody was there) |
    /// `already` (primed or decided before: nothing was asked).
    pub result: String,
    /// What the system says now (the info's `status` words).
    pub status: String,
    /// Said plainly: granted, or declined with how to change it later.
    pub words: String,
}

/// How long the sheet may stay unanswered before it counts as Not now.
pub const ASK_WAIT: Duration = Duration::from_secs(600);

const PRIMED_PREFIX: &str = "os_prompt_primed:";

/// The marks earlier versions left that say the user already went
/// through an entry's prompt (so it isn't primed a second time).
fn primed_before(state: &AppState, id: &str) -> bool {
    let on = |key: &str| state.app_setting(key).as_deref() == Some("1");
    match id {
        "accessibility" | "automation-system-events" => on("os_access_asked:focus"),
        "screen-recording" => on("os_access_asked:preview"),
        "local-network" => on("fleet_link_enabled"),
        "notifications" => crate::features::notifications::prefs::any_turned_on(state),
        _ => false,
    }
}

/// The app's ledger: one setting per entry.
pub struct AppLedger(pub Arc<AppState>);

impl Ledger for AppLedger {
    fn primed(&self, id: &str) -> bool {
        self.0
            .app_setting(&format!("{PRIMED_PREFIX}{id}"))
            .as_deref()
            == Some("1")
            || primed_before(&self.0, id)
    }

    fn set_primed(&self, id: &str) {
        if let Ok(storage) = self.0.storage.lock() {
            let _ = storage
                .app_settings()
                .set(&format!("{PRIMED_PREFIX}{id}"), "1");
        }
    }
}

/// The window's sheet.
pub struct WindowPrimer {
    app: tauri::AppHandle,
    state: Arc<AppState>,
    /// The calls waiting on each entry's open sheet.
    waiting: Mutex<HashMap<String, Vec<Sender<bool>>>>,
}

impl WindowPrimer {
    /// The primer for `app`'s window.
    #[must_use]
    pub fn new(app: tauri::AppHandle, state: Arc<AppState>) -> Self {
        Self {
            app,
            state,
            waiting: Mutex::new(HashMap::new()),
        }
    }

    /// The window's answer for the sheet of `id`.
    pub fn answer(&self, id: &str, go: bool) {
        let senders = self
            .waiting
            .lock()
            .ok()
            .and_then(|mut waiting| waiting.remove(id))
            .unwrap_or_default();
        for sender in senders {
            let _ = sender.send(go);
        }
    }
}

impl Primer for WindowPrimer {
    fn ask(&self, prompt: Prompt) -> Answer {
        let Some(entry) = registry::entry(prompt.id) else {
            return Answer::NotNow;
        };
        let (send, answer) = channel();
        let first = {
            let Ok(mut waiting) = self.waiting.lock() else {
                return Answer::NotNow;
            };
            let calls = waiting.entry(prompt.id.to_owned()).or_default();
            calls.push(send);
            calls.len() == 1
        };
        // One sheet per entry: a second call waits on the one shown.
        if first {
            crate::main_window::bring_forward(&self.app);
            self.app
                .emit_event(AppEvent::OsPromptAsk, info(&self.state, entry));
        }
        if answer.recv_timeout(ASK_WAIT) == Ok(true) {
            return Answer::Continue;
        }
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.remove(prompt.id);
        }
        Answer::NotNow
    }

    fn told(&self, prompt: Prompt, outcome: &Outcome) {
        self.app.emit_event(
            AppEvent::OsPromptOutcome,
            outcome_info(prompt.id, "called", outcome),
        );
    }
}

static PRIMER: std::sync::OnceLock<Arc<WindowPrimer>> = std::sync::OnceLock::new();

/// A primer the seam holds by box, sharing the one [`PRIMER`].
struct Shared(Arc<WindowPrimer>);

impl Primer for Shared {
    fn ask(&self, prompt: Prompt) -> Answer {
        self.0.ask(prompt)
    }

    fn told(&self, prompt: Prompt, outcome: &Outcome) {
        self.0.told(prompt, outcome);
    }
}

/// Installs the ledger and the window's sheet into the seam, once, at
/// start (before anything can ask).
pub fn install(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let primer = Arc::new(WindowPrimer::new(app.clone(), Arc::clone(state)));
    let _ = PRIMER.set(Arc::clone(&primer));
    os_prompt::install(
        Box::new(AppLedger(Arc::clone(state))),
        Box::new(Shared(primer)),
    );
}

fn home() -> Option<std::path::PathBuf> {
    dirs::home_dir()
}

/// Whether a background read of `path` may happen now (PAT-OSP-3): not in
/// a protected folder or another app's data, or its entry is primed.
#[must_use]
pub fn may_read(path: &std::path::Path) -> bool {
    home().is_none_or(|home| {
        os_prompt::may_read(path, &home, &registry::folder_id, registry::APP_DATA.id)
    })
}

/// Whether `command` may start now (PAT-OSP-3): it isn't one of AI
/// Juggler's own helpers (beside the app's executable, as the program or
/// a program it runs), or the `helper` entry is primed (Bot's setup
/// guide's check, or setup's Meet Bot). A first start may make the system ask ("differs
/// from previously opened versions"), and a probe nobody clicked never
/// may. Called by the helpers' one client right before it spawns.
#[must_use]
pub fn helper_may_start(command: &std::process::Command) -> bool {
    let ours = std::iter::once(command.get_program())
        .chain(command.get_args())
        .any(|part| juggler_platform::code_trust::beside_app(std::path::Path::new(part)));
    !ours
        || matches!(
            os_prompt::gate().through(registry::HELPER, Presence::Background, &|| None, || ()),
            Gated::Called { .. }
        )
}

/// What the system says of `prompt` now, never asking.
#[must_use]
pub fn status_of(state: &Arc<AppState>, prompt: Prompt) -> Status {
    let primed = os_prompt::gate().primed(prompt.id);
    let unknown = if primed {
        Status::Asked
    } else {
        Status::NotAsked
    };
    let said_no = if primed {
        Status::NotAllowed
    } else {
        Status::NotAsked
    };
    match prompt.kind {
        Kind::Accessibility => match os_prompt::accessibility_status() {
            Status::Allowed => Status::Allowed,
            _ => said_no,
        },
        Kind::ScreenRecording => match os_prompt::screen_recording_status() {
            Status::Allowed => Status::Allowed,
            _ => said_no,
        },
        // 0.5.1 F23: System Events through the one read Settings and Go
        // to it use.
        Kind::Automation(_) if prompt.id == registry::SYSTEM_EVENTS.id => {
            crate::os_access::system_events_status(state).unwrap_or(unknown)
        }
        Kind::Automation(target) => {
            os_prompt::automation_status(target.bundle_id).unwrap_or(unknown)
        }
        Kind::LoginItem => match os_prompt::login_item_status() {
            juggler_login_item::LoginItem::On => Status::Allowed,
            juggler_login_item::LoginItem::NeedsApproval => Status::NotAllowed,
            juggler_login_item::LoginItem::Off | juggler_login_item::LoginItem::Unavailable => {
                unknown
            }
        },
        Kind::Folder(folder) if primed => home()
            .map(|home| std::fs::read_dir(home.join(folder.name())))
            .map_or(Status::Asked, |read| match read {
                Ok(_) => Status::Allowed,
                Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                    Status::NotAllowed
                }
                Err(_) => Status::Asked,
            }),
        Kind::Keychain | Kind::FirstOpen => Status::Allowed,
        Kind::Folder(_)
        | Kind::Notifications
        | Kind::LocalNetwork
        | Kind::AppData
        | Kind::Differs => unknown,
    }
}

/// An entry as the window shows it.
#[must_use]
pub fn info(state: &Arc<AppState>, entry: &Entry) -> OsPromptInfo {
    let app = crate::edition::current().app_name();
    let dialog = os_prompt::dialog(entry.prompt.kind, app);
    let place = os_prompt::settings_place(entry.prompt.kind);
    OsPromptInfo {
        id: entry.prompt.id.to_owned(),
        title: entry.title.to_owned(),
        why: entry.why.to_owned(),
        system_says: dialog.as_ref().map_or_else(String::new, |dialog| {
            [dialog.title.as_str(), dialog.body.as_str()]
                .iter()
                .filter(|part| !part.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
        }),
        click: dialog
            .as_ref()
            .map_or_else(String::new, |dialog| click_words(dialog.click, place)),
        if_declined: entry.if_declined.to_owned(),
        needed: !entry.never_shown && (entry.needed)(state),
        status: status_of(state, entry.prompt).as_str().to_owned(),
        primed: os_prompt::gate().primed(entry.prompt.id),
        change_later: place.map(str::to_owned),
        background: entry.background.to_owned(),
        never_shown: entry.never_shown,
        dialog: dialog.map(|dialog| OsPromptDialogInfo {
            style: match dialog.style {
                os_prompt::Style::Alert => "alert",
                os_prompt::Style::Banner => "banner",
            }
            .to_owned(),
            title: dialog.title,
            body: dialog.body,
            buttons: dialog.buttons.iter().map(|b| (*b).to_owned()).collect(),
            click: dialog.click.to_owned(),
        }),
    }
}

/// What to click, in words: the button, and for a dialog that sends the
/// user to the settings, the switch to turn on there.
#[must_use]
pub fn click_words(click: &str, place: Option<&str>) -> String {
    match (click, place) {
        ("", _) => "Nothing: it's a notice.".to_owned(),
        ("Open System Settings", Some(place)) => format!(
            "Click Open System Settings, then turn on {} in {place}.",
            crate::edition::current().app_name()
        ),
        (click, _) => format!("Click {click}."),
    }
}

fn outcome_info(id: &str, result: &str, outcome: &Outcome) -> OsPromptOutcomeInfo {
    OsPromptOutcomeInfo {
        id: id.to_owned(),
        result: result.to_owned(),
        status: outcome.status.as_str().to_owned(),
        words: outcome.words.clone(),
    }
}

/// The outcome of a gated call, for the window.
fn gated_info<T>(state: &Arc<AppState>, prompt: Prompt, gated: &Gated<T>) -> OsPromptOutcomeInfo {
    match gated {
        Gated::Called { outcome, .. } => outcome_info(prompt.id, "called", outcome),
        Gated::NotNow => OsPromptOutcomeInfo {
            id: prompt.id.to_owned(),
            result: "not_now".to_owned(),
            status: status_of(state, prompt).as_str().to_owned(),
            words: "Nothing was asked. It will ask again the next time you use it.".to_owned(),
        },
        Gated::Waiting => OsPromptOutcomeInfo {
            id: prompt.id.to_owned(),
            result: "waiting".to_owned(),
            status: status_of(state, prompt).as_str().to_owned(),
            words: "Nothing was asked: it waits for you.".to_owned(),
        },
    }
}

/// A call with no work of its own: the entry is primed, and the system
/// asks the first time the feature runs (the user was told it would).
fn primes_only(prompt: Prompt, presence: Presence<'_>) -> Gated<()> {
    os_prompt::gate().through(prompt, presence, &|| None, || ())
}

/// Makes `entry`'s call through the gate with `presence`: the one place
/// each entry's asking call is chosen. Returns what came of it.
///
/// # Errors
///
/// [`AppError::Refused`] for an entry built never to be shown.
pub fn prime_call(
    state: &Arc<AppState>,
    entry: &Entry,
    presence: Presence<'_>,
) -> AppResult<OsPromptOutcomeInfo> {
    let prompt = entry.prompt;
    let status = || Some(status_of(state, prompt));
    let gate = os_prompt::gate();
    let info = match prompt.id {
        "accessibility" => gated_info(
            state,
            prompt,
            &os_prompt::ask_accessibility(prompt, presence),
        ),
        "automation-system-events" => {
            let gated = os_prompt::script(
                prompt,
                presence,
                "tell application \"System Events\" to return UI elements enabled",
                &[],
            );
            if let Gated::Called { value, .. } = &gated {
                let (ok, stderr) = match value {
                    Ok(_) => (true, ""),
                    Err(stderr) => (false, stderr.as_str()),
                };
                if let Some(allowed) = crate::os_access::automation_from_probe(ok, stderr) {
                    crate::os_access::store_automation(state, allowed)?;
                }
            }
            let mut info = gated_info(state, prompt, &gated);
            if matches!(gated, Gated::Called { .. }) {
                info = outcome_info(prompt.id, "called", &Outcome::of(prompt.kind, status()));
            }
            info
        }
        "automation-terminal" | "automation-iterm" => {
            let Kind::Automation(target) = prompt.kind else {
                return Err(AppError::Refused("not an app".to_owned()));
            };
            // Only a running app can be asked about; one that isn't is
            // asked the first time a tab is selected (the user was told).
            let source = format!(
                "if application id \"{0}\" is running then\ntell application id \"{0}\" to count windows\nend if",
                target.bundle_id
            );
            gated_info(
                state,
                prompt,
                &os_prompt::script(prompt, presence, &source, &[]),
            )
        }
        "notifications" => {
            let gated = gate.through(prompt, presence, &|| None, notify::show_first);
            gated_info(state, prompt, &gated)
        }
        "login-item" => gated_info(state, prompt, &os_prompt::login_item_on(prompt, presence)),
        "screen-recording" => gated_info(
            state,
            prompt,
            &os_prompt::ask_screen_recording(prompt, presence),
        ),
        "documents" | "desktop" | "downloads" => {
            let Kind::Folder(folder) = prompt.kind else {
                return Err(AppError::Refused("not a folder".to_owned()));
            };
            let gated = gate.through(prompt, presence, &status, || {
                home().map(|home| std::fs::read_dir(home.join(folder.name())).is_ok())
            });
            gated_info(state, prompt, &gated)
        }
        // The helper's first start: the engines are asked again at once
        // (Check again's own probe), so the system asks now if it will.
        "helper" => {
            let gated = gate.through(prompt, presence, &|| None, || {
                let _ = crate::features::ai::service::mac_checked(state);
            });
            gated_info(state, prompt, &gated)
        }
        "keychain" | "first-open" => {
            return Err(AppError::Refused(
                "the system never asks this: nothing to set up".to_owned(),
            ));
        }
        // The rest prime only: the system asks the first time the feature
        // runs (app links, the local network, another app's data).
        _ => gated_info(state, prompt, &primes_only(prompt, presence)),
    };
    Ok(info)
}

/// Every entry, as the window shows it.
#[must_use]
pub fn list_impl(state: &Arc<AppState>) -> Vec<OsPromptInfo> {
    registry::ENTRIES
        .iter()
        .map(|entry| info(state, entry))
        .collect()
}

fn entry_of(id: &str) -> AppResult<&'static Entry> {
    registry::entry(id).ok_or_else(|| AppError::Validation {
        subject: "system prompt",
        message: format!("{id:?} isn't one AI Juggler can cause"),
    })
}

/// Every system prompt AI Juggler can cause, with whether this setup
/// needs it and what the system says of it now (never asks).
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn os_prompts_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<OsPromptInfo>> {
    crate::ipc::blocking(state.inner(), "os prompts list", |state| {
        Ok(list_impl(state))
    })
    .await
}

/// One entry as it stands now (never asks).
///
/// # Errors
///
/// [`AppError::Validation`] for an id the registry doesn't have.
#[tauri::command]
pub async fn os_prompt_status(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<OsPromptInfo> {
    crate::ipc::blocking(state.inner(), "os prompt status", move |state| {
        Ok(info(state, entry_of(&id)?))
    })
    .await
}

/// Setup's Allow on a card that showed the dialog and what to click:
/// marks the entry primed and makes its call through the gate, then says
/// what came of it.
///
/// # Errors
///
/// An id the registry doesn't have, or one built never to be shown.
#[tauri::command]
pub async fn os_prompt_prime(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<OsPromptOutcomeInfo> {
    let outcome = crate::ipc::blocking(state.inner(), "os prompt prime", move |state| {
        prime_call(state, entry_of(&id)?, Presence::Primed)
    })
    .await?;
    if outcome.id == registry::HELPER.id {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
    Ok(outcome)
}

/// A click that is about to use what `id` guards (the Watches form's
/// Watch button for `notifications`, a Settings switch): the sheet first
/// when the entry isn't primed and the system hasn't decided, then the
/// call, then the outcome. An entry already primed or decided asks
/// nothing and calls nothing (never the same sheet twice, never a sheet
/// with no dialog after it).
///
/// # Errors
///
/// An id the registry doesn't have, or one built never to be shown.
#[tauri::command]
pub async fn os_prompt_request(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<OsPromptOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "os prompt request", move |state| {
        request_impl(state, entry_of(&id)?)
    })
    .await
}

/// Core of [`os_prompt_request`].
///
/// # Errors
///
/// An entry built never to be shown.
pub fn request_impl(state: &Arc<AppState>, entry: &Entry) -> AppResult<OsPromptOutcomeInfo> {
    let status = status_of(state, entry.prompt);
    if os_prompt::gate().primed(entry.prompt.id)
        || matches!(status, Status::Allowed | Status::NotAllowed)
    {
        return Ok(outcome_info(
            entry.prompt.id,
            "already",
            &Outcome::of(entry.prompt.kind, Some(status)),
        ));
    }
    prime_call(state, entry, os_prompt::user())
}

/// The sheet's answer: Continue (`go`) or Not now, for the entry `id`.
///
/// # Errors
///
/// [`AppError::Validation`] for an id the registry doesn't have.
#[tauri::command]
pub async fn os_prompt_answer(id: String, go: bool) -> AppResult<()> {
    entry_of(&id)?;
    if let Some(primer) = PRIMER.get() {
        primer.answer(&id, go);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
