//! The Floor's snapshot rules (AJ-E3505), from plain inputs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use juggler_core::storage::App;
use juggler_floor::snapshot::FloorAppKind;
use juggler_floor::state::FloorState;

use super::feed::{
    FolderUsage, HOT_AFTER_MS, Inputs, OwnRow, assemble, initials, machine_of, users_apps,
};
use crate::features::conversations::dto::ConversationInfo;

const NOW: i64 = 1_800_000_000_000;

fn app(id: &str, kind: &str, tool: &str, folder: Option<&str>) -> App {
    App {
        id: id.into(),
        kind: kind.into(),
        tool: tool.into(),
        host_app_id: None,
        instance_dir: (kind == "desktop").then(|| "/x/b-work".to_owned()),
        folder_id: folder.map(Into::into),
        label: None,
        hidden: false,
        keep_alive: false,
        paused_json: None,
        launch_json: None,
        sessions_json: None,
        first_seen: 1,
        last_seen: NOW - 1000,
        added: false,
    }
}

fn own(id: &str, status: &str, started: i64) -> OwnRow {
    OwnRow {
        id: id.into(),
        status: status.into(),
        title: format!("Task {id}"),
        folder: Some("f-work".into()),
        started_ms: started,
        last_ms: Some(started + 10),
        ..OwnRow::default()
    }
}

fn talk(id: &str, state: &str, last: i64) -> ConversationInfo {
    ConversationInfo {
        started_from: None,
        id: id.into(),
        tool: "tool-a".into(),
        account_id: "f-work".into(),
        account_label: "Work".into(),
        project_id: None,
        cwd: Some("/w/shop".into()),
        title: None,
        git_branch: Some("main".into()),
        started_in: None,
        started_ms: Some(last - 100),
        last_activity_ms: Some(last),
        tokens: None,
        live: true,
        session_id: None,
        places: Vec::new(),
        pid: None,
        state: Some(state.into()),
        probably: false,
        viewed_ms: None,
        background: false,
        inactive: false,
        subtask_count: Some(2),
        app_archived: false,
        archived_here: false,
        resume_refusal: None,
    }
}

fn inputs() -> Inputs {
    Inputs {
        apps: vec![
            app("cli:f-work", "cli", "tool-a", Some("f-work")),
            app("app:host:/x/b-work", "desktop", "tool-a", Some("f-work")),
        ],
        colour_slots: BTreeMap::from([("cli:f-work".into(), 3)]),
        tool_names: BTreeMap::from([("tool-a".into(), "Tool A".into())]),
        families: BTreeMap::from([(
            "tool-a".into(),
            (Some("workshop".into()), Some("studio".into())),
        )]),
        folder_names: BTreeMap::from([
            ("f-work".into(), "Work Max".into()),
            ("f-home".into(), "Home".into()),
        ]),
        folder_accounts: BTreeMap::from([
            ("f-work".into(), "acct:1".into()),
            ("f-home".into(), "acct:1".into()),
        ]),
        account_names: BTreeMap::from([("acct:1".into(), "Studio Pro".into())]),
        cores: 8,
        ..Inputs::default()
    }
}

fn by_id<'a>(
    snap: &'a juggler_floor::snapshot::FloorSnapshot,
    id: &str,
) -> &'a juggler_floor::snapshot::FloorApp {
    snap.apps.iter().find(|a| a.id == id).expect("the App")
}

