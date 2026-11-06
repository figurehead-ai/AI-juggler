//! 0.4.1 live evidence (the maintainer's 0.4.0 finding #1, D-84): every
//! app-lifecycle path on the real system, on the sanctioned `aij-test`
//! Claude app profile (restart and quit allowed; never another profile).
//!
//! 1. Seen running but never added: only an offer, never a place.
//! 2. Open on it closed: started; timed to its main process.
//! 3. Open on it running: brought forward, never a second copy.
//! 4. A confirmed Restart: quit, then started again; the old pid is no
//!    longer "its process" (program and start time checked).
//! 5. Keep alive: quit, and it comes back by itself.
//! 6. Left closed, as found.
//!
//! Run by hand only, with the profile closed:
//! `AIJ_LIVE_PROFILE="$HOME/Library/Application Support/ClaudeProfiles/aij-test"
//!  cargo test -p juggler-app --test aij_test_live -- --ignored --nocapture`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_app::features::conversations::places::{HostRegistry, running_apps};
use juggler_app::features::conversations::{goto, procs};
use juggler_app::features::places::{self, book};
use juggler_app::state::AppState;

const BUNDLE: &str = "/Applications/Claude.app";

fn profile_pids(state: &Arc<AppState>, dir: &Path) -> Vec<(u32, Option<i64>)> {
    let hosts = HostRegistry::of(&state.registry);
    let home = dirs::home_dir().unwrap();
    running_apps(&procs::table(), &hosts, &home)
        .into_iter()
        .filter(|app| app.instance.as_deref() == Some(dir))
        .map(|app| (app.pid, app.start))
        .collect()
}

