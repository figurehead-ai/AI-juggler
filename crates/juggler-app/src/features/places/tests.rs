//! The places over a real app state and a scripted system: Open starts
//! a place that isn't running (Codex at its thread), Keep alive starts it
//! again and pauses, a stop from AI Juggler pauses it, and the window's
//! setting writer can't name a program.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::fakes::{FakeDesktop, Recorder};
use super::*;

/// A state whose book holds a Claude profile and the Codex app, both
/// installed under `root` and not running.
fn state_with_places(root: &Path) -> (Arc<AppState>, String, String) {
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    let claude = root.join("Claude.app");
    let codex = root.join("ChatGPT.app");
    for bundle in [&claude, &codex] {
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    }
    let profile = root.join("Profiles/b-lab");
    let work = book::place_id("claude-app", Some(&profile));
    let chat = book::place_id("codex-app", None);
    with_book(&state, |book| {
        book.places.insert(
            work.clone(),
            KnownPlace {
                app_id: "claude-app".into(),
                name: "Claude".into(),
                instance: Some("b-lab".into()),
                bundle: claude.clone(),
                exe: claude.join("Contents/MacOS/Claude"),
                given: Some(profile.clone()),
                instance_dir: Some(profile.clone()),
                account_id: None,
                conversations: vec!["claude-code/s-1".into()],
                seen_ms: 1,
                keep_alive: false,
                paused: None,
                found: false,
                users: true,
                declined: false,
            },
        );
        book.places.insert(
            chat.clone(),
            KnownPlace {
                app_id: "codex-app".into(),
                name: "ChatGPT".into(),
                instance: None,
                bundle: codex.clone(),
                exe: codex.join("Contents/MacOS/ChatGPT"),
                given: None,
                instance_dir: None,
                account_id: None,
                conversations: vec!["codex/t-1".into()],
                seen_ms: 1,
                keep_alive: false,
                paused: None,
                found: false,
                users: true,
                declined: false,
            },
        );
    })
    .unwrap();
    (state, work, chat)
}

#[test]
fn open_starts_a_profile_that_isnt_running_with_only_its_folder() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    let starter = Arc::new(Recorder::default());
    let said = open(
        &state,
        &work,
        Some("s-1"),
        Arc::new(FakeDesktop::opening_links()),
        starter.clone(),
    )
    .unwrap();
    assert_eq!(said.reached, "started");
    // The window follows the start to its end by this id (the go sitting).
    assert_eq!(said.place_id.as_deref(), Some(work.as_str()));
    assert_eq!(
        said.outcome,
        "Starting Claude app · b-lab; pick the session in its sidebar"
    );
    let starts = starter.starts.lock().unwrap();
    let (bundle, how, args) = &starts[0];
    assert_eq!(bundle, &root.path().join("Claude.app"));
    assert!(
        how.new_instance && !how.background,
        "in front, beside other profiles"
    );
    assert_eq!(
        args,
        &[format!(
            "--user-data-dir={}",
            root.path().join("Profiles/b-lab").display()
        )]
    );
}

#[test]
fn open_in_codex_opens_the_thread_by_its_link_and_starts_nothing_else() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, chat) = state_with_places(root.path());
    let starter = Arc::new(Recorder::default());
    let links = Arc::new(FakeDesktop::opening_links());
    let said = open(&state, &chat, Some("t-1"), links.clone(), starter.clone()).unwrap();
    assert_eq!(said.reached, "thread");
    assert_eq!(*links.links.lock().unwrap(), ["codex://threads/t-1"]);
    assert!(starter.starts.lock().unwrap().is_empty());
    // The session's place is found by its tool and id.
    let found = with_book(&state, |book| {
        book.place_of("codex", "t-1").map(|(id, _)| id.clone())
    })
    .unwrap();
    assert_eq!(found, Some(chat));
}

