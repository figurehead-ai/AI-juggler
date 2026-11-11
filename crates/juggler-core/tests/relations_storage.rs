//! AJ-E3102 storage (schema v43): the sessions' indexes by project, launch
//! settings, App and Mac are there and used; an upgrade from v42 adds
//! them with every row intact; the relation stamps move when, and only
//! when, a value a relation is decided by changes (v44: who is in which
//! pool too).

#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic
)]

use std::path::PathBuf;

use juggler_core::model::{
    Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
    ProfileId, ProjectId, SessionId, SessionStatus, SettingsOverlay, Timestamp,
};
use juggler_core::storage::{InstanceColumn, MemberRef, PoolRole, Storage, StoredStamps};
use rusqlite::Connection;

/// v45 undone (`fixtures/undo_v45.sql`): every rollback past it starts here.
const UNDO_V45: &str = include_str!("fixtures/undo_v45.sql");

fn folder(id: &str) -> Account {
    Account {
        id: AccountId::new(id),
        harness: HarnessId::new("claude-code"),
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

fn launch(id: &str, folder: &str) -> LaunchRecipe {
    LaunchRecipe {
        id: ProfileId::new(id),
        name: "Main".into(),
        account: AccountId::new(folder),
        settings_overlay: SettingsOverlay::default(),
        default_cwd: None,
    }
}

fn session(id: &str, recipe: &str, project: Option<&str>, machine: &str) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(recipe),
        machine: MachineId::new(machine),
        kind: InstanceKind::Terminal,
        project: project.map(ProjectId::new),
        cwd: PathBuf::from("/nowhere"),
        status: SessionStatus::Running,
        started_at: Timestamp::from_millis(1_000),
        last_activity: Some(Timestamp::from_millis(2_000)),
        usage: Default::default(),
        attention: None,
        running: None,
        transcript_ref: None,
        window: None,
    }
}

fn seeded(storage: &Storage) {
    storage.accounts().insert(&folder("work")).unwrap();
    storage.accounts().insert(&folder("home")).unwrap();
    storage.profiles().insert(&launch("rec-w", "work")).unwrap();
    storage.profiles().insert(&launch("rec-h", "home")).unwrap();
    for (n, (recipe, project, machine)) in [
        ("rec-w", Some("p-1"), "mac-a"),
        ("rec-w", Some("p-1"), "mac-b"),
        ("rec-h", Some("p-2"), "mac-a"),
        ("rec-h", None, "mac-a"),
    ]
    .into_iter()
    .enumerate()
    {
        storage
            .sessions()
            .insert(&session(&format!("s-{n}"), recipe, project, machine))
            .unwrap();
    }
}

#[test]
fn every_relation_column_has_its_index_and_a_count_uses_it() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    seeded(&storage);
    let relations = storage.relations();
    for column in InstanceColumn::ALL {
        let plan = relations.count_plan(column).unwrap();
        assert!(
            plan.contains(column.index()),
            "{column:?} is read through {}: {plan}",
            column.index()
        );
    }
    // The indexed counts agree with the rows.
    assert_eq!(
        relations
            .count_where(InstanceColumn::Project, "p-1")
            .unwrap(),
        2
    );
    assert_eq!(
        relations
            .count_where(InstanceColumn::LaunchSettings, "rec-h")
            .unwrap(),
        2
    );
    assert_eq!(
        relations
            .count_where(InstanceColumn::Machine, "mac-a")
            .unwrap(),
        3
    );
    assert_eq!(
        relations
            .count_where(InstanceColumn::App, "cli:work")
            .unwrap(),
        0,
        "AI Juggler's insert leaves the App to E32's sign-in pass"
    );
    let facets = relations.session_facets().unwrap();
    assert_eq!(facets.len(), 4);
    let first = facets.iter().find(|facet| facet.id == "s-0").unwrap();
    assert_eq!(first.project.as_deref(), Some("p-1"));
    assert_eq!(first.launch_settings, "rec-w");
    assert_eq!(first.machine, "mac-a");
    assert_eq!(first.status, "running");
    assert_eq!(first.last_ms, 2_000);
}

