//! AJ-E2501: the one notification model, wired to its sources and views.
//! A finish is written once under Earlier; a crash restarted says so in
//! plain words; the system notifications are a view of the model (a
//! snoozed need, a muted kind or project doesn't interrupt; the model's
//! own events carry its words); and the count is the same on the bell,
//! the badge and the menu-bar item. Nothing here asks the system: the
//! notifier's sink is a list.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::notifications::centre::{self, NeedInput, NoticeTarget};
use juggler_app::features::notifications::engine::{Kind, Target};
use juggler_app::features::notifications::prefs::{self, SystemNotificationsInfo};
use juggler_app::features::notifications::views::OpenNowInput;
use juggler_app::features::notifications::{self, Outgoing, needs, sources, tray, views};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
};

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(dir.join("no-claude"))));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

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

fn session(state: &Arc<AppState>, folder: &Path, id: &str, account: &str) -> String {
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
    id.to_owned()
}

fn daytime(state: &Arc<AppState>) {
    state
        .notifier
        .local_hour_seen(Some(12), Timestamp::now().millis());
}

fn turn_on(state: &Arc<AppState>, kinds: SystemNotificationsInfo) {
    prefs::notifications_set_impl(state, kinds).unwrap();
}

fn earlier_titles(state: &Arc<AppState>) -> Vec<String> {
    let storage = state.storage.lock().unwrap();
    centre::page(
        &storage,
        "earlier",
        &PageRequest::first(50),
        Timestamp::now().millis(),
    )
    .unwrap()
    .items
    .into_iter()
    .map(|row| row.title)
    .collect()
}

fn asks(session_id: &str) -> NeedInput {
    NeedInput {
        key: format!("s:{session_id}"),
        kind: "asks".to_owned(),
        title: "“payments-api” needs your reply".to_owned(),
        detail: "payments-api · Work Max".to_owned(),
        project_id: None,
        project: None,
        target: NoticeTarget {
            session_id: Some(session_id.to_owned()),
            ..NoticeTarget::default()
        },
        action: Some("open".to_owned()),
    }
}

#[test]
fn a_finish_is_written_once_under_earlier() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let id = session(&state, &dir.path().join("payments-api"), "s-1", "Work Max");
    notifications::session_status(&state, &id, "finished");
    notifications::session_status(&state, &id, "running");
    notifications::session_status(&state, &id, "finished");
    assert_eq!(earlier_titles(&state), ["“payments-api” finished"]);
}

#[test]
fn a_crash_restarted_says_so_in_plain_words_and_may_interrupt() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(
        &state,
        SystemNotificationsInfo {
            crash: true,
            ..Default::default()
        },
    );
    let crashed = session(
        &state,
        &dir.path().join("nightly-fix"),
        "s-crash",
        "Work Max",
    );
    let again = session(
        &state,
        &dir.path().join("nightly-fix"),
        "s-again",
        "Personal",
    );
    sources::restarted(&state, &crashed, &again, 1, 3);
    sources::restarted(&state, &crashed, &again, 1, 3);
    assert_eq!(
        earlier_titles(&state),
        ["“nightly-fix” crashed and was restarted (try 1 of 3)"]
    );
    let told = list.lock().unwrap().clone();
    assert_eq!(told.len(), 1, "told once");
    assert_eq!(
        told[0].title,
        "“nightly-fix” crashed and was restarted (try 1 of 3)"
    );
    assert_eq!(
        state.notifier.target_of(told[0].id),
        Some(Target::Session(again))
    );
}

#[test]
fn a_snoozed_need_or_a_muted_kind_doesnt_interrupt() {
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
    let snoozed = session(
        &state,
        &dir.path().join("payments-api"),
        "s-snoozed",
        "Work Max",
    );
    let now = Timestamp::now().millis();
    {
        let storage = state.storage.lock().unwrap();
        centre::sync_needs(&storage, &[asks(&snoozed)], now).unwrap();
        centre::snooze(
            &storage,
            &format!("s:{snoozed}"),
            centre::Snooze::Until(now + 3_600_000),
            now,
        )
        .unwrap();
    }
    notifications::session_status(&state, &snoozed, "waiting_user");
    assert!(list.lock().unwrap().is_empty(), "snoozed: no notification");

    let muted = session(&state, &dir.path().join("web-app"), "s-muted", "Personal");
    centre::mute(&state.storage.lock().unwrap(), "kind", "asks", "", now).unwrap();
    notifications::session_status(&state, &muted, "waiting_user");
    assert!(
        list.lock().unwrap().is_empty(),
        "muted kind: no notification"
    );

    centre::unmute(&state.storage.lock().unwrap(), "kind", "asks").unwrap();
    let told = session(&state, &dir.path().join("api"), "s-told", "Third");
    notifications::session_status(&state, &told, "waiting_user");
    assert_eq!(list.lock().unwrap().len(), 1, "the rest are told as before");
}