#[test]
fn keep_alive_starts_a_stopped_place_behind_then_pauses_after_three() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    let starter = Recorder::default();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    let unlocked = || false;
    // Every 2 s, as the app ticks.
    let mut now = start;
    while now < start + keep::WAIT_MS + 2_000 {
        now += 2_000;
        tick_with(&state, now, &starter, &unlocked);
    }
    {
        let starts = starter.starts.lock().unwrap();
        assert_eq!(starts.len(), 1, "started once, after the wait");
        assert!(starts[0].1.background, "behind the app in front");
    }
    // It never came up: started again each time, then paused.
    while now < start + 5 * 60_000 {
        now += 2_000;
        tick_with(&state, now, &starter, &unlocked);
    }
    assert_eq!(
        starter.starts.lock().unwrap().len(),
        crate::features::sessions::restart::TRIES
    );
    let paused = with_book(&state, |book| book.places[&work].paused.clone()).unwrap();
    assert_eq!(paused.as_deref(), Some("it stopped 3 times in 10 minutes"));
    // Open ends the pause.
    open(
        &state,
        &work,
        None,
        Arc::new(FakeDesktop::opening_links()),
        Arc::new(Recorder::default()),
    )
    .unwrap();
    let paused = with_book(&state, |book| book.places[&work].paused.clone()).unwrap();
    assert_eq!(paused, None);
}

#[test]
fn nothing_starts_while_the_screen_is_locked() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    let starter = Recorder::default();
    let now = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(now);
    for at in 0..20 {
        tick_with(&state, now + at * 2_000, &starter, &|| true);
    }
    assert!(starter.starts.lock().unwrap().is_empty());
}

#[test]
fn a_stop_from_ai_juggler_pauses_keep_alive() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    state
        .places
        .watch
        .lock()
        .unwrap()
        .found(&work, 4242, Some(7));
    stopped_here(&state, 4242);
    let paused = with_book(&state, |book| book.places[&work].paused.clone()).unwrap();
    assert_eq!(paused.as_deref(), Some("you stopped it in AI Juggler"));
}

#[test]
fn the_windows_setting_writer_cant_name_a_program() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_places(root.path());
    let refused =
        crate::settings_policy::app_setting_set_impl(&state, book::KEY, r#"{"places":{}}"#);
    assert!(refused.is_err());
    let kept = with_book(&state, |book| book.places.len()).unwrap();
    assert_eq!(kept, 2, "the book is untouched");
}

#[test]
fn keep_alive_is_refused_for_a_place_that_cant_be_started() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    with_book(&state, |book| {
        book.places.get_mut(&work).unwrap().exe = PathBuf::from("/tmp/elsewhere");
    })
    .unwrap();
    assert!(set_keep_alive(&state, &work, true).is_err());
    assert!(set_keep_alive(&state, "nope|", true).is_err());
}

/// SPEC §1: keep alive is the one automated start, and only where the
/// user turned it on. A remembered place that isn't kept alive is never
/// started, however long it's been down.
#[test]
fn nothing_starts_unless_keep_alive_was_turned_on() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_places(root.path());
    let starter = Recorder::default();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    for tick in 1..=300 {
        tick_with(&state, start + tick * 2_000, &starter, &|| false);
    }
    assert!(starter.starts.lock().unwrap().is_empty());
}

/// D-62: a session belongs to the desktop App it was last seen open in.
#[test]
fn a_session_belongs_to_the_desktop_app_it_was_last_seen_in() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, chat) = state_with_places(root.path());
    let by_session = desktop_app_by_session(&state);
    assert_eq!(by_session.get("claude-code/s-1"), Some(&work));
    assert_eq!(by_session.get("codex/t-1"), Some(&chat));
    // Seen later in the Codex app too: the newer App has it.
    with_book(&state, |book| {
        let place = book.places.get_mut(&chat).unwrap();
        place.conversations.push("claude-code/s-1".into());
        place.seen_ms = 5;
    })
    .unwrap();
    assert_eq!(
        desktop_app_by_session(&state).get("claude-code/s-1"),
        Some(&chat)
    );
}

/// The go sitting's rerun (2026-11-07): a profile's page listed each
/// switched session twice. A switch copies a conversation into another
/// profile's folder under the same id: a place whose folder is known
/// claims only its own copy; the other copy is its folder's.
#[test]
fn a_profile_claims_only_its_own_folders_copy_of_a_conversation() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _chat) = state_with_places(root.path());
    with_book(&state, |book| {
        book.places.get_mut(&work).unwrap().account_id = Some("folder-aij".into());
    })
    .unwrap();
    let desktop = DesktopApps {
        by_session: desktop_app_by_session(&state),
        ..DesktopApps::default()
    };
    let key = Some("claude-code/s-1");
    assert_eq!(app_of(&desktop, None, Some("folder-aij"), key, false), work);
    assert_eq!(
        app_of(&desktop, None, Some("folder-other"), key, false),
        juggler_core::storage::cli_app_id("folder-other")
    );
}

