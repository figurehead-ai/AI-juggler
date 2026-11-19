//! AJ-E3210 (F-rc3c-3, D-135): a Claude app profile and its CLI share one
//! Claude Code folder that holds two sign-ins. Each App is on its own:
//! the desktop App on its records' sign-in (D-63 as amended), the CLI App
//! on the folder's CLI sign-in, moved only by a read a CLI wrote. A change
//! is recorded once and told once. (Acceptance tests 2, 3, 4 and 8 of
//! `docs/research/split-sign-in-folder.md`; test 1 is
//! `stored_accounts.rs`'s.)

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
use juggler_app::features::subscriptions::{SubscriptionInfo, list_with_apps_impl};
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

fn state_json(subject: &str, org: &str) -> String {
    format!(
        r#"{{"oauthAccount":{{"accountUuid":"{subject}","organizationUuid":"{org}","emailAddress":"you@example.invalid"}}}}"#
    )
}

/// A Claude Code folder signed in as `(subject, org)`; `cli`: with its own
/// CLI credential (its stand-in: AI Juggler only checks it's there).
fn folder(
    state: &Arc<AppState>,
    name: &str,
    signed_in: (&str, &str),
    cli: bool,
) -> (String, PathBuf) {
    let made =
        juggler_app::features::account::create_account(state, "claude-code", name, None).unwrap();
    let dir = commands::find_account_arc(state, &made.id)
        .unwrap()
        .state_dir;
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        state_json(signed_in.0, signed_in.1),
    )
    .unwrap();
    if cli {
        std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
        // The user's CLI App (one with a terminal session is listed; here:
        // named).
        state
            .storage
            .lock()
            .unwrap()
            .apps()
            .upsert_cli("claude-code", &made.id, 1)
            .unwrap();
        juggler_app::features::apps::rename_impl(
            state,
            &format!("cli:{}", made.id),
            Some(&format!("{name} CLI")),
        )
        .unwrap();
    }
    (made.id, dir)
}

fn set_time(path: &Path, at_s: u64) {
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(at_s);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(at)
        .unwrap();
}

/// A Code record of the profile's under sign-in `(subject, org)`, changed
/// at `at_s`.
fn record(instance: &Path, (subject, org): (&str, &str), id: &str, at_s: u64) {
    let dir = instance.join(format!("claude-code-sessions/{subject}/{org}"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("local_{id}.json"));
    std::fs::write(&path, "{}").unwrap();
    set_time(&path, at_s);
}

/// The Claude app profile on folder `folder`, its instance `instance`.
fn profile_app(state: &Arc<AppState>, instance: &Path, folder: &str) -> String {
    std::fs::create_dir_all(instance).unwrap();
    let app = juggler_core::storage::App {
        id: juggler_core::storage::desktop_app_id("claude-app", instance.to_str()),
        kind: "desktop".into(),
        tool: "claude-code".into(),
        host_app_id: Some("claude-app".into()),
        instance_dir: instance.to_str().map(str::to_owned),
        folder_id: Some(folder.to_owned()),
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
        .put_desktop(&app)
        .unwrap();
    app.id
}

/// A run in the folder: a transcript whose last line names `entrypoint`,
/// changed at `at_s`.
fn run(folder_dir: &Path, id: &str, entrypoint: &str, at_s: u64) {
    let dir = folder_dir.join("projects/-w-p");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{id}.jsonl"));
    std::fs::write(
        &path,
        format!(r#"{{"type":"user","sessionId":"{id}","entrypoint":"{entrypoint}"}}"#),
    )
    .unwrap();
    set_time(&path, at_s);
}

/// The Account (row) App `app` is listed under.
fn account_of_app<'a>(listed: &'a [SubscriptionInfo], app: &str) -> Option<&'a SubscriptionInfo> {
    listed.iter().find(|account| {
        account
            .apps
            .as_ref()
            .is_some_and(|apps| apps.iter().any(|one| one == app))
    })
}

/// The Account (row) folder `folder` is grouped under.
fn account_of_folder<'a>(listed: &'a [SubscriptionInfo], folder: &str) -> &'a SubscriptionInfo {
    listed
        .iter()
        .find(|account| account.profiles.iter().any(|p| p.id == folder))
        .unwrap()
}

const TEAM: (&str, &str) = ("subject-a", "org-team");
const PERSONAL: (&str, &str) = ("subject-a", "org-personal");
const OTHER: (&str, &str) = ("subject-b", "org-other");

fn account_id(signed: (&str, &str)) -> String {
    juggler_app::features::subscriptions::app_accounts::account_id_of(
        "anthropic",
        signed.0,
        Some(signed.1),
    )
}

