//! `environment_scan` is classed `Read` (`docs/qa/DEV-BRIDGE.md`): it
//! looks through folders and never runs a vendor CLI. Over a `HOME` that
//! holds a Claude Code and a Codex setup, with stand-in `claude`, `codex`
//! and `security` first on `PATH` recording every run, the scan leaves no
//! record of a vendor CLI. On macOS it asks the Keychain whether a sign-in
//! item exists, and every such ask is existence-only (never `-w`).
//!
//! The scan reads `HOME`, so it runs in a child copy of this test binary
//! with a cleared environment: nothing of the real setup can reach it.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

use std::path::Path;
use std::sync::Arc;
#[cfg(unix)]
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

const CHILD: &str = "AIJ_SCAN_TEST_CHILD";

fn stand_in(bin: &Path, name: &str) {
    let path = bin.join(name);
    let body = "#!/bin/sh\necho \"$(basename \"$0\") $*\" >> \"$AIJ_SCAN_RECORD\"\nexit 1\n";
    #[cfg(unix)]
    stand_in::write(&path, body);
    #[cfg(not(unix))]
    std::fs::write(&path, body).unwrap();
}

#[test]
fn the_environment_scan_runs_no_vendor_cli() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["claude", "codex", "security"] {
        stand_in(&bin, name);
    }
    // A setup of each tool, as the scan looks for them (fake contents).
    let claude = home.join(".claude");
    std::fs::create_dir_all(claude.join("projects")).unwrap();
    std::fs::write(claude.join("settings.json"), "{}").unwrap();
    std::fs::write(home.join(".claude.json"), "{}").unwrap();
    let codex = home.join(".codex");
    std::fs::create_dir_all(codex.join("sessions")).unwrap();
    std::fs::write(codex.join("config.toml"), "").unwrap();
    let record = dir.path().join("record.txt");
    std::fs::write(&record, "").unwrap();

    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "scan_in_a_child", "--ignored", "--nocapture"])
        .env_clear()
        .env(CHILD, "1")
        .env("HOME", &home)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("TMPDIR", dir.path())
        .env("AIJ_SCAN_RECORD", &record)
        .env("AIJ_SCAN_DATA", dir.path().join("data"))
        .status()
        .unwrap();
    assert!(status.success(), "the scan in the child failed");

    let runs = std::fs::read_to_string(&record).unwrap();
    for line in runs.lines() {
        let program = line.split_whitespace().next().unwrap_or_default();
        assert!(
            program != "claude" && program != "codex",
            "the scan ran a vendor CLI: {line}"
        );
        assert!(
            !line.split_whitespace().any(|arg| arg == "-w"),
            "a Keychain ask wanted the secret: {line}"
        );
    }
}

/// Runs only as the child of the test above.
#[test]
#[ignore = "run by the_environment_scan_runs_no_vendor_cli in a child process"]
fn scan_in_a_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let data = std::env::var_os("AIJ_SCAN_DATA").unwrap();
    let state = Arc::new(juggler_app::state::AppState::init(data.into()).unwrap());
    let found = juggler_app::commands::environment_scan_impl(&state);
    assert!(!found.is_empty(), "the scan found neither setup");
}