/// The place `id` runs as this test's own process (a real pid and start,
/// so it's found alive; nothing is ever signalled: the recorder only
/// notes the quit).
fn running_as_me(state: &Arc<AppState>, id: &str) -> (u32, Option<i64>) {
    let me = std::process::id();
    let start = crate::features::conversations::procs::some(&[me])
        .get(&me)
        .and_then(|process| process.start);
    let mut watch = state.places.watch.lock().unwrap();
    watch.found(id, me, start);
    // As found running: this test's program (0.4.1: the pid and start
    // alone aren't the app; its program must be too).
    watch.note_exe(id, &std::env::current_exe().unwrap());
    drop(watch);
    (me, start)
}

fn quick() -> QuitWait {
    QuitWait {
        limit: Duration::from_millis(100),
        every: Duration::from_millis(5),
    }
}

/// 0.3.8: Open on a profile that runs on with no window asks to restart
/// that place, and starts nothing beside it.
#[test]
fn open_on_a_profile_that_lost_its_window_asks_to_restart_it() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    running_as_me(&state, &work);
    let starter = Arc::new(Recorder::default());
    let said = open(
        &state,
        &work,
        None,
        Arc::new(FakeDesktop::lost()),
        starter.clone(),
    )
    .unwrap();
    assert_eq!(
        said.outcome,
        "Claude app · b-lab lost its window when the system restarted the display. Restart it?"
    );
    assert_eq!(said.reached, "lost");
    assert_eq!(said.place_id.as_deref(), Some(work.as_str()));
    assert!(
        starter.starts.lock().unwrap().is_empty(),
        "never a second process on one profile folder"
    );
}

/// Restart quits the lost process, waits for it to end, then starts the
/// place exactly as it ran (its folder, its tool folder), in front.
#[test]
fn restart_quits_the_lost_process_then_starts_it_as_it_ran() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    let (me, _) = running_as_me(&state, &work);
    let starter = Recorder {
        lost: true,
        ..Recorder::default()
    };
    let said = restart(
        &state,
        &work,
        &starter,
        false,
        RestartOf::LostWindow,
        quick(),
        10_000_000,
    )
    .unwrap();
    assert_eq!(said, "Restarting Claude app · b-lab");
    assert_eq!(*starter.quits.lock().unwrap(), [me]);
    let starts = starter.starts.lock().unwrap();
    assert_eq!(starts.len(), 1);
    assert!(
        starts[0].1.new_instance && !starts[0].1.background,
        "in front"
    );
    assert_eq!(
        starts[0].2,
        [format!(
            "--user-data-dir={}",
            root.path().join("Profiles/b-lab").display()
        )]
    );
    assert!(
        state
            .places
            .watch
            .lock()
            .unwrap()
            .coming_up(&work, 10_000_000),
        "keep alive starts nothing of its own meanwhile"
    );
}

/// 0.3.5's pairing holds through a restart: a profile on an account
/// starts on that account's tool folder.
#[test]
fn a_restarted_profile_keeps_its_tool_folder() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    let folder = root.path().join("CodeProfiles/work");
    let account = juggler_core::model::Account {
        id: juggler_core::model::AccountId::new("acct-work"),
        harness: juggler_core::model::HarnessId::new("claude-code"),
        label: "Work Max".into(),
        plan: None,
        state_dir: folder.clone(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    crate::commands::lock_storage_arc(&state)
        .unwrap()
        .accounts()
        .insert(&account)
        .unwrap();
    with_book(&state, |book| {
        book.places.get_mut(&work).unwrap().account_id = Some("acct-work".into());
    })
    .unwrap();
    running_as_me(&state, &work);
    let starter = Recorder {
        lost: true,
        ..Recorder::default()
    };
    restart(
        &state,
        &work,
        &starter,
        false,
        RestartOf::LostWindow,
        quick(),
        10_000_000,
    )
    .unwrap();
    assert_eq!(
        *starter.envs.lock().unwrap(),
        [vec![(
            "CLAUDE_CONFIG_DIR".to_owned(),
            folder.to_string_lossy().into_owned()
        )]]
    );
}

/// One that won't quit is reported and never started beside; one that
/// has its window is never quit.
#[test]
fn restart_never_starts_beside_one_that_stays_or_quits_one_that_has_its_window() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    running_as_me(&state, &work);
    let stays = Recorder {
        lost: true,
        stays: true,
        ..Recorder::default()
    };
    let error = restart(
        &state,
        &work,
        &stays,
        false,
        RestartOf::LostWindow,
        quick(),
        10_000_000,
    )
    .unwrap_err();
    assert!(error.contains("didn't quit"), "{error}");
    assert!(stays.starts.lock().unwrap().is_empty());
    let fine = Recorder::default();
    let error = restart(
        &state,
        &work,
        &fine,
        false,
        RestartOf::LostWindow,
        quick(),
        10_000_000,
    )
    .unwrap_err();
    assert!(error.contains("has its window"), "{error}");
    assert!(fine.quits.lock().unwrap().is_empty(), "never quit");
    assert!(fine.starts.lock().unwrap().is_empty());
}

