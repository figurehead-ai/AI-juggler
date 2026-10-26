//! AJ-E1510: system notifications (ADR-0019 §3) — what needs the user
//! reaches them outside the window, when they asked for it.
//!
//! The state changes come in where they're made (a heuristic's waiting
//! or stuck, a process's finish or usage limit, This Mac's room); the
//! [`engine`] decides what is news; Settings › Notifications says which
//! kinds may interrupt, and its quiet hours and accounts left out apply
//! here as they do to the digest. A click brings AI Juggler forward on
//! that session (or This Mac).

pub mod engine;
pub mod prefs;

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
        }
    }
}

impl Notifier {
    /// Where notifications go: the system in the app, a list in tests.
    /// Set once; until then nothing is shown.
    pub fn set_sink(&self, sink: Sink) {
        let _ = self.sink.set(sink);
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
        let allowed = |kind: Kind| kinds.allows(kind);
        let muted = |target: &Target| match target {
            Target::Session(id) => {
                account_of(&storage, id).is_some_and(|account| left_out.contains(&account))
            }
            Target::ThisMac => false,
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
    match &due.target {
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
                Kind::MacTight | Kind::LeftBehind => "",
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

struct About {
    name: String,
    place: Option<String>,
    account: Option<String>,
}

/// A session as its row names it: its conversation's title, else its
/// folder; where it runs, and on which account.
fn describe(state: &Arc<AppState>, session_id: &str) -> About {
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
        name,
    }
}

/// A notification was clicked: AI Juggler comes forward on its session
/// (or This Mac). An unknown id still brings the window forward.
pub fn clicked<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &Arc<AppState>, id: i32) {
    use tauri::{Emitter, Manager};
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
        None => None,
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
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
