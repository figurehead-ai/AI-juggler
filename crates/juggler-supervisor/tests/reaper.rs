//! The shared test reaper (`fixtures::Reaper`): what a test started dies
//! with it, a failed assertion's unwind included. Before it, a test that
//! failed before its cleanup left its servers and shells running (29 found
//! on the developer Mac, some 4 days old).
// Unix only: its fixtures are /bin/sh scripts, Unix sockets, modes or signals.
#![cfg(unix)]
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

/// A process the test already reaped is never signalled (its pid may be
/// another's by then), nor one whose pid now names another process; a
/// group whose leader is gone still is (what's left in it is ours).
#[test]
fn only_what_the_test_started_is_signalled() {
    let mut lone = Command::new("sleep").arg("300").spawn().unwrap();
    let mut leader = Command::new("sleep")
        .arg("300")
        .process_group(0)
        .spawn()
        .unwrap();
    let (lone_pid, group) = (lone.id(), leader.id());
    let mut reaper = fixtures::Reaper::default();
    reaper.pid(lone_pid);
    reaper.group(group);
    let lone_target = lone_pid.to_string();
    let group_target = format!("-{group}");

    let live = reaper.targets(|pid| {
        Command::new("ps")
            .args(["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .filter(|text| !text.is_empty())
    });
    assert_eq!(live, vec![group_target.clone(), lone_target.clone()]);
    // Both pids now name other processes: neither is signalled.
    let reused = reaper.targets(|_| Some("another process".to_owned()));
    assert!(reused.is_empty(), "{reused:?}");
    // Both gone (reaped by the test): the group still is, the pid isn't.
    lone.kill().unwrap();
    lone.wait().unwrap();
    leader.kill().unwrap();
    leader.wait().unwrap();
    assert_eq!(reaper.targets(|_| None), vec![group_target]);
}

/// `-1` reaches every process the user owns and `-0` the runner's own
/// group: ids 0 and 1 are never registered.
#[test]
fn ids_zero_and_one_are_never_signalled() {
    let mut reaper = fixtures::Reaper::default();
    reaper.group(0);
    reaper.group(1);
    reaper.pid(0);
    reaper.pid(1);
    let targets = reaper.targets(|_| None);
    assert!(targets.is_empty(), "{targets:?}");
    // Nothing registered, so dropping it signals nothing.
}