#[test]
fn an_account_almost_out_of_room_is_told_once_in_the_models_words() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(
        &state,
        SystemNotificationsInfo {
            quota: true,
            ..Default::default()
        },
    );
    let now = Timestamp::now().millis();
    let notice = sources::low_notice(
        "acct-work",
        "Work Max",
        "5-hour",
        Some(93.0),
        Some(now + 3_600_000),
        2,
        false,
    );
    let info = centre::open_need(&state.storage.lock().unwrap(), &notice, now)
        .unwrap()
        .unwrap();
    notifications::model_event(
        &state,
        Target::Account("acct-work".to_owned()),
        Kind::Quota,
        &info,
    );
    notifications::model_event(
        &state,
        Target::Account("acct-work".to_owned()),
        Kind::Quota,
        &info,
    );
    let told = list.lock().unwrap().clone();
    assert_eq!(told.len(), 1, "once while it holds");
    assert_eq!(told[0].title, "Work Max is almost out of room");
    assert_eq!(told[0].body, "7% of its 5-hour window left");
    // Muted, the next account's isn't told.
    centre::mute(&state.storage.lock().unwrap(), "kind", "quota", "", now).unwrap();
    let other = sources::low_notice("acct-b", "Personal", "weekly", Some(97.0), None, 0, false);
    let info = centre::open_need(&state.storage.lock().unwrap(), &other, now)
        .unwrap()
        .unwrap();
    notifications::model_event(
        &state,
        Target::Account("acct-b".to_owned()),
        Kind::Quota,
        &info,
    );
    assert_eq!(list.lock().unwrap().len(), 1);
}

#[test]
fn the_count_is_the_same_on_the_bell_the_badge_and_the_menu_bar_item() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let changes = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&changes);
    state.notifier.set_on_change(Box::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
    }));
    let now = Timestamp::now().millis();
    {
        let storage = state.storage.lock().unwrap();
        centre::sync_needs(&storage, &[asks("a"), asks("b")], now).unwrap();
        let quota = sources::low_notice(
            "acct-work",
            "Work Max",
            "5-hour",
            Some(93.0),
            None,
            1,
            false,
        );
        centre::open_need(&storage, &quota, now).unwrap();
        // D-49: a snoozed need leaves the count; a muted one stays.
        centre::snooze(&storage, "s:a", centre::Snooze::Until(now + 60_000), now).unwrap();
        centre::mute(&storage, "kind", "quota", "", now).unwrap();
    }
    notifications::changed(&state);
    assert_eq!(
        changes.load(Ordering::SeqCst),
        1,
        "a change reaches the views"
    );
    let summary = views::summary_now(&state);
    let needs = centre::open_needs(&state.storage.lock().unwrap(), now, tray::NEEDS_LISTED);
    let menu = tray::build(summary.count, &needs, &OpenNowInput::default(), now);
    assert_eq!(summary.count, 2, "the bell");
    assert_eq!(menu.title.as_deref(), Some("2"), "the menu-bar item");
    assert_eq!(
        juggler_platform::system::badge_value(summary.count),
        Some(2),
        "the badge"
    );
    assert_eq!(menu.lines.iter().filter(|line| matches!(line, tray::TrayLine::Item { id, .. } if id.starts_with(tray::NOTICE_PREFIX))).count(), 2);
}

