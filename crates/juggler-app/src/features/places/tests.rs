//! The places over a real app state and a scripted system: Open starts
//! a place that isn't running (Codex at its thread), Keep alive starts it
//! again and pauses, a stop from AI Juggler pauses it, and the window's
//! setting writer can't name a program.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use juggler_platform::apps::Start;

use super::*;
use crate::activation::{Front, Request};

#[derive(Default)]
struct Recorder {
    starts: Mutex<Vec<(PathBuf, Start, Vec<String>)>>,
}

impl start::Starter for Recorder {
    fn trusted(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }

    fn start(&self, bundle: &Path, how: Start, args: &[String]) -> Result<(), String> {
        self.starts
            .lock()
            .unwrap()
            .push((bundle.to_path_buf(), how, args.to_vec()));
        Ok(())
    }
}

#[derive(Default)]
struct Links(Mutex<Vec<String>>);

impl Front for Links {
    fn request(&self, _: u32) -> Request {
        Request::Gone
    }
    fn frontmost(&self) -> Option<u32> {
        None
    }
    fn pause(&self, _: Duration) {}
}

impl Desktop for Links {
    fn start_of(&self, _: u32) -> Option<i64> {
        None
    }
    fn open_link(&self, link: &str) -> bool {
        self.0.lock().unwrap().push(link.to_owned());
        true
    }
    fn open_folder(&self, _: &str, _: &Path) -> bool {
        false
    }
    fn osascript(&self, _: &str) -> Result<String, String> {
        Err("never".into())
    }
    fn tty(&self, _: u32) -> Option<String> {
        None
    }
}

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
    let said = open(&state, &work, Some("s-1"), Arc::new(Links::default()), starter.clone()).unwrap();
    assert_eq!(said.reached, "started");
    assert_eq!(said.outcome, "Starting Claude · b-lab; pick the session in its sidebar");
    let starts = starter.starts.lock().unwrap();
    let (bundle, how, args) = &starts[0];
    assert_eq!(bundle, &root.path().join("Claude.app"));
    assert!(how.new_instance && !how.background, "in front, beside other profiles");
    assert_eq!(
        args,
        &[format!("--user-data-dir={}", root.path().join("Profiles/b-lab").display())]
    );
}

#[test]
fn open_in_codex_opens_the_thread_by_its_link_and_starts_nothing_else() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, chat) = state_with_places(root.path());
    let starter = Arc::new(Recorder::default());
    let links = Arc::new(Links::default());
    let said = open(&state, &chat, Some("t-1"), links.clone(), starter.clone()).unwrap();
    assert_eq!(said.reached, "thread");
    assert_eq!(*links.0.lock().unwrap(), ["codex://threads/t-1"]);
    assert!(starter.starts.lock().unwrap().is_empty());
    // The session's place is found by its tool and id.
    let found = with_book(&state, |book| book.place_of("codex", "t-1").map(|(id, _)| id.clone())).unwrap();
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
    assert_eq!(starter.starts.lock().unwrap().len(), crate::features::sessions::restart::TRIES);
    let paused = with_book(&state, |book| book.places[&work].paused.clone()).unwrap();
    assert_eq!(paused.as_deref(), Some("it stopped 3 times in 10 minutes"));
    // Open ends the pause.
    open(&state, &work, None, Arc::new(Links::default()), Arc::new(Recorder::default())).unwrap();
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
    state.places.watch.lock().unwrap().found(&work, 4242, Some(7));
    stopped_here(&state, 4242);
    let paused = with_book(&state, |book| book.places[&work].paused.clone()).unwrap();
    assert_eq!(paused.as_deref(), Some("you stopped it in AI Juggler"));
}

#[test]
fn the_windows_setting_writer_cant_name_a_program() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_places(root.path());
    let refused = crate::settings_policy::app_setting_set_impl(
        &state,
        book::KEY,
        r#"{"places":{}}"#,
    );
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
