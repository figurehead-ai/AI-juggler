//! AJ-E1510: system notifications (ADR-0019 §3) — what needs the user
//! reaches them outside the window, when they asked for it.
//!
//! The state changes come in where they're made (a heuristic's waiting
//! or stuck, a process's finish or usage limit, This Mac's room); the
//! [`engine`] decides what is news; Settings › Notifications says which
//! kinds may interrupt, and its quiet hours and accounts left out apply
//! here as they do to the digest. A click brings AI Juggler forward on
//! that session (or This Mac).

pub mod centre;
pub mod commands;
pub mod engine;
pub mod needs;
pub mod prefs;
pub mod sources;
pub mod tray;
pub mod views;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use engine::{Due, Engine, Gate, Kind, Target};

use crate::state::AppState;

/// Asks the UI to show a session or This Mac (a notification's click).
pub const EVENT_OPEN: &str = "app://open";

/// What a click asks the UI to open.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenRequest {
    /// The session to open, when it's one.
    pub session_id: Option<String>,
    /// `this_mac` for the This Mac page.
    pub page: Option<String>,
    /// AJ-E1515: the paired Mac the session runs on (`None`: this one).
    #[serde(default)]
    pub machine_id: Option<String>,
}

/// One notification as the system shows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outgoing {
    /// The id a click comes back with.
    pub id: i32,
    /// What it's about (a session's name, or This Mac).
    pub title: String,
    /// Its state, where and on which account.
    pub body: String,
}

type Sink = Box<dyn Fn(&Outgoing) + Send + Sync>;
/// AJ-E2501: what follows a change in the model (the badge, the menu-bar
/// item, the window).
type OnChange = Box<dyn Fn() + Send + Sync>;

/// No offset known yet (the UI tells it within its first tick).
const UNKNOWN_OFFSET: i32 = i32::MIN;
/// Clicks remembered (the oldest go first).
const CLICKS_KEPT: usize = 200;

/// The notifier, held by [`AppState`].
pub struct Notifier {
    engine: Mutex<Engine>,
    sink: OnceLock<Sink>,
    clicks: Mutex<HashMap<i32, Target>>,
    next_id: AtomicI32,
    /// AI Juggler's main window has focus.
    in_front: AtomicBool,
    /// Local time minus UTC, in whole hours (the webview knows the
    /// local hour; the backend keeps no time zone).
    utc_offset_hours: AtomicI32,
    /// This Mac's room in words, as last seen (title, reason).
    mac_words: Mutex<Option<(String, String)>>,
    /// AJ-E2405: how many processes each session left running, as told.
    left: Mutex<HashMap<String, u32>>,
    /// AJ-E2501: the model's own words for what it tells (an account
    /// almost out of room, a crash restarted, a reset).
    told: Mutex<HashMap<(Target, Kind), (String, String)>>,
    /// AJ-E2501: the views of the model, once the app has them.
    on_change: OnceLock<OnChange>,
    /// AJ-E2501: what is open now, as the backend last worked it out (the
    /// menu-bar item lists it).
    open_now: Mutex<views::OpenNowInput>,
    /// D-49: This Mac as last sampled (the tick's Needs you reads it).
    this_mac: Mutex<Option<crate::machine_health::ThisMacInfo>>,
    /// D-49: the accounts' sign-ins as last checked, and when.
    sign_ins: Mutex<Option<(i64, Vec<crate::commands::AccountInfo>)>>,
    /// D-49: what Needs you was last time (a hash: the model is written
    /// only when it changes).
    needs_signature: Mutex<Option<u64>>,
}

