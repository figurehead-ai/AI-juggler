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

/// How the invariant's worlds differ: what runs on a Claude app profile's
/// folder, and how many profiles hold it.
#[derive(Clone, Copy, Debug)]
struct World {
    /// Profiles holding the folder (two: it's no one profile's).
    profiles: usize,
    /// A conversation another app ran in a terminal, in the folder's files.
    outside_terminal: bool,
    /// AI Juggler's session in the desktop app on that folder.
    own_desktop: bool,
    /// AI Juggler's session in a terminal on that folder.
    own_terminal: bool,
    /// AI Juggler's desktop sessions launched in each profile (stored at
    /// launch).
    launched: bool,
}

/// Profile `n`'s App id in [`world`].
fn profile_id(n: usize) -> String {
    juggler_core::storage::desktop_app_id(
        "claude-app",
        Some(&format!(
            "/Users/you/Library/Application Support/ClaudeProfiles/P{n}"
        )),
    )
}

/// `world`: a Claude Code folder brought in where it is, its files holding
/// one conversation if asked, `world.profiles` added profiles on it, and
/// AI Juggler's sessions as asked.
fn world(world: World) -> (tempfile::TempDir, Arc<AppState>) {
    use juggler_adapters::ClaudeCodeAdapter;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let repo = root.join("Dev/web");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let folder = root.join("ClaudeCodeProfiles/Work Max");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("settings.json"), "{}").unwrap();
    if world.outside_terminal {
        let at = folder
            .join("projects")
            .join(repo.to_string_lossy().replace(['/', '.'], "-"));
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(
            at.join("c-1.jsonl"),
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"c-1\",\"cwd\":\"{}\",\"timestamp\":\"2026-10-21T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Fix it\"}}}}\n",
                repo.display()
            ),
        )
        .unwrap();
    }
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("data"), registry).unwrap());
    juggler_app::features::cache::register_defaults(&state);
    let id = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        "claude-code",
        folder.to_str().unwrap(),
        "Work Max",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..Default::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    {
        let storage = state.storage.lock().unwrap();
        storage.projects().touch(&repo, None).unwrap();
        for n in 0..world.profiles {
            let instance = format!("/Users/you/Library/Application Support/ClaudeProfiles/P{n}");
            let profile = profile_id(n);
            storage
                .apps()
                .put_desktop(&App {
                    id: profile.clone(),
                    kind: "desktop".into(),
                    tool: "claude-code".into(),
                    host_app_id: Some("claude-app".into()),
                    instance_dir: Some(instance.clone()),
                    folder_id: Some(id.clone()),
                    label: None,
                    hidden: false,
                    keep_alive: false,
                    paused_json: None,
                    launch_json: Some(
                        serde_json::json!({
                            "name": "Claude",
                            "instance": format!("P{n}"),
                            "bundle": "/Applications/Claude.app",
                            "given": instance,
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
        }
        let recipe = storage
            .profiles()
            .list_by_account(&AccountId::new(id.as_str()))
            .unwrap()
            .remove(0)
            .id
            .to_string();
        if world.own_desktop {
            storage
                .sessions()
                .insert(&session("s-app", &recipe, InstanceKind::Desktop))
                .unwrap();
        }
        if world.launched {
            for n in 0..world.profiles {
                let id = format!("s-in-p{n}");
                storage
                    .sessions()
                    .insert(&session(&id, &recipe, InstanceKind::Desktop))
                    .unwrap();
                storage.apps().set_session(&id, &profile_id(n)).unwrap();
            }
        }
        if world.own_terminal {
            storage
                .sessions()
                .insert(&session("s-cli", &recipe, InstanceKind::Terminal))
                .unwrap();
        }
    }
    juggler_app::features::conversations::refresh_impl(&state).unwrap();
    (dir, state)
}

/// The edge cases the A3 fix left (an outside terminal conversation on a
/// profile's folder; a folder two profiles share; a session launched in
/// one of two profiles on one folder): every session's and
/// conversation's App is in the Apps list, and one launched in a profile
/// is that profile's. Each row is in exactly one
/// App, so the listed Apps' counts add up to every row's.
#[test]
fn every_sessions_app_is_in_the_apps_list() {
    let mut unlisted = Vec::new();
    for profiles in [0, 1, 2] {
        for bits in 0..16_u8 {
            let w = World {
                profiles,
                outside_terminal: bits & 1 != 0,
                own_desktop: bits & 2 != 0,
                own_terminal: bits & 4 != 0,
                launched: bits & 8 != 0,
            };
            let (_dir, state) = world(w);
            let apps = list_impl(&state).unwrap();
            // Each Account lists exactly the listed Apps on its folders
            // (its card, its page, Usage's and Home's runway read it).
            for account in
                juggler_app::features::subscriptions::list_with_apps_impl(&state).unwrap()
            {
                let want: Vec<String> = apps
                    .iter()
                    .filter(|one| {
                        one.folder_id
                            .as_deref()
                            .is_some_and(|folder| account.profiles.iter().any(|p| p.id == folder))
                    })
                    .map(|one| one.id.clone())
                    .collect();
                if account.apps.as_ref() != Some(&want) {
                    unlisted.push(format!(
                        "{w:?}: {} lists {:?}, the Apps list {want:?}",
                        account.display_name, account.apps
                    ));
                }
            }
            let asked: Vec<EntityRef> = apps.iter().map(|one| app(&one.id)).collect();
            let counted: u64 = relations::counts_many(&state, &asked)
                .unwrap()
                .iter()
                .map(|counts| counts.sessions.all)
                .sum();
            let all = relations::counts(
                &state,
                &EntityRef {
                    kind: EntityKind::All,
                    id: String::new(),
                },
            )
            .unwrap()
            .sessions
            .all;
            let launched = if w.launched { w.profiles as u64 } else { 0 };
            let rows = u64::from(w.outside_terminal)
                + u64::from(w.own_desktop)
                + u64::from(w.own_terminal)
                + launched;
            // Each one launched in a profile is that profile's, and its
            // row names that App (the chip opens the App it names).
            let infos = juggler_app::features::sessions::read_model::session_infos_snapshot(
                &state.storage.lock().unwrap(),
            )
            .unwrap();
            for n in (0..w.profiles).filter(|_| w.launched) {
                let id = format!("s-in-p{n}");
                if !listed(&state, &profile_id(n)).contains(&id) {
                    unlisted.push(format!("{w:?}: {id} isn't P{n}'s"));
                }
                let row = infos.iter().find(|row| row.id == id).unwrap();
                if row.app_id.as_deref() != Some(profile_id(n).as_str()) {
                    unlisted.push(format!("{w:?}: {id}'s row names {:?}", row.app_id));
                }
            }
            assert_eq!(all, rows, "{w:?}: the world's rows");
            if counted != all {
                let ids: Vec<&str> = apps.iter().map(|one| one.id.as_str()).collect();
                unlisted.push(format!("{w:?}: {counted} of {all} counted by {ids:?}"));
            }
        }
    }
    assert!(
        unlisted.is_empty(),
        "a row's App isn't listed:\n{}",
        unlisted.join("\n")
    );
}

/// 0.4.3, the real case: a Claude app profile with 3 records had about 40
/// transcripts its app wrote in its folder (lab leftovers, sessions
/// deleted in the app), and its App listed and counted them all. Its
/// folder here: 2 Code tab sessions the app keeps (one archived) and a
/// script's run it adopted, 4 desktop runs it keeps no record of, and 2
/// terminal sessions. The profile's App holds exactly its 3 records; the
/// 4 are in no App (History still lists them, under the profile's chip);
/// the folder's CLI App holds the 2 terminal ones.
#[test]
fn a_desktop_profiles_sessions_are_exactly_its_own_records() {
    use juggler_adapters::ClaudeCodeAdapter;
    use juggler_app::features::sessions::facets::sessions_facets_impl;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let repo = root.join("Dev/shop");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let folder = root.join("ClaudeCodeProfiles/lab");
    let transcripts = folder
        .join("projects")
        .join(repo.to_string_lossy().replace(['/', '.'], "-"));
    std::fs::create_dir_all(&transcripts).unwrap();
    std::fs::write(folder.join("settings.json"), "{}").unwrap();
    let mut minute = 0;
    let mut transcript = |id: &str, entrypoint: &str| {
        minute += 1;
        std::fs::write(
            transcripts.join(format!("{id}.jsonl")),
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"{id}\",\"cwd\":\"{}\",\"entrypoint\":\"{entrypoint}\",\"timestamp\":\"2026-10-21T10:{minute:02}:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"Tidy it\"}}}}\n",
                repo.display()
            ),
        )
        .unwrap();
    };
    for id in ["kept", "kept-archived"] {
        transcript(id, "claude-desktop");
    }
    transcript("adopted", "sdk-cli");
    for (id, entrypoint) in [
        ("gone-1", "claude-desktop"),
        ("gone-2", "claude-desktop"),
        ("gone-3", "sdk-ts"),
        ("gone-4", "sdk-ts"),
    ] {
        transcript(id, entrypoint);
    }
    for id in ["term-1", "term-2"] {
        transcript(id, "cli");
    }
    // The profile's own records (its Code tab's sidebar).
    let instance = root.join("ClaudeProfiles/lab");
    let records = instance.join("claude-code-sessions/acct/org");
    std::fs::create_dir_all(&records).unwrap();
    for (local, cli, archived) in [
        ("local_k1", "kept", false),
        ("local_k2", "kept-archived", true),
        ("local_k3", "adopted", false),
    ] {
        std::fs::write(
            records.join(format!("{local}.json")),
            format!(
                r#"{{"sessionId":"{local}","cliSessionId":"{cli}","title":"T {cli}","isArchived":{archived},"lastActivityAt":1792829000000}}"#
            ),
        )
        .unwrap();
    }
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("data"), registry).unwrap());
    juggler_app::features::cache::register_defaults(&state);
    let id = juggler_app::features::account::import::account_import_mode_impl(
        &state,
        "claude-code",
        folder.to_str().unwrap(),
        "Lab One",
        juggler_app::features::account::import::ImportRequest {
            in_place: true,
            ..Default::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    let profile = juggler_core::storage::desktop_app_id("claude-app", instance.to_str());
    // Seen lately (one unseen for 90 days is forgotten).
    let seen = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    {
        let storage = state.storage.lock().unwrap();
        storage.projects().touch(&repo, None).unwrap();
        storage
            .apps()
            .put_desktop(&App {
                id: profile.clone(),
                kind: "desktop".into(),
                tool: "claude-code".into(),
                host_app_id: Some("claude-app".into()),
                instance_dir: Some(instance.to_string_lossy().into_owned()),
                folder_id: Some(id.clone()),
                label: None,
                hidden: false,
                keep_alive: false,
                paused_json: None,
                launch_json: Some(
                    serde_json::json!({
                        "name": "Claude",
                        "instance": "lab",
                        "bundle": "/Applications/Claude.app",
                        "given": instance,
                    })
                    .to_string(),
                ),
                sessions_json: None,
                first_seen: seen,
                last_seen: seen,
                added: true,
            })
            .unwrap();
        storage.apps().add(&profile).unwrap();
    }
    juggler_app::features::conversations::refresh_impl(&state).unwrap();
    // The live view's scan reads the profile's records, open or closed.
    juggler_app::features::places::observe(&state, &[], &[]);

    let sorted = |mut ids: Vec<String>| {
        ids.sort();
        ids
    };
    let rows = juggler_app::features::conversations::commands::list_impl(&state).unwrap();
    let row = |id: &str| rows.iter().find(|row| row.id == id).unwrap();
    let cli = cli_app_id(&id);
    for kept in ["kept", "kept-archived", "adopted"] {
        assert_eq!(row(kept).app.as_deref(), Some(profile.as_str()), "{kept}");
        assert!(!row(kept).app_unlisted, "{kept}");
    }
    assert!(row("kept-archived").app_archived, "the app archived it");
    assert!(!row("kept").app_archived);
    for gone in ["gone-1", "gone-2", "gone-3", "gone-4"] {
        // Still the profile's (History's chip), only not in its lists.
        assert_eq!(row(gone).app.as_deref(), Some(profile.as_str()), "{gone}");
        assert!(row(gone).app_unlisted, "{gone}");
    }
    for term in ["term-1", "term-2"] {
        assert_eq!(row(term).app.as_deref(), Some(cli.as_str()), "{term}");
        assert!(!row(term).app_unlisted, "{term}");
    }

    // Every App count and the App's Sessions list: its 3 records.
    let count = |kind: EntityKind, of: &str| {
        relations::counts(
            &state,
            &EntityRef {
                kind,
                id: of.to_owned(),
            },
        )
        .unwrap()
        .sessions
        .all
    };
    assert_eq!(count(EntityKind::App, &profile), 3);
    assert_eq!(count(EntityKind::App, &cli), 2);
    assert_eq!(count(EntityKind::Folder, &id), 9, "its folder has all");
    let page = |query: SessionsQuery| {
        sorted(
            sessions_page_impl(
                &state,
                &query,
                &PageRequest {
                    limit: Some(50),
                    cursor: None,
                },
            )
            .unwrap()
            .items
            .into_iter()
            .map(|row| row.conversation.expect("another app's").id)
            .collect(),
        )
    };
    let of_app = |app: &str| SessionsQuery {
        app_id: Some(app.to_owned()),
        ..SessionsQuery::default()
    };
    assert_eq!(page(of_app(&profile)), ["adopted", "kept", "kept-archived"]);
    assert_eq!(page(of_app(&cli)), ["term-1", "term-2"]);
    // History lists all 9; its App filter offers each App with its own count.
    assert_eq!(page(SessionsQuery::default()).len(), 9);
    let facets = sessions_facets_impl(&state, &SessionsQuery::default()).unwrap();
    let apps: Vec<(String, u64)> = facets
        .apps
        .into_iter()
        .map(|facet| (facet.id, facet.count))
        .collect();
    assert_eq!(apps, [(profile.clone(), 3), (cli.clone(), 2)]);
}
