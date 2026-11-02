//! D-62 (AJ-E3202 req. 4): plan, renewal and fee are set once, on the
//! Account. Reading the Accounts moves what its folders agree on; where
//! they disagree, the Account asks once (A9) and each folder keeps its
//! own until then; once chosen, every reader (the folders' rows, the
//! runway's renewal, the cost view's fee) reads the Account, and the
//! folders' own values are kept but no longer written.

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
use juggler_app::features::subscriptions::{SubscriptionInfo, list_impl, terms};
use juggler_app::state::AppState;
use juggler_core::storage::fee_key;

#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

fn app_state(dir: &Path) -> Arc<AppState> {
    let bin: PathBuf = dir.join("claude");
    stand_in::write(&bin, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin(bin)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

/// A Claude Code folder signed in to `subject`, with its own plan,
/// renewal and fee as 0.3 kept them.
fn folder(
    state: &Arc<AppState>,
    name: &str,
    subject: &str,
    plan: &str,
    renewal: &str,
    fee: &str,
) -> String {
    let made = commands::create_account(state, "claude-code", name, Some(plan.into())).unwrap();
    // Before its sign-in is read it has no Account: set on the folder.
    commands::account_set_renewal_arc(state, &made.id, Some(renewal)).unwrap();
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(&fee_key(&made.id), fee)
        .unwrap();
    let dir = commands::find_account_arc(state, &made.id)
        .unwrap()
        .state_dir;
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        format!(
            r#"{{"oauthAccount":{{"accountUuid":"{subject}","organizationUuid":"org-1","emailAddress":"you@example.invalid"}}}}"#
        ),
    )
    .unwrap();
    made.id
}

fn account_of<'a>(listed: &'a [SubscriptionInfo], folder: &str) -> &'a SubscriptionInfo {
    listed
        .iter()
        .find(|account| account.profiles.iter().any(|p| p.id == folder))
        .unwrap()
}

fn renewal_of(state: &Arc<AppState>, folder: &str) -> Option<String> {
    commands::accounts_info(state)
        .unwrap()
        .into_iter()
        .find(|info| info.id == folder)
        .unwrap()
        .renewal_date
}

#[test]
fn agreed_terms_move_and_a_disagreement_is_chosen_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", "subject-a", "max", "2027-03-12", "200");
    let work_2 = folder(&state, "Work 2", "subject-a", "Max", "2027-04-03", "200");

    // Read once: plan and fee agree and move; the renewal waits.
    let listed = list_impl(&state).unwrap();
    let account = account_of(&listed, &work);
    let id = account.account_id.clone().unwrap();
    assert_eq!(account.plan.as_deref(), Some("max"));
    assert_eq!(account.fee_usd, Some(200.0));
    assert_eq!(account.renewal_date, None);
    assert_eq!(account.differs, ["renewal"]);
    assert_eq!(account.choices.len(), 1);
    // AJ-E3206: one colour for the Account, the same on both its folders.
    assert_eq!(account.colour_slot, Some(0));
    let folders = juggler_app::commands::accounts_info(&state).unwrap();
    assert!(
        folders
            .iter()
            .all(|row| row.account_colour_slot == account.colour_slot),
        "both folders wear their Account's colour"
    );
    let choice = &account.choices[0];
    assert_eq!(choice.term, "renewal");
    let options: Vec<(&str, &[String])> = choice
        .options
        .iter()
        .map(|option| (option.value.as_str(), option.folders.as_slice()))
        .collect();
    assert_eq!(
        options,
        [
            ("2027-03-12", std::slice::from_ref(&work)),
            ("2027-04-03", std::slice::from_ref(&work_2)),
        ]
    );
    // Until chosen, each folder keeps its own date: nothing is lost.
    assert_eq!(renewal_of(&state, &work).as_deref(), Some("2027-03-12"));
    assert_eq!(renewal_of(&state, &work_2).as_deref(), Some("2027-04-03"));

    // Chosen once: stored on the Account; never asked again.
    terms::set_impl(&state, &id, "renewal", Some("2027-04-03")).unwrap();
    let listed = list_impl(&state).unwrap();
    let account = account_of(&listed, &work);
    assert!(account.choices.is_empty() && account.differs.is_empty());
    assert_eq!(account.renewal_date.as_deref(), Some("2027-04-03"));
    // Each folder still shows its own value (kept), and reads the Account's.
    assert_eq!(
        account.profiles[0].renewal_date.as_deref(),
        Some("2027-03-12")
    );
    assert_eq!(renewal_of(&state, &work).as_deref(), Some("2027-04-03"));

    // The runway's renewal and the cost view's fee read the Account.
    {
        let storage = state.storage.lock().unwrap();
        let row = storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(work.clone()))
            .unwrap();
        let display = commands::quota_display_inner(&storage, &row, 0);
        assert!(
            display.renewal.label.contains("2027-04-03"),
            "{:?}",
            display.renewal
        );
    }

    // The older per-folder commands write the Account now, not the folder.
    commands::account_set_renewal_arc(&state, &work_2, Some("2027-05-01")).unwrap();
    assert_eq!(renewal_of(&state, &work).as_deref(), Some("2027-05-01"));
    let own = state
        .storage
        .lock()
        .unwrap()
        .terms()
        .own(&work_2)
        .unwrap()
        .unwrap();
    assert_eq!(own.renewal_date.as_deref(), Some("2027-04-03"));
    terms::set_impl(&state, &id, "fee", Some("100")).unwrap();
    assert_eq!(
        commands::cost_rollups_impl(&state, Some(&work))
            .subscription
            .flat_fee_usd,
        Some(100.0)
    );
    assert_eq!(
        state
            .storage
            .lock()
            .unwrap()
            .app_settings()
            .get(&fee_key(&work))
            .unwrap()
            .as_deref(),
        Some("200"),
        "the folder's own fee is kept as it was"
    );
}

#[test]
fn what_the_user_enters_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", "subject-a", "max", "2027-03-12", "200");
    let id = account_of(&list_impl(&state).unwrap(), &work)
        .account_id
        .clone()
        .unwrap();
    for (term, bad) in [
        ("renewal", "12 Nov"),
        ("renewal", "2027-02-30"),
        ("fee", "lots"),
        ("fee", "-5"),
        ("plan", &"x".repeat(41)),
    ] {
        assert!(
            terms::set_impl(&state, &id, term, Some(bad)).is_err(),
            "{term} {bad:?}"
        );
    }
    assert!(terms::set_impl(&state, &id, "colour", Some("red")).is_err());
    assert!(terms::set_impl(&state, "acct:nope:1", "plan", Some("Pro")).is_err());
    // Blank clears.
    terms::set_impl(&state, &id, "plan", Some("  ")).unwrap();
    assert_eq!(account_of(&list_impl(&state).unwrap(), &work).plan, None);
}
