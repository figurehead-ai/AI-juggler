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
    let account =
        juggler_app::features::account::create_account(state, CLAUDE_CODE_ID, account, None)
            .unwrap();
    let recipe =
        juggler_app::features::profile::create_profile(state, &account.id, "Default", None, None)
            .unwrap();
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
    // Told as soon as it waits: AJ-E2904's threshold (5 min by default)
    // at 0, so this test is about snoozing and muting only.
    let mut kept = juggler_app::settings_policy::notification_prefs_get_impl(&state).unwrap();
    kept.wait_after_min = 0;
    juggler_app::settings_policy::notification_prefs_set_impl(&state, &kept).unwrap();
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

/// A folder no Account lists: its notice is its own.
fn lone(id: &str, name: &str) -> sources::QuotaSubject {
    sources::QuotaSubject {
        id: id.to_owned(),
        name: name.to_owned(),
        folder: id.to_owned(),
        short: None,
    }
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
    // D-62: the limits are the Account's, so its notice names the Account
    // (never its folder, "Work Max") and acts on its folder.
    let notice = sources::low_notice(
        &sources::QuotaSubject {
            id: "acct:anthropic:work".to_owned(),
            name: "Claude Max 20x".to_owned(),
            folder: "acct-work".to_owned(),
            short: None,
        },
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
    assert_eq!(told[0].title, "Claude Max 20x is almost out of room");
    assert_eq!(told[0].body, "7% of its 5-hour window left");
    // Muted, the next account's isn't told.
    centre::mute(&state.storage.lock().unwrap(), "kind", "quota", "", now).unwrap();
    let other = sources::low_notice(
        &lone("acct-b", "Personal"),
        "weekly",
        Some(97.0),
        None,
        0,
        false,
    );
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
            &lone("acct-work", "Work Max"),
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
    // AJ-E3514 (the persona walk's pilot): the Floor says the same number,
    // from the same count, on the same state.
    let floor = juggler_app::features::floor::rebuild(&state).unwrap();
    assert_eq!(floor.needs, summary.count, "the Floor");
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
    let mut sessions =
        juggler_app::features::sessions::list::instances_shared_snapshot(&state).unwrap();
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

// ---------------------------------------------------------------------
// AJ-E2501 R3: Needs you from every source, worked out in the backend.
// ---------------------------------------------------------------------

use juggler_app::features::account::dto::AccountInfo;
use juggler_app::features::budgets::dto::BudgetInfo;
use juggler_app::features::conversations::dto::{ConversationInfo, PlaceInfo};
use juggler_app::features::messages::dto::MessageNeedInfo;
use juggler_app::features::sessions::dto::InstanceInfo;
use juggler_app::features::tasks::dto::TaskNeedInfo;
use juggler_app::machine_alerts::OverloadAlertInfo;
use juggler_app::machine_health::{self, ThisMacInfo, ToolTreeInfo};
use std::collections::{HashMap, HashSet};

/// AI Juggler's own sessions as the tick reads them, each in `status`.
fn own_rows(state: &Arc<AppState>, dir: &Path, ids: &[(&str, &str)]) -> Vec<InstanceInfo> {
    for (id, _) in ids {
        session(state, &dir.join(id), id, &format!("Account {id}"));
    }
    let mut rows = juggler_app::features::sessions::list::instances_shared_snapshot(state).unwrap();
    for row in &mut rows {
        let status = ids.iter().find(|(id, _)| *id == row.id).unwrap().1;
        status.clone_into(&mut row.status);
        row.needs_attention = None;
    }
    rows
}

/// Another app's session, live in `app`.
fn theirs(id: &str, app: &str, app_id: &str, state: Option<&str>) -> ConversationInfo {
    ConversationInfo {
        id: id.to_owned(),
        tool: "claude-code".to_owned(),
        account_id: "acct-x".to_owned(),
        account_label: "Work Max".to_owned(),
        project_id: None,
        cwd: Some(format!("/w/{id}")),
        title: Some(id.to_owned()),
        git_branch: None,
        started_in: None,
        started_from: None,
        app: None,
        started_ms: Some(1_000),
        last_activity_ms: Some(2_000),
        tokens: None,
        live: true,
        session_id: None,
        places: vec![PlaceInfo {
            app: app.to_owned(),
            shown: app.to_owned(),
            app_id: Some(app_id.to_owned()),
            profile: None,
            instance: None,
            pid: 4_242,
            kind: Some("desktop".to_owned()),
            app_pid: Some(4_200),
            label: app.to_owned(),
        }],
        pid: Some(4_242),
        state: state.map(str::to_owned),
        probably: false,
        viewed_ms: None,
        background: false,
        inactive: false,
        subtask_count: None,
        app_archived: false,
        archived_here: false,
        app_unlisted: false,
        resume_refusal: None,
    }
}

fn account(id: &str, label: &str, login_state: &str) -> AccountInfo {
    AccountInfo {
        id: id.to_owned(),
        provider: "claude-code".to_owned(),
        label: label.to_owned(),
        plan: None,
        login_state: login_state.to_owned(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
        is_default: None,
        service: None,
        subscription_id: None,
        app_colour_slot: None,
        account_colour_slot: None,
        agent_tools_why: None,
    }
}

fn cap(id: &str, name: &str, level: &str, seen: bool) -> BudgetInfo {
    BudgetInfo {
        id: id.to_owned(),
        name: name.to_owned(),
        monthly_usd: 100.0,
        account_id: None,
        project_ids: Vec::new(),
        spent_usd: Some(104.0),
        unpriced: false,
        level: level.to_owned(),
        seen,
    }
}

/// This Mac short of memory, a session working it hard, three processes
/// left running.
fn pressed_mac(hard_on: &str) -> ThisMacInfo {
    let mut machine = machine_health::health_of(
        &juggler_supervisor::machine::MachineSnapshot::default(),
        &juggler_supervisor::machine::RoomThresholds::default(),
    );
    machine.sampled = true;
    let mut mac = machine_health::assemble(machine, &[], (3, 0));
    mac.alerts.push(OverloadAlertInfo {
        key: "memory_pressure".to_owned(),
        kind: "memory_pressure".to_owned(),
        title: "Your Mac is short of memory".to_owned(),
        detail: "memory is low".to_owned(),
        session_id: None,
        place: None,
        subject_name: None,
        subject_bytes: None,
        subject_estimate: false,
        subject_cpu: None,
        subject_windows: None,
        other_apps: Vec::new(),
        since_ms: 1_000,
    });
    mac.heavy.push(ToolTreeInfo {
        root_pid: Some(77),
        session_id: Some(hard_on.to_owned()),
        place: None,
        label: None,
        tool: Some("claude-code".to_owned()),
        group: "claude-code".to_owned(),
        account_label: None,
        cpu_pct: Some(300.0),
        rss_bytes: 2_000_000_000,
        helpers: 2,
        heavy_note: Some("CPU 300%".to_owned()),
        title: None,
        memory_estimate: false,
    });
    mac
}

/// Every source at once: what needs the user, and what is left out.
fn every_source(state: &Arc<AppState>, dir: &Path) -> needs::NeedSources {
    let mut sessions = own_rows(
        state,
        dir,
        &[
            ("s-wait", "waiting_user"),
            ("s-stuck", "stuck"),
            ("s-limit", "quota_blocked"),
            ("s-failed", "failed"),
            ("s-parked", "waiting_user"),
            ("s-archived", "waiting_user"),
            ("s-forgot", "waiting_user"),
            ("s-run", "running"),
        ],
    );
    for row in &mut sessions {
        match row.id.as_str() {
            "s-wait" => row.project_id = Some("p-web".to_owned()),
            "s-parked" => row.parked = Some("limit".to_owned()),
            "s-archived" => row.lifecycle = Some("archived".to_owned()),
            "s-forgot" => row.project_id = Some("p-gone".to_owned()),
            _ => {}
        }
    }
    let mut done_unseen = theirs(
        "c-done-unseen",
        "Codex",
        "codex-desktop",
        Some("done_review"),
    );
    done_unseen.viewed_ms = Some(1_500);
    let done_seen = theirs("c-done-seen", "Codex", "codex-desktop", Some("done_review"));
    let mut done_viewed = theirs(
        "c-done-viewed",
        "Codex",
        "codex-desktop",
        Some("done_review"),
    );
    done_viewed.viewed_ms = Some(2_500);
    let mut background = theirs("c-bg", "Claude app", "claude-desktop", Some("working"));
    background.background = true;
    needs::NeedSources {
        sessions,
        conversations: vec![
            theirs("c-asks", "Claude app", "claude-desktop", Some("asks_you")),
            theirs("c-working", "Claude app", "claude-desktop", Some("working")),
            done_unseen,
            done_seen,
            done_viewed,
            background,
            theirs("c-hidden", "Cursor", "cursor", Some("asks_you")),
        ],
        accounts: vec![
            account("acct-in", "Work Max", "signed_in"),
            account("acct-out", "Personal", "logged_out"),
        ],
        projects: HashMap::from([
            ("p-web".to_owned(), "web".to_owned()),
            ("p-gone".to_owned(), "old-site".to_owned()),
        ]),
        forgotten: HashSet::from(["p-gone".to_owned()]),
        hidden_apps: HashSet::from(["cursor".to_owned()]),
        // Looked at here after it finished.
        seen: HashMap::from([("c:acct-x/c-done-seen".to_owned(), 3_000)]),
        this_mac: Some(pressed_mac("s-run")),
        caps: vec![
            cap("cap-over", "Team", "over", false),
            cap("cap-under", "Side", "under", false),
            cap("cap-seen", "Old", "over", true),
        ],
        messages: vec![MessageNeedInfo {
            key: "ask-1".to_owned(),
            kind: "ask".to_owned(),
            pool_id: "pool-1".to_owned(),
            pool_name: "Checkout".to_owned(),
            thread_id: Some("t-1".to_owned()),
            message_id: None,
            session_id: Some("s-run".to_owned()),
            ask_id: Some("ask-1".to_owned()),
            offer_always: false,
            title: "An agent asks to read the roster".to_owned(),
            detail: "From reviewer".to_owned(),
            preview: None,
            at_ms: 1_000,
            line: None,
        }],
        tasks: vec![TaskNeedInfo {
            id: "task-1".to_owned(),
            kind: "review".to_owned(),
            pool_id: "pool-1".to_owned(),
            pool_name: "Checkout".to_owned(),
            task_id: Some("t-9".to_owned()),
            task_title: Some("Fix the cart".to_owned()),
            title: "A task waits for your review".to_owned(),
            detail: "Fix the cart".to_owned(),
            offer_always: false,
            project_id: Some("p-web".to_owned()),
            at_ms: 1_000,
        }],
        sign_in_words: HashMap::from([("claude-code".to_owned(), "claude /login".to_owned())]),
        computer: "Mac",
        ..needs::NeedSources::default()
    }
}

/// The rc19 walk: a held message's row in the rail read "«Untitled
/// session · r3-tools started 01:15 PM» wants t…", its reason (Ask first)
/// out of sight. Its line is who wants to message whom, and the reason
/// with its pool under it.
#[test]
fn a_held_message_says_who_and_why_in_the_rails_room() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut sources = every_source(&state, dir.path());
    sources.messages[0].pool_name = "Team".to_owned();
    sources.messages[0].line = Some(juggler_app::features::messages::dto::NeedLine {
        title: "«r3-tools» wants to message «web»".to_owned(),
        reason: "Ask first".to_owned(),
    });
    let (needs, _) = needs::work_out(&sources, &HashSet::<String>::new());
    let row = needs
        .iter()
        .find(|need| need.key == "message:ask-1")
        .unwrap();
    assert_eq!(row.title, "«r3-tools» wants to message «web»");
    assert_eq!(row.detail, "Ask first in «Team»");
}

/// 0.4 (walk A1): a folder a Claude app profile signs in for (its Code
/// tab) isn't told to sign in "in a terminal here": you sign in inside
/// that app, and its terminal tool is never set up.
#[test]
fn a_folder_signed_in_through_a_desktop_profile_needs_no_terminal_sign_in() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut sources = every_source(&state, dir.path());
    sources.through_app.insert("acct-out".to_owned());
    let (needs, _) = needs::work_out(&sources, &HashSet::<String>::new());
    assert!(needs.iter().all(|need| need.kind != "signin"), "{needs:?}");
}

/// An App whose sign-in is a key it holds (OpenCode's `auth login` with a
/// pasted key reads as `api_key`) has signed in: Needs you doesn't ask it
/// to sign in (rc17 walk: the card stayed after the key was added).
#[test]
fn an_app_signed_in_with_an_api_key_needs_no_sign_in() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut sources = every_source(&state, dir.path());
    for account in &mut sources.accounts {
        if account.id == "acct-out" {
            account.login_state = "api_key".to_owned();
        }
    }
    let (needs, _) = needs::work_out(&sources, &HashSet::<String>::new());
    assert!(needs.iter().all(|need| need.kind != "signin"), "{needs:?}");
}

