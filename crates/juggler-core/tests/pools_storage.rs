//! AJ-E1702 storage: the pools migration, the default pools, members,
//! roles and access.
//!
//! - Upgrading from 0.1.0 keeps a copy of the database as it was (that
//!   version opens it), gives each project its default pool (and the
//!   sessions with no project theirs), and leaves every row as it was.
//! - Each project has one default pool (D-33), made on its first
//!   appearance; the repository won't rename or delete a default pool.
//! - Deleting a pool ends its rows (history stays) and frees the name.
//! - A session row made before a switch stands for the Work item after.
//! - Since v45 (D-62) a session can be in several pools: one active row
//!   per pool and member; a default pool is "everyone in scope".
//! - Access (D-28): the starting level, the user's level, the second
//!   allow-once offering a rule, revoking, the audit.

#![allow(
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use juggler_core::agent_access::{AccessLevel, AgentAction, Decision};
use juggler_core::model::{ProjectId, SessionId, WorkItemId};
use juggler_core::pools::{Memberships, SessionFacts};
use juggler_core::storage::{
    MemberRef, NO_PROJECT_POOL_ID, Pool, PoolKind, PoolMember, PoolRole, Scope, Storage,
    default_pool_id, pre_migration_copies,
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
         INSERT INTO projects (id, root, name, created_at) VALUES ('p-shop', '/tmp/shop', 'shop', 1);
         INSERT INTO projects (id, root, name, created_at) VALUES ('p-blog', '/tmp/blog', 'blog', 2);
         INSERT INTO instances (id, conversation_id, launch_recipe_id, machine, cwd, status,
                                started_at, last_activity, usage_json, kind, project)
             VALUES ('s-1', NULL, 'rec-work', 'this-mac', '/tmp/shop', 'running', 222, 333,
                     '{}', 'terminal', 'p-shop'),
                    ('s-2', NULL, 'rec-work', 'this-mac', '/tmp/shop', 'running', 222, 333,
                     '{}', 'terminal', 'p-shop'),
                    ('s-3', NULL, 'rec-work', 'this-mac', '/tmp/blog', 'running', 222, 333,
                     '{}', 'terminal', 'p-blog'),
                    ('s-4', NULL, 'rec-work', 'this-mac', '/tmp/loose', 'running', 222, 333,
                     '{}', 'terminal', NULL);",
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
fn upgrading_from_0_1_keeps_a_copy_gives_each_project_its_default_pool_and_leaves_every_row() {
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

    // The upgraded database (D-33): each project its own default pool,
    // one for sessions with no project, and the old rows.
    let pools = storage.pools().list().unwrap();
    let ids: BTreeSet<&str> = pools.iter().map(|pool| pool.id.as_str()).collect();
    assert_eq!(
        ids,
        BTreeSet::from([
            "pool-default:p-shop",
            "pool-default:p-blog",
            NO_PROJECT_POOL_ID
        ])
    );
    assert!(pools.iter().all(|pool| pool.kind == PoolKind::Default));
    let shop = pools
        .iter()
        .find(|pool| pool.id == "pool-default:p-shop")
        .unwrap();
    assert_eq!(shop.project, Some(ProjectId::new("p-shop")));
    assert_eq!(shop.scope, Scope::one(Some(&ProjectId::new("p-shop"))));
    // Each default pool's one member is everyone in its scope.
    let members = storage.pools().members().unwrap();
    assert_eq!(members.len(), 3);
    assert!(members.iter().all(|row| row.member == MemberRef::Everyone));
    // Membership: every session is in its own project's default pool.
    let sessions: Vec<(SessionId, Option<ProjectId>)> = storage
        .raw_conn()
        .prepare("SELECT id, project FROM instances ORDER BY id")
        .unwrap()
        .query_map([], |row| {
            Ok((
                SessionId::new(row.get::<_, String>(0)?),
                row.get::<_, Option<String>>(1)?.map(ProjectId::new),
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let index = Memberships::new(&pools, &members, HashMap::new());
    let place = |id: &str| {
        let facts = sessions
            .iter()
            .find(|(session, _)| session.as_str() == id)
            .map(|(session, project)| SessionFacts::bare(session.clone(), project.clone()))
            .unwrap();
        index.place(&facts).unwrap().0
    };
    assert_eq!(place("s-1"), "pool-default:p-shop");
    assert_eq!(place("s-2"), "pool-default:p-shop");
    assert_eq!(place("s-3"), "pool-default:p-blog");
    assert_eq!(place("s-4"), NO_PROJECT_POOL_ID);
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
    assert_eq!(storage.pools().list().unwrap().len(), 3);
}

#[test]
fn each_project_gets_one_default_pool_the_repository_wont_rename_or_delete() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open_in_memory().unwrap();
    // A project's first appearance makes its default pool.
    let root = dir.path().join("shop");
    std::fs::create_dir_all(&root).unwrap();
    let shop = storage.projects().touch(&root, Some("shop")).unwrap();
    let id = default_pool_id(Some(&shop.id));
    let pool = storage.pools().get(&id).unwrap();
    assert_eq!(
        (pool.kind, pool.project.clone()),
        (PoolKind::Default, Some(shop.id.clone()))
    );
    // Seen again: still one.
    storage.projects().touch(&root, Some("shop")).unwrap();
    let defaults = storage
        .pools()
        .list()
        .unwrap()
        .into_iter()
        .filter(|pool| pool.kind == PoolKind::Default)
        .count();
    assert_eq!(defaults, 2, "shop's and the no-project one");
    // A second default pool for one project can't be stored.
    let second = storage.raw_conn().execute(
        "INSERT INTO pools (id, name, kind, project, created_at) VALUES ('p2', 'Two', 'default', ?1, 1)",
        [shop.id.as_str()],
    );
    assert!(second.is_err(), "a second default pool was stored");
    for default in [id.as_str(), NO_PROJECT_POOL_ID] {
        assert!(storage.pools().rename(default, "Mine").is_err());
        assert!(storage.pools().delete(default).is_err());
    }
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
    let in_pool = |storage: &Storage| {
        storage
            .pools()
            .members()
            .unwrap()
            .into_iter()
            .filter(|row| row.pool_id == pool.id)
            .count()
    };
    assert_eq!(in_pool(&storage), 2);

    assert_eq!(storage.pools().delete(&pool.id).unwrap(), 2);
    assert_eq!(in_pool(&storage), 0);
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
fn one_active_row_per_pool_and_member_a_move_ends_the_old_one_and_a_join_keeps_it() {
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
    let mine = |storage: &Storage| -> Vec<String> {
        storage
            .pools()
            .members()
            .unwrap()
            .into_iter()
            .filter(|row| row.member == a)
            .map(|row| row.pool_id)
            .collect()
    };
    assert_eq!(mine(&storage), [two.id.as_str()], "a move ends the old row");
    // Joining keeps its other pools (D-62 decision 10).
    storage
        .pools()
        .join(&one.id, &a, &[], PoolRole::Reviewer, None)
        .unwrap();
    assert_eq!(mine(&storage), [two.id.clone(), one.id.clone()]);
    // Joining again changes its row there; no second row.
    storage
        .pools()
        .join(&one.id, &a, &[], PoolRole::Observer, None)
        .unwrap();
    assert_eq!(mine(&storage).len(), 2);
    // The database refuses two active rows for one member in one pool.
    let twice = storage.raw_conn().execute(
        "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES (?1, 'session:s-a', 'member', 9)",
        [&one.id],
    );
    assert!(twice.is_err());
    // Leaving one pool keeps the other.
    assert_eq!(
        storage
            .pools()
            .leave(&one.id, std::slice::from_ref(&a))
            .unwrap(),
        1
    );
    assert_eq!(mine(&storage), [two.id.as_str()]);
}

fn member(pool: &str, member: MemberRef, role: PoolRole, joined_at: i64) -> PoolMember {
    PoolMember {
        pool_id: pool.to_owned(),
        member,
        role,
        joined_at,
        level: None,
    }
}

/// A user pool over every project, as the resolver sees it.
fn every(id: &str, created_at: i64) -> Pool {
    Pool {
        id: id.to_owned(),
        name: id.to_owned(),
        kind: PoolKind::User,
        project: None,
        created_at,
        spans_macs: false,
        scope: Scope::Every,
        sealed: false,
        skip_held: false,
    }
}

/// The no-project default pool, everyone in it.
fn no_project() -> (Pool, PoolMember) {
    let pool = Pool {
        kind: PoolKind::Default,
        scope: Scope::one(None),
        skip_held: true,
        ..every(NO_PROJECT_POOL_ID, 0)
    };
    (
        pool,
        member(NO_PROJECT_POOL_ID, MemberRef::Everyone, PoolRole::Member, 0),
    )
}

#[test]
fn a_session_row_from_before_a_switch_stands_for_the_work_item_after() {
    let a = SessionId::new("s-a");
    let b = SessionId::new("s-b");
    let loner = SessionId::new("s-loner");
    let (default, everyone) = no_project();
    let pools = [every("pool-x", 1), every("pool-y", 2), default];
    let rows = [
        member("pool-x", MemberRef::Session(a.clone()), PoolRole::Lead, 1),
        everyone.clone(),
    ];
    let place = |index: &Memberships, id: &SessionId| {
        index.place(&SessionFacts::bare(id.clone(), None)).unwrap()
    };

    // Before the switch: only a; the loner is in the no-project pool.
    let index = Memberships::new(&pools, &rows, HashMap::new());
    assert_eq!(place(&index, &a), ("pool-x".to_owned(), PoolRole::Lead));
    assert_eq!(
        place(&index, &loner),
        (NO_PROJECT_POOL_ID.to_owned(), PoolRole::Member)
    );
    // a is named in pool-x, so the no-project pool skips it (0.3's move).
    let resolved = index.resolve(&SessionFacts::bare(a.clone(), None));
    assert_eq!(resolved.placements.len(), 1);
    assert_eq!(resolved.skipped[0].pool_id, NO_PROJECT_POOL_ID);

    // a switched to b: both are in the Work item, so both are in the pool.
    let works: HashMap<_, _> = [
        (a.clone(), WorkItemId::new("work-1")),
        (b.clone(), WorkItemId::new("work-1")),
        (loner.clone(), WorkItemId::new("work-2")),
    ]
    .into();
    let index = Memberships::new(&pools, &rows, works.clone());
    assert_eq!(place(&index, &b), ("pool-x".to_owned(), PoolRole::Lead));
    assert_eq!(place(&index, &loner).0, NO_PROJECT_POOL_ID);

    // Rows for one Work item in two pools: in both, the newest first.
    let rows = [
        member("pool-x", MemberRef::Session(a.clone()), PoolRole::Lead, 1),
        member(
            "pool-y",
            MemberRef::Work(WorkItemId::new("work-1")),
            PoolRole::Observer,
            2,
        ),
        everyone,
    ];
    let index = Memberships::new(&pools, &rows, works);
    for id in [&a, &b] {
        let resolved = index.resolve(&SessionFacts::bare(id.clone(), None));
        let pools: Vec<(&str, PoolRole)> = resolved
            .placements
            .iter()
            .map(|place| (place.pool_id.as_str(), place.role))
            .collect();
        assert_eq!(
            pools,
            [("pool-y", PoolRole::Observer), ("pool-x", PoolRole::Lead)]
        );
    }
}

#[test]
fn member_refs_round_trip_and_unknown_words_are_skipped() {
    for wire in [
        "work:work-1",
        "session:s-1",
        "app:cli:work",
        "app:app:claude-app:/p/Work",
        "account:acct:anthropic:1a2b",
        "tool:codex",
        "mac:studio",
        "all",
    ] {
        assert_eq!(MemberRef::parse(wire).unwrap().to_wire(), wire);
    }
    assert!(MemberRef::parse("agent:x").is_none());
    let storage = Storage::open_in_memory().unwrap();
    storage
        .raw_conn()
        .execute(
            "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES ('pool-default:none', 'agent:x', 'lead', 1)",
            [],
        )
        .unwrap();
    // Only the no-project pool's own "everyone" row reads back.
    let rows = storage.pools().members().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].member, MemberRef::Everyone);
}

#[test]
fn a_pool_starts_at_work_together_and_the_user_sets_its_level() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Checkout", None).unwrap();
    let access = storage.agent_access();
    for id in [NO_PROJECT_POOL_ID, pool.id.as_str()] {
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
                None,
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
                NO_PROJECT_POOL_ID,
                "session:s-1",
                true,
                false,
                AgentAction::AssignTask,
                None
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
fn a_pool_keeps_its_hop_limit_and_delivery_stop_d29() {
    let storage = Storage::open_in_memory().unwrap();
    let access = storage.agent_access();
    let start = access.access(NO_PROJECT_POOL_ID).unwrap();
    assert_eq!((start.hop_limit, start.delivery_stopped), (5, false));
    access.set_delivery(NO_PROJECT_POOL_ID, 3, true).unwrap();
    let after = access.access(NO_PROJECT_POOL_ID).unwrap();
    assert_eq!((after.hop_limit, after.delivery_stopped), (3, true));
    assert_eq!(
        access
            .allowed(
                NO_PROJECT_POOL_ID,
                "session:s",
                true,
                false,
                AgentAction::DeliverToSession,
                Some(0)
            )
            .unwrap(),
        Decision::Deny,
        "stopped: no delivery, whatever the level"
    );
    for wrong in [0, 21] {
        assert!(
            access
                .set_delivery(NO_PROJECT_POOL_ID, wrong, false)
                .is_err()
        );
    }
    assert!(access.set_delivery("pool-gone", 5, false).is_err());
}

#[test]
fn not_a_member_or_no_pool_is_a_deny_and_still_audited() {
    let storage = Storage::open_in_memory().unwrap();
    let access = storage.agent_access();
    assert_eq!(
        access
            .allowed(
                NO_PROJECT_POOL_ID,
                "session:x",
                false,
                false,
                AgentAction::ReadRoster,
                None
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
                AgentAction::ReadRoster,
                None
            )
            .unwrap(),
        Decision::Deny
    );
    assert_eq!(access.audit(10).unwrap().len(), 2);
}
