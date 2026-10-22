//! QA batch 9: after a forced quit, the ending AI Juggler's lock outlived
//! it for a moment, and the relaunch said "already open and isn't
//! answering" with nothing open. A launch waits a few seconds for the
//! lock before calling the data in use. One test in its own binary: the
//! lock, once taken, is held for the rest of the process.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::time::{Duration, Instant};

use juggler_app::newer_data;

#[test]
fn a_launch_waits_for_an_ending_instance_to_let_go_of_the_data() {
    let work = tempfile::tempdir().unwrap();
    let dir = work.path().join("data");
    std::fs::create_dir_all(&dir).unwrap();
    let open = |dir: &std::path::Path| {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("juggler.lock"))
            .unwrap()
    };
    // Held the whole time: it gives up after the grace, without the lock.
    let other = open(&dir);
    other.try_lock().unwrap();
    let started = Instant::now();
    assert!(!newer_data::wait_for_instance_lock(
        &dir,
        Duration::from_millis(600)
    ));
    assert!(started.elapsed() >= Duration::from_millis(600), "it waited");
    // Let go a moment later (an instance ending): the launch takes it.
    let ending = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        drop(other);
    });
    let started = Instant::now();
    assert!(newer_data::wait_for_instance_lock(
        &dir,
        newer_data::LOCK_GRACE
    ));
    assert!(
        started.elapsed() < newer_data::LOCK_GRACE,
        "as soon as it's free"
    );
    ending.join().unwrap();
}