/// AJ-E2501 R3 (D-49): Needs you over every source: AI Juggler's sessions
/// that wait, may be stuck, hit their limit or failed (not parked,
/// archived or in a forgotten project); other apps' sessions that ask or
/// finished unseen (the seen marks and the app's own last view; apps
/// turned off in Settings left out); sign-ins; This Mac (its alerts, a
/// session working it hard, processes left running); monthly caps;
/// agents' messages and tasks.
#[test]
fn needs_you_is_worked_out_from_every_source() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let sources = every_source(&state, dir.path());
    let (needs, _) = needs::work_out(&sources, &HashSet::<String>::new());
    let mut kinds: Vec<(String, String)> = needs
        .iter()
        .map(|need| (need.key.clone(), need.kind.clone()))
        .collect();
    kinds.sort();
    let pair = |key: &str, kind: &str| (key.to_owned(), kind.to_owned());
    let mut want = vec![
        pair("s:s-wait", "asks"),
        pair("s:s-stuck", "stuck"),
        pair("s:s-limit", "limit"),
        pair("s:s-failed", "failed"),
        pair("c:acct-x/c-asks", "asks"),
        pair("c:acct-x/c-done-unseen", "review"),
        pair("signin:acct-out", "signin"),
        pair("room", "room"),
        pair("heavy:s-run", "heavy"),
        pair("leftovers", "leftovers"),
        pair("cap:cap-over", "cap"),
        pair("message:ask-1", "message"),
        pair("task:task-1", "task"),
    ];
    want.sort();
    assert_eq!(kinds, want);

    let by_key = |key: &str| needs.iter().find(|need| need.key == key).unwrap();
    // D-96: an untitled one by its facts (its folder, its App; no time
    // before the sources say when they were read).
    assert_eq!(
        by_key("s:s-wait").title,
        "“s-wait · Account s-wait” needs your reply"
    );
    assert_eq!(by_key("s:s-wait").project.as_deref(), Some("web"));
    assert_eq!(by_key("s:s-limit").action.as_deref(), Some("limit"));
    assert_eq!(
        by_key("s:s-failed").title,
        "“s-failed · Account s-failed” stopped with an error"
    );
    assert_eq!(
        by_key("c:acct-x/c-asks").detail,
        "c-asks · Claude app · Work Max"
    );
    assert_eq!(
        by_key("c:acct-x/c-done-unseen").title,
        "“c-done-unseen” is done — review it"
    );
    assert_eq!(
        by_key("signin:acct-out").title,
        "Personal needs its sign-in"
    );
    assert!(
        by_key("signin:acct-out")
            .detail
            .starts_with("claude /login, in a terminal here")
    );
    assert_eq!(by_key("room").title, "Your Mac is short of memory");
    assert_eq!(by_key("room").detail, "Memory is low");
    assert_eq!(
        by_key("heavy:s-run").title,
        "Working your Mac hard — CPU 300%"
    );
    assert!(by_key("heavy:s-run").detail.ends_with("2 more processes"));
    assert_eq!(by_key("leftovers").title, "3 processes left running");
    assert_eq!(by_key("cap:cap-over").title, "Team is over its monthly cap");
    assert_eq!(by_key("message:ask-1").detail, "Checkout · From reviewer");
    assert_eq!(by_key("task:task-1").project_id.as_deref(), Some("p-web"));

    // A session already listed for its own need isn't listed again as
    // working the Mac hard.
    let mut busy = sources;
    busy.this_mac = Some(pressed_mac("s-wait"));
    let (needs, _) = needs::work_out(&busy, &HashSet::<String>::new());
    assert!(!needs.iter().any(|need| need.key == "heavy:s-wait"));
    // Under pressure with no alert of its own, This Mac still says so.
    let mut calm = busy;
    let mut mac = pressed_mac("s-run");
    mac.alerts.clear();
    mac.machine.summary.level = "pressure".to_owned();
    calm.this_mac = Some(mac);
    let (needs, _) = needs::work_out(&calm, &HashSet::<String>::new());
    let room = needs.iter().find(|need| need.key == "room").unwrap();
    assert_eq!(room.title, "Your Mac is under pressure");
}