fn wait_for(limit: Duration, mut done: impl FnMut() -> bool) -> Option<Duration> {
    let started = Instant::now();
    while started.elapsed() < limit {
        if done() {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

fn term(pid: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-TERM", &pid.to_string()])
        .status();
}

fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

#[test]
#[ignore = "starts, restarts and quits the aij-test Claude app profile; run by hand"]
fn every_lifecycle_path_on_aij_test() {
    let Some(dir) = std::env::var_os("AIJ_LIVE_PROFILE").map(PathBuf::from) else {
        println!("SKIP: set AIJ_LIVE_PROFILE to the aij-test profile");
        return;
    };
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()),
        Some("aij-test"),
        "only the sanctioned aij-test profile"
    );
    if !Path::new(BUNDLE).is_dir() || !dir.is_dir() {
        println!("SKIP: no Claude app or no aij-test profile");
        return;
    }
    let dir = std::fs::canonicalize(&dir).unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState::init(data.path().join("data")).unwrap());
    assert!(
        profile_pids(&state, &dir).is_empty(),
        "close aij-test before the walk"
    );
    let id = book::place_id("claude-app", Some(&dir));
    let ours = |users: bool| book::KnownPlace {
        app_id: "claude-app".into(),
        name: "Claude".into(),
        instance: Some("aij-test".into()),
        bundle: PathBuf::from(BUNDLE),
        exe: PathBuf::from(BUNDLE).join("Contents/MacOS/Claude"),
        given: Some(dir.clone()),
        instance_dir: Some(dir.clone()),
        account_id: None,
        conversations: Vec::new(),
        seen_ms: 0,
        keep_alive: false,
        paused: None,
        found: false,
        users,
        declined: false,
    };
    places::with_book(&state, |book| book.places.insert(id.clone(), ours(true))).unwrap();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 2. Open, closed: started.
        let clicked = Instant::now();
        let said = places::open(
            &state,
            &id,
            None,
            Arc::new(goto::MacDesktop),
            Arc::new(places::start::System),
        )
        .unwrap();
        let answered = clicked.elapsed();
        assert_eq!(said.reached, "started", "{}", said.outcome);
        let up = wait_for(Duration::from_secs(30), || {
            !profile_pids(&state, &dir).is_empty()
        })
        .expect("aij-test never came up");
        println!(
            "Open (closed): answered in {answered:?} ({}); main process after {up:?}",
            said.outcome
        );
        let first = profile_pids(&state, &dir);
        assert_eq!(first.len(), 1, "one copy: {first:?}");

        // 1. Seen running by a fresh AI Juggler that never added it: an
        // offer only (D-84).
        let fresh_data = tempfile::tempdir().unwrap();
        let fresh = Arc::new(AppState::init(fresh_data.path().join("data")).unwrap());
        let hosts = HostRegistry::of(&fresh.registry);
        let running = running_apps(&procs::table(), &hosts, &dirs::home_dir().unwrap());
        places::observe(&fresh, &running, &[]);
        let offers = places::offers(&fresh).unwrap();
        println!(
            "Seen running, never added: offers {:?}",
            offers.iter().map(|(_, label)| label).collect::<Vec<_>>()
        );
        assert!(
            offers.iter().any(|(offer, _)| *offer == id),
            "aij-test is offered"
        );
        let refused = places::open(
            &fresh,
            &id,
            None,
            Arc::new(goto::MacDesktop),
            Arc::new(places::start::System),
        );
        assert!(refused.is_err(), "an offer can't be opened");
        assert!(
            places::set_keep_alive(&fresh, &id, true).is_err(),
            "nor kept alive"
        );
        let rows = fresh.storage.lock().unwrap().apps().list().unwrap();
        assert!(
            rows.iter()
                .filter(|row| row.kind == "desktop")
                .all(|row| !row.is_users()),
            "no desktop App was added by itself"
        );

        // 3. Open, running: forward, never a second copy.
        let clicked = Instant::now();
        let said = places::open(
            &state,
            &id,
            None,
            Arc::new(goto::MacDesktop),
            Arc::new(places::start::System),
        )
        .unwrap();
        println!(
            "Open (running): {:?} in {:?}: {}",
            said.reached,
            clicked.elapsed(),
            said.outcome
        );
        assert_ne!(said.reached, "started");
        std::thread::sleep(Duration::from_secs(2));
        assert_eq!(profile_pids(&state, &dir), first, "never a second copy");

        // 4. A confirmed Restart.
        places::observe(
            &state,
            &running_apps(
                &procs::table(),
                &HostRegistry::of(&state.registry),
                &dirs::home_dir().unwrap(),
            ),
            &[],
        );
        let clicked = Instant::now();
        let said = places::restart(
            &state,
            &id,
            &places::start::System,
            false,
            places::RestartOf::Confirmed,
            places::QuitWait::default(),
            now_ms(),
        );
        println!("Restart: {said:?} answered in {:?}", clicked.elapsed());
        assert!(said.is_ok(), "{said:?}");
        let back = wait_for(Duration::from_secs(30), || {
            profile_pids(&state, &dir)
                .iter()
                .any(|(pid, _)| *pid != first[0].0)
        })
        .expect("aij-test never came back after Restart");
        println!(
            "  running again {:?} after the click",
            clicked.elapsed().max(back)
        );
        let (old_pid, old_start) = first[0];
        let old_alive = procs::some(&[old_pid])
            .get(&old_pid)
            .is_some_and(|process| {
                places::is_its_process(
                    &PathBuf::from(BUNDLE).join("Contents/MacOS/Claude"),
                    old_start,
                    process,
                )
            });
        assert!(!old_alive, "the old pid is no longer aij-test");
        let second = profile_pids(&state, &dir);
        assert_eq!(second.len(), 1, "one copy: {second:?}");

        // 5. Keep alive: quit (as ⌘Q), and it comes back.
        places::observe(
            &state,
            &running_apps(
                &procs::table(),
                &HostRegistry::of(&state.registry),
                &dirs::home_dir().unwrap(),
            ),
            &[],
        );
        places::set_keep_alive(&state, &id, true).unwrap();
        state
            .places
            .watch
            .lock()
            .unwrap()
            .settled_at(now_ms());
        for _ in 0..3 {
            places::tick(&state);
            std::thread::sleep(Duration::from_secs(2));
        }
        let quit = Instant::now();
        term(second[0].0);
        let mut back = None;
        while quit.elapsed() < Duration::from_secs(45) {
            places::tick(&state);
            if profile_pids(&state, &dir)
                .iter()
                .any(|(pid, _)| *pid != second[0].0)
            {
                back = Some(quit.elapsed());
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        println!("Keep alive: quit → running again after {back:?}");
        assert!(back.is_some(), "keep alive never brought it back");
        places::set_keep_alive(&state, &id, false).unwrap();
    }));
    // 6. Left closed, as found.
    for (pid, _) in profile_pids(&state, &dir) {
        term(pid);
    }
    let closed = wait_for(Duration::from_secs(15), || {
        profile_pids(&state, &dir).is_empty()
    });
    println!("aij-test closed again: {}", closed.is_some());
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
