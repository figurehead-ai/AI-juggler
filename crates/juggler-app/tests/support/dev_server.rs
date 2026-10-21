//! The dev-server fixture (`fixtures/http.server.py`) for the process
//! tests, and the guard that stops it.
//!
//! A test run killed mid-test used to leave `python3 -m http.server`
//! behind (the release driver once stopped 133). Now:
//! - the fixture follows this test process and exits when it is gone;
//! - [`Guard`] kills the fixtures on its ports when the test ends —
//!   passed, failed or panicked;
//! - `scripts/leaked_fixtures.sh` fails the suite on any left behind.
//!
//! Included with `#[path = "support/dev_server.rs"] mod dev_server;`.

#![allow(dead_code)]

use std::path::PathBuf;

/// The fixture script, by its absolute path in this checkout (the leak
/// check finds this checkout's fixtures by it).
#[must_use]
pub fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/http.server.py")
}

/// The shell words that start the fixture on `port`, following this test
/// process — for a supervised `sh -c` root.
#[must_use]
pub fn shell(port: u16) -> String {
    format!(
        "python3 '{}' {port} --bind 127.0.0.1 --follow {}",
        script().display(),
        std::process::id()
    )
}

/// [`shell`], moved to its own session first (an escaped helper).
#[must_use]
pub fn shell_setsid(port: u16) -> String {
    format!("{} --setsid", shell(port))
}

/// The fixture started directly (not under the supervisor).
#[must_use]
pub fn command(port: u16) -> std::process::Command {
    let mut command = std::process::Command::new("python3");
    command
        .arg(script())
        .arg(port.to_string())
        .args(["--bind", "127.0.0.1", "--follow"])
        .arg(std::process::id().to_string());
    command
}

/// Kills this test's fixtures on its ports when dropped. Tests in one
/// binary share a pid, so the port tells them apart.
pub struct Guard {
    ports: Vec<u16>,
}

impl Guard {
    /// A guard for the fixtures on `ports`.
    #[must_use]
    pub fn new(ports: &[u16]) -> Self {
        Self {
            ports: ports.to_vec(),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        for pid in fixture_pids(&self.ports) {
            let _ = std::process::Command::new("/bin/kill")
                .args(["-9", &pid.to_string()])
                .status();
        }
    }
}

/// The live fixtures this test process started on `ports`.
#[must_use]
pub fn fixture_pids(ports: &[u16]) -> Vec<u32> {
    let Ok(out) = std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,command="])
        .output()
    else {
        return Vec::new();
    };
    let script = script().display().to_string();
    let follow = format!("--follow {}", std::process::id());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.contains(&script) && line.contains(&follow))
        .filter(|line| {
            ports
                .iter()
                .any(|port| line.contains(&format!("http.server.py {port} ")))
        })
        .filter_map(|line| line.split_whitespace().next()?.parse().ok())
        .collect()
}