/// AJ-E2506 R3: what is open now, by app, from the same tick: AI
/// Juggler's sessions that work or ask, other apps' live ones in the
/// foreground (an app turned off left out), most first; a snoozed ask
/// waits in its place.
#[test]
fn open_now_is_counted_by_app_in_the_backend() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let sources = every_source(&state, dir.path());
    let (_, open) = needs::work_out(&sources, &HashSet::<String>::new());
    let apps: Vec<(&str, u32)> = open
        .apps
        .iter()
        .map(|app| (app.app.as_str(), app.count))
        .collect();
    assert_eq!(
        apps,
        [("AI Juggler", 4), ("Codex", 3), ("Claude app", 2)],
        "the waiting, stuck and limited sessions and the running one; never a failed, parked, archived or forgotten one, a background one or Cursor's"
    );
    assert_eq!((open.total, open.working), (9, 2));
    let snoozed = HashSet::from(["s:s-wait".to_owned()]);
    let (_, open) = needs::work_out(&sources, &snoozed);
    assert_eq!(open.total, 8);
    assert_eq!(open.apps[0].count, 3);
    // The menu-bar item lists them as worked out.
    let menu = tray::build(0, &[], &open, 0);
    assert!(menu.lines.contains(&tray::TrayLine::Heading {
        text: "Open now · 8 · 2 working".to_owned()
    }));
    assert!(menu.lines.contains(&tray::TrayLine::Note {
        text: "AI Juggler · 3 · Codex · 3 · Claude app · 2".to_owned()
    }));
}

