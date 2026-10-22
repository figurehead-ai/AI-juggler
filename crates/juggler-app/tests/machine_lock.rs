//! Review of #67: the machine tick never holds the sampler's lock over its
//! slow half. A disk tick that has to wait on storage (finding the folders
//! in use) — and then build the disk list — leaves the lock free for the
//! UI's reads all along.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::sync::Arc;
use std::sync::{TryLockError, mpsc};
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
    // The tick takes the lock for its cheap part only: once its count
    // moved (read under the lock, so that part is over), the tick goes on
    // to storage, which is held — it can't finish …
    // try_lock: a tick that kept the lock makes this fail, not hang.
    let started = Instant::now();
    loop {
        match state.machine.try_lock() {
            Ok(watch) if watch.ticks() > 0 => break,
            Ok(_) | Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Poisoned(_)) => panic!("the sampler's lock is poisoned"),
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the tick's count never showed: it never ran, or it kept the lock"
        );
        std::thread::yield_now();
    }
    // … and the lock stays free while it waits: a blocking lock() returns.
    // If the tick held the lock across its storage wait, this would wait
    // until storage is let go — which happens only after this check. (The
    // bound is a safety net, not a timing assumption.)
    for _ in 0..3 {
        let (sent, got) = mpsc::channel();
        let reader = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                let _watch = state.machine.lock().unwrap();
                sent.send(()).unwrap();
            })
        };
        assert!(
            got.recv_timeout(Duration::from_secs(60)).is_ok(),
            "the sampler's lock is free while the tick waits on storage"
        );
        reader.join().unwrap();
        assert!(
            !ticker.is_finished(),
            "the tick is still waiting on storage"
        );
    }
    release.send(()).unwrap();
    holder.join().unwrap();
    ticker.join().unwrap();
    // The disk half landed once storage answered.
    let health = machine_health::machine_health_impl(&state).unwrap();
    assert!(health.sampled);
    assert!(!health.volumes.is_empty(), "the data folder's volume");
}
