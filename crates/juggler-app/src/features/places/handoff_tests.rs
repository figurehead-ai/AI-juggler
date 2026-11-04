//! Desktop hand-offs over a real app state and a scripted system.

use std::sync::Mutex;

use super::*;
use crate::features::places::book;

const SESSION: &str = "a71cc8f5-72e1-4195-8797-e07e3801b771";

/// A profile of the Claude app under `root`, remembered on `account`.
fn state_with_profile(root: &Path, account: Option<&str>) -> (Arc<AppState>, String, PathBuf) {
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    let bundle = root.join("Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    let profile = root.join("Profiles/b-lab");
    std::fs::create_dir_all(&profile).unwrap();
    let id = book::place_id("claude-app", Some(&profile));
    with_book(&state, |book| {
        book.places.insert(
            id.clone(),
            KnownPlace {
                app_id: "claude-app".into(),
                name: "Claude".into(),
                instance: Some("b-lab".into()),
                bundle: bundle.clone(),
                exe: bundle.join("Contents/MacOS/Claude"),
                given: Some(profile.clone()),
                instance_dir: Some(profile.clone()),
                account_id: account.map(str::to_owned),
                conversations: Vec::new(),
                seen_ms: 1,
                keep_alive: false,
                paused: None,
                found: false,
            },
        );
    })
    .unwrap();
    (state, id, profile)
}

/// The system: a profile that runs from the start or once started, and
/// that records a session it was sent when `records` is set.
#[derive(Default)]
struct Scripted {
    pid: Mutex<Option<u32>>,
    /// 0.3.8: the running one lost its window.
    lost: bool,
    started: Mutex<u32>,
    sent: Mutex<Vec<(u32, String)>>,
    records: Option<PathBuf>,
}

impl System for Scripted {
    fn running(&self, _: &Arc<AppState>, _: &str) -> Option<Running> {
        self.pid.lock().unwrap().map(|pid| Running {
            pid,
            lost: self.lost,
        })
    }
    fn start(&self, _: &Arc<AppState>, _: &KnownPlace) -> Result<(), String> {
        *self.started.lock().unwrap() += 1;
        *self.pid.lock().unwrap() = Some(77);
        Ok(())
    }
    fn send(&self, pid: u32, link: &str) -> Result<(), String> {
        self.sent.lock().unwrap().push((pid, link.to_owned()));
        if let Some(records) = &self.records {
            let below = records.join("account").join("org");
            std::fs::create_dir_all(&below).unwrap();
            std::fs::write(below.join(format!("local_{SESSION}.json")), "{}").unwrap();
        }
        Ok(())
    }
}

fn quick() -> Waits {
    Waits {
        start: Duration::from_millis(200),
        arrive: Duration::from_millis(200),
        every: Duration::from_millis(10),
    }
}

#[test]
fn a_running_profile_gets_the_session_at_its_own_process_and_is_never_started_again() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, profile) = state_with_profile(root.path(), None);
    let system = Scripted {
        pid: Mutex::new(Some(42)),
        records: Some(profile.join("claude-code-sessions")),
        ..Scripted::default()
    };
    assert_eq!(hand_off(&state, &id, SESSION, &system, quick()), Ok(42));
    assert_eq!(
        *system.started.lock().unwrap(),
        0,
        "never a second process on one profile"
    );
    assert_eq!(
        *system.sent.lock().unwrap(),
        [(42, format!("claude://resume?session={SESSION}"))]
    );
}

#[test]
fn a_profile_that_isnt_running_is_started_first_then_handed_the_session() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, profile) = state_with_profile(root.path(), None);
    let system = Scripted {
        records: Some(profile.join("claude-code-sessions")),
        ..Scripted::default()
    };
    assert_eq!(hand_off(&state, &id, SESSION, &system, quick()), Ok(77));
    assert_eq!(*system.started.lock().unwrap(), 1);
    assert_eq!(system.sent.lock().unwrap()[0].0, 77);
}