/// AJ-E2501 R3: the list is written only when it changed: new needs
/// unread, a changed one brought up to date with its read mark kept, a
/// gone one ended; an unchanged tick writes nothing.
#[test]
fn every_source_is_written_only_when_it_changed() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let mut sources = every_source(&state, dir.path());
    let now = Timestamp::now().millis();
    assert!(needs::refresh_from(&state, &sources, now));
    let rows = |at: i64| {
        centre::page(
            &state.storage.lock().unwrap(),
            "needs",
            &PageRequest::first(50),
            at,
        )
        .unwrap()
        .items
    };
    assert_eq!(rows(now).len(), 13);
    assert!(rows(now).iter().all(|row| row.unread), "new: unread");
    assert!(
        !needs::refresh_from(&state, &sources, now + 1),
        "the same: nothing written"
    );

    let asks = rows(now)
        .into_iter()
        .find(|row| row.subject == "c:acct-x/c-asks")
        .unwrap();
    centre::read(&state.storage.lock().unwrap(), Some(asks.id), now).unwrap();
    // Its title changes; the cap is seen; a new sign-in is due.
    sources.conversations[0].title = Some("checkout-flow".to_owned());
    sources.caps.retain(|cap| cap.id != "cap-over");
    sources
        .accounts
        .push(account("acct-3", "Side", "logged_out"));
    assert!(needs::refresh_from(&state, &sources, now + 2));
    let after = rows(now + 2);
    assert_eq!(after.len(), 13);
    let changed = after.iter().find(|row| row.id == asks.id).unwrap();
    assert_eq!(changed.title, "“checkout-flow” needs your reply");
    assert!(!changed.unread, "brought up to date, its read mark kept");
    assert!(
        !after.iter().any(|row| row.subject == "cap:cap-over"),
        "ended"
    );
    assert!(
        after
            .iter()
            .find(|row| row.subject == "signin:acct-3")
            .unwrap()
            .unread
    );
}

