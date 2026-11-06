//! D-84 (2026-11-06): AI Juggler never adds an app by itself. One seen
//! running that the user never added is offered ("Found on this computer"),
//! never listed, opened or kept alive; Add makes it an App, Not now
//! hides the offer. A profile under the system's temporary folder is
//! never remembered at all, and a reused pid is never the app.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::features::conversations::places::RunningApp;
use crate::features::conversations::procs::Proc;

/// A Claude profile running in `dir` (as pid 4242, never signalled).
fn claude_in(root: &Path, dir: &Path) -> RunningApp {
    let bundle = root.join("Claude.app");
    std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    RunningApp {
        app_id: "claude-app".into(),
        exe: bundle.join("Contents/MacOS/Claude"),
        bundle,
        pid: 4242,
        start: Some(1_000),
        given: Some(dir.to_path_buf()),
        instance: Some(dir.to_path_buf()),
    }
}

/// A profile folder outside the system's temporary folder (those are
/// never remembered), removed when the test ends.
fn profile_folder() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(".d84-profile-")
        .tempdir_in(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
}

fn state(root: &Path) -> Arc<AppState> {
    Arc::new(AppState::init(root.join("data")).unwrap())
}

fn listed(state: &Arc<AppState>) -> (Vec<String>, Vec<String>) {
    let places = commands::known(state)
        .unwrap()
        .into_iter()
        .map(|place| place.id)
        .collect();
    let apps = crate::features::apps::list_impl(state)
        .unwrap()
        .into_iter()
        .map(|app| app.id)
        .collect();
    (places, apps)
}

#[test]
fn a_profile_seen_running_but_never_added_is_only_offered() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let id = book::place_id("claude-app", Some(folder.path()));

    let (places, apps) = listed(&state);
    assert!(!places.contains(&id), "not a place: {places:?}");
    assert!(!apps.contains(&id), "not an App: {apps:?}");
    let offers = offers(&state).unwrap();
    assert_eq!(offers.len(), 1, "{offers:?}");
    assert_eq!(offers[0].0, id);
    // Nothing acts on an offer.
    assert!(set_keep_alive(&state, &id, true).is_err());
    let refused = open(&state, &id, None, Arc::new(NoDesktop), Arc::new(NoStarter)).unwrap_err();
    assert!(refused.to_string().contains("add it first"), "{refused}");

    // Add: it's the user's App now, and no longer offered.
    answer_offer(&state, &id, true).unwrap();
    let (places, apps) = listed(&state);
    assert!(places.contains(&id) && apps.contains(&id));
    assert!(offers_of(&state).is_empty());
}

#[test]
fn not_now_hides_the_offer_and_adds_nothing() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    let running = [claude_in(root.path(), folder.path())];
    observe(&state, &running, &[]);
    let id = book::place_id("claude-app", Some(folder.path()));
    answer_offer(&state, &id, false).unwrap();
    assert!(offers_of(&state).is_empty());
    // Seen running again: still declined, still no App.
    observe(&state, &running, &[]);
    assert!(offers_of(&state).is_empty());
    let (places, apps) = listed(&state);
    assert!(!places.contains(&id) && !apps.contains(&id));
}

#[test]
fn an_offer_whose_folder_is_gone_is_forgotten() {
    let root = tempfile::tempdir().unwrap();
    let folder = profile_folder();
    let state = state(root.path());
    observe(&state, &[claude_in(root.path(), folder.path())], &[]);
    let gone = folder.path().to_path_buf();
    drop(folder);
    assert!(!gone.exists());
    assert!(offers_of(&state).is_empty());
}

#[test]
fn a_profile_in_the_temporary_folder_is_never_remembered() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let temp = root.path().join("Lab Claude profile");
    std::fs::create_dir_all(&temp).unwrap();
    assert!(juggler_core::storage::in_temporary_folder(
        temp.to_str().unwrap()
    ));
    observe(&state, &[claude_in(root.path(), &temp)], &[]);
    assert!(offers_of(&state).is_empty());
    assert!(with_book(&state, |book| book.places.is_empty()).unwrap());
}

#[test]
fn a_reused_pid_is_not_the_app() {
    let exe = PathBuf::from("/Applications/Claude.app/Contents/MacOS/Claude");
    let process = |exe: &str, start: Option<i64>| Proc {
        pid: 4242,
        ppid: None,
        exe: Some(PathBuf::from(exe)),
        args: Vec::new(),
        start,
    };
    let same = process(exe.to_str().unwrap(), Some(1_000));
    assert!(is_its_process(&exe, Some(1_000), &same));
    assert!(is_its_process(&exe, Some(1_001), &same), "to the second");
    assert!(!is_its_process(&exe, Some(2_000), &same), "started later");
    assert!(!is_its_process(&exe, None, &same), "no start recorded");
    assert!(
        !is_its_process(&exe, Some(1_000), &process("/bin/zsh", Some(1_000))),
        "another program"
    );
    assert!(!is_its_process(
        &exe,
        Some(1_000),
        &process(exe.to_str().unwrap(), None)
    ));
}

fn offers_of(state: &Arc<AppState>) -> Vec<(String, String)> {
    offers(state).unwrap()
}

struct NoDesktop;

impl crate::activation::Front for NoDesktop {
    fn request(&self, _: u32) -> crate::activation::Request {
        crate::activation::Request::Gone
    }
    fn frontmost(&self) -> Option<u32> {
        None
    }
    fn pause(&self, _: std::time::Duration) {}
}

impl Desktop for NoDesktop {
    fn start_of(&self, _: u32) -> Option<i64> {
        None
    }
    fn detached(&self, _: u32, _: Option<i64>) -> bool {
        false
    }
    fn open_link(&self, _: &str) -> bool {
        false
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

struct NoStarter;

impl start::Starter for NoStarter {
    fn trusted(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }
    fn start(&self, _: &start::Launch) -> Result<(), String> {
        Err("an offer is never started".into())
    }
    fn detached(&self, _: u32, _: Option<i64>) -> bool {
        false
    }
    fn quit(&self, _: u32) -> Result<(), String> {
        Err("never".into())
    }
    fn alive(&self, _: u32, _: Option<i64>) -> bool {
        false
    }
}

#[test]
fn a_session_started_in_a_closed_profile_is_that_profiles() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let place = |dir: &str, account: Option<&str>, users: bool| book::KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude app".into(),
        instance: Some(dir.into()),
        bundle: PathBuf::new(),
        exe: PathBuf::new(),
        given: None,
        instance_dir: Some(PathBuf::from("/u").join(dir)),
        account_id: account.map(str::to_owned),
        conversations: Vec::new(),
        seen_ms: 1,
        keep_alive: false,
        paused: None,
        found: false,
        users,
        declined: false,
    };
    with_book(&state, |book| {
        book.places
            .insert("anela".into(), place("b-anela", Some("f-anela"), true));
        book.places
            .insert("twin-1".into(), place("b-twin", Some("f-twin"), true));
        book.places
            .insert("twin-2".into(), place("b-twin2", Some("f-twin"), true));
        book.places
            .insert("offer".into(), place("b-offer", Some("f-offer"), false));
        assert_eq!(
            book.users_place_on("claude-app", "f-anela"),
            Some(&"anela".to_owned())
        );
        assert_eq!(
            book.users_place_on("codex-app", "f-anela"),
            None,
            "another app"
        );
        assert_eq!(
            book.users_place_on("claude-app", "f-twin"),
            None,
            "which one isn't known"
        );
        assert_eq!(
            book.users_place_on("claude-app", "f-offer"),
            None,
            "an offer is no App"
        );
    })
    .unwrap();
}
