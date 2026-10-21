//! AJ-E1010 Req.2 — before a migration touches an existing database, a
//! consistent copy of it is kept beside it (`<file>.before-v<N>`), so the
//! previous version can open the data again. The copy holds exactly the
//! rows the database had; only the two newest copies are kept; a fresh
//! database makes none; and if the copy can't be written, nothing is
//! migrated.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;

use juggler_core::storage::{self, KEPT_PRE_MIGRATION_COPIES, Storage};
use rusqlite::{Connection, OpenFlags};

const PREVIOUS: i64 = 27;

/// A database exactly as schema `version` left it, with real rows.
fn database_at(path: &Path, version: i64) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    for (index, sql) in Storage::migrations()
        .iter()
        .enumerate()
        .take(usize::try_from(version).unwrap())
    {
        conn.execute_batch(sql).unwrap();
        conn.pragma_update(None, "user_version", i64::try_from(index).unwrap() + 1)
            .unwrap();
    }
    conn.execute(
        "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
         VALUES ('work', 'claude-code', 'Work Max', 'max-20x', '/tmp/work', 111),
                ('personal', 'claude-code', 'Personal', NULL, '/tmp/personal', 222)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('update_channel', 'beta')",
        [],
    )
    .unwrap();
}

/// Every row of every table, as text, sorted (read-only).
fn rows(path: &Path) -> (i64, Vec<String>) {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = Vec::new();
    for table in tables {
        let mut statement = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
        let columns = statement.column_count();
        let mut found = statement
            .query_map([], |row| {
                Ok((0..columns)
                    .map(|i| match row.get_ref(i).unwrap() {
                        rusqlite::types::ValueRef::Text(text) => {
                            format!("{:?}", String::from_utf8_lossy(text))
                        }
                        other => format!("{other:?}"),
                    })
                    .collect::<Vec<_>>()
                    .join("|"))
            })
            .unwrap()
            .map(|row| format!("{table}: {}", row.unwrap()))
            .collect::<Vec<_>>();
        found.sort();
        out.extend(found);
    }
    (version, out)
}

#[test]
fn migrating_keeps_a_copy_with_exactly_the_rows_it_had() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, PREVIOUS);
    let before = rows(&db);
    assert_eq!(before.0, PREVIOUS);

    let storage = Storage::open(&db).unwrap();
    assert!(storage.migrated_on_open());
    assert_eq!(storage.user_version().unwrap(), Storage::schema_version());
    drop(storage);

    let copy = dir
        .path()
        .join(format!("juggler.sqlite.before-v{PREVIOUS}"));
    assert_eq!(
        rows(&copy),
        before,
        "the copy is the database as it was, at its schema"
    );
    assert_eq!(
        storage::pre_migration_copies(&db),
        vec![(PREVIOUS, copy)],
        "one copy, named for the schema being left"
    );
    assert!(
        !dir.path()
            .join(format!("juggler.sqlite.before-v{PREVIOUS}.partial"))
            .exists()
    );
}

#[test]
fn only_the_two_newest_copies_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    for old in [19, 20, 26] {
        std::fs::write(
            dir.path().join(format!("juggler.sqlite.before-v{old}")),
            b"x",
        )
        .unwrap();
    }
    // A copy that a crash left half-written never counts and is replaced.
    std::fs::write(
        dir.path()
            .join(format!("juggler.sqlite.before-v{PREVIOUS}.partial")),
        b"half",
    )
    .unwrap();
    database_at(&db, PREVIOUS);
    drop(Storage::open(&db).unwrap());
    let kept: Vec<i64> = storage::pre_migration_copies(&db)
        .into_iter()
        .map(|(schema, _)| schema)
        .collect();
    assert_eq!(kept.len(), KEPT_PRE_MIGRATION_COPIES);
    assert_eq!(kept, [PREVIOUS, 26]);
    assert!(
        !dir.path()
            .join(format!("juggler.sqlite.before-v{PREVIOUS}.partial"))
            .exists()
    );
}

#[test]
fn a_fresh_or_current_database_makes_no_copy() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    let fresh = Storage::open(&db).unwrap();
    assert!(
        fresh.migrated_on_open(),
        "a new database is migrated from 0"
    );
    drop(fresh);
    drop(Storage::open(&db).unwrap());
    assert!(storage::pre_migration_copies(&db).is_empty());
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("before"))
        .collect();
    assert!(names.is_empty(), "{names:?}");
    drop(Storage::open_in_memory().unwrap());
}

#[test]
fn without_a_copy_nothing_is_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, PREVIOUS);
    // The copy's name is taken by a folder: the copy can't be put there.
    std::fs::create_dir(
        dir.path()
            .join(format!("juggler.sqlite.before-v{PREVIOUS}")),
    )
    .unwrap();
    std::fs::write(
        dir.path()
            .join(format!("juggler.sqlite.before-v{PREVIOUS}/keep")),
        b"x",
    )
    .unwrap();
    let error = Storage::open(&db).unwrap_err().to_string();
    assert!(
        error.contains("couldn't keep a copy") && error.contains("nothing was changed"),
        "{error}"
    );
    assert_eq!(rows(&db).0, PREVIOUS, "the database was not migrated");
}

/// What is still only in the WAL (another connection wrote it and hasn't
/// checkpointed) is in the copy too; the copy is the owner's alone.
#[test]
fn the_copy_has_what_is_still_only_in_the_wal() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("juggler.sqlite");
    database_at(&db, PREVIOUS);
    let writer = Connection::open(&db).unwrap();
    writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    writer
        .execute(
            "INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
             VALUES ('late', 'claude-code', 'Only in the WAL', NULL, '/tmp/late', 333)",
            [],
        )
        .unwrap();
    assert!(
        std::fs::metadata(dir.path().join("juggler.sqlite-wal"))
            .unwrap()
            .len()
            > 0,
        "the row is in the WAL"
    );
    drop(Storage::open(&db).unwrap());
    drop(writer);
    let copy = dir
        .path()
        .join(format!("juggler.sqlite.before-v{PREVIOUS}"));
    let (_, rows) = rows(&copy);
    assert!(
        rows.iter().any(|row| row.contains("Only in the WAL")),
        "{rows:?}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        for file in [&copy, &db] {
            let mode = std::fs::metadata(file).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{}", file.display());
        }
    }
}