/// AJ-E2501 R5: dismissing a banner leaves the count as it was (the
/// notice stays a need until it ends).
#[test]
fn a_dismissal_leaves_the_count_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let now = Timestamp::now().millis();
    let count = || views::summary_now(&state).count;
    {
        let storage = state.storage.lock().unwrap();
        centre::sync_needs(&storage, &[asks("a"), asks("b")], now).unwrap();
        let quota = sources::low_notice(
            &lone("acct-work", "Work Max"),
            "5-hour",
            Some(93.0),
            None,
            1,
            false,
        );
        centre::open_need(&storage, &quota, now).unwrap();
    }
    assert_eq!(count(), 3);
    let id = centre::page(
        &state.storage.lock().unwrap(),
        "needs",
        &PageRequest::first(50),
        now,
    )
    .unwrap()
    .items
    .into_iter()
    .find(|row| row.kind == "quota")
    .unwrap()
    .id;
    centre::dismiss(&state.storage.lock().unwrap(), id, now).unwrap();
    assert_eq!(count(), 3, "dismissed, it still counts");
    centre::read(&state.storage.lock().unwrap(), None, now).unwrap();
    assert_eq!(count(), 3, "read, they still count");
}

/// AJ-E2501 R7: "an account has room again" reaches a system notification
/// when its kind is on, in the model's words; a muted kind doesn't; and a
/// snoozed need never interrupts (an account's room need snoozed until
/// woken, opening again).
#[test]
fn room_again_is_told_when_on_and_never_muted_or_snoozed() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let list = shown(&state);
    daytime(&state);
    turn_on(
        &state,
        SystemNotificationsInfo {
            reset: true,
            quota: true,
            ..Default::default()
        },
    );
    let now = Timestamp::now().millis();
    let told = |subject: &sources::QuotaSubject| {
        let notice = sources::reset_notice(subject, "weekly", now, None);
        let info = centre::record(&state.storage.lock().unwrap(), &notice, now)
            .unwrap()
            .unwrap();
        notifications::model_event(
            &state,
            Target::Account(subject.folder.clone()),
            Kind::Reset,
            &info,
        );
    };
    told(&lone("acct-a", "Work Max"));
    {
        let list = list.lock().unwrap();
        assert_eq!(list.len(), 1, "{list:?}");
        assert_eq!(list[0].title, "Work Max has room again — its week reset");
        assert_eq!(list[0].body, "Sessions on it can carry on");
    }
    assert_eq!(
        state.notifier.target_of(list.lock().unwrap()[0].id),
        Some(Target::Account("acct-a".to_owned()))
    );

    centre::mute(&state.storage.lock().unwrap(), "kind", "reset", "", now).unwrap();
    told(&lone("acct-b", "Personal"));
    assert_eq!(list.lock().unwrap().len(), 1, "muted: not told");
    centre::unmute(&state.storage.lock().unwrap(), "kind", "reset").unwrap();

    // Snoozed until woken, the account's room need opens again: it waits
    // in the centre and doesn't interrupt.
    let subject = lone("acct-c", "Side");
    {
        let storage = state.storage.lock().unwrap();
        centre::snooze(&storage, "quota:acct-c", centre::Snooze::UntilWoken, now).unwrap();
    }
    let low = sources::low_notice(&subject, "5-hour", Some(96.0), None, 1, false);
    let info = centre::open_need(&state.storage.lock().unwrap(), &low, now)
        .unwrap()
        .unwrap();
    assert!(info.snoozed);
    notifications::model_event(
        &state,
        Target::Account("acct-c".to_owned()),
        Kind::Quota,
        &info,
    );
    assert_eq!(list.lock().unwrap().len(), 1, "snoozed: not told");
    // The same need for an account not snoozed is told.
    let low = sources::low_notice(
        &lone("acct-d", "Fourth"),
        "5-hour",
        Some(96.0),
        None,
        1,
        false,
    );
    let info = centre::open_need(&state.storage.lock().unwrap(), &low, now)
        .unwrap()
        .unwrap();
    notifications::model_event(
        &state,
        Target::Account("acct-d".to_owned()),
        Kind::Quota,
        &info,
    );
    assert_eq!(list.lock().unwrap().len(), 2);
}

