//! AJ-E1702 storage: the pools migration, the default pool, members,
//! roles and grants.
//!
//! - Upgrading keeps a copy of the database as it was (the previous
//!   version opens it), adds the pool tables with "All sessions" in
//!   them, and leaves every existing row as it was.
//! - There is one default pool, ever; the repository won't rename or
//!   delete it.
//! - Deleting a pool ends its rows (history stays) and frees the name.
//! - A session row made before a switch stands for the Work item after.
//! - Grants: the defaults, overrides only, fixed rows never written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::pedantic)]

use std::collections::HashMap;
use std::path::Path;

use juggler_core::model::{SessionId, WorkItemId};
use juggler_core::storage::{
    Capability, DEFAULT_POOL_ID, DEFAULT_POOL_NAME, MemberRef, PoolKind, PoolMember, PoolRole,
    Storage, default_grant, pre_migration_copies, resolve_memberships,
};
use rusqlite::{Connection, OpenFlags};

/// The schema before pools.
fn previous() -> i64 {
    Storage::schema_version() - 1
}

/// A database exactly as the previous schema left it, with rows.
fn database_before_pools(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(previous()).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
             VALUES ('work', 'claude-code', 'Work Max', NULL, '/tmp/work', 111);
         INSERT INTO launch_recipes (id, account_id, settings_json, default_cwd, created_at, name)
             VALUES ('rec-work', 'work', '{}', NULL, 111, 'Main');
         INSERT INTO instances (id, conversation_id, launch_recipe_id, machine, cwd, status,
                                started_at, last_activity, usage_json, kind)
             VALUES ('s-1', NULL, 'rec-work', 'this-mac', '/tmp/shop', 'running', 222, 333,
                     '{}', 'terminal');",
    )
    .unwrap();
}

fn tables(path: &Path) -> Vec<String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn upgrading_keeps_a_copy_adds_all_sessions_and_leaves_every_row() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_before_pools(&db);
    let before = tables(&db);
    assert!(!before.contains(&"pools".to_owned()));

    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());

    // The copy is the database as the previous version left it.
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, previous());
    assert_eq!(tables(&copies[0].1), before);
    let copy = Connection::open_with_flags(&copies[0].1, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version: i64 = copy
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, previous());

    // The upgraded database: the default pool, and the old rows.
    let pools = storage.pools().list().unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0].id, DEFAULT_POOL_ID);
    assert_eq!(pools[0].name, DEFAULT_POOL_NAME);
    assert_eq!(pools[0].kind, PoolKind::Default);
    assert!(storage.pools().members().unwrap().is_empty());
    let cwd: String = storage
        .raw_conn()
        .query_row("SELECT cwd FROM instances WHERE id = 's-1'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(cwd, "/tmp/shop");
    assert_eq!(storage.accounts().list().unwrap()[0].label, "Work Max");

    // Opening again migrates nothing and makes no second default.
    drop(storage);
    let storage = Storage::open(&db).unwrap();
    assert!(!storage.migrated_on_open());
    assert_eq!(storage.pools().list().unwrap().len(), 1);
}

#[test]
fn there_is_one_default_pool_and_the_repository_wont_rename_or_delete_it() {
    let storage = Storage::open_in_memory().unwrap();
    let second = storage.raw_conn().execute(
        "INSERT INTO pools (id, name, kind, project, created_at) VALUES ('p2', 'Two', 'default', NULL, 1)",
        [],
    );
    assert!(second.is_err(), "a second default pool was stored");
    assert!(storage.pools().rename(DEFAULT_POOL_ID, "Mine").is_err());
    assert!(storage.pools().delete(DEFAULT_POOL_ID).is_err());
    assert_eq!(storage.pools().list().unwrap()[0].name, DEFAULT_POOL_NAME);
}

#[test]
fn deleting_a_pool_ends_its_rows_and_keeps_its_history() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Checkout", None).unwrap();
    let a = MemberRef::Session(SessionId::new("s-a"));
    let b = MemberRef::Work(WorkItemId::new("work-1"));
    storage.pools().place(&a, &[], &pool.id, PoolRole::Lead).unwrap();
    storage.pools().place(&b, &[], &pool.id, PoolRole::Member).unwrap();
    assert_eq!(storage.pools().members().unwrap().len(), 2);

    assert_eq!(storage.pools().delete(&pool.id).unwrap(), 2);
    assert!(storage.pools().members().unwrap().is_empty());
    assert!(storage.pools().get(&pool.id).is_err());
    assert_eq!(storage.pools().list().unwrap().len(), 1);
    // The rows are ended, not gone.
    let ended: i64 = storage
        .raw_conn()
        .query_row(
            "SELECT COUNT(*) FROM pool_members WHERE pool_id = ?1 AND left_at IS NOT NULL",
            [&pool.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(ended, 2);
    // Twice is refused; a deleted pool can't be renamed.
    assert!(storage.pools().delete(&pool.id).is_err());
    assert!(storage.pools().rename(&pool.id, "Back").is_err());
}

#[test]
fn one_active_row_per_member_and_a_move_ends_the_old_one() {
    let storage = Storage::open_in_memory().unwrap();
    let one = storage.pools().create("One", None).unwrap();
    let two = storage.pools().create("Two", None).unwrap();
    let a = MemberRef::Session(SessionId::new("s-a"));
    storage.pools().place(&a, &[], &one.id, PoolRole::Lead).unwrap();
    storage.pools().place(&a, &[], &two.id, PoolRole::Member).unwrap();
    let active = storage.pools().members().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].pool_id, two.id);
    // The database refuses two active rows for one member, whoever writes.
    let twice = storage.raw_conn().execute(
        "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES (?1, 'session:s-a', 'member', 9)",
        [&one.id],
    );
    assert!(twice.is_err());
}