/// D-71: once the user confirmed it, a running one with its window is
/// quit and started again, on the same profile; never a second beside it.
#[test]
fn a_confirmed_restart_quits_and_starts_one_that_has_its_window() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    running_as_me(&state, &work);
    let fine = Recorder::default();
    restart(
        &state,
        &work,
        &fine,
        false,
        RestartOf::Confirmed,
        quick(),
        10_000_000,
    )
    .unwrap();
    assert_eq!(fine.quits.lock().unwrap().len(), 1, "quit once");
    assert_eq!(
        fine.starts.lock().unwrap().len(),
        1,
        "started once, after it quit"
    );
    // One that won't quit is never started beside.
    let stays = Recorder {
        stays: true,
        ..Recorder::default()
    };
    running_as_me(&state, &work);
    let error = restart(
        &state,
        &work,
        &stays,
        false,
        RestartOf::Confirmed,
        quick(),
        20_000_000,
    )
    .unwrap_err();
    assert!(error.contains("didn't quit"), "{error}");
    assert!(stays.starts.lock().unwrap().is_empty());
}

/// Keep alive treats a kept place that lost its window as down: after the
/// wait it's quit and started once, behind; told as a restart.
#[test]
fn keep_alive_restarts_a_place_that_lost_its_window_once() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    let (me, _) = running_as_me(&state, &work);
    let starter = Recorder {
        lost: true,
        ..Recorder::default()
    };
    let mut now = start;
    while now < start + keep::WAIT_MS + 10_000 {
        now += 2_000;
        tick_with(&state, now, &starter, &|| false);
    }
    assert_eq!(*starter.quits.lock().unwrap(), [me], "quit once");
    let starts = starter.starts.lock().unwrap();
    assert_eq!(starts.len(), 1, "started once");
    assert!(starts[0].1.background, "behind the app in front");
}

/// Never while locked, and never while a session in it is working (a
/// restart would end the turn).
#[test]
fn keep_alive_leaves_a_lost_place_while_locked_or_while_a_session_works_in_it() {
    use crate::features::conversations::places::Host;
    use crate::features::conversations::scan::{LiveEntry, LiveModel, Place, State};
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    let (me, at) = running_as_me(&state, &work);
    let starter = Recorder {
        lost: true,
        ..Recorder::default()
    };
    for tick in 1..=20 {
        tick_with(&state, start + tick * 2_000, &starter, &|| true);
    }
    assert!(starter.quits.lock().unwrap().is_empty(), "locked");
    state.conversations.set_live_model(LiveModel {
        entries: vec![LiveEntry {
            tool: "claude-code".into(),
            account_id: None,
            session_id: "s-1".into(),
            places: vec![Place {
                pid: 77,
                start: None,
                host: Some(Host {
                    app_id: Some("claude-app".into()),
                    name: "Claude".into(),
                    shown: "Claude".into(),
                    kind: Some(juggler_adapter::live::HostKind::Desktop),
                    pid: me,
                    start: at,
                    instance_dir: None,
                    instance: Some("b-lab".into()),
                }),
                profile: None,
                app_session: None,
            }],
            state: Some(State::Working),
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        }],
        ..LiveModel::default()
    });
    for tick in 21..=60 {
        tick_with(&state, start + tick * 2_000, &starter, &|| false);
    }
    assert!(
        starter.quits.lock().unwrap().is_empty(),
        "a working session holds it"
    );
}

