//! D-62 (AJ-E3202) storage: v40 makes one CLI App per folder in use; an
//! Account is one row per vendor sign-in however often it's seen; an App
//! is one row however often it runs; Forget leaves its sessions' rows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_core::model::{Account, AccountId, HarnessId};
use juggler_core::storage::{
    App, COLOUR_SLOTS, Storage, cli_app_id, desktop_app_id, subscription_id,
};
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
    // Back to v39, as an upgrade finds it: v40 (and v41) run on the next
    // open.
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "ALTER TABLE accounts DROP COLUMN terms_moved;
             ALTER TABLE instances DROP COLUMN app_id;
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

// --- v42 (AJ-E3206 req. 2): one colour per App and per Account ------------

fn desktop(id: &str, first_seen: i64) -> App {
    App {
        id: id.into(),
        kind: "desktop".into(),
        tool: "claude-code".into(),
        host_app_id: Some("claude-app".into()),
        instance_dir: Some(format!("/u/{id}")),
        folder_id: None,
        label: None,
        hidden: false,
        keep_alive: false,
        paused_json: None,
        launch_json: None,
        sessions_json: None,
        first_seen,
        last_seen: first_seen,
    }
}

/// The folders a CLI App needs (`apps.folder_id` → `accounts.id`).
fn folders(storage: &Storage, ids: &[&str]) {
    for id in ids {
        storage
            .accounts()
            .insert(&folder(id, "claude-code"))
            .unwrap();
    }
}

fn slot(storage: &Storage, id: &str) -> u8 {
    storage
        .apps()
        .colour_slot(id)
        .unwrap()
        .expect("a stored App has a slot")
}

#[test]
fn a_new_app_takes_the_least_used_slot_lowest_first_whichever_way_its_inserted() {
    let storage = Storage::open_in_memory().unwrap();
    folders(&storage, &["f0", "f3", "f6", "f8", "f9"]);
    let apps = storage.apps();
    let mut ids = Vec::new();
    for n in 0..COLOUR_SLOTS {
        let id = match n % 3 {
            0 => apps.upsert_cli("claude-code", &format!("f{n}"), n).unwrap(),
            1 => apps
                .upsert_desktop("claude-code", "claude-app", Some(&format!("/u/p{n}")), n)
                .unwrap(),
            _ => {
                let app = desktop(&format!("app:claude-app:/u/d{n}"), n);
                apps.put_desktop(&app).unwrap();
                app.id
            }
        };
        ids.push(id);
    }
    let slots: Vec<u8> = ids.iter().map(|id| slot(&storage, id)).collect();
    assert_eq!(
        slots,
        (0..8).collect::<Vec<u8>>(),
        "the first eight never share a colour"
    );
    // A ninth shares the lowest slot; a tenth the next.
    let ninth = apps.upsert_cli("codex", "f8", 8).unwrap();
    let tenth = apps.upsert_cli("codex", "f9", 9).unwrap();
    assert_eq!((slot(&storage, &ninth), slot(&storage, &tenth)), (0, 1));
}

#[test]
fn an_apps_slot_holds_through_renames_reruns_rewrites_and_others_coming_and_going() {
    let storage = Storage::open_in_memory().unwrap();
    folders(&storage, &["a", "c", "d", "e"]);
    let apps = storage.apps();
    let a = apps.upsert_cli("claude-code", "a", 1).unwrap();
    let b = apps
        .upsert_desktop("claude-code", "claude-app", Some("/u/b"), 2)
        .unwrap();
    let c = apps.upsert_cli("codex", "c", 3).unwrap();
    let before: Vec<u8> = [&a, &b, &c].iter().map(|id| slot(&storage, id)).collect();
    assert_eq!(before, vec![0, 1, 2]);

    apps.rename(&a, Some("Zebra")).unwrap();
    apps.upsert_cli("claude-code", "a", 50).unwrap();
    apps.upsert_desktop("claude-code", "claude-app", Some("/u/b"), 60)
        .unwrap();
    let mut whole = desktop(&b, 2);
    whole.keep_alive = true;
    whole.last_seen = 70;
    apps.put_desktop(&whole).unwrap();
    apps.set_keep_alive(&b, false, None).unwrap();
    // One forgotten: nobody else moves, and the next new App takes the
    // slot it freed (the least used, lowest first).
    apps.forget(&a).unwrap();
    assert_eq!((slot(&storage, &b), slot(&storage, &c)), (1, 2));
    let d = apps.upsert_cli("claude-code", "d", 80).unwrap();
    assert_eq!(slot(&storage, &d), 0, "the slot a forgotten App freed");
    let e = apps.upsert_cli("claude-code", "e", 90).unwrap();
    assert_eq!(slot(&storage, &e), 3);
    assert_eq!(apps.colour_slot("app:nothing:").unwrap(), None);
}

