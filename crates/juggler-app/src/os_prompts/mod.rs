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
#[allow(clippy::struct_excessive_bools)] // wire DTO: each flag is its own fact the sheet and setup read
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
    /// After Not now: where (or when) it's offered again, one sentence.
    pub offered_again: String,
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
    /// 0.5.2 G36: whether the system shows a dialog after Continue. Bot's
    /// helper's first start brings none (macOS asks only when the helper
    /// on disk changed since it last opened, `helper-changed`), so its
    /// sheet never says "macOS will ask next" and draws no dialog.
    pub dialog_follows: bool,
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
    /// asked: said now, or remembered for an entry offered again from its
    /// setting, 0.5.2 G33) | `waiting` (nobody was there) |
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

/// 0.5.2 G33 (PAT-OSP): the user said Not now to an entry's sheet. For an
/// entry offered again from its setting ([`registry::Again::Setting`]) it
/// is remembered: a use elsewhere asks nothing more, only that setting.
const NOT_NOW_PREFIX: &str = "os_prompt_not_now:";

/// Whether the user said Not now to `id`'s sheet before.
#[must_use]
pub fn said_not_now(state: &AppState, id: &str) -> bool {
    state
        .app_setting(&format!("{NOT_NOW_PREFIX}{id}"))
        .as_deref()
        == Some("1")
}

fn remember_not_now(state: &AppState, id: &str) {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .app_settings()
            .set(&format!("{NOT_NOW_PREFIX}{id}"), "1");
    }
}

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
        // 0.5.2 G36: the helper's start says what came of it (its probe:
        // started, stopped, not answered yet), told by `prime_call`; the
        // system won't say, so the gate's outcome would be "If you allowed
        // it… If not…".
        if is_helper(prompt.id) {
            return;
        }
        self.tell(&outcome_info(prompt.id, "called", outcome));
    }
}

impl WindowPrimer {
    /// The sheet's outcome, said.
    fn tell(&self, outcome: &OsPromptOutcomeInfo) {
        self.app
            .emit_event(AppEvent::OsPromptOutcome, outcome.clone());
    }
}

fn is_helper(id: &str) -> bool {
    id == registry::HELPER.id || id == registry::HELPER_CHANGED.id
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
    let _ = STATE.set(Arc::clone(state));
    let primer = Arc::new(WindowPrimer::new(app.clone(), Arc::clone(state)));
    let _ = PRIMER.set(Arc::clone(&primer));
    os_prompt::install(
        Box::new(AppLedger(Arc::clone(state))),
        Box::new(Shared(primer)),
    );
}