#[test]
fn reading_snoozing_and_muting_change_no_session_and_start_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let id = session(&state, &dir.path().join("payments-api"), "s-1", "Work Max");
    let now = Timestamp::now().millis();
    let before = state.storage.lock().unwrap().sessions().list(None).unwrap();
    {
        let storage = state.storage.lock().unwrap();
        centre::sync_needs(&storage, &[asks(&id)], now).unwrap();
        let notice = centre::page(&storage, "needs", &PageRequest::first(5), now)
            .unwrap()
            .items[0]
            .id;
        centre::read(&storage, Some(notice), now).unwrap();
        centre::snooze(
            &storage,
            &format!("s:{id}"),
            centre::Snooze::Until(now + 60_000),
            now,
        )
        .unwrap();
        centre::dismiss(&storage, notice, now).unwrap();
        centre::mute(&storage, "kind", "asks", "", now).unwrap();
        centre::read(&storage, None, now).unwrap();
    }
    let after = state.storage.lock().unwrap().sessions().list(None).unwrap();
    assert_eq!(before, after, "the user's marks touch no session");
}

/// D-49: Needs you is worked out in the backend, on its own tick, with no
/// window: one list and one count for the rail, the bell, the badge and
/// the menu-bar item. A snoozed need leaves the count and comes back by
/// itself when its snooze ends.
#[test]
fn needs_you_is_worked_out_in_the_backend_with_no_window() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let changes = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&changes);
    state.notifier.set_on_change(Box::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
    }));
    session(&state, &dir.path().join("payments-api"), "s-a", "Work Max");
    session(&state, &dir.path().join("web-app"), "s-b", "Personal");
    session(&state, &dir.path().join("docs-site"), "s-c", "Team");
    let mut sessions = commands::instances_shared_snapshot(&state).unwrap();
    for row in &mut sessions {
        if row.id != "s-c" {
            row.status = "waiting_user".to_owned();
        }
    }
    let sources = needs::NeedSources {
        sessions,
        computer: "Mac",
        ..needs::NeedSources::default()
    };
    let now = Timestamp::now().millis();
    let counts = |at: i64| {
        let storage = state.storage.lock().unwrap();
        let clock = centre::Clock {
            now: at,
            utc_offset_hours: None,
        };
        let bell = centre::summary(&storage, clock).unwrap().count;
        let rail = centre::page(&storage, "needs", &PageRequest::first(50), at)
            .unwrap()
            .items
            .iter()
            .filter(|row| !row.snoozed)
            .count();
        let listed = centre::open_needs(&storage, at, tray::NEEDS_LISTED);
        let menu = tray::build(bell, &listed, &state.notifier.open_now(), at);
        let badge = juggler_platform::system::badge_value(bell);
        (bell, u32::try_from(rail).unwrap(), menu.title, badge)
    };

    assert!(
        needs::refresh_from(&state, &sources, now),
        "the first tick writes the list"
    );
    assert_eq!(changes.load(Ordering::SeqCst), 1, "the views follow");
    assert_eq!(counts(now), (2, 2, Some("2".to_owned()), Some(2)));
    let open = state.notifier.open_now();
    assert_eq!(
        (open.total, open.working),
        (3, 1),
        "D-47: open now holds the sessions that ask you too; one works"
    );
    assert!(
        !needs::refresh_from(&state, &sources, now),
        "nothing changed: nothing written"
    );
    assert_eq!(changes.load(Ordering::SeqCst), 1);

    // Snoozed: out of every count until it wakes.
    centre::snooze(
        &state.storage.lock().unwrap(),
        "s:s-a",
        centre::Snooze::Until(now + 60_000),
        now,
    )
    .unwrap();
    assert_eq!(counts(now), (1, 1, Some("1".to_owned()), Some(1)));
    // Its time come, the tick ends it: back in every count, the views told.
    assert!(needs::refresh_from(&state, &sources, now + 61_000));
    assert_eq!(changes.load(Ordering::SeqCst), 2);
    assert_eq!(counts(now + 61_000), (2, 2, Some("2".to_owned()), Some(2)));
    assert!(
        state
            .storage
            .lock()
            .unwrap()
            .notices()
            .snoozes(now)
            .unwrap()
            .is_empty(),
        "the ended snooze is gone"
    );

    // A need that ends leaves the list on the next tick.
    let mut fewer = needs::NeedSources {
        sessions: sources.sessions.clone(),
        computer: "Mac",
        ..needs::NeedSources::default()
    };
    fewer.sessions.retain(|row| row.id != "s-b");
    assert!(needs::refresh_from(&state, &fewer, now + 62_000));
    assert_eq!(counts(now + 62_000).0, 1);
}
