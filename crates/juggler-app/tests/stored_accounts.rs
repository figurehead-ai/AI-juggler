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
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};
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
    let made =
        juggler_app::features::account::create_account(state, "claude-code", name, None).unwrap();
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

    // Named once: every read says it; blank is its default again.
    let id = folders[&work].clone().unwrap();
    juggler_app::features::subscriptions::rename_impl(&state, &id, Some(" Work plan ")).unwrap();
    let named = list_impl(&state).unwrap();
    let plan = named
        .iter()
        .find(|a| a.account_id.as_deref() == Some(id.as_str()))
        .unwrap();
    assert_eq!(plan.name.as_deref(), Some("Work plan"));
    juggler_app::features::subscriptions::rename_impl(&state, &id, Some("  ")).unwrap();
    assert!(list_impl(&state).unwrap().iter().all(|a| a.name.is_none()));
    assert!(
        juggler_app::features::subscriptions::rename_impl(&state, "acct:nope:1", Some("x"))
            .is_err()
    );
}

fn state_file(state: &Arc<AppState>, folder: &str) -> PathBuf {
    commands::find_account_arc(state, folder)
        .unwrap()
        .state_dir
        .join(".claude.json")
}

fn links(state: &Arc<AppState>) -> std::collections::BTreeMap<String, Option<String>> {
    state
        .storage
        .lock()
        .unwrap()
        .subscriptions()
        .folders()
        .unwrap()
        .into_iter()
        .collect()
}

fn row_of<'a>(
    listed: &'a [juggler_app::features::subscriptions::SubscriptionInfo],
    folder: &str,
) -> (
    &'a juggler_app::features::subscriptions::SubscriptionInfo,
    &'a juggler_app::features::subscriptions::SubscriptionProfileInfo,
) {
    listed
        .iter()
        .find_map(|account| {
            account
                .profiles
                .iter()
                .find(|p| p.id == folder)
                .map(|p| (account, p))
        })
        .unwrap()
}

/// 0.4.1 (main's ruling): a sign-in that couldn't be checked (here a
/// state file caught mid-write) never drops a folder from its Account. It
/// stays linked, says "couldn't check", and the next good read clears it.
#[test]
fn a_sign_in_that_couldnt_be_checked_keeps_its_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", Some(("subject-a", "org-a")));
    let side = folder(&state, "Side", Some(("subject-a", "org-a")));
    list_impl(&state).unwrap();
    let before = links(&state);
    let account = before[&work].clone().unwrap();

    std::fs::write(state_file(&state, &work), "{\"oauthAcc").unwrap();
    let listed = list_impl(&state).unwrap();
    assert_eq!(links(&state), before, "the stored link stays");
    let (row, profile) = row_of(&listed, &work);
    assert_eq!(row.account_id.as_deref(), Some(account.as_str()));
    assert_eq!(row.state, "signed_in");
    assert!(row.profiles.iter().any(|p| p.id == side), "beside its twin");
    assert_eq!(
        profile.unchecked.as_deref(),
        Some("Claude Code's state file couldn't be read")
    );
    let names = juggler_app::features::subscriptions::folder_accounts(&state);
    assert_eq!(names[&work].id, account, "notices still name its Account");

    // The next read checks again; a good one clears the mark.
    std::fs::write(
        state_file(&state, &work),
        r#"{"oauthAccount":{"accountUuid":"subject-a","organizationUuid":"org-a"}}"#,
    )
    .unwrap();
    let listed = list_impl(&state).unwrap();
    assert_eq!(row_of(&listed, &work).1.unchecked, None);
    assert_eq!(links(&state), before);
}

/// Alone too: the Account it was linked to is still its Account.
#[test]
fn a_lone_folder_that_couldnt_be_checked_is_still_its_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", Some(("subject-a", "org-a")));
    list_impl(&state).unwrap();
    let account = links(&state)[&work].clone().unwrap();
    std::fs::write(state_file(&state, &work), "").unwrap();
    let listed = list_impl(&state).unwrap();
    let (row, profile) = row_of(&listed, &work);
    assert_eq!(row.account_id.as_deref(), Some(account.as_str()));
    assert_ne!(row.display_name, "Account unknown");
    assert!(profile.unchecked.is_some());
    assert_eq!(links(&state)[&work].as_deref(), Some(account.as_str()));
}