fn home() -> Option<std::path::PathBuf> {
    juggler_platform::user::home()
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
/// a program it runs), or the helper's entry ([`helper_prompt`]) is primed
/// (Bot's setup guide's check, or setup's Meet Bot). A first start may make the system ask ("differs
/// from previously opened versions"), and a probe nobody clicked never
/// may. Called by the helpers' one client right before it spawns.
#[must_use]
pub fn helper_may_start(command: &std::process::Command) -> bool {
    let Some(path) = ours_in(command) else {
        return true;
    };
    let changed = STATE
        .get()
        .is_some_and(|state| helper_changed_at(state, &path));
    helper_start_through(os_prompt::gate(), changed)
}

/// The app's own helper `command` runs (as its program or a program it
/// runs), when it runs one.
fn ours_in(command: &std::process::Command) -> Option<std::path::PathBuf> {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(std::path::Path::new)
        .find(|part| juggler_platform::code_trust::beside_app(part))
        .map(std::path::Path::to_path_buf)
}

/// After one of the app's own helpers said hello (`command`'s): its
/// signature on disk is the one last opened (AJ-E1555).
pub fn helper_opened(command: &std::process::Command) {
    if let (Some(path), Some(state)) = (ours_in(command), STATE.get()) {
        note_opened(state, &path);
    }
}

/// The gate's half of [`helper_may_start`]: a start nobody clicked passes
/// only once the helper's entry now ([`helper_prompt`]) is primed.
#[must_use]
pub fn helper_start_through(gate: os_prompt::Gate<'_>, changed: bool) -> bool {
    matches!(
        gate.through(
            helper_prompt(gate, changed),
            Presence::Background,
            &|| None,
            || ()
        ),
        Gated::Called { .. }
    )
}

/// The app, for the helper checks the seam makes without one.
static STATE: std::sync::OnceLock<Arc<AppState>> = std::sync::OnceLock::new();

/// Where the signature of the helper last opened is kept, per helper.
const OPENED_PREFIX: &str = "os_prompt_helper_opened:";

fn opened_key(path: &std::path::Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{OPENED_PREFIX}{name}"))
}

/// AJ-E1555: macOS asks whether to open a program that "differs from
/// previously opened versions" only when its signature changed since it
/// last opened: `stored` (the one last opened) and `now` (on disk) both
/// known and different. Never because some other build ran here.
#[must_use]
pub fn signature_changed(stored: Option<&str>, now: Option<&str>) -> bool {
    matches!((stored, now), (Some(stored), Some(now)) if stored != now)
}

/// Whether the helper at `path` changed on disk since it last opened.
#[must_use]
pub fn helper_changed_at(state: &AppState, path: &std::path::Path) -> bool {
    let Some(key) = opened_key(path) else {
        return false;
    };
    signature_changed(
        state.app_setting(&key).as_deref(),
        juggler_platform::code_trust::signature_of(path).as_deref(),
    )
}

/// Whether any of the app's own helpers changed on disk since it last
/// opened (the `helper-changed` entry's `needed`).
#[must_use]
pub fn helper_changed_here(state: &AppState) -> bool {
    crate::features::ai::engines::own_helpers()
        .iter()
        .any(|path| helper_changed_at(state, path))
}

/// Keeps `path`'s signature as the one last opened; a change seen means
/// the next change is told again (its `helper-changed` priming is spent).
pub fn note_opened(state: &AppState, path: &std::path::Path) {
    let (Some(key), Some(now)) = (
        opened_key(path),
        juggler_platform::code_trust::signature_of(path),
    ) else {
        return;
    };
    let changed = helper_changed_at(state, path);
    if let Ok(storage) = state.storage.lock() {
        let settings = storage.app_settings();
        let _ = settings.set(&key, &now);
        if changed {
            let _ = settings.set(
                &format!("{PRIMED_PREFIX}{}", registry::HELPER_CHANGED.id),
                "0",
            );
        }
    }
}

fn changed_now() -> bool {
    STATE.get().is_some_and(|state| helper_changed_here(state))
}

/// The entry the helper's next start goes through: `helper-changed`
/// while the helper on disk differs from the one last opened (`changed`)
/// until that entry is primed, else `helper`. Priming `helper-changed`
/// primes `helper` too ([`prime_call`]): one sheet, never two.
#[must_use]
pub fn helper_prompt(gate: os_prompt::Gate<'_>, changed: bool) -> Prompt {
    if changed && !gate.primed(registry::HELPER_CHANGED.id) {
        registry::HELPER_CHANGED
    } else {
        registry::HELPER
    }
}

/// `entry`, or for one of the helper's two entries the one its next
/// start goes through ([`helper_prompt`]).
fn helper_entry_now(entry: &Entry) -> &Entry {
    let id = entry.prompt.id;
    if id != registry::HELPER.id && id != registry::HELPER_CHANGED.id {
        return entry;
    }
    registry::entry(helper_prompt(os_prompt::gate(), changed_now()).id).unwrap_or(entry)
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
    let dialog_follows = dialog_follows(entry);
    let dialog = os_prompt::dialog(entry.prompt.kind, app).filter(|_| dialog_follows);
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
        offered_again: entry.offered_again.to_owned(),
        needed: !entry.never_shown && (entry.needed)(state),
        status: status_of(state, entry.prompt).as_str().to_owned(),
        primed: os_prompt::gate().primed(helper_entry_now(entry).prompt.id),
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
        dialog_follows,
    }
}