/// The rc3c replica: aij-test's folder holds the Team seat's CLI sign-in,
/// its Claude app profile is on the personal plan (as the personal
/// folder), and the app wrote its own sign-in into the folder last.
struct Split {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    shared: String,
    shared_dir: PathBuf,
    personal: String,
    instance: PathBuf,
    desktop: String,
}

fn split() -> Split {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (shared, shared_dir) = folder(&state, "aij-test", TEAM, true);
    let (personal, _) = folder(&state, "Personal", PERSONAL, true);
    let instance = dir.path().join("ClaudeProfiles/aij-test");
    record(&instance, PERSONAL, "1", 1_000);
    let desktop = profile_app(&state, &instance, &shared);
    // A terminal run on the Team seat, then the app's Code tab.
    run(
        &shared_dir,
        "00000000-0000-4000-8000-00000000000a",
        "cli",
        1_000,
    );
    list_with_apps_impl(&state).unwrap();
    run(
        &shared_dir,
        "00000000-0000-4000-8000-00000000000b",
        "claude-desktop",
        2_000,
    );
    std::fs::write(
        shared_dir.join(".claude.json"),
        state_json(PERSONAL.0, PERSONAL.1),
    )
    .unwrap();
    Split {
        _dir: dir,
        state,
        shared,
        shared_dir,
        personal,
        instance,
        desktop,
    }
}

fn changes(state: &Arc<AppState>, account: &str) -> usize {
    state
        .storage
        .lock()
        .unwrap()
        .account_audit()
        .list_for(&juggler_core::model::AccountId::new(account))
        .unwrap()
        .iter()
        .filter(|row| row.action == "sign_in_moved" && row.actor == "sign-in read")
        .count()
}

fn told(state: &Arc<AppState>) -> u64 {
    state
        .storage
        .lock()
        .unwrap()
        .notices()
        .earlier_kinds_since(0)
        .unwrap()
        .into_iter()
        .find(|(kind, _)| kind == "account-changed")
        .map_or(0, |(_, count)| count)
}

/// Acceptance test 2: one read lists the desktop App under the personal
/// plan (its records say so) and the CLI App under the Team seat.
#[test]
fn each_app_is_listed_under_its_own_sign_ins_account() {
    let split = split();
    let listed = list_with_apps_impl(&split.state).unwrap();
    let team = account_id(TEAM);
    let personal = account_id(PERSONAL);
    let cli = format!("cli:{}", split.shared);
    assert_eq!(
        account_of_app(&listed, &cli).and_then(|a| a.account_id.clone()),
        Some(team.clone()),
        "the CLI App stays on the Team seat"
    );
    assert_eq!(
        account_of_app(&listed, &split.desktop).and_then(|a| a.account_id.clone()),
        Some(personal.clone()),
        "the desktop App is on its own sign-in's"
    );
    // Its folder is the CLI's: grouped (and its readings) under the Team
    // seat; the personal folder's group holds the desktop App too.
    assert_eq!(
        account_of_folder(&listed, &split.shared).account_id,
        Some(team)
    );
    let plan = account_of_folder(&listed, &split.personal);
    assert_eq!(plan.account_id.as_ref(), Some(&personal));
    assert!(plan.apps.as_ref().unwrap().contains(&split.desktop));
    // Neither moves when the other runs again: the app's run again.
    run(
        &split.shared_dir,
        "00000000-0000-4000-8000-00000000000c",
        "claude-desktop",
        3_000,
    );
    let again = list_with_apps_impl(&split.state).unwrap();
    assert_eq!(
        account_of_app(&again, &cli).and_then(|a| a.account_id.clone()),
        Some(account_id(TEAM))
    );
    assert_eq!(told(&split.state), 0, "nothing moved: nothing told");
}

/// Acceptance test 2's other half, the stored App: its `account_id` is
/// its own, the one every screen reads by App.
#[test]
fn the_apps_list_carries_each_apps_own_account() {
    let split = split();
    list_with_apps_impl(&split.state).unwrap();
    let apps = juggler_app::features::apps::list_impl(&split.state).unwrap();
    let of = |id: &str| {
        apps.iter()
            .find(|app| app.id == id)
            .unwrap()
            .account_id
            .clone()
    };
    assert_eq!(of(&split.desktop), Some(account_id(PERSONAL)));
    assert_eq!(of(&format!("cli:{}", split.shared)), Some(account_id(TEAM)));
}

