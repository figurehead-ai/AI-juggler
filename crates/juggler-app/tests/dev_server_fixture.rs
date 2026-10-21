//! The dev-server fixture never outlives its test: it exits when the test
//! process is killed mid-run, and the guard stops it when a test ends.
//! (A killed run used to leave `python3 -m http.server` behind; the
//! release driver once stopped 133.)

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::time::{Duration, Instant};

#[path = "support/dev_server.rs"]
mod dev_server;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn listening(port: u16) -> bool {
    std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn wait_for(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    done()
}

#[test]
fn the_fixture_exits_when_the_test_it_follows_is_killed() {
    // A stand-in for a test process, killed mid-run with SIGKILL (no
    // destructor, no guard runs).
    let mut test_process = std::process::Command::new("/bin/sleep")
        .arg("60")
        .spawn()
        .unwrap();
    let port = free_port();
    // Started through a shell that exits at once, as the process tests
    // orphan their helpers.
    let status = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "python3 '{}' {port} --bind 127.0.0.1 --follow {} >/dev/null 2>&1 &",
            dev_server::script().display(),
            test_process.id()
        ))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        wait_for(Duration::from_secs(10), || listening(port)),
        "the fixture never listened"
    );
    let pids: Vec<u32> = {
        let out = std::process::Command::new("/bin/ps")
            .args(["-axo", "pid=,command="])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|line| line.contains(&format!("http.server.py {port} ")))
            .filter_map(|line| line.split_whitespace().next()?.parse().ok())
            .collect()
    };
    assert_eq!(pids.len(), 1, "one fixture process");

    test_process.kill().unwrap();
    let _ = test_process.wait();

    assert!(
        wait_for(Duration::from_secs(5), || !alive(pids[0])),
        "the fixture outlived the test it follows"
    );
    assert!(!listening(port), "its port is free again");
}

#[test]
fn the_guard_stops_the_fixture_when_the_test_ends() {
    let port = free_port();
    let mut child = dev_server::command(port)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    {
        let _servers = dev_server::Guard::new(&[port]);
        assert!(
            wait_for(Duration::from_secs(10), || listening(port)),
            "the fixture never listened"
        );
        assert_eq!(dev_server::fixture_pids(&[port]), vec![child.id()]);
    }
    let gone = wait_for(Duration::from_secs(5), || {
        matches!(child.try_wait(), Ok(Some(_)))
    });
    assert!(gone, "the guard left the fixture running");
    // Another test's port is never touched.
    assert!(dev_server::fixture_pids(&[port]).is_empty());
}