#[test]
fn sessions_sit_in_their_apps_live_first_then_by_start() {
    let mut i = inputs();
    i.own = vec![
        (own("late", "running", NOW - 10), "cli:f-work".into()),
        (own("gone", "finished", NOW - 900), "cli:f-work".into()),
        (
            OwnRow {
                needs_attention: Some("waiting_user"),
                ..own("early", "running", NOW - 500)
            },
            "cli:f-work".into(),
        ),
    ];
    i.other = vec![(
        talk("t1", "done_review", NOW - 5),
        "app:host:/x/b-work".into(),
    )];
    i.tasks = BTreeMap::from([("late".into(), 2)]);
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    let cli = by_id(&snap, "cli:f-work");
    let keys: Vec<&str> = cli.sessions.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["s:early", "s:late", "s:gone"]);
    assert_eq!(
        (cli.sessions[0].state, cli.sessions[0].word.as_str()),
        (FloorState::Asks, "Asks you")
    );
    assert_eq!(cli.sessions[1].tasks, 2);
    assert_eq!(cli.sessions[2].state, FloorState::Ended);
    assert_eq!(
        (
            cli.name.as_str(),
            cli.family.as_str(),
            cli.colour_slot,
            cli.initials.as_str()
        ),
        ("Tool A · Work Max", "workshop", 3, "SP")
    );
    assert!(cli.open);

    let desktop = by_id(&snap, "app:host:/x/b-work");
    assert_eq!(
        (desktop.kind, desktop.family.as_str(), desktop.name.as_str()),
        (FloorAppKind::Desktop, "studio", "Tool A app · b-work")
    );
    let s = &desktop.sessions[0];
    assert_eq!(
        (s.key.as_str(), s.state, s.word.as_str(), s.subagents),
        ("c:f-work/t1", FloorState::Done, "Done, review it", 2)
    );
    assert_eq!(s.title, "Untitled session · shop", "never a bare main");
    assert_eq!(snap.waiting().len(), 1);
}

#[test]
fn an_app_is_hot_only_after_two_minutes_over_half_the_mac() {
    let mut i = inputs();
    i.own = vec![(own("busy", "running", NOW - 10), "cli:f-work".into())];
    i.cpu = BTreeMap::from([("s:busy".into(), 450.0)]);
    let mut hot = BTreeMap::new();
    let first = assemble(&i, NOW, &mut hot);
    assert_eq!(by_id(&first, "cli:f-work").hot_since_ms, None, "not yet");
    assert!((by_id(&first, "cli:f-work").cpu_pct - 450.0).abs() < f32::EPSILON);
    let later = assemble(&i, NOW + HOT_AFTER_MS, &mut hot);
    assert_eq!(by_id(&later, "cli:f-work").hot_since_ms, Some(NOW));
    i.cpu = BTreeMap::from([("s:busy".into(), 100.0)]);
    assert_eq!(
        by_id(
            &assemble(&i, NOW + HOT_AFTER_MS + 1, &mut hot),
            "cli:f-work"
        )
        .hot_since_ms,
        None
    );
    assert!(hot.is_empty(), "the clock starts again");
}

#[test]
fn an_account_shows_its_tightest_folder_and_its_last_use() {
    let mut i = inputs();
    i.usage = BTreeMap::from([
        (
            "f-work".into(),
            FolderUsage {
                left_pct: Some(62.0),
                resets_ms: Some(NOW + 9),
                out: false,
            },
        ),
        (
            "f-home".into(),
            FolderUsage {
                left_pct: Some(14.0),
                resets_ms: Some(NOW + 5),
                out: true,
            },
        ),
    ]);
    i.own = vec![(own("a", "running", NOW - 50), "cli:f-work".into())];
    i.other = vec![(talk("t", "idle", NOW - 20), "cli:f-work".into())];
    let account = &assemble(&i, NOW, &mut BTreeMap::new()).accounts[0];
    assert_eq!(
        (account.id.as_str(), account.label.as_str()),
        ("acct:1", "Studio Pro")
    );
    assert_eq!(
        (account.left_pct, account.resets_ms, account.out),
        (Some(14.0), Some(NOW + 5), true)
    );
    assert_eq!(account.last_used_ms, Some(NOW - 20));
    // when it resets, in the user's own words (the Floor's OUT · RESETS)
    if cfg!(target_os = "macos") {
        assert!(
            account
                .resets_words
                .as_deref()
                .is_some_and(|w| !w.is_empty())
        );
    }
}

