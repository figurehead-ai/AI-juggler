//! A database from elsewhere (a backup being restored), checked before
//! AI Juggler uses it (AJ-E2006).
//!
//! A backup can be crafted. Its database is opened in SQLite's defensive
//! mode with `trusted_schema` off (as every AI Juggler connection is, see
//! `super::harden`), checked for damage and for the schema version its
//! manifest claims, then a copy is brought up to this build's schema and
//! compared with a fresh database: the same tables with the same columns,
//! the same indexes, and the same triggers and views, word for word. Any
//! code AI Juggler didn't write (a trigger, a view) is refused, so nothing
//! in a backup runs later.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::Storage;
use crate::error::CoreError;

fn refused(message: impl Into<String>) -> CoreError {
    CoreError::Validation {
        subject: "backup database",
        message: message.into(),
    }
}

/// What a schema holds, for comparing two databases.
#[derive(Debug, Default, PartialEq, Eq)]
struct Shape {
    /// Table → its column names.
    tables: BTreeMap<String, BTreeSet<String>>,
    /// Index names (SQLite's own automatic ones left out).
    indexes: BTreeSet<String>,
    /// Trigger or view name → its SQL, as written.
    code: BTreeMap<String, String>,
}

fn shape(conn: &Connection) -> Result<Shape, CoreError> {
    let mut shape = Shape::default();
    let mut objects = conn.prepare(
        "SELECT type, name, COALESCE(sql, '') FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
    )?;
    let rows = objects.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (kind, name, sql) = row?;
        match kind.as_str() {
            "table" => {
                let mut columns = BTreeSet::new();
                let mut info = conn.prepare("SELECT name FROM pragma_table_info(?1)")?;
                for column in info.query_map([&name], |r| r.get::<_, String>(0))? {
                    columns.insert(column?);
                }
                shape.tables.insert(name, columns);
            }
            "index" => {
                shape.indexes.insert(name);
            }
            _ => {
                shape.code.insert(format!("{kind} {name}"), sql);
            }
        }
    }
    Ok(shape)
}

fn first_difference(theirs: &Shape, ours: &Shape) -> Option<String> {
    for (name, sql) in &theirs.code {
        if ours.code.get(name) != Some(sql) {
            return Some(format!(
                "it holds database code AI Juggler didn't write ({name})"
            ));
        }
    }
    for name in theirs.tables.keys() {
        if !ours.tables.contains_key(name) {
            return Some(format!("it holds a table AI Juggler doesn't have ({name})"));
        }
    }
    for (name, columns) in &ours.tables {
        if theirs.tables.get(name) != Some(columns) {
            return Some(format!("its table {name} isn't the one AI Juggler makes"));
        }
    }
    if theirs.indexes != ours.indexes || theirs.code.len() != ours.code.len() {
        return Some("its indexes or triggers aren't the ones AI Juggler makes".to_owned());
    }
    None
}

/// Checks the database at `path`, said to be at schema `version`, before
/// a restore uses it. Nothing in it runs: the copy brought up to date is
/// opened hardened and deleted afterwards.
///
/// # Errors
///
/// [`CoreError::Validation`] naming what's wrong (damage, another schema
/// version, code or tables AI Juggler didn't write);
/// [`CoreError::Storage`] when SQLite can't read it.
pub fn vet(path: &Path, version: i64) -> Result<(), CoreError> {
    {
        let conn = super::open_hardened_with(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let check: String = conn.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
        if check != "ok" {
            return Err(refused("its database is damaged"));
        }
        let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if found != version {
            return Err(refused(format!(
                "its database is at schema {found}, not the {version} its manifest says"
            )));
        }
    }
    let copy = path.with_extension("vetting");
    let _ = std::fs::remove_file(&copy);
    std::fs::copy(path, &copy)
        .map_err(|e| CoreError::Io(format!("copy the backup's database: {e}")))?;
    let compared = Storage::open(&copy).and_then(|theirs| {
        let ours = Storage::open_in_memory()?;
        Ok(first_difference(&shape(&theirs.conn)?, &shape(&ours.conn)?))
    });
    for leftover in [
        copy.clone(),
        copy.with_extension("vetting-wal"),
        copy.with_extension("vetting-shm"),
    ] {
        let _ = std::fs::remove_file(leftover);
    }
    match compared? {
        Some(reason) => Err(refused(reason)),
        None => Ok(()),
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "test fixtures build databases raw (other tools' files, old schemas)"
)]
mod tests {
    use super::*;

    fn fresh(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("juggler.sqlite");
        drop(Storage::open(&path).unwrap());
        // One file, as a backup holds it.
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "journal_mode", "DELETE").unwrap();
        path
    }

    #[test]
    fn a_database_ai_juggler_made_passes() {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(dir.path());
        vet(&path, Storage::schema_version()).unwrap();
        assert!(!path.with_extension("vetting").exists(), "the copy is gone");
    }

    #[test]
    fn a_backup_from_every_older_schema_passes() {
        // Built the way each older AI Juggler left it: its migrations, then
        // the derived tables and triggers it ran on every open.
        for version in 1..Storage::schema_version() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("juggler.sqlite");
            let conn = Connection::open(&path).unwrap();
            for sql in &super::super::MIGRATIONS[..usize::try_from(version).unwrap()] {
                conn.execute_batch(sql).unwrap();
            }
            conn.pragma_update(None, "user_version", version).unwrap();
            drop(conn);
            assert!(
                vet(&path, version).is_ok(),
                "schema {version}: {:?}",
                vet(&path, version)
            );
        }
    }

    #[test]
    fn a_trigger_ai_juggler_didnt_write_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(dir.path());
        Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER planted AFTER INSERT ON accounts BEGIN DELETE FROM accounts; END;",
            )
            .unwrap();
        let error = vet(&path, Storage::schema_version())
            .unwrap_err()
            .to_string();
        assert!(error.contains("didn't write"), "{error}");
    }

    #[test]
    fn an_extra_table_or_the_wrong_version_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = fresh(dir.path());
        assert!(vet(&path, Storage::schema_version() - 1).is_err());
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE extra (x)")
            .unwrap();
        let error = vet(&path, Storage::schema_version())
            .unwrap_err()
            .to_string();
        assert!(error.contains("table"), "{error}");
    }

    #[test]
    fn a_damaged_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("juggler.sqlite");
        std::fs::write(&path, b"SQLite format 3\0 but not really").unwrap();
        assert!(vet(&path, Storage::schema_version()).is_err());
    }
}
