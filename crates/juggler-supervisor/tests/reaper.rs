//! The shared test reaper (`fixtures::Reaper`): what a test started dies
//! with it, a failed assertion's unwind included. Before it, a test that
//! failed before its cleanup left its servers and shells running (29 found
//! on the developer Mac, some 4 days old).

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

// The reaper and the wait helper (the rest of the kit serves other suites).
#[allow(dead_code)]
mod fixtures;

use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// The processes in a group (`pgrep -g`).
fn members(pgid: u32) -> usize {
    Command::new("pgrep")
        .args(["-g", &pgid.to_string()])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).lines().count())
        .unwrap_or_default()
}

/// The signal that ended `child`, waiting at most 5 s; a child still
/// running then is killed here, so a failing test never hangs.
fn ended_by(child: &mut Child) -> Option<i32> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            return status.signal();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    None
}

#[test]
fn a_test_that_panics_midway_leaves_nothing_behind() {
    // A group, as a supervised root with its helpers, and a lone process
    // in a group of its own, registered by pid only.
    let mut leader = Command::new("/bin/sh")
        .args(["-c", "sleep 300 & sleep 300 & wait"])
        .process_group(0)
        .spawn()
        .unwrap();
    let mut lone = Command::new("sleep")
        .arg("300")
        .process_group(0)
        .spawn()
        .unwrap();
    let (group, lone_pid) = (leader.id(), lone.id());
    assert!(
        fixtures::wait_for(10_000, || members(group) == 3),
        "the group started"
    );

    let unwound = std::panic::catch_unwind(move || {
        let mut reaper = fixtures::Reaper::default();
        reaper.group(group);
        reaper.pid(lone_pid);
        panic!("an assertion fails mid-test");
    });

    assert!(unwound.is_err());
    assert_eq!(ended_by(&mut leader), Some(9), "the group's leader");
    assert_eq!(ended_by(&mut lone), Some(9), "the lone process");
    assert!(
        fixtures::wait_for(5_000, || members(group) == 0),
        "the group's children: {} left",
        members(group)
    );
}