impl Default for Notifier {
    fn default() -> Self {
        Self {
            engine: Mutex::new(Engine::default()),
            sink: OnceLock::new(),
            clicks: Mutex::new(HashMap::new()),
            next_id: AtomicI32::new(1),
            in_front: AtomicBool::new(false),
            utc_offset_hours: AtomicI32::new(UNKNOWN_OFFSET),
            mac_words: Mutex::new(None),
            left: Mutex::new(HashMap::new()),
            told: Mutex::new(HashMap::new()),
            on_change: OnceLock::new(),
            open_now: Mutex::new(views::OpenNowInput::default()),
            this_mac: Mutex::new(None),
            sign_ins: Mutex::new(None),
            needs_signature: Mutex::new(None),
        }
    }
}

impl Notifier {
    /// Where notifications go: the system in the app, a list in tests.
    /// Set once; until then nothing is shown.
    pub fn set_sink(&self, sink: Sink) {
        let _ = self.sink.set(sink);
    }

    /// AJ-E2501: what follows a change in the model. Set once; until then
    /// (and in tests) a change only changes the model.
    pub fn set_on_change(&self, on_change: OnChange) {
        let _ = self.on_change.set(on_change);
    }

    /// The UTC offset learned from the window, when known.
    #[must_use]
    pub fn utc_offset(&self) -> Option<i64> {
        let offset = self.utc_offset_hours.load(Ordering::Relaxed);
        (offset != UNKNOWN_OFFSET).then_some(i64::from(offset))
    }

    /// AJ-E2501: the model's clock now (the time, and the local hour's offset).
    #[must_use]
    pub fn clock(&self) -> centre::Clock {
        centre::Clock {
            now: crate::commands::now_millis(),
            utc_offset_hours: self.utc_offset(),
        }
    }

    /// AJ-E2501: what is open now, as the window last said.
    #[must_use]
    pub fn open_now(&self) -> views::OpenNowInput {
        self.open_now
            .lock()
            .map(|open| open.clone())
            .unwrap_or_default()
    }

    /// AJ-E2501: what is open now, as the backend worked it out.
    pub fn set_open_now(&self, open: views::OpenNowInput) {
        if let Ok(mut now) = self.open_now.lock() {
            *now = open;
        }
    }

    /// D-49: This Mac as just sampled.
    pub fn set_this_mac(&self, info: crate::machine_health::ThisMacInfo) {
        if let Ok(mut this_mac) = self.this_mac.lock() {
            *this_mac = Some(info);
        }
    }

    /// D-49: This Mac as last sampled.
    #[must_use]
    pub fn this_mac(&self) -> Option<crate::machine_health::ThisMacInfo> {
        self.this_mac
            .lock()
            .ok()
            .and_then(|this_mac| this_mac.clone())
    }

    /// D-49: the accounts with their sign-ins, checked again (`check`)
    /// only when the last check is older than `fresh_ms`.
    pub fn sign_ins(
        &self,
        now: i64,
        fresh_ms: i64,
        check: impl FnOnce() -> Vec<crate::commands::AccountInfo>,
    ) -> Vec<crate::commands::AccountInfo> {
        if let Ok(cached) = self.sign_ins.lock()
            && let Some((at, accounts)) = cached.as_ref()
            && now - at < fresh_ms
        {
            return accounts.clone();
        }
        let accounts = check();
        if let Ok(mut cached) = self.sign_ins.lock() {
            *cached = Some((now, accounts.clone()));
        }
        accounts
    }

    /// D-49: records what Needs you is now; `true` when it is what it was.
    pub fn needs_signature(&self, signature: u64) -> bool {
        self.needs_signature
            .lock()
            .is_ok_and(|mut last| last.replace(signature) == Some(signature))
    }

    /// AI Juggler's main window gained or lost focus.
    pub fn set_in_front(&self, in_front: bool) {
        self.in_front.store(in_front, Ordering::Relaxed);
    }

    /// The UI's local hour, as it ticks: the offset from UTC follows.
    pub fn local_hour_seen(&self, local_hour: Option<u32>, now_ms: i64) {
        let Some(local) = local_hour.filter(|hour| *hour < 24) else {
            return;
        };
        let offset = (i64::from(local) - utc_hour(now_ms)).rem_euclid(24);
        self.utc_offset_hours
            .store(i32::try_from(offset).unwrap_or(0), Ordering::Relaxed);
    }

