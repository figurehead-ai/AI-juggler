//! Desktop hand-offs over a real app state and a scripted system.

use std::sync::Mutex;

use super::*;
use crate::features::places::book;

const SESSION: &str = "a71cc8f5-72e1-4195-8797-e07e3801b771";

/// A profile of the Claude app under `root`, remembered on `account`.
fn state_with_profile(root: &Path, account: Option<&str>) -> (Arc<AppState>, String, PathBuf) {
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    let (id, _, profile) =
        crate::features::places::remember_test_profile(&state, root, "b-lab", account);
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
        "Claude app · b-lab lost its window when the system restarted the display. Restart it from Home or Settings › Desktop apps, then switch again"
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

/// 0.4: a Claude app profile used in its Code tab: no terminal sign-in,
/// so Desktop leads and CLI says why it can't be picked.
#[test]
fn an_app_profile_not_signed_in_at_the_terminal_continues_in_the_app() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_profile(root.path(), Some("acct-work"));
    assert!(signs_in_through_app(&state, "acct-work"));
    assert!(!signs_in_through_app(&state, "acct-other"));
    let folders = folders_signed_in_through_apps(&state);
    assert!(folders.contains("acct-work") && !folders.contains("acct-other"));
    let offered = destinations(&state, "acct-work", "Work Max", false, false);
    assert!(offered.desktop.is_some(), "Desktop is there to pick");
    assert_eq!(offered.suggested, "cli", "still where the original ran");
    let why = offered.cli_why_not.expect("CLI says why not");
    assert!(why.starts_with("Work Max signs in through"), "{why}");
    // Signed in at the terminal too: both are open.
    assert!(
        destinations(&state, "acct-work", "Work Max", false, true)
            .cli_why_not
            .is_none()
    );
    // No app profile: nothing to continue in, nothing said here (the
    // target list holds it back for its sign-in).
    let none = destinations(&state, "acct-other", "Personal", false, false);
    assert!(none.cli_why_not.is_none());
    assert_eq!(none.suggested, "cli");
}

#[test]
fn desktop_is_offered_for_an_account_with_a_profile_and_said_why_not_otherwise() {
    let root = tempfile::tempdir().unwrap();
    let (state, id, _) = state_with_profile(root.path(), Some("acct-work"));
    let with = destinations(&state, "acct-work", "Work Max", true, true);
    let desktop = with.desktop.expect("the profile on that account");
    assert_eq!(desktop.place_id, id);
    assert_eq!(desktop.app_id, "claude-app");
    assert_eq!(
        with.suggested, "desktop",
        "a session from the app stays in the app"
    );
    assert_eq!(
        destinations(&state, "acct-work", "Work Max", false, true).suggested,
        "cli"
    );
    let without = destinations(&state, "acct-other", "Personal", true, true);
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
                users: true,
                declined: false,
            },
        );
    })
    .unwrap();
    let codex = destinations(&state, "acct-codex", "Codex work", true, true);
    assert!(codex.desktop.is_none());
    assert_eq!(codex.suggested, "cli");
    let why = codex.desktop_why_not.unwrap();
    assert!(
        why.ends_with("can't pick up a switched session yet."),
        "{why}"
    );
}

/// 0.4.1: Go to it for a Claude app session that isn't open now.
mod open_session_tests {
    use super::*;
    use crate::features::conversations::goto::{Desktop, LinkSent};
    use crate::features::places::open_session;