/// AJ-E2904 R1: with the 5-minute setting (the default), the system
/// notification waits for the threshold, but Needs you, the bell and the
/// count show the wait at once (D-49, one count).
#[test]
fn a_wait_shows_in_needs_you_at_once_whatever_the_threshold() {
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
    assert_eq!(
        juggler_app::settings_policy::notification_prefs_get_impl(&state)
            .unwrap()
            .wait_after_min,
        5
    );
    let rows = own_rows(&state, dir.path(), &[("s-wait", "waiting_user")]);
    notifications::session_status(&state, "s-wait", "waiting_user");
    assert!(list.lock().unwrap().is_empty(), "not yet five minutes");
    let sources = needs::NeedSources {
        sessions: rows,
        computer: "Mac",
        ..needs::NeedSources::default()
    };
    let now = Timestamp::now().millis();
    assert!(needs::refresh_from(&state, &sources, now));
    let summary = views::summary_now(&state);
    assert_eq!(summary.count, 1, "the bell");
    let listed = centre::open_needs(&state.storage.lock().unwrap(), now, tray::NEEDS_LISTED);
    assert_eq!(listed.len(), 1, "Needs you");
    assert_eq!(
        listed[0].title,
        "“s-wait · Account s-wait” needs your reply"
    );
    let menu = tray::build(summary.count, &listed, &state.notifier.open_now(), now);
    assert_eq!(menu.title.as_deref(), Some("1"), "the menu-bar count");
    assert_eq!(
        juggler_platform::system::badge_value(summary.count),
        Some(1),
        "the badge"
    );
    assert!(
        list.lock().unwrap().is_empty(),
        "still no system notification"
    );
}

/// 0.4.1 (main's ruling): archiving an ended session clears its Needs
/// you row at once (it stayed until the app restarted), and unarchiving
/// brings it back, with no tick between: the archive works Needs you out
/// again itself.
#[test]
fn archiving_a_failed_session_takes_its_need_away_at_once_and_unarchiving_brings_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let id = session(&state, &dir.path().join("billing"), "s-broke", "Work Max");
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&SessionId::new(id.as_str()))
            .unwrap();
        record.status = SessionStatus::Failed;
        storage.sessions().update(&record).unwrap();
    }
    let rows = || -> Vec<String> {
        let storage = state.storage.lock().unwrap();
        centre::open_needs(&storage, Timestamp::now().millis(), tray::NEEDS_LISTED)
            .into_iter()
            .filter(|row| row.target.session_id.as_deref() == Some(id.as_str()))
            .map(|row| row.title)
            .collect()
    };
    needs::refresh(&state);
    // D-96: by its facts — folder · App · when it started (where the
    // system can say a time).
    let said = rows();
    assert_eq!(said.len(), 1, "the failed session needs you");
    let lead = if cfg!(target_os = "macos") {
        "“billing · Work Max · started "
    } else {
        "“billing · Work Max”"
    };
    assert!(
        said[0].starts_with(lead) && said[0].ends_with("” stopped with an error"),
        "{said:?}"
    );

    let bell = views::summary_now(&state).count;

    juggler_app::history::instance_archive_impl(&state, &id).unwrap();
    assert!(
        rows().is_empty(),
        "archived: its row left Needs you at once"
    );
    assert_eq!(views::summary_now(&state).count, bell - 1, "the bell too");

    juggler_app::history::instance_unarchive_impl(&state, &id).unwrap();
    assert_eq!(rows().len(), 1, "unarchived: it needs you again");
    assert_eq!(views::summary_now(&state).count, bell);
}