    /// The local hour now, when known.
    fn local_hour(&self, now_ms: i64) -> Option<u32> {
        let offset = self.utc_offset_hours.load(Ordering::Relaxed);
        (offset != UNKNOWN_OFFSET)
            .then(|| u32::try_from((utc_hour(now_ms) + i64::from(offset)).rem_euclid(24)).ok())
            .flatten()
    }

    /// Where a click goes.
    #[must_use]
    pub fn target_of(&self, id: i32) -> Option<Target> {
        self.clicks.lock().ok()?.get(&id).cloned()
    }

    fn remember(&self, target: Target) -> i32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut clicks) = self.clicks.lock() {
            if clicks.len() >= CLICKS_KEPT
                && let Some(oldest) = clicks.keys().min().copied()
            {
                clicks.remove(&oldest);
            }
            clicks.insert(id, target);
        }
        id
    }
}

fn utc_hour(now_ms: i64) -> i64 {
    now_ms.div_euclid(3_600_000).rem_euclid(24)
}

/// A session's status now (`SessionStatus::as_str`); shows what is due.
pub fn session_status(state: &Arc<AppState>, session_id: &str, status: &str) {
    // AJ-E2501: a finish is told once in the centre, whatever the
    // system notifications allow.
    if status == "finished" {
        sources::finished(state, session_id);
    }
    observe(
        state,
        Target::Session(session_id.to_owned()),
        Kind::of_status(status),
    );
}

/// This Mac's room now (its summary: `plenty` | `tight` | `pressure`).
pub fn this_mac_room(state: &Arc<AppState>, room: &crate::machine_health::RoomSummaryInfo) {
    let tight = matches!(room.level.as_str(), "tight" | "pressure");
    if tight && let Ok(mut words) = state.notifier.mac_words.lock() {
        *words = Some((
            format!("This Mac: {}", room.words.to_lowercase()),
            room.reasons
                .first()
                .map_or_else(String::new, |reason| reason.words.clone()),
        ));
    }
    observe(state, Target::ThisMac, tight.then_some(Kind::MacTight));
}

/// AJ-E2405: a session ended and left `count` processes running (told
/// once; a click opens the session's page).
pub fn left_behind(state: &Arc<AppState>, session_id: &str, count: u32) {
    if let Ok(mut left) = state.notifier.left.lock() {
        left.insert(session_id.to_owned(), count);
    }
    let target = Target::Session(session_id.to_owned());
    observe(state, target.clone(), Some(Kind::LeftBehind));
    // Told (or held for quiet hours): the next ending can be told again.
    let now = crate::commands::now_millis();
    if let Ok(mut engine) = state.notifier.engine.lock()
        && !engine.pending()
    {
        engine.observe(target, None, now);
    }
}

fn observe(state: &Arc<AppState>, target: Target, kind: Option<Kind>) {
    let now = crate::commands::now_millis();
    if let Ok(mut engine) = state.notifier.engine.lock() {
        engine.observe(target, kind, now);
    }
    show_due(state);
}

/// AJ-E2501: the model changed — the views follow (the badge, the
/// menu-bar item, the window).
pub fn changed(state: &Arc<AppState>) {
    if let Some(on_change) = state.notifier.on_change.get() {
        on_change();
    }
}

/// AJ-E2501: something the model wrote may become a system notification
/// (its kind on in Settings, its project and kind not muted, and the
/// rules every notification keeps: told once, quiet hours, never while
/// AI Juggler is in front). One that happened (a crash, a reset) can be
/// told again the next time; one that needs the user while it holds (an
/// account almost out of room) once while it holds.
pub fn model_event(state: &Arc<AppState>, target: Target, kind: Kind, info: &centre::NoticeInfo) {
    let muted = state.storage.lock().is_ok_and(|storage| {
        !centre::may_interrupt(&storage, &info.kind, info.project_id.as_deref())
    });
    if muted {
        return;
    }
    if let Ok(mut told) = state.notifier.told.lock() {
        told.insert(
            (target.clone(), kind),
            (info.title.clone(), info.detail.clone()),
        );
    }
    observe(state, target.clone(), Some(kind));
    if kind != Kind::Quota {
        let now = crate::commands::now_millis();
        if let Ok(mut engine) = state.notifier.engine.lock()
            && !engine.pending()
        {
            engine.observe(target, None, now);
        }
    }
}