/// A folder never linked (a Codex API key's sign-in is never readable)
/// stands alone as before: keeping a link invents none.
#[test]
fn a_folder_never_linked_stays_alone_when_it_cant_be_checked() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let odd = folder(&state, "Odd", None);
    let file = state_file(&state, &odd);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "not json").unwrap();
    let listed = list_impl(&state).unwrap();
    let (row, profile) = row_of(&listed, &odd);
    assert_eq!(row.state, "unverified");
    assert_eq!(row.account_id, None);
    assert_eq!(row.profiles.len(), 1);
    assert_eq!(profile.unchecked, None, "nothing kept, nothing said twice");
    assert_eq!(links(&state)[&odd], None);
}

/// A definite answer still moves it: signed out unlinks; signed in as
/// another account moves it there.
#[test]
fn signed_out_unlinks_and_another_account_moves_it() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", Some(("subject-a", "org-a")));
    let other = folder(&state, "Other", Some(("subject-b", "org-b")));
    list_impl(&state).unwrap();
    let before = links(&state);

    std::fs::write(
        state_file(&state, &work),
        r#"{"oauthAccount":{"accountUuid":"subject-b","organizationUuid":"org-b"}}"#,
    )
    .unwrap();
    let listed = list_impl(&state).unwrap();
    assert_eq!(links(&state)[&work], before[&other], "moved to the other");
    assert!(
        row_of(&listed, &work)
            .0
            .profiles
            .iter()
            .any(|p| p.id == other)
    );

    std::fs::remove_file(state_file(&state, &work)).unwrap();
    let listed = list_impl(&state).unwrap();
    assert_eq!(links(&state)[&work], None, "signed out: in none");
    let (row, profile) = row_of(&listed, &work);
    assert_eq!(row.state, "signed_out");
    assert_eq!(profile.unchecked, None);
}

fn codex_state(dir: &Path) -> Arc<AppState> {
    let bin: PathBuf = dir.join("codex");
    stand_in::write(&bin, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(CodexAdapter::with_bin(bin)));
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn codex_auth(state: &Arc<AppState>, folder: &str, auth: &str) {
    let dir = commands::find_account_arc(state, folder).unwrap().state_dir;
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("auth.json"), auth).unwrap();
}

const CHATGPT: &str = r#"{"auth_mode":"chatgpt","tokens":{"id_token":"fixture.id.token","refresh_token":"fixture","account_id":"acct-fixture-9f2c"}}"#;
const KEY: &str = r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-fixture"}"#;

/// 0.4.1: an API key is a definite answer, not "couldn't tell": a folder
/// that leaves its ChatGPT sign-in for a key leaves that Account.
#[test]
fn a_folder_that_moves_to_an_api_key_leaves_its_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = codex_state(dir.path());
    let made =
        juggler_app::features::account::create_account(&state, "codex", "Work", None).unwrap();
    codex_auth(&state, &made.id, CHATGPT);
    list_impl(&state).unwrap();
    assert!(
        links(&state)[&made.id].is_some(),
        "linked to its ChatGPT account"
    );

    codex_auth(&state, &made.id, KEY);
    let listed = list_impl(&state).unwrap();
    assert_eq!(links(&state)[&made.id], None, "a key: in no Account");
    let (row, profile) = row_of(&listed, &made.id);
    assert_eq!(row.account_id, None);
    assert_eq!(row.state, "unverified", "said as key folders always were");
    assert_eq!(
        row.reason.as_deref(),
        Some("API-key sign-ins carry no account id")
    );
    assert_eq!(profile.unchecked, None, "no «couldn't check» for a key");
}

/// A key folder from the start stands alone, as it always did.
#[test]
fn a_key_folder_stands_alone() {
    let dir = tempfile::tempdir().unwrap();
    let state = codex_state(dir.path());
    let a = juggler_app::features::account::create_account(&state, "codex", "A", None).unwrap();
    let b = juggler_app::features::account::create_account(&state, "codex", "B", None).unwrap();
    codex_auth(&state, &a.id, KEY);
    codex_auth(&state, &b.id, KEY);
    let listed = list_impl(&state).unwrap();
    for id in [&a.id, &b.id] {
        let (row, profile) = row_of(&listed, id);
        assert_eq!(row.profiles.len(), 1, "never grouped with another key");
        assert_eq!(row.state, "unverified");
        assert_eq!(row.display_name, "Account unknown");
        assert_eq!(profile.unchecked, None);
        assert_eq!(links(&state)[id], None);
    }
}

