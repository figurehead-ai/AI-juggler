//! Review of #67: the machine tick never holds the sampler's lock over its
//! slow half. A disk tick that has to wait on storage (finding the folders
//! in use) — and then build the disk list — leaves the lock free for the
//! UI's reads all along.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use juggler_adapter::AdapterRegistry;
use juggler_app::machine_health;
use juggler_app::state::AppState;

#[test]
fn a_slow_disk_tick_never_holds_the_machine_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    // Storage stays busy (a slow write) until the test lets it go — no
    // timing guesses, so a loaded machine can't make this flaky.
    let (release, released) = mpsc::channel::<()>();
    let storage_held = Arc::new(std::sync::Barrier::new(2));
    let holder = {
        let state = Arc::clone(&state);
        let storage_held = Arc::clone(&storage_held);
        std::thread::spawn(move || {
            let _storage = state.storage.lock().unwrap();
            storage_held.wait();
            released.recv().unwrap();
        })
    };
    storage_held.wait();
    // The first tick is a disk tick that also builds the disk list.
    let ticker = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || machine_health::tick(&state, 0))
    };
    let mut ticker_slot = Some(ticker);
    let ticker = ticker_slot.as_ref().unwrap();
    // The tick takes the lock for its cheap part only, then waits on
    // storage: the lock comes free while the tick is still running …
    let deadline = Instant::now() + Duration::from_secs(10);
    while state.machine.try_lock().is_err() {
        assert!(Instant::now() < deadline, "the lock never came free");
        std::thread::sleep(Duration::from_millis(5));
    }
    // … and stays free the whole time the tick waits.
    for _ in 0..10 {
        assert!(
            !ticker.is_finished(),
            "the tick is still waiting on storage"
        );
        assert!(
            state.machine.try_lock().is_ok(),
            "the sampler's lock is free while the tick waits"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
    release.send(()).unwrap();
    holder.join().unwrap();
    ticker_slot.take().unwrap().join().unwrap();
    // The disk half landed once storage answered.
    let health = machine_health::machine_health_impl(&state).unwrap();
    assert!(health.sampled);
    assert!(!health.volumes.is_empty(), "the data folder's volume");
}