fn member(pool: &str, member: MemberRef, role: PoolRole, joined_at: i64) -> PoolMember {
    PoolMember {
        pool_id: pool.to_owned(),
        member,
        role,
        joined_at,
    }
}

#[test]
fn a_session_row_from_before_a_switch_stands_for_the_work_item_after() {
    let a = SessionId::new("s-a");
    let b = SessionId::new("s-b");
    let loner = SessionId::new("s-loner");
    let rows = [member("pool-x", MemberRef::Session(a.clone()), PoolRole::Lead, 1)];

    // Before the switch: only a.
    let places = resolve_memberships(&[a.clone(), loner.clone()], &rows, &HashMap::new());
    assert_eq!(places[&a], ("pool-x".to_owned(), PoolRole::Lead));
    assert_eq!(places[&loner], (DEFAULT_POOL_ID.to_owned(), PoolRole::Member));

    // a switched to b: both are in the Work item, so both are in the pool.
    let works: HashMap<_, _> = [
        (a.clone(), WorkItemId::new("work-1")),
        (b.clone(), WorkItemId::new("work-1")),
        (loner.clone(), WorkItemId::new("work-2")),
    ]
    .into();
    let places = resolve_memberships(&[a.clone(), b.clone(), loner.clone()], &rows, &works);
    assert_eq!(places[&b], ("pool-x".to_owned(), PoolRole::Lead));
    assert_eq!(places[&loner].0, DEFAULT_POOL_ID);

    // Two rows for one Work item: the newest wins.
    let rows = [
        member("pool-x", MemberRef::Session(a.clone()), PoolRole::Lead, 1),
        member("pool-y", MemberRef::Work(WorkItemId::new("work-1")), PoolRole::Observer, 2),
    ];
    let places = resolve_memberships(&[a.clone(), b.clone()], &rows, &works);
    assert_eq!(places[&a], ("pool-y".to_owned(), PoolRole::Observer));
    assert_eq!(places[&b], ("pool-y".to_owned(), PoolRole::Observer));
}

#[test]
fn member_refs_round_trip_and_unknown_words_are_skipped() {
    for wire in ["work:work-1", "session:s-1"] {
        assert_eq!(MemberRef::parse(wire).unwrap().to_wire(), wire);
    }
    assert!(MemberRef::parse("agent:x").is_none());
    let storage = Storage::open_in_memory().unwrap();
    storage
        .raw_conn()
        .execute(
            "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES ('pool-default', 'agent:x', 'lead', 1)",
            [],
        )
        .unwrap();
    assert!(storage.pools().members().unwrap().is_empty());
}

#[test]
fn grants_are_the_defaults_until_the_user_changes_them() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Team", None).unwrap();
    for capability in Capability::ALL {
        for role in PoolRole::ALL {
            assert_eq!(
                storage.pools().allowed(&pool.id, capability, role).unwrap(),
                default_grant(capability, role),
                "{capability:?} {role:?}"
            );
        }
    }
    // Observers only look; leads may do it all.
    assert!(!default_grant(Capability::Message, PoolRole::Observer));
    assert!(default_grant(Capability::Roster, PoolRole::Observer));
    assert!(Capability::ALL.into_iter().all(|cap| default_grant(cap, PoolRole::Lead)));

    storage
        .pools()
        .set_grant(&pool.id, Capability::Broadcast, PoolRole::Member, true)
        .unwrap();
    assert!(storage.pools().allowed(&pool.id, Capability::Broadcast, PoolRole::Member).unwrap());
    assert!(
        !storage
            .pools()
            .allowed(DEFAULT_POOL_ID, Capability::Broadcast, PoolRole::Member)
            .unwrap()
    );
    assert_eq!(storage.pools().grant_overrides(&pool.id).unwrap().len(), 1);
    storage
        .pools()
        .set_grant(&pool.id, Capability::Broadcast, PoolRole::Member, false)
        .unwrap();
    assert!(storage.pools().grant_overrides(&pool.id).unwrap().is_empty());

    assert!(
        storage
            .pools()
            .set_grant(&pool.id, Capability::Roster, PoolRole::Observer, false)
            .is_err()
    );
    assert!(storage.pools().grant_overrides(&pool.id).unwrap().is_empty());
}
