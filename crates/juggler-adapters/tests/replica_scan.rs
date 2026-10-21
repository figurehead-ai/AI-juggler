//! AJ-E1531: the replica is shaped like a real heavy user's Mac. The
//! adapters' own scans find every account the generator says it made
//! (`~/.replica/truth.json`), and nothing else.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use juggler_adapter::environment::LoginProbe;
use juggler_adapters::claude_environment::scan_claude_environment;
use juggler_adapters::codex_environment::scan_codex_environment;

fn probe(_path: &Path) -> LoginProbe {
    LoginProbe::LoggedIn
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[test]
fn the_scans_find_every_replica_account() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/replica.py");
    let built = Command::new("python3")
        .arg("-I")
        .arg(&script)
        .args(["build", "--seed", "7", "--days", "30", "--home"])
        .arg(&home)
        .output()
        .expect("python3 runs the replica generator");
    assert!(
        built.status.success(),
        "the replica builds: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let truth: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(home.join(".replica/truth.json")).unwrap())
            .unwrap();
    let expected = |tool: &str| {
        let mut folders: Vec<PathBuf> = truth["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|account| account["tool"] == tool)
            .map(|account| canonical(Path::new(account["folder"].as_str().unwrap())))
            .collect();
        folders.sort();
        folders
    };
    let found = |installs: Vec<juggler_adapter::environment::DiscoveredInstall>| {
        let mut folders: Vec<PathBuf> = installs
            .iter()
            .map(|install| canonical(&install.path))
            .collect();
        folders.sort();
        folders.dedup();
        folders
    };
    assert_eq!(
        found(scan_claude_environment(&home, None, &[], &probe)),
        expected("claude-code"),
        "Claude: ~/.claude and the six profiles"
    );
    assert_eq!(
        found(scan_codex_environment(&home, None, &probe)),
        expected("codex"),
        "Codex: ~/.codex and ~/.codex-work"
    );
}