#[test]
fn a_folders_cli_app_slot_is_read_and_its_app_made_when_missing() {
    let storage = Storage::open_in_memory().unwrap();
    folders(&storage, &["work"]);
    let apps = storage.apps();
    apps.upsert_desktop("claude-code", "claude-app", None, 1)
        .unwrap();
    assert_eq!(
        apps.cli_colour_slot("claude-code", "work", 2).unwrap(),
        Some(1),
        "made now, after the desktop App"
    );
    assert!(apps.get(&cli_app_id("work")).unwrap().is_some());
    assert_eq!(
        apps.cli_colour_slot("claude-code", "work", 3).unwrap(),
        Some(1)
    );
    assert_eq!(
        apps.colour_slots().unwrap().get(&cli_app_id("work")),
        Some(&1)
    );
}

#[test]
fn an_account_takes_the_least_used_slot_and_keeps_it() {
    let storage = Storage::open_in_memory().unwrap();
    let subs = storage.subscriptions();
    let ids: Vec<String> = (0..9)
        .map(|n| subs.upsert("anthropic", &format!("{n:04}"), n).unwrap())
        .collect();
    let slots: Vec<u8> = ids
        .iter()
        .map(|id| subs.colour_slot(id).unwrap().unwrap())
        .collect();
    assert_eq!(slots, vec![0, 1, 2, 3, 4, 5, 6, 7, 0]);
    // Seen again, renamed, its terms set: the same colour. Apps don't
    // take an Account's slots (each table counts its own).
    subs.upsert("anthropic", "0003", 99).unwrap();
    subs.rename(&ids[3], Some("Team")).unwrap();
    subs.set_terms(&ids[3], Some("max"), None, None).unwrap();
    assert_eq!(subs.colour_slot(&ids[3]).unwrap(), Some(3));
    folders(&storage, &["f"]);
    let app = storage.apps().upsert_cli("claude-code", "f", 1).unwrap();
    assert_eq!(slot(&storage, &app), 0);
    assert_eq!(subs.colour_slots().unwrap().len(), 9);
}

#[test]
fn v42_gives_the_rows_already_there_their_slots_in_list_order() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    {
        let storage = Storage::open(&db).unwrap();
        folders(
            &storage,
            &["f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "new"],
        );
        let apps = storage.apps();
        // Inserted out of list order: desktop first, CLI after.
        apps.upsert_desktop("claude-code", "claude-app", Some("/u/late"), 30)
            .unwrap();
        apps.upsert_desktop("claude-code", "claude-app", Some("/u/early"), 10)
            .unwrap();
        for n in 0..8 {
            apps.upsert_cli("claude-code", &format!("f{n}"), 100 + n)
                .unwrap();
        }
        storage.subscriptions().upsert("openai", "bbbb", 5).unwrap();
        storage
            .subscriptions()
            .upsert("anthropic", "aaaa", 5)
            .unwrap();
        storage
            .subscriptions()
            .upsert("anthropic", "cccc", 1)
            .unwrap();
    }
    // Back to v41, as an upgrade finds it: v42 runs on the next open.
    Connection::open(&db)
        .unwrap()
        .execute_batch(
            "DROP TRIGGER apps_colour_slot;
             DROP TRIGGER subscriptions_colour_slot;
             ALTER TABLE apps DROP COLUMN colour_slot;
             ALTER TABLE subscriptions DROP COLUMN colour_slot;
             PRAGMA user_version = 41;",
        )
        .unwrap();
    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), 42);
    // The list's order (CLI first, then by when first seen), round robin.
    let listed: Vec<(String, u8)> = storage
        .apps()
        .list()
        .unwrap()
        .into_iter()
        .map(|app| {
            let slot = slot(&storage, &app.id);
            (app.id, slot)
        })
        .collect();
    let slots: Vec<u8> = listed.iter().map(|(_, slot)| *slot).collect();
    assert_eq!(slots, vec![0, 1, 2, 3, 4, 5, 6, 7, 0, 1], "{listed:?}");
    assert_eq!(
        listed[8].0,
        desktop_app_id("claude-app", Some("/u/early")),
        "the desktop Apps after the CLI ones, first seen first"
    );
    let subs = storage.subscriptions();
    let by_id = subs.colour_slots().unwrap();
    assert_eq!(by_id[&subscription_id("anthropic", "cccc")], 0);
    // Seen at the same time: by id ("acct:anthropic:…" before "acct:openai:…").
    assert_eq!(by_id[&subscription_id("anthropic", "aaaa")], 1);
    assert_eq!(by_id[&subscription_id("openai", "bbbb")], 2);
    // And the rule holds for the next one.
    let next = storage.apps().upsert_cli("codex", "new", 500).unwrap();
    assert_eq!(slot(&storage, &next), 2);
}
