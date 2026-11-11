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