    /// The profile's record of `SESSION`, as the Claude app keeps it.
    fn recorded(profile: &Path) {
        let file = profile.join("claude-code-sessions/a/o/local_9f1c2a3b.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(
            &file,
            format!(r#"{{"cliSessionId":"{SESSION}","title":"AWS migration planning"}}"#),
        )
        .unwrap();
    }

    /// The Mac: what was asked to come forward and what was sent where.
    #[derive(Default)]
    struct Mac {
        consent: bool,
        log: Mutex<Vec<String>>,
    }

    impl crate::activation::Front for Mac {
        fn request(&self, pid: u32) -> crate::activation::Request {
            self.log.lock().unwrap().push(format!("forward {pid}"));
            crate::activation::Request::Asked { in_front: true }
        }
        fn frontmost(&self) -> Option<u32> {
            None
        }
        fn pause(&self, _: Duration) {}
    }

    impl Desktop for Mac {
        fn start_of(&self, _: u32) -> Option<i64> {
            None
        }
        fn detached(&self, _: u32, _: Option<i64>) -> bool {
            false
        }
        fn open_link(&self, link: &str) -> bool {
            self.log.lock().unwrap().push(format!("open {link}"));
            true
        }
        fn open_folder(&self, _: &str, _: &Path) -> bool {
            false
        }
        fn osascript(&self, _: &str) -> Result<String, String> {
            Err("no".into())
        }
        fn tty(&self, _: u32) -> Option<String> {
            None
        }
        fn send_link_to(&self, pid: u32, link: &str, may_ask: bool) -> LinkSent {
            self.log.lock().unwrap().push(format!("send {pid} {link}"));
            if self.consent && !may_ask {
                LinkSent::NeedsConsent
            } else {
                LinkSent::Sent
            }
        }
    }

    #[test]
    fn a_closed_profile_is_started_then_the_session_itself_opens_in_it_alone() {
        let root = tempfile::tempdir().unwrap();
        let (state, _, profile) = state_with_profile(root.path(), Some("acct-work"));
        recorded(&profile);
        let system = Scripted::default();
        let mac = Mac::default();
        let done = open_session::open(&state, "acct-work", SESSION, &system, &mac, false, quick())
            .unwrap();
        assert_eq!(*system.started.lock().unwrap(), 1);
        assert_eq!(done.reached, "started");
        assert!(
            done.outcome.ends_with("and opened the session"),
            "{}",
            done.outcome
        );
        assert_eq!(
            *mac.log.lock().unwrap(),
            [
                "forward 77",
                "send 77 claude://claude.ai/epitaxy/local_9f1c2a3b"
            ],
            "the started profile's own process, never `open`"
        );
    }

    #[test]
    fn a_running_profile_isnt_started_again_and_a_link_that_needs_asking_waits_for_the_users_click()
    {
        let root = tempfile::tempdir().unwrap();
        let (state, _, profile) = state_with_profile(root.path(), Some("acct-work"));
        recorded(&profile);
        let system = Scripted {
            pid: Mutex::new(Some(42)),
            ..Scripted::default()
        };
        let mac = Mac {
            consent: true,
            ..Mac::default()
        };
        let done = open_session::open(&state, "acct-work", SESSION, &system, &mac, false, quick())
            .unwrap();
        assert_eq!(*system.started.lock().unwrap(), 0);
        assert_eq!(done.reached, "consent");
        let done =
            open_session::open(&state, "acct-work", SESSION, &system, &mac, true, quick()).unwrap();
        assert_eq!(done.reached, "thread");
        assert_eq!(
            done.outcome,
            format!(
                "Opened the session in {}",
                super::super::named(
                    &state,
                    &with_book(&state, |b| b.places.values().next().cloned())
                        .unwrap()
                        .unwrap()
                )
            )
        );
    }

    #[test]
    fn another_accounts_profile_or_one_that_doesnt_keep_it_is_never_used() {
        let root = tempfile::tempdir().unwrap();
        let (state, _, profile) = state_with_profile(root.path(), Some("acct-other"));
        recorded(&profile);
        assert!(open_session::keeping(&state, "acct-work", SESSION).is_none());
        let root = tempfile::tempdir().unwrap();
        let (state, _, _) = state_with_profile(root.path(), Some("acct-work"));
        assert!(
            open_session::keeping(&state, "acct-work", SESSION).is_none(),
            "no record of it there"
        );
    }

    #[test]
    fn a_profile_that_lost_its_window_offers_its_restart_and_is_never_started_beside_itself() {
        let root = tempfile::tempdir().unwrap();
        let (state, id, profile) = state_with_profile(root.path(), Some("acct-work"));
        recorded(&profile);
        let system = Scripted {
            pid: Mutex::new(Some(42)),
            lost: true,
            ..Scripted::default()
        };
        let done = open_session::open(
            &state,
            "acct-work",
            SESSION,
            &system,
            &Mac::default(),
            false,
            quick(),
        )
        .unwrap();
        assert_eq!(done.reached, "lost");
        assert_eq!(done.place_id.as_deref(), Some(id.as_str()));
        assert_eq!(*system.started.lock().unwrap(), 0);
    }
}