#[test]
fn initials_take_two_words() {
    assert_eq!(initials("Work Max"), "WM");
    assert_eq!(initials("personal-max"), "PM");
    assert_eq!(initials("solo"), "S");
    assert_eq!(initials(""), "");
}

/// AJ-E3505's budget: a snapshot of 20 Apps and 150 sessions ≤ 2 ms on a
/// release build (a debug build gets ten times as long).
#[test]
fn building_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick() {
    let scale = if cfg!(debug_assertions) { 10 } else { 1 };
    let mut i = inputs();
    i.apps = (0..20)
        .map(|n| app(&format!("cli:f{n}"), "cli", "tool-a", Some("f-work")))
        .collect();
    for n in 0..150 {
        let app = format!("cli:f{}", n % 20);
        if n % 2 == 0 {
            i.own.push((own(&format!("s{n}"), "running", NOW - n), app));
        } else {
            i.other
                .push((talk(&format!("c{n}"), "working", NOW - n), app));
        }
        i.cpu.insert(format!("s:s{n}"), 3.0);
    }
    let started = std::time::Instant::now();
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    let took = started.elapsed();
    assert_eq!(
        snap.apps.iter().map(|a| a.sessions.len()).sum::<usize>(),
        150
    );
    assert!(took.as_micros() <= 2_000 * scale, "{took:?}");
}

#[test]
fn a_reading_that_wiggles_between_drawn_steps_is_not_news() {
    let base = assemble(&inputs(), NOW, &mut BTreeMap::new());
    let mut wiggle = base.clone();
    wiggle.at_ms += 2_000;
    wiggle.machine.load = base.machine.load + 0.01;
    for app in &mut wiggle.apps {
        if app.hot_since_ms.is_none() {
            app.cpu_pct += 3.0;
        }
    }
    assert!(super::same(&base, &wiggle), "no new step: nothing to draw");
    let (mut edge_a, mut edge_b) = (base.clone(), base.clone());
    edge_a.machine.load = 0.349;
    edge_b.machine.load = 0.351;
    assert!(
        super::same(&edge_a, &edge_b),
        "a reading on a step's edge doesn't flap"
    );
    let mut rack = base.clone();
    rack.machine.load = base.machine.load + 0.3;
    assert!(!super::same(&base, &rack), "the rack's lights change");
    let mut state = base.clone();
    if let Some(session) = state
        .apps
        .iter_mut()
        .flat_map(|a| a.sessions.iter_mut())
        .next()
    {
        session.word = "Asks you".into();
        assert!(!super::same(&base, &state), "a state change is news");
    }
}

/// 0.4.1 bug (a): a desktop profile is named as the rail names it, its
/// app ("Claude app") and its profile, never as the tool's CLI.
#[test]
fn a_desktop_app_is_named_by_its_app_never_its_tool() {
    let mut i = inputs();
    i.tool_names.insert("tool-a".into(), "Desk Code".into());
    i.desktop_apps.insert("tool-a".into(), "Desk".into());
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    assert_eq!(by_id(&snap, "app:host:/x/b-work").name, "Desk app · b-work");
    assert_eq!(by_id(&snap, "cli:f-work").name, "Desk Code · Work Max");
    i.desktop_words
        .insert("app:host:/x/b-work".into(), "Desk app".into());
    i.desktop_apps.insert("tool-a".into(), "Other".into());
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    assert_eq!(
        by_id(&snap, "app:host:/x/b-work").name,
        "Desk app · b-work",
        "the places book's word wins"
    );
}