/// Shows what is due now: called on each change, and on the heuristic
/// tick so what quiet hours held comes once they end.
pub fn show_due(state: &Arc<AppState>) {
    let notifier = &state.notifier;
    let Some(sink) = notifier.sink.get() else {
        return;
    };
    let now = crate::commands::now_millis();
    if !notifier.engine.lock().is_ok_and(|engine| engine.pending()) {
        return;
    }
    let due = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let kinds = prefs::read(&storage);
        let quiet = notifier
            .local_hour(now)
            .is_none_or(|hour| crate::settings_policy::quiet_hours(&storage).contains(hour));
        let left_out = crate::settings_policy::muted_accounts(&storage);
        // AJ-E2501: a muted kind doesn't interrupt either.
        let allowed = |kind: Kind| {
            kinds.allows(kind) && centre::may_interrupt(&storage, model_kind(kind), None)
        };
        // AJ-E2501: nor does a snoozed need or a muted project (the
        // model's marks; the count still holds them).
        let muted = |target: &Target| match target {
            Target::Session(id) => {
                account_of(&storage, id).is_some_and(|account| left_out.contains(&account))
                    || centre::session_snoozed(&storage, id, now)
                    || !centre::may_interrupt(&storage, "", project_of(&storage, id).as_deref())
            }
            Target::ThisMac => false,
            Target::Account(id) => left_out.contains(id),
        };
        let gate = Gate {
            allowed: &allowed,
            quiet,
            in_front: notifier.in_front.load(Ordering::Relaxed),
            muted: &muted,
        };
        let Ok(mut engine) = notifier.engine.lock() else {
            return;
        };
        engine.due(&gate, now)
    };
    for item in due {
        let (title, body) = words(state, &item);
        let id = notifier.remember(item.target);
        sink(&Outgoing { id, title, body });
    }
}

/// The model's kind for a system notification's kind (Mute this kind).
const fn model_kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Waiting => "asks",
        Kind::Stuck => "stuck",
        Kind::Limit => "limit",
        Kind::Finished => "finished",
        Kind::MacTight => "room",
        Kind::LeftBehind => "leftovers",
        Kind::Quota => "quota",
        Kind::Crashed => "restarted",
        Kind::Reset => "reset",
    }
}

fn project_of(storage: &juggler_core::storage::Storage, session_id: &str) -> Option<String> {
    storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session_id))
        .ok()?
        .project
        .map(|project| project.as_str().to_owned())
}

fn account_of(storage: &juggler_core::storage::Storage, session_id: &str) -> Option<String> {
    let record = storage
        .sessions()
        .get(&juggler_core::model::SessionId::new(session_id))
        .ok()?;
    let recipe = storage.profiles().get(&record.profile).ok()?;
    Some(recipe.account.to_string())
}