/// 0.5.2 G36: whether the system shows a dialog after `entry`'s Continue.
/// Bot's helper's first start (`helper`) brings none; only a helper that
/// changed on disk since it last opened (`helper-changed`) may be asked
/// about. Every other entry's call makes the system ask.
#[must_use]
pub fn dialog_follows(entry: &Entry) -> bool {
    entry.prompt.id != registry::HELPER.id
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
        words: registry::entry(id).map_or_else(
            || outcome.words.clone(),
            |entry| outcome_words(entry, outcome.status),
        ),
    }
}

/// AJ-E1555: what the sheet says after the call, from the registry's own
/// words: what works now, or what declining means and where to change
/// it; never a state word ("Asked.", "Allowed") as the body.
#[must_use]
pub fn outcome_words(entry: &Entry, status: Status) -> String {
    let place = os_prompt::settings_place(entry.prompt.kind);
    let later = |lead: &str| match place {
        Some(place) => format!("{lead} You can change it any time in {place}."),
        None => lead.to_owned(),
    };
    match status {
        Status::Allowed => entry.after.to_owned(),
        Status::NotAllowed => later(entry.if_declined),
        Status::NotAsked => format!(
            "The system didn't ask, so nothing changed. {}",
            entry.offered_again
        ),
        // The system won't say what was answered: what follows either way.
        Status::Asked => later(&format!(
            "If you allowed it: {} If not: {}",
            entry.after, entry.if_declined
        )),
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
            words: not_now_words(prompt.id),
        },
        Gated::Waiting => OsPromptOutcomeInfo {
            id: prompt.id.to_owned(),
            result: "waiting".to_owned(),
            status: status_of(state, prompt).as_str().to_owned(),
            words: "Nothing was asked: it waits for you.".to_owned(),
        },
    }
}

/// After Not now: nothing was asked, and where (or when) it's offered
/// again, the registry's own sentence.
fn not_now_words(id: &str) -> String {
    registry::entry(id).map_or_else(
        || "Nothing was asked.".to_owned(),
        |entry| format!("Nothing was asked. {}", entry.offered_again),
    )
}