/// 0.4.1 bug (b), D-84: the Floor's Apps are the rail's. An app only
/// seen running (no folder, not added, kept alive or named) is an offer,
/// never an area; a CLI App leaves with its folder.
#[test]
fn the_floor_shows_only_the_users_apps() {
    let mut seen = app("app:host:/tmp/lab", "desktop", "tool-a", None);
    seen.instance_dir = Some("/tmp/lab".into());
    let added = App {
        added: true,
        ..app("app:host:/x/added", "desktop", "tool-a", None)
    };
    let hidden = App {
        hidden: true,
        ..app("cli:f-home", "cli", "tool-a", Some("f-home"))
    };
    let all = vec![
        app("cli:f-work", "cli", "tool-a", Some("f-work")),
        app("cli:f-gone", "cli", "tool-a", Some("f-gone")),
        app("app:host:/x/b-work", "desktop", "tool-a", Some("f-work")),
        seen,
        added,
        hidden,
    ];
    let in_use = ["f-work".to_owned(), "f-home".to_owned()].into();
    // f-work ran a terminal session: a CLI App beside its profile.
    let terminal = ["f-work".to_owned()].into();
    let ids: Vec<String> = users_apps(all, &in_use, &terminal)
        .into_iter()
        .map(|a| a.id)
        .collect();
    assert_eq!(
        ids,
        ["cli:f-work", "app:host:/x/b-work", "app:host:/x/added"]
    );
}

/// AJ-E3505 R3: an App's last active time is its newest session's, else
/// when it was last seen; never a zero.
#[test]
fn an_app_is_last_active_at_its_newest_session_or_when_last_seen() {
    let mut i = inputs();
    i.own = vec![(own("a", "running", NOW - 50), "cli:f-work".into())];
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    // the session's last move (started + 10) is newer than last seen
    assert_eq!(by_id(&snap, "cli:f-work").last_active_ms, Some(NOW - 40));
    // no sessions: when it was last seen
    assert_eq!(
        by_id(&snap, "app:host:/x/b-work").last_active_ms,
        Some(NOW - 1000)
    );
    // never seen and no sessions: nothing, not 1970
    i.own.clear();
    for app in &mut i.apps {
        app.last_seen = 0;
    }
    let snap = assemble(&i, NOW, &mut BTreeMap::new());
    assert_eq!(by_id(&snap, "cli:f-work").last_active_ms, None);
}

/// AJ-E3505 R3: the overload alert on the Floor is the one on longest,
/// with since when; the load is a fraction of the whole Mac.
#[test]
fn the_mac_shows_its_oldest_overload_alert_and_since_when() {
    let mac = machine_of(
        Some(250.0),
        [(NOW - 5, "Memory is short"), (NOW - 90, "The Mac is busy")].into_iter(),
    );
    assert_eq!(mac.alert_since_ms, Some(NOW - 90));
    assert_eq!(mac.alert.as_deref(), Some("The Mac is busy"));
    assert!(
        (mac.load - 1.0).abs() < f32::EPSILON,
        "clamped to the whole Mac"
    );
    let calm = machine_of(Some(40.0), std::iter::empty());
    assert_eq!((calm.alert_since_ms, calm.alert), (None, None));
    assert!((calm.load - 0.4).abs() < 1e-6);
    assert!(machine_of(None, std::iter::empty()).load.abs() < f32::EPSILON);
}

/// SPEC §9.7's time limits are news when the clock alone passes them:
/// a done session 15 minutes on is drawn idle, though nothing else
/// changed in the snapshot.
#[test]
fn a_limit_the_clock_passes_is_news() {
    let mut i = inputs();
    i.other = vec![(
        talk("t", "done_review", NOW - 14 * 60_000),
        "cli:f-work".into(),
    )];
    let base = assemble(&i, NOW, &mut BTreeMap::new());
    let state = |snap: &juggler_floor::snapshot::FloorSnapshot| {
        snap.apps
            .iter()
            .flat_map(|a| &a.sessions)
            .map(|s| s.state)
            .collect::<Vec<_>>()
    };
    assert!(state(&base).contains(&FloorState::Done));
    let mut later = base.clone();
    later.at_ms = NOW + 30_000;
    assert!(super::same(&base, &later), "still done: nothing to draw");
    later.at_ms = NOW + 2 * 60_000;
    assert!(!super::same(&base, &later), "15 minutes on: drawn idle");
}
