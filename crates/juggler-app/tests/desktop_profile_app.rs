//! rc22 (AJ-E3306, walk A3): a Claude app profile's session is that
//! profile's, and only that App counts it.
//!
//! Adding a Claude app profile brings in its Claude Code folder (the
//! profile launcher's `ClaudeCodeProfiles/<name>`), where its Code
//! sessions run. Before: every folder was also a CLI App ("Claude Code ·
//! Work Max", made by no one: D-84), the session's App resolved to it,
//! and the profile's Overview said "No sessions yet" beside a Sessions
//! count of 1 on that second App.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test: a failed step is its failure"
)]

use std::path::PathBuf;
use std::sync::Arc;

use juggler_app::features::apps::list_impl;
use juggler_app::features::relations::{self, EntityKind, EntityRef};
use juggler_app::features::sessions::rows::{SessionsQuery, sessions_page_impl};
use juggler_app::paging::PageRequest;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};
use juggler_core::storage::{App, cli_app_id};

/// The profile's own folder (never read: a name in a row).
const PROFILE_DIR: &str = "/Users/you/Library/Application Support/ClaudeProfiles/Work Max";

fn folder(id: &str, root: &std::path::Path) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
        label: id.to_owned(),
        plan: None,
        state_dir: root.join(id),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: true,
    }
}

fn session(id: &str, recipe: &str, kind: InstanceKind) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(recipe),
        machine: MachineId::new("mac-here"),
        kind,
        project: None,
        cwd: PathBuf::from("/nowhere"),
        status: SessionStatus::Finished,
        started_at: Timestamp::from_millis(1_760_000_000_000),
        last_activity: Some(Timestamp::from_millis(1_760_000_001_000)),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

/// A profile the user added, on folder `work-max`, with one session AI
/// Juggler started in it (as the product records one: no App stored on
/// the session); and a plain CLI folder `plain` with a terminal session.
fn fixture() -> (tempfile::TempDir, Arc<AppState>, String) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState::init(dir.path().join("data")).unwrap());
    let profile = juggler_core::storage::desktop_app_id("claude-app", Some(PROFILE_DIR));
    {
        let storage = state.storage.lock().unwrap();
        for (id, recipe) in [("work-max", "rec-wm"), ("plain", "rec-plain")] {
            storage.accounts().insert(&folder(id, dir.path())).unwrap();
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new(recipe),
                    name: "Main".into(),
                    account: AccountId::new(id),
                    settings_overlay: SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
        }
        storage
            .apps()
            .put_desktop(&App {
                id: profile.clone(),
                kind: "desktop".into(),
                tool: "claude-code".into(),
                host_app_id: Some("claude-app".into()),
                instance_dir: Some(PROFILE_DIR.into()),
                folder_id: Some("work-max".into()),
                label: None,
                hidden: false,
                keep_alive: false,
                paused_json: None,
                launch_json: Some(
                    serde_json::json!({
                        "name": "Claude",
                        "instance": "Work Max",
                        "bundle": "/Applications/Claude.app",
                        "given": PROFILE_DIR,
                    })
                    .to_string(),
                ),
                sessions_json: None,
                first_seen: 1,
                last_seen: 1,
                added: true,
            })
            .unwrap();
        storage.apps().add(&profile).unwrap();
        let sessions = storage.sessions();
        sessions
            .insert(&session("s-app", "rec-wm", InstanceKind::Desktop))
            .unwrap();
        sessions
            .insert(&session("s-term", "rec-plain", InstanceKind::Terminal))
            .unwrap();
    }
    (dir, state, profile)
}

fn app(id: &str) -> EntityRef {
    EntityRef {
        kind: EntityKind::App,
        id: id.to_owned(),
    }
}

/// The ids of App `id`'s Sessions list.
fn listed(state: &Arc<AppState>, id: &str) -> Vec<String> {
    let query = SessionsQuery {
        app_id: Some(id.to_owned()),
        ..SessionsQuery::default()
    };
    sessions_page_impl(
        state,
        &query,
        &PageRequest {
            limit: Some(50),
            cursor: None,
        },
    )
    .unwrap()
    .items
    .into_iter()
    .map(|row| row.session.expect("AI Juggler's session").id)
    .collect()
}

fn listed_apps(state: &Arc<AppState>) -> Vec<String> {
    list_impl(state)
        .unwrap()
        .into_iter()
        .map(|a| a.id)
        .collect()
}

#[test]
fn a_desktop_profiles_session_is_that_profiles_and_counted_once() {
    let (_dir, state, profile) = fixture();
    // Its App is the profile: its count and its list say so.
    assert_eq!(
        relations::counts(&state, &app(&profile))
            .unwrap()
            .sessions
            .all,
        1
    );
    assert_eq!(listed(&state, &profile), ["s-app"]);
    // No other App counts it.
    let folder_app = cli_app_id("work-max");
    assert_eq!(
        relations::counts(&state, &app(&folder_app))
            .unwrap()
            .sessions
            .all,
        0
    );
    assert!(listed(&state, &folder_app).is_empty());
    // A pool's App rule and the Floor read the same App.
    let facts = state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .pool_facts()
        .unwrap();
    let of = |id: &str| {
        facts
            .iter()
            .find(|f| f.session.as_str() == id)
            .and_then(|f| f.app.clone())
    };
    assert_eq!(of("s-app").as_deref(), Some(profile.as_str()));
    assert_eq!(of("s-term"), Some(cli_app_id("plain")));
    // A plain CLI folder's terminal session stays its CLI App's.
    assert_eq!(listed(&state, &cli_app_id("plain")), ["s-term"]);
}

#[test]
fn a_desktop_profiles_folder_is_no_cli_app_until_one_runs_in_a_terminal() {
    let (_dir, state, profile) = fixture();
    let apps = listed_apps(&state);
    assert!(apps.contains(&profile), "{apps:?}");
    assert!(apps.contains(&cli_app_id("plain")), "{apps:?}");
    assert!(
        !apps.contains(&cli_app_id("work-max")),
        "D-84: nobody added a CLI App on the profile's folder: {apps:?}"
    );
    // A terminal session AI Juggler ran on that folder: now it is one,
    // and that session is its; the profile's stays the profile's.
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&session("s-cli", "rec-wm", InstanceKind::Terminal))
        .unwrap();
    relations::changed(&state, relations::Change::SessionsUpdated);
    assert!(listed_apps(&state).contains(&cli_app_id("work-max")));
    assert_eq!(listed(&state, &cli_app_id("work-max")), ["s-cli"]);
    assert_eq!(listed(&state, &profile), ["s-app"]);
    assert_eq!(
        relations::counts(&state, &app(&profile))
            .unwrap()
            .sessions
            .all,
        1
    );
}
