//! AJ-E2016: reading another tool's SQLite file without changing it.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

/// `path` for a `file:` URI: every byte outside the unreserved set and
/// `/` percent-encoded (spaces, `#` and `?` included), so a folder named
/// with them still opens the file it names.
pub(crate) fn uri_path(path: &Path) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// `db` opened read-only and `immutable` (no locks, no log, nothing
/// written next to it), hardened like every AI Juggler connection.
///
/// # Errors
///
/// The SQLite error when the file can't be opened.
pub(crate) fn open_immutable(db: &Path) -> rusqlite::Result<Connection> {
    juggler_core::storage::open_hardened_with(
        format!("file:{}?mode=ro&immutable=1", uri_path(db)),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )
}

/// How long a read of a tool's own store waits for the tool's write.
const TOOL_BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// A tool's own SQLite store (`OpenCode`'s, `ZCode`'s), opened read-only and
/// never written beside, waiting briefly for the tool's own write.
///
/// While the tool runs, its write-ahead log sits beside the file and the
/// read joins it as any reader does. When the tool is closed there is no
/// log, and a read-only open would either fail (no `-shm` it may create:
/// "unable to open database file", seen on `ZCode`'s) or create the log and
/// its index in the user's folder; so the file is opened `immutable`: no
/// log, no lock, nothing written. Should the tool start during that read,
/// the read may fail; the next pass reads again.
///
/// # Errors
///
/// A plain message naming `tool` when the file is missing or can't be
/// opened.
pub(crate) fn open_tool_store(db: &Path, tool: &str) -> Result<Connection, String> {
    if !db.is_file() {
        return Err(format!("no {tool} database at {}", db.display()));
    }
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = if Path::new(&wal).exists() {
        juggler_core::storage::open_hardened_with(db, flags)
    } else {
        juggler_core::storage::open_hardened_with(
            format!("file:{}?immutable=1", uri_path(db)),
            flags | OpenFlags::SQLITE_OPEN_URI,
        )
    }
    .map_err(|error| format!("{tool} database open (read-only): {error}"))?;
    conn.busy_timeout(TOOL_BUSY_TIMEOUT)
        .map_err(|error| format!("{tool} database busy timeout: {error}"))?;
    Ok(conn)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "a fixture tool's store, written as the tool itself writes it"
)]
mod tests {
    use super::open_tool_store;

    /// A tool's store in WAL mode, closed (no `-wal`, no `-shm` beside
    /// it), as ZCode's is when ZCode isn't running.
    fn closed_wal_store(dir: &std::path::Path) -> std::path::PathBuf {
        let db = dir.join("db.sqlite");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; CREATE TABLE session (id TEXT PRIMARY KEY);
             INSERT INTO session VALUES ('sess_1');",
        )
        .unwrap();
        drop(conn);
        assert!(
            !dir.join("db.sqlite-wal").exists(),
            "a clean close leaves no log"
        );
        db
    }

    #[test]
    fn a_closed_tools_store_opens_and_nothing_is_written_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let db = closed_wal_store(dir.path());
        let before = std::fs::read(&db).unwrap();
        let conn = open_tool_store(&db, "ZCode").unwrap();
        let id: String = conn
            .query_row("SELECT id FROM session", [], |row| row.get(0))
            .unwrap();
        assert_eq!(id, "sess_1");
        drop(conn);
        let mut beside: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        beside.sort();
        assert_eq!(
            beside,
            ["db.sqlite"],
            "no -wal or -shm made in the tool's folder"
        );
        assert_eq!(std::fs::read(&db).unwrap(), before, "not a byte changed");
    }

    #[test]
    fn a_running_tools_log_is_joined() {
        let dir = tempfile::tempdir().unwrap();
        let db = closed_wal_store(dir.path());
        // The tool running: its connection holds the log open, with a row
        // only the log has yet.
        let tool = rusqlite::Connection::open(&db).unwrap();
        tool.execute_batch("PRAGMA wal_autocheckpoint = 0; INSERT INTO session VALUES ('sess_2');")
            .unwrap();
        assert!(dir.path().join("db.sqlite-wal").exists());
        let conn = open_tool_store(&db, "ZCode").unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM session", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "the log's row is read");
    }

    #[test]
    fn a_missing_store_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let error = open_tool_store(&dir.path().join("db.sqlite"), "ZCode").unwrap_err();
        assert!(error.starts_with("no ZCode database at "), "{error}");
    }
}
