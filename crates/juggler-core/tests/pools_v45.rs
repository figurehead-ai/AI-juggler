//! AJ-E3003 (D-62 decision 16, scenario P9): moving 0.3's pools to the
//! v45 model with nothing lost.
//!
//! - A v44 database holding every 0.3 pool kind (a project's default pool,
//!   the no-project pool, a user pool with a project and one without) and
//!   a row of every kind of content each holds (every table keyed by a
//!   pool: rules, approvals, audit, messages, docs, links, pauses, tasks,
//!   layout…), plus members of both 0.3 kinds and roles.
//! - After the move every row is still there with the same values: the
//!   reference diff shows additions only (new columns, the default pools'
//!   "everyone" rows).
//! - Each kind maps as D-62 says, every session is where 0.3 had it, and a
//!   copy of the database was kept first.
//! - The move takes under a second for 100 pools and 10 000 messages.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::Instant;

use juggler_core::model::{ProjectId, SessionId};
use juggler_core::pools::{Memberships, SessionFacts};
use juggler_core::storage::{MemberRef, PoolRole, Scope, Storage, pre_migration_copies};
use rusqlite::Connection;
use rusqlite::types::Value;

/// 0.3's last schema: the pools as 0.3 left them.
const SCHEMA_0_3: usize = 44;

fn database_at_0_3(path: &Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    // Content rows point at threads and sessions the fixture doesn't
    // make; the move must keep them as they are all the same.
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for (index, sql) in Storage::migrations().iter().enumerate().take(SCHEMA_0_3) {
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
             VALUES ('s-1', NULL, 'rec-work', 'this-mac', '/tmp/shop', 'running', 222, 333, '{}', 'terminal', 'p-shop'),
                    ('s-2', NULL, 'rec-work', 'this-mac', '/tmp/shop', 'running', 222, 333, '{}', 'terminal', 'p-shop'),
                    ('s-3', NULL, 'rec-work', 'this-mac', '/tmp/loose', 'running', 222, 333, '{}', 'terminal', NULL),
                    ('s-4', NULL, 'rec-work', 'this-mac', '/tmp/blog', 'running', 222, 333, '{}', 'terminal', 'p-blog'),
                    ('s-5', NULL, 'rec-work', 'this-mac', '/tmp/loose', 'running', 222, 333, '{}', 'terminal', NULL);
         INSERT INTO pools (id, name, kind, project, created_at, access_level, spans_macs)
             VALUES ('pool-a', 'Checkout', 'user', 'p-shop', 10, 'full', 1),
                    ('pool-b', 'Everything', 'user', NULL, 11, 'read', 0),
                    ('pool-gone', 'Old', 'user', NULL, 12, 'work_together', 0);
         UPDATE pools SET deleted_at = 13 WHERE id = 'pool-gone';
         INSERT INTO pool_members (pool_id, member_ref, role, joined_at, left_at)
             VALUES ('pool-a', 'session:s-2', 'lead', 20, NULL),
                    ('pool-b', 'session:s-3', 'observer', 21, NULL),
                    ('pool-b', 'work:w-blog', 'member', 22, NULL),
                    ('pool-default:p-shop', 'session:s-1', 'lead', 23, NULL),
                    ('pool-gone', 'session:s-5', 'member', 12, 13);",
    )
    .unwrap();
    conn
}

/// The columns that name a pool.
const POOL_COLUMNS: [&str; 3] = ["pool_id", "a_pool", "b_pool"];

/// Every table with a column naming a pool, besides the pools' own.
fn pool_tables(conn: &Connection) -> Vec<String> {
    tables(conn)
        .into_iter()
        .filter(|table| table != "pool_members" && table != "pools")
        .filter(|table| {
            columns(conn, table)
                .iter()
                .any(|(name, ..)| POOL_COLUMNS.contains(&name.as_str()))
        })
        .collect()
}

