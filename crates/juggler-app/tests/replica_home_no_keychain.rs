//! 0.4.1: a run under a replica `HOME` (the Floor harness, a lab, a test)
//! never reaches a keychain. Before this, the harness's release app made
//! macOS start `SecurityAgent` and offer to create or reset a keychain.
//!
//! The no-keychain choice is made once per process, so the test runs
//! itself again as a child with `HOME` pointed at an empty replica and
//! checks there:
//! - the platform says the keychain is off, and why;
//! - the app's secrets (the device token, the vault key, service keys, the
//!   Fleet identity) go to owner-only files under the data folder;
//! - the tools' sign-in probes answer "not found" without running
//!   `security`;
//! - `AIJ_NO_KEYCHAIN=1` turns it off even with the real `HOME`.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test")]

use std::process::Command;

const CHILD: &str = "AIJ_TEST_REPLICA_CHILD";

fn run_child(test: &str, env: &[(&str, &std::path::Path)], force: bool) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([test, "--exact", "--nocapture"])
        .env(CHILD, "1");
    for (key, value) in env {
        command.env(key, value);
    }
    if force {
        command.env("AIJ_NO_KEYCHAIN", "1");
    } else {
        command.env_remove("AIJ_NO_KEYCHAIN");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// What the child checks: run only inside the child.
fn check_no_keychain(data: &std::path::Path, why: &str) {
    assert_eq!(juggler_platform::keychain::off(), Some(why));
    assert_eq!(juggler_app::edition::keep_secrets(data), Some(why));
    assert!(juggler_account::in_files());
    // A secret saved now lands in a private file, not the keychain.
    let store = juggler_account::KeychainStore::for_service("ai.figurehead.juggler.test");
    juggler_account::SecretStore::set(&store, "fleet/identity.key", "seed").unwrap();
    assert_eq!(
        juggler_account::SecretStore::get(&store, "fleet/identity.key").as_deref(),
        Some("seed")
    );
    let files = std::fs::read_dir(data.join("secrets")).unwrap().count();
    assert_eq!(files, 1, "one owner-only file");
    // A tool's sign-in probe runs nothing and finds nothing.
    assert!(!juggler_platform::keychain::item_exists(
        "Claude Code-credentials",
        None
    ));
}

#[test]
fn a_replica_home_never_reaches_a_keychain() {
    if std::env::var_os(CHILD).is_some() {
        let data = std::path::PathBuf::from(std::env::var_os("AIJ_TEST_DATA").unwrap());
        check_no_keychain(&data, "HOME isn't this user's home");
        return;
    }
    let replica = tempfile::tempdir().unwrap();
    let data = replica.path().join("data");
    run_child(
        "a_replica_home_never_reaches_a_keychain",
        &[("HOME", replica.path()), ("AIJ_TEST_DATA", &data)],
        false,
    );
}

#[test]
fn aij_no_keychain_turns_it_off_with_the_real_home() {
    if std::env::var_os(CHILD).is_some() {
        let data = std::path::PathBuf::from(std::env::var_os("AIJ_TEST_DATA").unwrap());
        check_no_keychain(&data, "AIJ_NO_KEYCHAIN is set");
        return;
    }
    let Some(real) = juggler_platform::user::record_home() else {
        return;
    };
    let scratch = tempfile::tempdir().unwrap();
    let data = scratch.path().join("data");
    run_child(
        "aij_no_keychain_turns_it_off_with_the_real_home",
        &[("HOME", &real), ("AIJ_TEST_DATA", &data)],
        true,
    );
}