/// 0.4.1 (main's ruling): AI Juggler's own Archive for another app's
/// session. One archived here is out of Needs you, as an archived
/// instance is, and archiving or unarchiving works Needs you out again at
/// once, with no tick between.
#[test]
fn archiving_another_apps_session_here_takes_its_need_away_at_once_and_unarchiving_brings_it_back()
{
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let done = || theirs("c-done", "Codex", "codex-desktop", Some("done_review"));
    let sources = |archived_here: bool| needs::NeedSources {
        conversations: vec![needs_conversation(done(), archived_here)],
        computer: "Mac",
        ..needs::NeedSources::default()
    };
    let rows = || -> Vec<String> {
        let storage = state.storage.lock().unwrap();
        centre::open_needs(&storage, Timestamp::now().millis(), tray::NEEDS_LISTED)
            .into_iter()
            .filter(|row| row.target.conversation_id.as_deref() == Some("c-done"))
            .map(|row| row.title)
            .collect()
    };
    let now = Timestamp::now().millis();
    assert!(needs::refresh_from(&state, &sources(false), now));
    assert_eq!(rows().len(), 1, "finished, not looked at: it needs you");
    let bell = views::summary_now(&state).count;

    // Archived here: the next working out leaves it out ...
    assert!(needs::refresh_from(&state, &sources(true), now + 1));
    assert!(rows().is_empty(), "archived here: out of Needs you");
    // ... and back when unarchived.
    assert!(needs::refresh_from(&state, &sources(false), now + 2));
    assert_eq!(rows().len(), 1);

    // The archive itself works Needs you out again at once (here the
    // tick's own sources no longer list it, so its row goes).
    juggler_app::features::conversations::archive::set(&state, "acct-x", "c-done", true).unwrap();
    assert!(rows().is_empty(), "the archive took its row away at once");
    assert_eq!(views::summary_now(&state).count, bell - 1, "the bell too");
    assert!(needs::refresh_from(&state, &sources(false), now + 3));
    juggler_app::features::conversations::archive::set(&state, "acct-x", "c-done", false).unwrap();
    assert!(
        juggler_app::features::conversations::archive::archived(&state).is_empty(),
        "unarchived"
    );
}

/// `c` as the tick reads it, archived here or not.
fn needs_conversation(mut c: ConversationInfo, archived_here: bool) -> ConversationInfo {
    c.archived_here = archived_here;
    c
}

// ---------------------------------------------------------------------
// The views: the badge and the menu-bar item (AJ-E2502 R4, AJ-E2506).
// ---------------------------------------------------------------------

/// What the views were given, in order (the seam the app draws on).
#[derive(Default)]
struct Drawn {
    window: Mutex<Vec<u32>>,
    drawn: Mutex<Vec<(u32, tray::TrayMenu)>>,
}

impl views::Surface for Drawn {
    fn tell_window(&self, summary: centre::NoticeSummaryInfo) {
        self.window.lock().unwrap().push(summary.count);
    }

    fn draw(&self, count: u32, menu: tray::TrayMenu) {
        self.drawn.lock().unwrap().push((count, menu));
    }
}