/// Acceptance test 3: a folder used only through the Claude app (no CLI
/// credential) follows the app's sign-in when it changes, and its CLI App
/// reads "Not signed in"; it is never stuck on the old Account.
#[test]
fn an_app_only_folder_follows_the_app_and_its_cli_isnt_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    let (only, only_dir) = folder(&state, "b-work", PERSONAL, false);
    let instance = dir.path().join("ClaudeProfiles/b-work");
    record(&instance, PERSONAL, "1", 1_000);
    let desktop = profile_app(&state, &instance, &only);
    run(
        &only_dir,
        "00000000-0000-4000-8000-00000000000a",
        "claude-desktop",
        1_000,
    );
    let listed = list_with_apps_impl(&state).unwrap();
    let out = account_of_folder(&listed, &only);
    assert_eq!(out.state, "signed_out", "its CLI has no sign-in here");
    assert_eq!(out.display_name, "Not signed in");
    assert_eq!(
        account_of_app(&listed, &desktop).and_then(|a| a.account_id.clone()),
        Some(account_id(PERSONAL))
    );
    assert!(
        account_of_app(&listed, &desktop)
            .unwrap()
            .profiles
            .is_empty(),
        "an Account only the desktop App is on is listed on its own"
    );

    // The app signs in to another Account and runs a Code session there.
    record(&instance, OTHER, "2", 2_000);
    std::fs::write(only_dir.join(".claude.json"), state_json(OTHER.0, OTHER.1)).unwrap();
    let listed = list_with_apps_impl(&state).unwrap();
    assert_eq!(
        account_of_app(&listed, &desktop).and_then(|a| a.account_id.clone()),
        Some(account_id(OTHER)),
        "it follows the app's sign-in"
    );
    assert_eq!(account_of_folder(&listed, &only).state, "signed_out");
    assert_eq!(changes(&state, &account_id(OTHER)), 1);
    assert_eq!(told(&state), 1, "told once");
}

/// Acceptance test 4 (the shared folder): a CLI that signs in as another
/// Account still moves the CLI App, whether or not it's the app's.
#[test]
fn a_cli_that_signs_in_again_still_moves_its_app() {
    let split = split();
    let cli = format!("cli:{}", split.shared);
    // A terminal `claude` there signs in as another Account (no App on it).
    run(
        &split.shared_dir,
        "00000000-0000-4000-8000-00000000000d",
        "cli",
        3_000,
    );
    std::fs::write(
        split.shared_dir.join(".claude.json"),
        state_json(OTHER.0, OTHER.1),
    )
    .unwrap();
    let listed = list_with_apps_impl(&split.state).unwrap();
    assert_eq!(
        account_of_app(&listed, &cli).and_then(|a| a.account_id.clone()),
        Some(account_id(OTHER))
    );
    // Then as the app's own, from a terminal run (the newest run is a CLI's).
    run(
        &split.shared_dir,
        "00000000-0000-4000-8000-00000000000e",
        "cli",
        4_000,
    );
    std::fs::write(
        split.shared_dir.join(".claude.json"),
        state_json(PERSONAL.0, PERSONAL.1),
    )
    .unwrap();
    let listed = list_with_apps_impl(&split.state).unwrap();
    assert_eq!(
        account_of_app(&listed, &cli).and_then(|a| a.account_id.clone()),
        Some(account_id(PERSONAL))
    );
    assert_eq!(
        account_of_app(&listed, &split.desktop).and_then(|a| a.account_id.clone()),
        Some(account_id(PERSONAL)),
        "the desktop App never moved"
    );
}

/// Acceptance test 8: an Account change writes one audit row and one
/// notice; a second read with no change writes neither.
#[test]
fn a_change_is_recorded_once_and_told_once() {
    let split = split();
    let other = account_id(OTHER);
    list_with_apps_impl(&split.state).unwrap();
    assert_eq!(told(&split.state), 0, "the app's write is no change");
    run(
        &split.shared_dir,
        "00000000-0000-4000-8000-00000000000d",
        "cli",
        3_000,
    );
    std::fs::write(
        split.shared_dir.join(".claude.json"),
        state_json(OTHER.0, OTHER.1),
    )
    .unwrap();
    list_with_apps_impl(&split.state).unwrap();
    assert_eq!(changes(&split.state, &other), 1, "one row in its history");
    assert_eq!(told(&split.state), 1, "one notice");
    let title = split
        .state
        .storage
        .lock()
        .unwrap()
        .notices()
        .earlier(None, 10)
        .unwrap()
        .into_iter()
        .find(|notice| notice.kind == "account-changed")
        .unwrap()
        .title;
    assert!(
        title.starts_with("Claude Code CLI · aij-test CLI is signed in to ")
            || title.starts_with("aij-test CLI is signed in to "),
        "{title}"
    );
    assert!(!title.contains("acct:"), "never an id: {title}");
    // Read again: nothing changed, nothing written.
    list_with_apps_impl(&split.state).unwrap();
    list_with_apps_impl(&split.state).unwrap();
    assert_eq!(changes(&split.state, &other), 1);
    assert_eq!(told(&split.state), 1);
    let _ = &split.instance;
}
