//! D-62 (AJ-E3202): reading the Accounts list stores each sign-in as an
//! Account, once: two folders signed in to one plan point at one Account,
//! a team seat on the same email is another, and a folder signed out
//! points at none. Ids never reach the row (only their hash).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::ClaudeCodeAdapter;
use juggler_app::commands;
use juggler_app::features::subscriptions::list_impl;
use juggler_app::state::AppState;

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn app_state(dir: &Path) -> Arc<AppState> {
    let bin: PathBuf = dir.join("claude");
    stand_in::write(&bin, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(bin)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn folder(state: &Arc<AppState>, name: &str, signed_in: Option<(&str, &str)>) -> String {
    let made = commands::create_account(state, "claude-code", name, None).unwrap();
    let dir = commands::find_account_arc(state, &made.id)
        .unwrap()
        .state_dir;
    if let Some((subject, org)) = signed_in {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"{subject}","organizationUuid":"{org}","emailAddress":"you@example.invalid"}}}}"#
            ),
        )
        .unwrap();
    }
    made.id
}

#[test]
fn one_plan_on_two_folders_is_one_stored_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", Some(("subject-a", "org-personal")));
    let side = folder(&state, "Side", Some(("subject-a", "org-personal")));
    let team = folder(&state, "Team", Some(("subject-a", "org-team")));
    let out = folder(&state, "Out", None);

    let listed = list_impl(&state).unwrap();
    let stored = state
        .storage
        .lock()
        .unwrap()
        .subscriptions()
        .list()
        .unwrap();
    assert_eq!(stored.len(), 2, "the plan and the team seat: {stored:?}");
    assert!(stored.iter().all(|account| account.vendor == "anthropic"));
    assert!(
        stored
            .iter()
            .all(|account| !account.identity_hash.contains("subject")),
        "only a hash of the sign-in is kept"
    );
    let folders: std::collections::BTreeMap<String, Option<String>> = state
        .storage
        .lock()
        .unwrap()
        .subscriptions()
        .folders()
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(folders[&work], folders[&side]);
    assert!(folders[&work].is_some());
    assert_ne!(folders[&team], folders[&work]);
    assert_eq!(folders[&out], None);

    let plan = listed
        .iter()
        .find(|account| account.profiles.iter().any(|p| p.id == work))
        .unwrap();
    assert_eq!(plan.account_id, folders[&work]);

    // Read again: nothing new is made.
    list_impl(&state).unwrap();
    assert_eq!(
        state
            .storage
            .lock()
            .unwrap()
            .subscriptions()
            .list()
            .unwrap(),
        stored
    );
}