/// The title and line a notification shows.
fn words(state: &Arc<AppState>, due: &Due) -> (String, String) {
    if let Some(told) = state
        .notifier
        .told
        .lock()
        .ok()
        .and_then(|mut told| told.remove(&(due.target.clone(), due.kind)))
    {
        return told;
    }
    match &due.target {
        Target::Account(_) => ("An account needs a look".to_owned(), String::new()),
        Target::ThisMac => state
            .notifier
            .mac_words
            .lock()
            .ok()
            .and_then(|words| words.clone())
            .unwrap_or_else(|| ("This Mac is getting tight".to_owned(), String::new())),
        Target::Session(id) => {
            let about = describe(state, id);
            let state_words = match due.kind {
                Kind::Waiting => "Waiting for your reply",
                Kind::Stuck => "No output for a while",
                Kind::Limit => "Hit its usage limit",
                Kind::Finished => "Finished",
                Kind::MacTight | Kind::LeftBehind | Kind::Quota | Kind::Crashed | Kind::Reset => "",
            };
            if due.kind == Kind::LeftBehind {
                let count = state
                    .notifier
                    .left
                    .lock()
                    .ok()
                    .and_then(|left| left.get(id).copied())
                    .unwrap_or(1);
                let mut line = vec![if count == 1 {
                    "Left 1 process running".to_owned()
                } else {
                    format!("Left {count} processes running")
                }];
                line.extend(about.place);
                line.extend(about.account);
                return (about.name, line.join(" · "));
            }
            let mut line = vec![state_words.to_owned()];
            line.extend(about.place);
            line.extend(about.account);
            (about.name, line.join(" · "))
        }
    }
}

pub(super) struct About {
    pub(super) name: String,
    pub(super) place: Option<String>,
    pub(super) account: Option<String>,
    pub(super) project_id: Option<String>,
}

/// A session as its row names it: its conversation's title, else its
/// folder; where it runs, and on which account.
pub(super) fn describe(state: &Arc<AppState>, session_id: &str) -> About {
    let info = {
        let rows = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
            .unwrap_or_default();
        crate::features::conversations::titled(state, rows)
            .unwrap_or_default()
            .into_iter()
            .find(|row| row.id == session_id)
    };
    let Some(info) = info else {
        return About {
            name: "A session".to_owned(),
            place: None,
            account: None,
            project_id: None,
        };
    };
    let folder = std::path::Path::new(&info.cwd)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty());
    let name = info
        .label
        .clone()
        .or_else(|| folder.clone())
        .unwrap_or_else(|| "A session".to_owned());
    About {
        place: folder.filter(|folder| *folder != name),
        account: Some(info.account_label).filter(|label| !label.is_empty()),
        project_id: info.project_id.clone(),
        name,
    }
}

/// A notification was clicked: AI Juggler comes forward on its session
/// (or This Mac). An unknown id still brings the window forward.
pub fn clicked<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &Arc<AppState>, id: i32) {
    use tauri::Emitter;
    let open = match state.notifier.target_of(id) {
        Some(Target::Session(session_id)) => Some(OpenRequest {
            session_id: Some(session_id),
            page: None,
            machine_id: None,
        }),
        Some(Target::ThisMac) => Some(OpenRequest {
            session_id: None,
            page: Some("this_mac".to_owned()),
            machine_id: None,
        }),
        // AJ-E2501: an account's notice opens the notification centre,
        // where its banner and its actions are.
        Some(Target::Account(_)) => Some(OpenRequest {
            session_id: None,
            page: Some("notifications".to_owned()),
            machine_id: None,
        }),
        None => None,
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::main_window::bring_forward(&handle);
        if let Some(open) = open {
            let _ = handle.emit(EVENT_OPEN, open);
        }
    });
}

/// The system's own notifications (the Tauri plugin), clicks routed back.
pub fn connect(app: &tauri::AppHandle, state: &Arc<AppState>) {
    use tauri_plugin_notification::NotificationExt;
    let shower = app.clone();
    state
        .notifier
        .set_sink(Box::new(move |outgoing: &Outgoing| {
            let _ = shower
                .notification()
                .builder()
                .id(outgoing.id)
                .title(&outgoing.title)
                .body(&outgoing.body)
                .show();
        }));
    let clicks = app.clone();
    let click_state = Arc::clone(state);
    let _ = app.notification().on_action(move |performed| {
        if performed.action_id() != "tap" {
            return;
        }
        if let Some(id) = performed
            .notification()
            .map(tauri_plugin_notification::ActiveNotification::id)
        {
            clicked(&clicks, &click_state, id);
        }
    });
}
