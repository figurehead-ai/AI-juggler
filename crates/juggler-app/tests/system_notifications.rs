//! AJ-E1510: system notifications through the app's own notifier — off
//! until a kind is turned on; quiet hours hold them and accounts left out
//! of the digest stay quiet; each state is told once; nothing while AI
//! Juggler is in front; a click opens that session. The system itself is
//! never asked (tests never grant the macOS permission): the notifier's
//! sink is a list here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::notifications::engine::Target;
use juggler_app::features::notifications::prefs::{self, SystemNotificationsInfo};
use juggler_app::features::notifications::{self, OpenRequest, Outgoing};
use juggler_app::state::AppState;
use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};
use tauri::Listener;

fn now_ms() -> i64 {
    Timestamp::now().millis()
}

fn app_state(dir: &Path) -> Arc<AppState> {
    // The adapter is only named here: nothing is launched.
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(dir.join("no-claude"))));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// Everything shown, in order.
fn shown(state: &Arc<AppState>) -> Arc<Mutex<Vec<Outgoing>>> {
    let list = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&list);
    state
        .notifier
        .set_sink(Box::new(move |outgoing: &Outgoing| {
            sink.lock().unwrap().push(outgoing.clone())
        }));
    list
}

/// An account, its launch settings and a session in `folder`; the
/// session's id and the account's.
fn session(state: &Arc<AppState>, folder: &Path, id: &str, account: &str) -> (String, String) {
    let account = commands::create_account(state, CLAUDE_CODE_ID, account, None).unwrap();
    let recipe = commands::create_profile(state, &account.id, "Default", None, None).unwrap();
    let record = InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(recipe.id),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd: folder.to_path_buf(),
        status: SessionStatus::Running,
        started_at: Timestamp::now(),
        last_activity: None,
        usage: Default::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    };
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&record)
        .unwrap();
    (id.to_owned(), account.id)
}

fn turn_on(state: &Arc<AppState>, kinds: SystemNotificationsInfo) {
    prefs::notifications_set_impl(state, kinds).unwrap();
}

const ALL: SystemNotificationsInfo = SystemNotificationsInfo {
    waiting: true,
    stuck: true,
    limit: true,
    finished: true,
    this_mac: true,
};

/// A local hour outside the default quiet hours (22–7), so nothing is
/// held unless a test asks.
fn daytime(state: &Arc<AppState>) {
    state.notifier.local_hour_seen(Some(12), now_ms());
}

#[test]
fn off_by_default_nothing_is_shown() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    let (id, _) = session(&state, &dir.path().join("payments-api"), "s-1", "Work Max");
    let stored = prefs::read(&state.storage.lock().unwrap());
    assert_eq!(
        stored,
        SystemNotificationsInfo::default(),
        "every kind starts off"
    );
    for status in ["waiting_user", "stuck", "quota_blocked", "finished"] {
        notifications::session_status(&state, &id, status);
    }
    assert!(list.lock().unwrap().is_empty());
}

#[test]
fn a_waiting_session_is_told_once_with_its_name_folder_and_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(
        &state,
        SystemNotificationsInfo {
            waiting: true,
            ..Default::default()
        },
    );
    let (id, _) = session(&state, &dir.path().join("payments-api"), "s-1", "Work Max");

    notifications::session_status(&state, &id, "waiting_user");
    notifications::session_status(&state, &id, "waiting_user");
    notifications::show_due(&state);
    // Only waiting is on: a finish isn't told.
    notifications::session_status(&state, &id, "finished");

    let list = list.lock().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0].title, "payments-api");
    assert_eq!(list[0].body, "Waiting for your reply · Work Max");
    assert_eq!(
        state.notifier.target_of(list[0].id),
        Some(Target::Session(id))
    );
}

#[test]
fn each_kind_says_what_happened() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(&state, ALL);
    for (n, status) in ["waiting_user", "stuck", "quota_blocked", "finished"]
        .into_iter()
        .enumerate()
    {
        let (id, _) = session(
            &state,
            &dir.path().join(format!("repo-{n}")),
            &format!("s-{n}"),
            &format!("Account {n}"),
        );
        notifications::session_status(&state, &id, status);
    }
    let bodies: Vec<String> = list
        .lock()
        .unwrap()
        .iter()
        .map(|o| o.body.clone())
        .collect();
    assert_eq!(
        bodies,
        [
            "Waiting for your reply · Account 0",
            "No output for a while · Account 1",
            "Hit its usage limit · Account 2",
            "Finished · Account 3",
        ]
    );
}

