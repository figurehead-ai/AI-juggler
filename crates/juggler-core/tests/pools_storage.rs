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
//! - Access (D-28): the starting level, the user's level, the second
//!   allow-once offering a rule, revoking, the audit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use juggler_core::agent_access::{AccessLevel, AgentAction, Decision};
use juggler_core::model::{SessionId, WorkItemId};
use juggler_core::storage::{
    DEFAULT_POOL_ID, DEFAULT_POOL_NAME, MemberRef, PoolKind, PoolMember, PoolRole, Storage,
    pre_migration_copies, resolve_memberships,
};
use rusqlite::{Connection, OpenFlags};

/// The schema before pools: 0.1.0's (pools are v33).
const SCHEMA_0_1: i64 = 32;

/// A database exactly as the previous schema left it, with rows.
fn database_before_pools(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(SCHEMA_0_1).unwrap())
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
fn upgrading_from_0_1_keeps_a_copy_adds_all_sessions_and_leaves_every_row() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_before_pools(&db);
    let before = tables(&db);
    assert!(!before.contains(&"pools".to_owned()));
    assert!(
        before.contains(&"worktree_homes".to_owned()),
        "0.1.0's last table (v32)"
    );

    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());

    // The copy is the database as the previous version left it.
    let copies = pre_migration_copies(&db);
    assert_eq!(copies[0].0, SCHEMA_0_1);
    assert_eq!(tables(&copies[0].1), before);
    let copy = Connection::open_with_flags(&copies[0].1, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version: i64 = copy
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_0_1);

    // The upgraded database: the default pool, and the old rows.
    let pools = storage.pools().list().unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0].id, DEFAULT_POOL_ID);
    assert_eq!(pools[0].name, DEFAULT_POOL_NAME);
    assert_eq!(pools[0].kind, PoolKind::Default);
    assert!(storage.pools().members().unwrap().is_empty());
    let cwd: String = storage
        .raw_conn()
        .query_row("SELECT cwd FROM instances WHERE id = 's-1'", [], |row| {
            row.get(0)
        })
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
    storage
        .pools()
        .place(&a, &[], &pool.id, PoolRole::Lead)
        .unwrap();
    storage
        .pools()
        .place(&b, &[], &pool.id, PoolRole::Member)
        .unwrap();
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
    storage
        .pools()
        .place(&a, &[], &one.id, PoolRole::Lead)
        .unwrap();
    storage
        .pools()
        .place(&a, &[], &two.id, PoolRole::Member)
        .unwrap();
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
    let rows = [member(
        "pool-x",
        MemberRef::Session(a.clone()),
        PoolRole::Lead,
        1,
    )];

    // Before the switch: only a.
    let places = resolve_memberships(&[a.clone(), loner.clone()], &rows, &HashMap::new());
    assert_eq!(places[&a], ("pool-x".to_owned(), PoolRole::Lead));
    assert_eq!(
        places[&loner],
        (DEFAULT_POOL_ID.to_owned(), PoolRole::Member)
    );

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
        member(
            "pool-y",
            MemberRef::Work(WorkItemId::new("work-1")),
            PoolRole::Observer,
            2,
        ),
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
fn a_pool_starts_at_work_together_and_the_user_sets_its_level() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Checkout", None).unwrap();
    let access = storage.agent_access();
    for id in [DEFAULT_POOL_ID, pool.id.as_str()] {
        assert_eq!(
            access.access(id).unwrap().level,
            Some(AccessLevel::WorkTogether)
        );
    }
    let custom: BTreeSet<_> = [AgentAction::ReadRoster, AgentAction::AssignTask].into();
    access
        .set_level(&pool.id, AccessLevel::Custom, &custom)
        .unwrap();
    let read = access.access(&pool.id).unwrap();
    assert_eq!(read.level, Some(AccessLevel::Custom));
    assert_eq!(read.custom, custom);
    // A named level keeps no Custom set.
    access
        .set_level(&pool.id, AccessLevel::Full, &custom)
        .unwrap();
    assert!(access.access(&pool.id).unwrap().custom.is_empty());
    // A deleted pool has no level, and none can be set.
    storage.pools().delete(&pool.id).unwrap();
    assert_eq!(access.access(&pool.id).unwrap().level, None);
    assert!(
        access
            .set_level(&pool.id, AccessLevel::Read, &BTreeSet::new())
            .is_err()
    );
}

#[test]
fn the_second_allow_once_offers_a_rule_and_a_revoked_rule_stops_at_once() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Checkout", None).unwrap();
    let access = storage.agent_access();
    let ask = |offer| Decision::Ask {
        offer_always: offer,
    };
    let decide = || {
        access
            .allowed(
                &pool.id,
                "session:s-1",
                true,
                false,
                AgentAction::AssignTask,
            )
            .unwrap()
    };

    assert_eq!(decide(), ask(false), "above Work together: an ask");
    access
        .allow_once(&pool.id, AgentAction::AssignTask)
        .unwrap();
    assert_eq!(
        decide(),
        ask(true),
        "the second identical ask offers Always allow"
    );
    // Another pool's approvals don't count here.
    assert_eq!(
        access
            .allowed(
                DEFAULT_POOL_ID,
                "session:s-1",
                true,
                false,
                AgentAction::AssignTask
            )
            .unwrap(),
        ask(false)
    );

    let rule = access
        .always_allow(&pool.id, AgentAction::AssignTask)
        .unwrap();
    assert_eq!(rule.made_by, "user");
    assert!(rule.created_at > 0);
    assert_eq!(access.rules(None).unwrap(), std::slice::from_ref(&rule));
    assert_eq!(decide(), Decision::Allow);
    // Making it again changes nothing.
    access
        .always_allow(&pool.id, AgentAction::AssignTask)
        .unwrap();
    assert_eq!(access.rules(Some(&pool.id)).unwrap().len(), 1);

    access.revoke(rule.id).unwrap();
    assert!(
        matches!(decide(), Decision::Ask { .. }),
        "revoked: the very next call asks"
    );
    assert!(access.revoke(rule.id).is_err());

    // Every request is in the audit, newest first.
    let audit = access.audit(10).unwrap();
    assert_eq!(audit.len(), 5);
    assert_eq!(audit[0].decision, "ask");
    assert_eq!(audit[1].decision, "allow");
    assert!(
        audit
            .iter()
            .all(|entry| entry.actor == "session:s-1" && entry.action == "assign_task")
    );
}

#[test]
fn not_a_member_or_no_pool_is_a_deny_and_still_audited() {
    let storage = Storage::open_in_memory().unwrap();
    let access = storage.agent_access();
    assert_eq!(
        access
            .allowed(
                DEFAULT_POOL_ID,
                "session:x",
                false,
                false,
                AgentAction::ReadRoster
            )
            .unwrap(),
        Decision::Deny
    );
    assert_eq!(
        access
            .allowed(
                "pool-gone",
                "session:x",
                true,
                false,
                AgentAction::ReadRoster
            )
            .unwrap(),
        Decision::Deny
    );
    assert_eq!(access.audit(10).unwrap().len(), 2);
}
