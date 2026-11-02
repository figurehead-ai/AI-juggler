//! D-62 (AJ-E3202) storage: v40 makes one CLI App per folder in use; an
//! Account is one row per vendor sign-in however often it's seen; an App
//! is one row however often it runs; Forget leaves its sessions' rows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::model::{Account, AccountId, HarnessId};
use juggler_core::storage::{Storage, cli_app_id, desktop_app_id, subscription_id};
use rusqlite::Connection;

fn folder(id: &str, tool: &str) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new(tool),
        label: id.into(),
        plan: None,
        state_dir: format!("/tmp/aij-test/{id}").into(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

#[test]
fn v40_makes_one_cli_app_per_folder_in_use_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    {
        let storage = Storage::open(&db).unwrap();
        storage
            .accounts()
            .insert(&folder("work", "claude-code"))
            .unwrap();
        storage.accounts().insert(&folder("home", "codex")).unwrap();
        storage
            .accounts()
            .insert(&folder("old", "claude-code"))
            .unwrap();
        storage
            .accounts()
            .mark_removed(&AccountId::new("old"))
            .unwrap();
    }
    // Back to v39, as an upgrade finds it: v40 runs on the next open.
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "ALTER TABLE instances DROP COLUMN app_id;
             ALTER TABLE accounts DROP COLUMN subscription_id;
             DROP TABLE apps;
             DROP TABLE subscriptions;
             PRAGMA user_version = 39;",
        )
        .unwrap();
    }
    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    let apps = storage.apps().list().unwrap();
    let made: Vec<(String, String, String)> = apps
        .iter()
        .map(|app| (app.id.clone(), app.kind.clone(), app.tool.clone()))
        .collect();
    assert_eq!(
        made.iter()
            .map(|(id, _, _)| id.as_str())
            .collect::<Vec<_>>()
            .len(),
        2,
        "a removed folder makes no App: {made:?}"
    );
    assert!(made.contains(&(cli_app_id("work"), "cli".into(), "claude-code".into())));
    assert!(made.contains(&(cli_app_id("home"), "cli".into(), "codex".into())));
    assert!(
        storage.subscriptions().list().unwrap().is_empty(),
        "Accounts come from sign-ins, read later"
    );
}

#[test]
fn an_account_is_one_row_per_vendor_sign_in_and_names_trim() {
    let storage = Storage::open_in_memory().unwrap();
    let subs = storage.subscriptions();
    let a = subs.upsert("anthropic", "ab12cd34ef56ab78cd90", 1).unwrap();
    assert_eq!(a, subscription_id("anthropic", "ab12cd34ef56ab78cd90"));
    assert_eq!(
        subs.upsert("anthropic", "ab12cd34ef56ab78cd90", 9).unwrap(),
        a,
        "seen again: the same Account"
    );
    let other = subs.upsert("openai", "ab12cd34ef56ab78cd90", 2).unwrap();
    assert_ne!(other, a, "the vendor is part of the key");
    assert_eq!(subs.list().unwrap().len(), 2);
    subs.rename(&a, Some("  Acme Team ")).unwrap();
    assert_eq!(
        subs.get(&a).unwrap().unwrap().name.as_deref(),
        Some("Acme Team")
    );
    subs.rename(&a, Some("   ")).unwrap();
    assert_eq!(
        subs.get(&a).unwrap().unwrap().name,
        None,
        "blank is back to the default"
    );
    subs.set_terms(&a, Some("max-20x"), Some("2026-12-01"), Some(200.0))
        .unwrap();
    let terms = subs.get(&a).unwrap().unwrap();
    assert_eq!(
        (
            terms.plan.as_deref(),
            terms.renewal_date.as_deref(),
            terms.fee_usd
        ),
        (Some("max-20x"), Some("2026-12-01"), Some(200.0))
    );
    assert_eq!(
        terms.created_at, 1,
        "made once; a later sighting doesn't move it"
    );
}

