//! ADR-0018 §3: the vendor's default folder (`~/.claude`) used where it
//! is runs as Claude runs it — `CLAUDE_CONFIG_DIR` unset, so the same
//! Keychain sign-in and `~/.claude.json` apply. Any other folder is set
//! exactly, as the profile manager sets it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::pedantic)]
#![cfg(unix)]

use juggler_adapter::launch::ENV_UNSET;
use juggler_adapters::claude_code::{
    claude_state_file_in, config_dir_value_in, is_vendor_default_dir_in,
};

#[test]
fn the_default_folder_runs_unset_and_others_run_exactly() {
    let home = tempfile::tempdir().unwrap();
    let default = home.path().join(".claude");
    std::fs::create_dir_all(&default).unwrap();
    std::fs::write(home.path().join(".claude.json"), "{}").unwrap();
    let other = home.path().join("profiles/work-a");
    std::fs::create_dir_all(&other).unwrap();
    // A link to the default folder is the default folder.
    let link = home.path().join("claude-link");
    std::os::unix::fs::symlink(&default, &link).unwrap();

    assert!(is_vendor_default_dir_in(&default, home.path()));
    assert!(is_vendor_default_dir_in(&link, home.path()));
    assert!(!is_vendor_default_dir_in(&other, home.path()));

    assert_eq!(config_dir_value_in(&default, home.path()), ENV_UNSET);
    assert_eq!(config_dir_value_in(&other, home.path()), other.to_str().unwrap());

    // Claude keeps the default folder's state file beside it.
    assert_eq!(
        claude_state_file_in(&default, home.path()),
        home.path().join(".claude.json")
    );
    assert_eq!(claude_state_file_in(&other, home.path()), other.join(".claude.json"));
}