#[test]
fn it_isnt_switched_until_the_profile_records_the_session() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, _) = state_with_profile(root.path(), None);
    let system = Scripted {
        pid: Mutex::new(Some(42)),
        ..Scripted::default()
    };
    let error = hand_off(&state, &id, SESSION, &system, quick()).unwrap_err();
    assert!(error.contains("hasn't opened it yet"), "{error}");
}

/// 0.3.8: a profile that runs on with no window after macOS restarted its
/// display takes no link, and nothing is started beside it: the switch
/// says what happened and where the restart is.
#[test]
fn a_profile_that_lost_its_window_is_never_sent_the_session_or_started_beside() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, profile) = state_with_profile(root.path(), None);
    let system = Scripted {
        pid: Mutex::new(Some(42)),
        lost: true,
        records: Some(profile.join("claude-code-sessions")),
        ..Scripted::default()
    };
    let error = hand_off(&state, &id, SESSION, &system, quick()).unwrap_err();
    assert_eq!(
        error,
        "Claude · b-lab lost its window when the system restarted the display. Restart it from Home or Settings › Desktop apps, then switch again"
    );
    assert_eq!(*system.started.lock().unwrap(), 0, "never a second process");
    assert!(system.sent.lock().unwrap().is_empty(), "no link to it");
}

#[test]
fn an_id_that_isnt_plain_is_never_sent() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, _) = state_with_profile(root.path(), None);
    let system = Scripted {
        pid: Mutex::new(Some(42)),
        ..Scripted::default()
    };
    assert!(hand_off(&state, &id, "x&evil=1", &system, quick()).is_err());
    assert!(system.sent.lock().unwrap().is_empty());
}

#[test]
fn desktop_is_offered_for_an_account_with_a_profile_and_said_why_not_otherwise() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, _) = state_with_profile(root.path(), Some("acct-work"));
    let with = destinations(&state, "acct-work", "Work Max", true);
    let desktop = with.desktop.expect("the profile on that account");
    assert_eq!(desktop.place_id, id);
    assert_eq!(desktop.app_id, "claude-app");
    assert_eq!(
        with.suggested, "desktop",
        "a session from the app stays in the app"
    );
    assert_eq!(
        destinations(&state, "acct-work", "Work Max", false).suggested,
        "cli"
    );
    let without = destinations(&state, "acct-other", "Personal", true);
    assert!(without.desktop.is_none());
    assert_eq!(without.suggested, "cli");
    assert!(
        without
            .desktop_why_not
            .unwrap()
            .contains("No desktop app profile on Personal")
    );
}

#[test]
fn a_record_is_found_by_its_name_alone_and_the_walk_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let records = root.path().join("claude-code-sessions");
    assert!(!taken(&records, SESSION), "no folder yet");
    let deep = records.join("a/b/c/d/e/f");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join(format!("local_{SESSION}.json")), "{}").unwrap();
    assert!(
        !taken(&records, SESSION),
        "deeper than a profile keeps them"
    );
    let near = records.join("account/org");
    std::fs::create_dir_all(&near).unwrap();
    std::fs::write(near.join(format!("local_{SESSION}.json")), "{}").unwrap();
    assert!(taken(&records, SESSION));
}

#[test]
fn an_app_that_cant_take_a_session_is_named_as_the_reason() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_profile(root.path(), None);
    with_book(&state, |book| {
        book.places.insert(
            book::place_id("codex-app", None),
            KnownPlace {
                app_id: "codex-app".into(),
                name: "ChatGPT".into(),
                instance: None,
                bundle: root.path().join("ChatGPT.app"),
                exe: root.path().join("ChatGPT.app/Contents/MacOS/ChatGPT"),
                given: None,
                instance_dir: None,
                account_id: Some("acct-codex".into()),
                conversations: Vec::new(),
                seen_ms: 1,
                keep_alive: false,
                paused: None,
                found: false,
            },
        );
    })
    .unwrap();
    let codex = destinations(&state, "acct-codex", "Codex work", true);
    assert!(codex.desktop.is_none());
    assert_eq!(codex.suggested, "cli");
    let why = codex.desktop_why_not.unwrap();
    assert!(
        why.ends_with("can't pick up a switched session yet."),
        "{why}"
    );
}
