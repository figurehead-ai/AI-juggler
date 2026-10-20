//! ADR-0018 §3: the vendor's default folder (`~/.claude`) used where it
//! is runs as Claude runs it — `CLAUDE_CONFIG_DIR` unset, so the same
//! Keychain sign-in and `~/.claude.json` apply. Any other folder is set
//! exactly, as the profile manager sets it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
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
    // A link to the default folder is NOT it: launched through the link,
    // CLAUDE_CONFIG_DIR is the link and its sign-in is namespaced.
    let link = home.path().join("claude-link");
    std::os::unix::fs::symlink(&default, &link).unwrap();

    assert!(is_vendor_default_dir_in(&default, home.path()));
    assert!(!is_vendor_default_dir_in(&link, home.path()));
    assert!(is_vendor_default_dir_in(
        &home.path().join(".claude/"),
        home.path()
    ));
    assert!(!is_vendor_default_dir_in(&other, home.path()));

    assert_eq!(config_dir_value_in(&default, home.path()), ENV_UNSET);
    assert_eq!(
        config_dir_value_in(&other, home.path()),
        other.to_str().unwrap()
    );

    // Claude keeps the default folder's state file beside it — even when a
    // stale one sits inside.
    std::fs::write(default.join(".claude.json"), "{\"stale\":true}").unwrap();
    assert_eq!(
        claude_state_file_in(&default, home.path()),
        home.path().join(".claude.json")
    );
    assert_eq!(
        claude_state_file_in(&other, home.path()),
        other.join(".claude.json")
    );
}

/// Started from a terminal that exports a key, AI Juggler would run every
/// account on it: launches and sign-ins remove those variables.
#[test]
fn inherited_credentials_never_reach_a_launch() {
    use juggler_adapter::{HarnessAdapter, LaunchMode};
    use juggler_adapters::ClaudeCodeAdapter;
    use juggler_adapters::claude_code::INHERITED_CREDENTIAL_KEYS;
    use juggler_core::model::{Account, AccountId, HarnessId, Profile, ProfileId, SettingsOverlay};
    let dir = tempfile::tempdir().unwrap();
    let account = Account {
        id: AccountId::new("a"),
        harness: HarnessId::new("claude-code"),
        label: "A".into(),
        plan: None,
        state_dir: dir.path().to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    };
    let profile = Profile {
        id: ProfileId::new("p"),
        name: "Main".into(),
        account: AccountId::new("a"),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    };
    let adapter = ClaudeCodeAdapter::with_bin("/bin/true");
    let spec = adapter
        .launch_spec(&account, &profile, dir.path(), &LaunchMode::Interactive)
        .unwrap();
    let login = adapter.login_spec(dir.path()).unwrap();
    for key in INHERITED_CREDENTIAL_KEYS {
        assert_eq!(
            spec.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
        assert_eq!(
            login.env.get(key).map(String::as_str),
            Some(ENV_UNSET),
            "{key}"
        );
    }
}
