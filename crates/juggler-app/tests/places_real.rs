//! 0.3.3 live evidence (D-63): Open and Keep alive on the real system,
//! with a throwaway Claude profile in a temp folder (never signed in).
//! It starts the profile through the production Open (`/usr/bin/open -n
//! -a Claude --args --user-data-dir=<lab>`), finds its main process the
//! way the live view does, quits it with SIGTERM (what ⌘Q sends), and
//! times Keep alive's start; then a stop from AI Juggler pauses it.
//! Ignored in CI (no apps): run locally with
//! `cargo test -p juggler-app --test places_real -- --ignored --nocapture`
//! and keep the output as the walk's record.
//!
//! Safety: only processes whose instance folder is this test's own temp
//! folder are signalled (found by the live view's own reader); the
//! user's Claude profiles are never touched. Every lab process is ended
//! before the folder is removed.

// Test crates are pragmatic by policy (QUALITY.md §2).
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

fn lab_pids(state: &Arc<AppState>, lab: &Path) -> Vec<u32> {
    let hosts = HostRegistry::of(&state.registry);
    let home = dirs::home_dir().unwrap();
    running_apps(&procs::table(), &hosts, &home)
        .into_iter()
        .filter(|app| app.instance.as_deref() == Some(lab))
        .map(|app| app.pid)
        .collect()
}

fn wait_for(limit: Duration, mut done: impl FnMut() -> bool) -> Option<Duration> {
    let started = Instant::now();
    while started.elapsed() < limit {
        if done() {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

fn term(pid: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-TERM", &pid.to_string()])
        .status();
}

/// Ends every lab process (TERM, then KILL after 10 s).
fn reap(state: &Arc<AppState>, lab: &Path) {
    for pid in lab_pids(state, lab) {
        term(pid);
    }
    if wait_for(Duration::from_secs(10), || lab_pids(state, lab).is_empty()).is_none() {
        for pid in lab_pids(state, lab) {
            let _ = std::process::Command::new("/bin/kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    }
}

/// Keep alive's tick, every 2 s, until `done` or `limit`.
fn tick_until(state: &Arc<AppState>, limit: Duration, mut done: impl FnMut() -> bool) -> Option<Duration> {
    let started = Instant::now();
    while started.elapsed() < limit {
        places::tick(state);
        if done() {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    None
}

#[test]
#[ignore = "starts a throwaway Claude profile on this computer; run by hand"]
fn open_starts_a_closed_profile_and_keep_alive_brings_it_back() {
    if !Path::new(BUNDLE).is_dir() {
        println!("SKIP: no {BUNDLE}");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let lab: PathBuf = std::fs::canonicalize(root.path()).unwrap().join("Lab Claude profile");
    std::fs::create_dir_all(&lab).unwrap();
    let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
    let id = book::place_id("claude-app", Some(&lab));
    places::with_book(&state, |book| {
        book.places.insert(
            id.clone(),
            book::KnownPlace {
                app_id: "claude-app".into(),
                name: "Claude".into(),
                instance: Some("Lab Claude profile".into()),
                bundle: PathBuf::from(BUNDLE),
                exe: PathBuf::from(BUNDLE).join("Contents/MacOS/Claude"),
                given: Some(lab.clone()),
                instance_dir: Some(lab.clone()),
                account_id: None,
                conversations: Vec::new(),
                seen_ms: 0,
                keep_alive: false,
                paused: None,
            },
        );
    })
    .unwrap();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 1. Open, with the profile closed: started, in front.
        assert!(lab_pids(&state, &lab).is_empty());
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
        println!("Open (closed): {:?} in {answered:?}: {}", said.reached, said.outcome);
        assert_eq!(said.reached, "started");
        let up = wait_for(Duration::from_secs(30), || !lab_pids(&state, &lab).is_empty())
            .expect("the lab profile never came up");
        println!("  its main process seen after {up:?}");
        let first = lab_pids(&state, &lab);
        assert_eq!(first.len(), 1, "one instance: {first:?}");

        // 2. Open again, running: brought forward, never a second one.
        let clicked = Instant::now();
        let said = places::open(
            &state,
            &id,
            None,
            Arc::new(goto::MacDesktop),
            Arc::new(places::start::System),
        )
        .unwrap();
        println!("Open (running): {:?} in {:?}: {}", said.reached, clicked.elapsed(), said.outcome);
        assert_ne!(said.reached, "started");
        std::thread::sleep(Duration::from_secs(2));
        assert_eq!(lab_pids(&state, &lab), first, "never a second instance");

        // 3. Keep alive on; it's quit (SIGTERM, as ⌘Q): started again.
        places::set_keep_alive(&state, &id, true).unwrap();
        let now = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        state.places.watch.lock().unwrap().settled_at(now);
        tick_until(&state, Duration::from_secs(6), || false);
        let quit = Instant::now();
        term(first[0]);
        let back = tick_until(&state, Duration::from_secs(45), || {
            lab_pids(&state, &lab).iter().any(|pid| *pid != first[0])
        })
        .expect("keep alive never started it again");
        println!("Keep alive: quit → running again after {:?}", quit.elapsed());
        let _ = back;
        let second = lab_pids(&state, &lab);
        assert_eq!(second.len(), 1, "one instance again: {second:?}");

        // 4. A stop from AI Juggler pauses it: quit again, and it stays down.
        tick_until(&state, Duration::from_secs(4), || false);
        places::stopped_here(&state, second[0]);
        term(second[0]);
        let restarted = tick_until(&state, Duration::from_secs(25), || {
            lab_pids(&state, &lab).iter().any(|pid| *pid != second[0])
        });
        let paused = places::with_book(&state, |book| book.places[&id].paused.clone()).unwrap();
        println!("Stopped from AI Juggler: paused {paused:?}; started again: {}", restarted.is_some());
        assert!(restarted.is_none(), "a stop from AI Juggler must stay stopped");
        assert_eq!(paused.as_deref(), Some("you stopped it in AI Juggler"));
    }));
    reap(&state, &lab);
    assert!(lab_pids(&state, &lab).is_empty(), "a lab process outlived the walk");
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