/// AJ-E2502 R4 and AJ-E2506 R1: the views are drawn when they connect
/// (the menu-bar item made at start) and again on every change in the
/// model: the badge's count and the menu-bar item's title, tooltip and
/// lines, the same count as the bell.
#[test]
fn the_badge_and_the_menu_bar_item_follow_every_change() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let drawn = Arc::new(Drawn::default());
    views::connect_on(&(drawn.clone() as Arc<dyn views::Surface>), &state);
    {
        let all = drawn.drawn.lock().unwrap();
        assert_eq!(all.len(), 1, "made at start");
        assert_eq!(all[0].0, 0);
        assert_eq!(all[0].1.title, None);
        assert_eq!(all[0].1.tooltip, "AI Juggler · nothing needs you");
    }
    assert_eq!(*drawn.window.lock().unwrap(), [0]);

    // A model change (the tick writes two needs): every view follows.
    let rows = own_rows(
        &state,
        dir.path(),
        &[("s-a", "waiting_user"), ("s-b", "stuck")],
    );
    let sources = needs::NeedSources {
        sessions: rows,
        computer: "Mac",
        ..needs::NeedSources::default()
    };
    let now = Timestamp::now().millis();
    assert!(needs::refresh_from(&state, &sources, now));
    {
        let all = drawn.drawn.lock().unwrap();
        assert_eq!(all.len(), 2, "redrawn on the change");
        let (count, menu) = &all[1];
        assert_eq!(*count, 2, "the badge's count");
        assert_eq!(juggler_platform::system::badge_value(*count), Some(2));
        assert_eq!(menu.title.as_deref(), Some("2"));
        assert_eq!(menu.tooltip, "AI Juggler · 2 things need you");
        let notices = menu
            .lines
            .iter()
            .filter(|line| matches!(line, tray::TrayLine::Item { id, .. } if id.starts_with(tray::NOTICE_PREFIX)))
            .count();
        assert_eq!(notices, 2);
    }
    // Nothing changed: nothing redrawn.
    assert!(!needs::refresh_from(&state, &sources, now));
    assert_eq!(drawn.drawn.lock().unwrap().len(), 2);
    // A snooze is a change too: one fewer on the badge.
    centre::snooze(
        &state.storage.lock().unwrap(),
        "s:s-a",
        centre::Snooze::Until(now + 60_000),
        now,
    )
    .unwrap();
    notifications::changed(&state);
    let all = drawn.drawn.lock().unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[2].0, 1);
    assert_eq!(all[2].1.title.as_deref(), Some("1"));
    assert_eq!(
        *drawn.window.lock().unwrap(),
        [0, 2, 1],
        "the bell heard each"
    );
}

/// AJ-E2502 R4: the badge goes through the platform seam onto the main
/// window's icon (on macOS the Dock's); with no window there is nothing to
/// carry it.
#[test]
fn the_badge_is_set_on_the_main_window_through_the_platform_seam() {
    use juggler_platform::system::{AppBadge, app_badge};
    let app = tauri::test::mock_app();
    let dock = app_badge() == AppBadge::Dock;
    let before = views::show_badge(app.handle(), 3);
    assert_eq!(
        before,
        if dock {
            views::BadgeShown::NoWindow
        } else {
            views::BadgeShown::NotYet
        }
    );
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let after = views::show_badge(app.handle(), 3);
    assert_eq!(
        after,
        if dock {
            views::BadgeShown::Shown
        } else {
            views::BadgeShown::NotYet
        }
    );
    assert_eq!(
        views::show_badge(app.handle(), 0),
        after,
        "zero clears it the same way"
    );
}

/// AJ-E2506 R2: a notice's line brings AI Juggler forward on that notice
/// and nothing else happens; Open AI Juggler only brings it forward; a
/// line that only says something does nothing.
#[test]
fn a_menu_line_brings_ai_juggler_forward_on_its_notice_and_nothing_else() {
    use juggler_app::events::AppEvent;
    use tauri::Listener;
    let app = tauri::test::mock_app();
    tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let heard = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    for event in AppEvent::ALL {
        let sink = Arc::clone(&heard);
        let name = event.name().to_owned();
        app.listen(event.name(), move |e| {
            sink.lock()
                .unwrap()
                .push((name.clone(), e.payload().to_owned()));
        });
    }
    views::tray_clicked(app.handle(), &format!("{}42", tray::NOTICE_PREFIX));
    assert_eq!(
        *heard.lock().unwrap(),
        [(views::EVENT_OPEN.to_owned(), "42".to_owned())],
        "the window opens that notice; nothing else is asked"
    );
    heard.lock().unwrap().clear();
    views::tray_clicked(app.handle(), tray::OPEN_ID);
    views::tray_clicked(app.handle(), "tray:line:3");
    views::tray_clicked(app.handle(), "not-ours");
    assert!(
        heard.lock().unwrap().is_empty(),
        "{:?}",
        heard.lock().unwrap()
    );
    views::tray_clicked(app.handle(), tray::CENTRE_ID);
    assert_eq!(
        *heard.lock().unwrap(),
        [(views::EVENT_OPEN.to_owned(), "null".to_owned())],
        "and N more… opens the centre"
    );
}