#[test]
fn a_folder_points_at_the_account_its_sign_in_was_read_as_and_says_when_that_changes() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&folder("work", "claude-code"))
        .unwrap();
    let subs = storage.subscriptions();
    let a = subs.upsert("anthropic", "1111", 1).unwrap();
    let b = subs.upsert("anthropic", "2222", 1).unwrap();
    assert!(subs.set_folder("work", Some(&a)).unwrap());
    assert!(
        !subs.set_folder("work", Some(&a)).unwrap(),
        "the same: no change"
    );
    assert!(
        subs.set_folder("work", Some(&b)).unwrap(),
        "signed in elsewhere: a change"
    );
    assert!(subs.set_folder("work", None).unwrap());
    assert_eq!(subs.folders().unwrap(), [("work".to_owned(), None)]);
}

#[test]
fn an_app_is_one_row_however_often_it_runs_and_forget_keeps_its_sessions_rows() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&folder("work", "claude-code"))
        .unwrap();
    let apps = storage.apps();
    let cli = apps.upsert_cli("claude-code", "work", 5).unwrap();
    assert_eq!(apps.upsert_cli("claude-code", "work", 50).unwrap(), cli);
    let desk = apps
        .upsert_desktop("claude-code", "claude-app", Some("/u/b-work"), 10)
        .unwrap();
    assert_eq!(desk, desktop_app_id("claude-app", Some("/u/b-work")));
    apps.upsert_desktop("claude-code", "claude-app", Some("/u/b-work"), 30)
        .unwrap();
    apps.upsert_desktop("claude-code", "claude-app", Some("/u/b-work"), 20)
        .unwrap();
    let row = apps.get(&desk).unwrap().unwrap();
    assert_eq!(
        (row.first_seen, row.last_seen),
        (10, 30),
        "last seen only moves on"
    );
    let default = apps
        .upsert_desktop("claude-code", "claude-app", None, 40)
        .unwrap();
    assert_ne!(default, desk, "the default instance is its own App");
    assert_eq!(apps.list().unwrap().len(), 3);
    apps.set_keep_alive(&desk, true, None).unwrap();
    apps.set_launch(&desk, Some(r#"{"bundle":"/Applications/Claude.app"}"#))
        .unwrap();
    apps.rename(&desk, Some("Work desk")).unwrap();
    let row = apps.get(&desk).unwrap().unwrap();
    assert!(row.keep_alive);
    assert_eq!(row.label.as_deref(), Some("Work desk"));
    apps.forget(&desk).unwrap();
    assert!(apps.get(&desk).unwrap().is_none());
    assert_eq!(apps.list().unwrap().len(), 2);
}

#[test]
fn a_desktop_app_is_written_whole_and_keeps_the_users_name() {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .accounts()
        .insert(&folder("work", "claude-code"))
        .unwrap();
    let apps = storage.apps();
    let id = desktop_app_id("claude-app", Some("/u/b-work"));
    let mut app = juggler_core::storage::App {
        id: id.clone(),
        kind: "desktop".into(),
        tool: "claude-code".into(),
        host_app_id: Some("claude-app".into()),
        instance_dir: Some("/u/b-work".into()),
        folder_id: Some("work".into()),
        label: None,
        hidden: false,
        keep_alive: true,
        paused_json: None,
        launch_json: Some(r#"{"bundle":"/Applications/Claude.app"}"#.into()),
        sessions_json: Some(r#"["claude-code/s-1"]"#.into()),
        first_seen: 10,
        last_seen: 10,
    };
    apps.put_desktop(&app).unwrap();
    apps.rename(&id, Some("Work desk")).unwrap();
    app.keep_alive = false;
    app.paused_json = Some(r#""it stopped too often""#.into());
    app.sessions_json = Some(r#"["claude-code/s-1","claude-code/s-2"]"#.into());
    app.folder_id = Some("gone".into());
    app.first_seen = 99;
    app.last_seen = 20;
    apps.put_desktop(&app).unwrap();
    let row = apps.get(&id).unwrap().unwrap();
    assert_eq!(
        row.label.as_deref(),
        Some("Work desk"),
        "the user's name stays"
    );
    assert!(!row.keep_alive);
    assert_eq!(
        row.paused_json.as_deref(),
        Some(r#""it stopped too often""#)
    );
    assert_eq!(row.sessions_json, app.sessions_json);
    assert_eq!(
        row.folder_id, None,
        "a folder that isn't stored is left unset"
    );
    assert_eq!((row.first_seen, row.last_seen), (10, 20));
}