#[test]
fn an_upgrade_from_v42_adds_the_indexes_and_stamps_with_every_row_kept() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    {
        let storage = Storage::open(&db).unwrap();
        seeded(&storage);
    }
    // Back to v42, as an upgrade finds it.
    {
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(UNDO_V45).unwrap();
        let names: Vec<(String, String)> = conn
            .prepare(
                "SELECT type, name FROM sqlite_master WHERE \
                 (type = 'trigger' AND name LIKE 'relation\\_stamp\\_%' ESCAPE '\\') \
                 OR (type = 'index' AND name LIKE 'idx\\_instances\\_%' ESCAPE '\\')",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        // v43's 17 triggers and 4 indexes, v44's 9 pool triggers.
        assert_eq!(names.len(), 17 + 4 + 9, "{names:?}");
        for (kind, name) in names {
            conn.execute_batch(&format!("DROP {} {name};", kind.to_uppercase()))
                .unwrap();
        }
        conn.execute_batch("DROP TABLE relation_stamps; PRAGMA user_version = 42;")
            .unwrap();
    }
    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    assert_eq!(storage.sessions().list(None).unwrap().len(), 4);
    // Fresh stamps; v45, running after v44, gives each default pool its
    // everyone, which is a pools change like any other.
    assert_eq!(
        storage.relations().stamps().unwrap(),
        StoredStamps {
            pools: 1,
            ..StoredStamps::default()
        }
    );
    for column in InstanceColumn::ALL {
        assert!(
            storage
                .relations()
                .count_plan(column)
                .unwrap()
                .contains(column.index())
        );
    }
}

#[test]
fn a_stamp_moves_only_when_what_decides_a_relation_changes() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    seeded(&storage);
    let stamps = || storage.relations().stamps().unwrap();
    let start = stamps();
    assert!(start.sessions >= 4 && start.folders >= 4, "{start:?}");

    // A heartbeat, its usage, a label or a pin: no relation changed.
    let mut record = storage.sessions().get(&SessionId::new("s-0")).unwrap();
    record.last_activity = Some(Timestamp::from_millis(9_000));
    storage.sessions().update(&record).unwrap();
    storage
        .sessions()
        .set_label(&record.id, Some("Login fix"))
        .unwrap();
    storage.sessions().set_pinned(&record.id, true).unwrap();
    assert_eq!(stamps(), start);

    // It ends: the sessions' stamp moves, the others don't.
    record.status = SessionStatus::Finished;
    storage.sessions().update(&record).unwrap();
    let ended = stamps();
    assert_eq!(ended.sessions, start.sessions + 1);
    assert_eq!(
        (ended.folders, ended.projects),
        (start.folders, start.projects)
    );
    // Archived (a lifecycle step) and moved to another project.
    storage
        .sessions()
        .record_lifecycle(&record.id, "archived")
        .unwrap();
    record.project = Some(ProjectId::new("p-2"));
    storage.sessions().update(&record).unwrap();
    assert_eq!(stamps().sessions, ended.sessions + 2);

    // A folder's sign-in read as an Account, then read the same again.
    let before = stamps();
    let account = storage
        .subscriptions()
        .upsert("anthropic", "subject-hash", 1)
        .unwrap();
    assert!(
        storage
            .subscriptions()
            .set_folder("work", Some(&account))
            .unwrap()
    );
    assert_eq!(stamps().folders, before.folders + 1);
    assert!(
        !storage
            .subscriptions()
            .set_folder("work", Some(&account))
            .unwrap()
    );
    assert_eq!(stamps().folders, before.folders + 1);

    // A project added; the same folder touched again only refreshes it.
    let before = stamps();
    storage
        .projects()
        .touch(std::path::Path::new("/nowhere/app"), None)
        .unwrap();
    assert_eq!(stamps().projects, before.projects + 1);
    storage
        .projects()
        .touch(std::path::Path::new("/nowhere/app"), None)
        .unwrap();
    assert_eq!(stamps().projects, before.projects + 1);
}

#[test]
fn the_pools_stamp_moves_when_a_session_changes_pool() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("juggler.sqlite")).unwrap();
    seeded(&storage);
    let stamps = || storage.relations().stamps().unwrap();
    let start = stamps();

    // A pool made, a session put in it, then the pool deleted.
    let pool = storage.pools().create("Review", None).unwrap();
    let made = stamps();
    assert_eq!(made.pools, start.pools + 1);
    let member = MemberRef::of_session(&SessionId::new("s-0"), None);
    storage
        .pools()
        .place(&member, &[], &pool.id, PoolRole::Member)
        .unwrap();
    let placed = stamps();
    assert!(placed.pools > made.pools, "{placed:?}");
    assert_eq!(
        (placed.sessions, placed.folders, placed.projects),
        (start.sessions, start.folders, start.projects),
        "a pool move moves no other stamp"
    );
    // Renamed: nobody moved.
    storage.pools().rename(&pool.id, "Reviews").unwrap();
    assert_eq!(stamps(), placed);
    storage.pools().delete(&pool.id).unwrap();
    assert!(stamps().pools > placed.pools);
}