#[test]
fn quiet_hours_hold_it_until_they_end() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    turn_on(&state, ALL);
    let now = now_ms();
    // 23:00 local: inside the default quiet hours (22–7).
    state.notifier.local_hour_seen(Some(23), now);
    let (waiting, _) = session(&state, &dir.path().join("a"), "s-a", "Work Max");
    let (stuck, _) = session(&state, &dir.path().join("b"), "s-b", "Personal");
    notifications::session_status(&state, &waiting, "waiting_user");
    notifications::session_status(&state, &stuck, "stuck");
    notifications::show_due(&state);
    assert!(list.lock().unwrap().is_empty(), "held in quiet hours");
    // The stuck one got going again overnight.
    notifications::session_status(&state, &stuck, "running");
    // Morning: the webview's tick says 08:00.
    state.notifier.local_hour_seen(Some(8), now_ms());
    notifications::show_due(&state);
    let list = list.lock().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(
        state.notifier.target_of(list[0].id),
        Some(Target::Session(waiting))
    );
}

#[test]
fn an_unknown_hour_holds_it_as_the_digest_does() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    turn_on(&state, ALL);
    let (id, _) = session(&state, &dir.path().join("a"), "s-a", "Work Max");
    notifications::session_status(&state, &id, "waiting_user");
    assert!(list.lock().unwrap().is_empty());
    daytime(&state);
    notifications::show_due(&state);
    assert_eq!(list.lock().unwrap().len(), 1);
}

#[test]
fn an_account_left_out_of_the_digest_stays_quiet() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(&state, ALL);
    let (muted, muted_account) = session(&state, &dir.path().join("a"), "s-a", "Personal");
    let (heard, _) = session(&state, &dir.path().join("b"), "s-b", "Work Max");
    let mut digest = juggler_app::settings_policy::notification_prefs_get_impl(&state).unwrap();
    digest.digest_muted_accounts = vec![muted_account];
    juggler_app::settings_policy::notification_prefs_set_impl(&state, &digest).unwrap();
    notifications::session_status(&state, &muted, "finished");
    notifications::session_status(&state, &heard, "finished");
    let list = list.lock().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(
        state.notifier.target_of(list[0].id),
        Some(Target::Session(heard))
    );
}

#[test]
fn nothing_while_ai_juggler_is_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(&state, ALL);
    let (id, _) = session(&state, &dir.path().join("a"), "s-a", "Work Max");
    state.notifier.set_in_front(true);
    notifications::session_status(&state, &id, "waiting_user");
    // Seen in the window: not told later either.
    state.notifier.set_in_front(false);
    notifications::show_due(&state);
    assert!(list.lock().unwrap().is_empty());
}

#[test]
fn this_mac_getting_tight_says_why_and_opens_this_mac() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(
        &state,
        SystemNotificationsInfo {
            this_mac: true,
            ..Default::default()
        },
    );
    let room = |level: &str, words: &str, why: &str| juggler_app::machine_health::RoomSummaryInfo {
        level: level.into(),
        words: words.into(),
        reasons: if why.is_empty() {
            Vec::new()
        } else {
            vec![juggler_app::machine_health::RoomReasonInfo {
                kind: "memory".into(),
                level: level.into(),
                words: why.into(),
            }]
        },
    };
    notifications::this_mac_room(&state, &room("plenty", "Plenty of room", ""));
    notifications::this_mac_room(&state, &room("tight", "Getting tight", "Memory 88% used"));
    notifications::this_mac_room(
        &state,
        &room("pressure", "Under pressure", "Memory pressure"),
    );
    let list = list.lock().unwrap();
    assert_eq!(list.len(), 1, "tight then pressure is one state: {list:?}");
    assert_eq!(list[0].title, "This Mac: getting tight");
    assert_eq!(list[0].body, "Memory 88% used");
    assert_eq!(state.notifier.target_of(list[0].id), Some(Target::ThisMac));
}

#[test]
fn a_click_brings_ai_juggler_forward_on_that_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(&state, ALL);
    let (id, _) = session(&state, &dir.path().join("a"), "s-a", "Work Max");
    notifications::session_status(&state, &id, "waiting_user");
    let clicked = list.lock().unwrap()[0].id;

    let app = tauri::test::mock_app();
    let opened = Arc::new(Mutex::new(Vec::<OpenRequest>::new()));
    let sink = Arc::clone(&opened);
    app.listen(notifications::EVENT_OPEN, move |event| {
        sink.lock()
            .unwrap()
            .push(serde_json::from_str(event.payload()).unwrap());
    });
    notifications::clicked(app.handle(), &state, clicked);
    // An id from before a restart opens nothing but the window.
    notifications::clicked(app.handle(), &state, 9_999);
    assert_eq!(
        *opened.lock().unwrap(),
        vec![OpenRequest {
            session_id: Some(id),
            page: None
        }]
    );
}