/// rc22 (A3): the one order a session's App is found by, as the window
/// finds it (`ui/src/lib/fixtures/session-apps.json`, read by the UI's
/// `session-apps.test.ts` too).
#[test]
fn every_session_is_in_the_app_the_shared_cases_say() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../ui/src/lib/fixtures/session-apps.json"
    ))
    .unwrap();
    let mut book = Book::default();
    for place in cases["places"].as_array().unwrap() {
        let s = |key: &str| place[key].as_str().unwrap().to_owned();
        book.places.insert(
            s("id"),
            KnownPlace {
                app_id: "claude-app".into(),
                name: "Claude".into(),
                instance: Some(s("instance")),
                bundle: "/Applications/Claude.app".into(),
                exe: PathBuf::new(),
                given: None,
                instance_dir: Some(PathBuf::from(format!("/p/{}", s("instance")))),
                account_id: Some(s("folder")),
                conversations: place["recent"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| key.as_str().unwrap().to_owned())
                    .collect(),
                seen_ms: place["seen_ms"].as_i64().unwrap(),
                keep_alive: false,
                paused: None,
                found: false,
                users: true,
                declined: false,
            },
        );
    }
    let desktop = DesktopApps::of(&book);
    let mut wrong = Vec::new();
    for row in cases["own"].as_array().unwrap() {
        let got = app_of(
            &desktop,
            row["launched_in"].as_str(),
            row["folder"].as_str(),
            None,
            row["kind"] == "terminal",
        );
        if got != row["app"] {
            wrong.push(format!("{}: {got}", row["id"]));
        }
    }
    for row in cases["other"].as_array().unwrap() {
        let key = format!("claude-code/{}", row["id"].as_str().unwrap());
        let got = app_of(
            &desktop,
            None,
            row["folder"].as_str(),
            Some(&key),
            crate::features::conversations::dto::ran_in_a_terminal(
                row["live"].as_bool().unwrap(),
                row["place"].as_str(),
                row["started_in"].as_str(),
            ),
        );
        if got != row["app"] {
            wrong.push(format!("{}: {got}", row["id"]));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

/// rc22 (A3): each Account's Apps in the shared cases are the Apps list
/// (`apps::users_apps`) narrowed to its folders: a profile's folder is a
/// CLI App only while a case counts under it. The window's screens read
/// the same lists (`session-apps.test.ts`).
#[test]
fn every_accounts_apps_are_the_apps_list_on_its_folders() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../ui/src/lib/fixtures/session-apps.json"
    ))
    .unwrap();
    let text = |v: &serde_json::Value| v.as_str().unwrap().to_owned();
    let app = |id: String, kind: &str, folder: String, added: bool| juggler_core::storage::App {
        id,
        kind: kind.into(),
        tool: "claude-code".into(),
        host_app_id: None,
        instance_dir: None,
        folder_id: Some(folder),
        label: None,
        hidden: false,
        keep_alive: false,
        paused_json: None,
        launch_json: None,
        sessions_json: None,
        first_seen: 0,
        last_seen: 0,
        added,
    };
    let folders: std::collections::BTreeSet<String> = cases["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| text(&f["id"]))
        .collect();
    let mut rows: Vec<juggler_core::storage::App> = folders
        .iter()
        .map(|f| {
            app(
                juggler_core::storage::cli_app_id(f),
                "cli",
                f.clone(),
                false,
            )
        })
        .collect();
    rows.extend(
        cases["places"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| app(text(&p["id"]), "desktop", text(&p["folder"]), true)),
    );
    // What counts under a CLI App: the cases' rows.
    let counted: std::collections::BTreeSet<String> = ["own", "other"]
        .iter()
        .flat_map(|kind| cases[kind].as_array().unwrap())
        .filter_map(|row| text(&row["app"]).strip_prefix("cli:").map(str::to_owned))
        .collect();
    let listed = crate::features::apps::users_apps(rows, &folders, &counted);
    for account in cases["accounts"].as_array().unwrap() {
        let on: Vec<String> = account["folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect();
        let mut got: Vec<String> = listed
            .iter()
            .filter(|a| a.folder_id.as_ref().is_some_and(|f| on.contains(f)))
            .map(|a| a.id.clone())
            .collect();
        got.sort();
        let want: Vec<String> = account["apps"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect();
        // the Account's record is never printed (CodeQL rust/cleartext-logging)
        assert!(got == want, "an Account's Apps differ from the reference");
    }
}