fn tables(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// `(name, declared type, not null, has a default, primary key)`.
fn columns(conn: &Connection, table: &str) -> Vec<(String, String, bool, bool, bool)> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, Option<String>>(4)?.is_some(),
                row.get::<_, i64>(5)? > 0,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// One row of content in `table` for `pool`: every column filled (an
/// integer key left to SQLite), so nothing is NULL that could hide a loss.
fn content_row(conn: &Connection, table: &str, pool: &str, n: usize) {
    let cols: Vec<_> = columns(conn, table)
        .into_iter()
        .filter(|(_, kind, _, _, pk)| !(*pk && kind.eq_ignore_ascii_case("INTEGER")))
        .collect();
    let names: Vec<&str> = cols.iter().map(|(name, ..)| name.as_str()).collect();
    let values: Vec<Value> = cols
        .iter()
        .map(|(name, kind, ..)| match name.as_str() {
            "pool_id" | "a_pool" => Value::Text(pool.to_owned()),
            "b_pool" => Value::Text("pool-default:none".to_owned()),
            _ if kind.to_ascii_uppercase().contains("INT") => {
                Value::Integer(i64::try_from(n).unwrap() + 1)
            }
            _ if kind.to_ascii_uppercase().contains("REAL") => Value::Real(1.5),
            _ => Value::Text(format!("{table}-{name}-{n}")),
        })
        .collect();
    let marks = vec!["?"; names.len()].join(", ");
    conn.execute(
        &format!(
            "INSERT INTO {table} ({}) VALUES ({marks})",
            names.join(", ")
        ),
        rusqlite::params_from_iter(values),
    )
    .unwrap_or_else(|err| panic!("{table}: {err}"));
}

/// Every row of every table, each as its columns' text, by table.
fn snapshot(
    conn: &Connection,
    only: Option<&BTreeMap<String, Vec<String>>>,
) -> BTreeMap<String, BTreeSet<Vec<String>>> {
    let mut out = BTreeMap::new();
    for table in tables(conn) {
        let cols: Vec<String> = match only {
            Some(only) => match only.get(&table) {
                Some(cols) => cols.clone(),
                None => continue,
            },
            None => columns(conn, &table).into_iter().map(|col| col.0).collect(),
        };
        let sql = format!(
            "SELECT {} FROM {table}",
            cols.iter()
                .map(|col| format!("quote({col})"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let rows: BTreeSet<Vec<String>> = conn
            .prepare(&sql)
            .unwrap()
            .query_map([], |row| {
                (0..cols.len()).map(|at| row.get::<_, String>(at)).collect()
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        out.insert(table, rows);
    }
    out
}

fn shape(conn: &Connection) -> BTreeMap<String, Vec<String>> {
    tables(conn)
        .into_iter()
        .map(|table| {
            let cols = columns(conn, &table).into_iter().map(|col| col.0).collect();
            (table, cols)
        })
        .collect()
}

#[test]
fn every_0_3_pool_kind_and_its_content_moves_with_additions_only() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let conn = database_at_0_3(&db);
    let pools = [
        "pool-default:p-shop",
        "pool-default:p-blog",
        "pool-default:none",
        "pool-a",
        "pool-b",
        "pool-gone",
    ];
    let content = pool_tables(&conn);
    for table in [
        "agent_rules",
        "agent_approvals",
        "agent_audit",
        "agent_threads",
        "agent_messages",
        "agent_docs",
        "agent_asks",
        "pool_links",
        "pool_pauses",
        "pool_layout",
        "tasks",
        "task_events",
    ] {
        assert!(
            content.contains(&table.to_owned()),
            "{table} isn't keyed by pool: {content:?}"
        );
    }
    for table in &content {
        for (n, pool) in pools.iter().enumerate() {
            content_row(&conn, table, pool, n);
        }
    }
    let before_shape = shape(&conn);
    let before = snapshot(&conn, None);
    drop(conn);

    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    // The copy first (P9: "a backup was taken first").
    assert_eq!(
        pre_migration_copies(&db)[0].0,
        i64::try_from(SCHEMA_0_3).unwrap()
    );

    // Additions only: every column and every row as it was.
    let conn = storage.raw_conn();
    let after = snapshot(conn, Some(&before_shape));
    for (table, rows) in &before {
        if table == "relation_stamps" {
            // A change counter the counts are checked against, not content:
            // the move's new rows bump the pools' stamp, as any change does.
            continue;
        }
        let now = &after[table];
        let lost: Vec<_> = rows.difference(now).collect();
        assert!(lost.is_empty(), "{table} lost or changed rows: {lost:?}");
        let added: Vec<_> = now.difference(rows).collect();
        if table == "pool_members" {
            // The default pools' "everyone in scope" rows, one per pool.
            assert_eq!(added.len(), 3, "{added:?}");
            assert!(added.iter().all(|row| row[2] == "'all'"), "{added:?}");
        } else {
            assert!(added.is_empty(), "{table} gained rows: {added:?}");
        }
    }
    for (table, cols) in &before_shape {
        let now: Vec<String> = columns(conn, table).into_iter().map(|col| col.0).collect();
        assert!(
            cols.iter().all(|col| now.contains(col)),
            "{table} lost a column"
        );
    }

    // Each kind maps as D-62 decision 16 says.
    let list = storage.pools().list().unwrap();
    let pool = |id: &str| list.iter().find(|pool| pool.id == id).unwrap().clone();
    let shop = ProjectId::new("p-shop");
    assert_eq!(pool("pool-default:p-shop").scope, Scope::one(Some(&shop)));
    assert!(pool("pool-default:p-shop").skip_held);
    assert_eq!(pool("pool-default:none").scope, Scope::one(None));
    assert_eq!(pool("pool-a").scope, Scope::one(Some(&shop)));
    assert!(!pool("pool-a").skip_held);
    assert_eq!(
        pool("pool-b").scope,
        Scope::Every,
        "a pool with no project now says Every project"
    );
    assert!(list.iter().all(|pool| !pool.sealed));
    assert!(list.iter().all(|pool| pool.id != "pool-gone"));
    // Tasks keep their pool's project.
    let projects: Vec<(String, Option<String>)> = conn
        .prepare("SELECT pool_id, project FROM tasks ORDER BY pool_id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(projects.contains(&("pool-a".to_owned(), Some("p-shop".to_owned()))));
    assert!(projects.contains(&("pool-b".to_owned(), None)));

    // Every session is where 0.3 had it, and no more.
    let index = Memberships::new(&list, &storage.pools().members().unwrap(), HashMap::new());
    let at = |id: &str, project: Option<&str>| {
        index
            .resolve(&SessionFacts::bare(
                SessionId::new(id),
                project.map(ProjectId::new),
            ))
            .placements
            .into_iter()
            .map(|place| (place.pool_id, place.role))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        at("s-1", Some("p-shop")),
        [("pool-default:p-shop".to_owned(), PoolRole::Lead)]
    );
    assert_eq!(
        at("s-2", Some("p-shop")),
        [("pool-a".to_owned(), PoolRole::Lead)]
    );
    assert_eq!(at("s-3", None), [("pool-b".to_owned(), PoolRole::Observer)]);
    assert_eq!(
        at("s-4", Some("p-blog")),
        [("pool-default:p-blog".to_owned(), PoolRole::Member)]
    );
    assert_eq!(
        at("s-5", None),
        [("pool-default:none".to_owned(), PoolRole::Member)],
        "a deleted pool's member is back"
    );

    // A project seen after the move gets its default pool, everyone in it.
    let root = dir.path().join("docs");
    std::fs::create_dir_all(&root).unwrap();
    let docs = storage.projects().touch(&root, Some("docs")).unwrap();
    let fresh = storage
        .pools()
        .get(&format!("pool-default:{}", docs.id.as_str()))
        .unwrap();
    assert_eq!(fresh.scope, Scope::one(Some(&docs.id)));
    assert!(fresh.skip_held);
    assert!(
        storage
            .pools()
            .members()
            .unwrap()
            .iter()
            .any(|row| row.pool_id == fresh.id && row.member == MemberRef::Everyone)
    );
}

#[test]
fn the_move_takes_under_a_second_for_100_pools_and_10_000_messages() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let conn = database_at_0_3(&db);
    conn.execute_batch("BEGIN").unwrap();
    for n in 0..100 {
        let project = if n % 2 == 0 { "'p-shop'" } else { "NULL" };
        conn.execute(
            &format!("INSERT INTO pools (id, name, kind, project, created_at) VALUES ('pool-{n}', 'P{n}', 'user', {project}, {n})"),
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pool_members (pool_id, member_ref, role, joined_at) VALUES (?1, ?2, 'member', 1)",
            [format!("pool-{n}"), format!("session:x-{n}")],
        )
        .unwrap();
    }
    for n in 0..10_000 {
        content_row(&conn, "agent_messages", &format!("pool-{}", n % 100), n);
    }
    conn.execute_batch("COMMIT").unwrap();
    drop(conn);
    let started = Instant::now();
    let storage = Storage::open(&db).unwrap();
    let took = started.elapsed();
    assert!(storage.migrated_on_open());
    assert!(took.as_millis() < 1_000, "the move took {took:?}");
    // Every user pool with no project says Every project (the fixture's
    // pool-b is one of them).
    assert_eq!(
        storage
            .pools()
            .list()
            .unwrap()
            .iter()
            .filter(|pool| pool.scope == Scope::Every)
            .count(),
        51
    );
}
