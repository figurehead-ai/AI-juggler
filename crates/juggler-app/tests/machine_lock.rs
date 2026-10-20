//! Review of #67: the machine tick never holds the sampler's lock over its
//! slow half. A disk tick that has to wait on storage (finding the folders
//! in use) — and then rebuild the disk list — leaves the lock free for the
//! UI's reads all along.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::AdapterRegistry;
use juggler_app::machine_health;
use juggler_app::state::AppState;

#[test]
fn a_slow_disk_tick_never_holds_the_machine_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    // The first tick is a disk tick that also builds the disk list.
    let storage_held = Arc::new(std::sync::Barrier::new(2));
    let holder = {
        let state = Arc::clone(&state);
        let storage_held = Arc::clone(&storage_held);
        std::thread::spawn(move || {
            let _storage = state.storage.lock().unwrap();
            storage_held.wait();
            // Storage stays busy (a slow write) while the tick needs it.
            std::thread::sleep(Duration::from_millis(800));
        })
    };
    storage_held.wait();
    let ticker = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || machine_health::tick(&state, 0))
    };
    std::thread::sleep(Duration::from_millis(200));
    assert!(!ticker.is_finished(), "the tick is waiting on storage");
    for _ in 0..5 {
        assert!(
            state.machine.try_lock().is_ok(),
            "the sampler's lock is free while the tick waits"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    holder.join().unwrap();
    ticker.join().unwrap();
    // The disk half landed once storage answered.
    let health = machine_health::machine_health_impl(&state).unwrap();
    assert!(health.sampled);
    assert!(!health.volumes.is_empty(), "the data folder's volume");
}