/// 0.4.1: a folder alone in its Account that can't be checked keeps the
/// name and email its last read gave.
#[test]
fn a_lone_unchecked_folder_keeps_its_last_name_and_email() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = folder(&state, "Work", Some(("subject-a", "org-a")));
    let listed = list_impl(&state).unwrap();
    let (row, _) = row_of(&listed, &work);
    let (name, email) = (row.display_name.clone(), row.email.clone());
    assert_eq!(email.as_deref(), Some("y…@example.invalid"));

    std::fs::write(state_file(&state, &work), "{\"oauthAcc").unwrap();
    let listed = list_impl(&state).unwrap();
    let (row, profile) = row_of(&listed, &work);
    assert!(profile.unchecked.is_some());
    assert_eq!(row.display_name, name, "its name, as last read");
    assert_eq!(row.email, email, "its email, as last read");
}

/// A Claude Code folder `~/.claude-<word>` signed in as `subject`,
/// brought in where it is under `name`; `accepted`: the user saw the
/// name and kept it (the window's bring-in), else it was given without
/// them (a desktop profile's linked folder).
fn imported(state: &Arc<AppState>, dir: &Path, word: &str, name: &str, accepted: bool) -> String {
    let folder = dir.join(format!(".claude-{word}"));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"subject-a","organizationUuid":"org-a","emailAddress":"you@example.invalid"}}"#,
    )
    .unwrap();
    juggler_app::features::account::import::account_import_mode_impl(
        state,
        "claude-code",
        folder.to_str().unwrap(),
        name,
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            accepted_name: accepted,
            ..juggler_app::features::account::import::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .as_str()
    .to_owned()
}

fn name_of(state: &Arc<AppState>, folder: &str) -> String {
    let listed = list_impl(state).unwrap();
    row_of(&listed, folder).0.display_name.clone()
}

/// AJ-E3202 req. 3 (0.4.1): an Account with no team and no plan takes the
/// name all its Apps were accepted under at import ("Work", first run's
/// found step), stored, so it reads the same after a restart.
#[test]
fn the_name_accepted_at_import_names_the_account_and_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = imported(&state, dir.path(), "work", "Work", true);
    let again = imported(&state, dir.path(), "work-b", "Work", true);
    let listed = list_impl(&state).unwrap();
    let (row, profile) = row_of(&listed, &work);
    assert_eq!(row.profiles.len(), 2, "one sign-in, one Account");
    assert!(row.profiles.iter().any(|p| p.id == again));
    assert_eq!(profile.accepted_name.as_deref(), Some("Work"));
    assert_eq!(row.display_name, "Work");
    drop(listed);
    drop(state);

    let state = app_state(dir.path());
    assert_eq!(name_of(&state, &work), "Work", "kept across a restart");
}

#[test]
fn a_rename_wins_over_the_name_accepted_at_import() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = imported(&state, dir.path(), "work", "Work", true);
    assert_eq!(name_of(&state, &work), "Work");
    let id = links(&state)[&work].clone().unwrap();
    juggler_app::features::subscriptions::rename_impl(&state, &id, Some("Day job")).unwrap();
    assert_eq!(name_of(&state, &work), "Day job");
    juggler_app::features::subscriptions::rename_impl(&state, &id, None).unwrap();
    assert_eq!(
        name_of(&state, &work),
        "Work",
        "cleared: the accepted name again"
    );
}

/// One App the user never accepted a name for at import (a name given
/// without them, a folder made in AI Juggler, a desktop App on one of its
/// folders): the order it always had, here its masked email.
#[test]
fn an_app_not_accepted_at_import_leaves_the_account_its_usual_name() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = imported(&state, dir.path(), "work", "Work", true);
    assert_eq!(name_of(&state, &work), "Work");

    // A desktop App the user added, on the folder.
    let desktop = juggler_core::storage::App {
        id: juggler_core::storage::desktop_app_id("claude-app", Some("/p/work")),
        kind: "desktop".into(),
        tool: "claude-code".into(),
        host_app_id: Some("claude-app".into()),
        instance_dir: Some("/p/work".into()),
        folder_id: Some(work.clone()),
        label: None,
        hidden: false,
        keep_alive: false,
        paused_json: None,
        launch_json: None,
        sessions_json: None,
        first_seen: 1,
        added: true,
        last_seen: 1,
    };
    state
        .storage
        .lock()
        .unwrap()
        .apps()
        .put_desktop(&desktop)
        .unwrap();
    assert_eq!(name_of(&state, &work), "y…@example.invalid");

    // A second folder, its name given without the user.
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = imported(&state, dir.path(), "work", "Work", true);
    imported(&state, dir.path(), "side", "Work", false);
    assert_eq!(name_of(&state, &work), "y…@example.invalid");

    // A folder made in AI Juggler, on the same sign-in.
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let work = imported(&state, dir.path(), "work", "Work", true);
    folder(&state, "Work", Some(("subject-a", "org-a")));
    assert_eq!(name_of(&state, &work), "y…@example.invalid");
}