/// What came of starting the helper (its probe's reading of this Mac):
/// it started and said hello, the system stopped it, or it hasn't
/// answered yet.
fn helper_outcome(
    prompt: Prompt,
    mac: Option<&crate::features::ai::models::recommend::MacFacts>,
) -> OsPromptOutcomeInfo {
    use crate::features::ai::models::recommend::AppleAi;
    let Some(entry) = registry::entry(prompt.id) else {
        return outcome_info(prompt.id, "called", &Outcome::of(prompt.kind, None));
    };
    let (status, words) = match mac.map(|mac| mac.apple_ai) {
        Some(AppleAi::On | AppleAi::Off | AppleAi::Preparing | AppleAi::Unavailable) => {
            (Status::Allowed, entry.after.to_owned())
        }
        Some(AppleAi::Blocked) => (Status::NotAllowed, entry.if_declined.to_owned()),
        _ => (
            Status::Asked,
            format!(
                "Bot's helper hasn't answered yet; nothing else changed. {}",
                entry.offered_again
            ),
        ),
    };
    OsPromptOutcomeInfo {
        id: prompt.id.to_owned(),
        result: "called".to_owned(),
        status: status.as_str().to_owned(),
        words,
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
        // After an update that renamed it, its `helper-changed` sheet
        // first, once; that primes `helper` too.
        "helper" | "helper-changed" => {
            let prompt = helper_entry_now(entry).prompt;
            let sheet_shown = matches!(presence, Presence::User(_)) && !gate.primed(prompt.id);
            let gated = gate.through(prompt, presence, &|| None, || {
                if prompt.id == registry::HELPER_CHANGED.id {
                    AppLedger(Arc::clone(state)).set_primed(registry::HELPER.id);
                }
                crate::features::ai::service::mac_checked(state)
            });
            // AJ-E1555 R3: the helper's start is the action; the sheet says
            // what came of it, never "asked" for a prompt that didn't come.
            match &gated {
                Gated::Called { value, .. } => {
                    let outcome = helper_outcome(prompt, value.as_ref());
                    // 0.5.2 G36: the sheet says what came of the start.
                    if sheet_shown && let Some(primer) = PRIMER.get() {
                        primer.tell(&outcome);
                    }
                    outcome
                }
                _ => gated_info(state, prompt, &gated),
            }
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
    let outcome = crate::ipc::blocking_with(
        state.inner(),
        "os prompt prime",
        crate::ipc::Wait::Person,
        move |state| prime_call(state, entry_of(&id)?, Presence::Primed),
    )
    .await?;
    if outcome.id == registry::HELPER.id || outcome.id == registry::HELPER_CHANGED.id {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
    Ok(outcome)
}

/// A click that is about to use what `id` guards (the Watches form's
/// Watch button for `notifications`, a reminder's Set; `from_setting`
/// for the entry's own setting, Check this computer for Bot's helper):
/// the sheet first when the entry isn't primed and the system hasn't
/// decided, then the call, then the outcome. An entry already primed or
/// decided asks nothing and calls nothing (never the same sheet twice,
/// never a sheet with no dialog after it), and so does an entry offered
/// again from its setting that the user said Not now to, unless this is
/// that setting (0.5.2 G33).
///
/// # Errors
///
/// An id the registry doesn't have, or one built never to be shown.
#[tauri::command]
pub async fn os_prompt_request(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    from_setting: Option<bool>,
) -> AppResult<OsPromptOutcomeInfo> {
    let from = if from_setting.unwrap_or(false) {
        Asked::FromSetting
    } else {
        Asked::ByUse
    };
    crate::ipc::blocking_with(
        state.inner(),
        "os prompt request",
        crate::ipc::Wait::Person,
        move |state| request_impl(state, entry_of(&id)?, from),
    )
    .await
}

/// Where a request comes from (0.5.2 G33).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// A use of what the entry guards (a watch's Watch, a reminder's Set,
    /// a move's link).
    ByUse,
    /// The entry's own setting (the place its `offered_again` names).
    FromSetting,
}

/// 0.5.2 G33 (PAT-OSP): whether a request shows `entry`'s sheet again
/// after the user said Not now to it (`said_not_now`): always from its
/// setting, and at a use only for an entry offered again at its next use.
/// Never by itself: a watch's Watch or a reminder's Set after Not now to
/// notifications asks nothing more.
#[must_use]
pub fn asks_again(entry: &Entry, from: Asked, said_not_now: bool) -> bool {
    !said_not_now || from == Asked::FromSetting || entry.again == registry::Again::NextUse
}

/// Core of [`os_prompt_request`].
///
/// # Errors
///
/// An entry built never to be shown.
pub fn request_impl(
    state: &Arc<AppState>,
    entry: &Entry,
    from: Asked,
) -> AppResult<OsPromptOutcomeInfo> {
    let entry = helper_entry_now(entry);
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
    // 0.5.2 G33 (PAT-OSP): Not now is remembered for an entry offered
    // again from its setting; a use elsewhere goes on without asking.
    if !asks_again(entry, from, said_not_now(state, entry.prompt.id)) {
        return Ok(OsPromptOutcomeInfo {
            id: entry.prompt.id.to_owned(),
            result: "not_now".to_owned(),
            status: status.as_str().to_owned(),
            words: not_now_words(entry.prompt.id),
        });
    }
    let outcome = prime_call(state, entry, os_prompt::user())?;
    if outcome.result == "not_now" {
        remember_not_now(state, entry.prompt.id);
    }
    Ok(outcome)
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
