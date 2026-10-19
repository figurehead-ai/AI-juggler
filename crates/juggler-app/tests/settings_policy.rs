//! Wireframe 26 (Settings › the policies the user owns): notification
//! quiet hours and digest mutes, and the cleanup default for new
//! Accounts — off unless the user chooses it, applied only to Accounts
//! created after the choice, and never extended to crash tidying.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::settings_policy::{self, NotificationPrefsInfo};
use juggler_app::state::AppState;

fn app_state(dir: &Path) -> Arc<AppState> {
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(
        &dir.join("no-claude"),
    )));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

#[test]
fn notification_prefs_default_to_the_anti_fatigue_hours_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let prefs = settings_policy::notification_prefs_get_impl(&state).unwrap();
    assert_eq!((prefs.quiet_start, prefs.quiet_end), (22, 7));
    assert!(prefs.digest_muted_accounts.is_empty());

    let saved = settings_policy::notification_prefs_set_impl(
        &state,
        &NotificationPrefsInfo {
            quiet_start: 23,
            quiet_end: 6,
            digest_muted_accounts: vec!["acc-b".into(), "acc-a".into(), "acc-a".into()],
        },
    )
    .unwrap();
    assert_eq!((saved.quiet_start, saved.quiet_end), (23, 6));
    assert_eq!(saved.digest_muted_accounts, ["acc-a", "acc-b"]);
    let storage = state.storage.lock().unwrap();
    let quiet = settings_policy::quiet_hours(&storage);
    assert!(quiet.contains(23) && quiet.contains(5) && !quiet.contains(6) && !quiet.contains(22));
    drop(storage);

    let refused = settings_policy::notification_prefs_set_impl(
        &state,
        &NotificationPrefsInfo {
            quiet_start: 24,
            quiet_end: 6,
            digest_muted_accounts: vec![],
        },
    )
    .unwrap_err();
    assert!(refused.to_string().contains("0 to 23"), "{refused}");
    assert_eq!(
        settings_policy::notification_prefs_get_impl(&state)
            .unwrap()
            .quiet_start,
        23,
        "a refused save changes nothing"
    );
}

#[test]
fn the_cleanup_default_is_off_unless_chosen_and_only_reaches_new_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    assert!(!settings_policy::cleanup_default_get_impl(&state).unwrap());

    let before = commands::create_account(&state, CLAUDE_CODE_ID, "Before", None).unwrap();
    let tiers = commands::cleanup_tiers_get_arc(&state, &before.id).unwrap();
    assert!(
        !tiers.tier1_opt_in && !tiers.tier2_opt_in,
        "D-08: off by default"
    );

    assert!(settings_policy::cleanup_default_set_impl(&state, true).unwrap());
    let after = commands::create_account(&state, CLAUDE_CODE_ID, "After", None).unwrap();
    let tiers = commands::cleanup_tiers_get_arc(&state, &after.id).unwrap();
    assert!(
        tiers.tier1_opt_in,
        "the user's default applies to a new Account"
    );
    assert!(!tiers.tier2_opt_in, "crash tidying is never a default");
    assert!(after.cleanup_opt_in, "the account row mirrors Tier 1");

    let unchanged = commands::cleanup_tiers_get_arc(&state, &before.id).unwrap();
    assert!(
        !unchanged.tier1_opt_in,
        "an existing Account keeps its own choice"
    );

    assert!(!settings_policy::cleanup_default_set_impl(&state, false).unwrap());
    let later = commands::create_account(&state, CLAUDE_CODE_ID, "Later", None).unwrap();
    assert!(
        !commands::cleanup_tiers_get_arc(&state, &later.id)
            .unwrap()
            .tier1_opt_in
    );
}
