//! `PRAGMA secure_delete` for one hold, and the WAL fold after a forget.

use rusqlite::Connection;

/// Turns `secure_delete` on for as long as it lives, then puts back what
/// the connection had: SQLite zeroes what each delete frees (the cell on
/// its page and every freed page), so a removed row's words aren't left
/// in the file. Every write of the memory store and the log runs inside
/// one, so an FTS segment merge during an ordinary write frees its old
/// pages zeroed too. It also turns `recursive_triggers` on: a `REPLACE`
/// conflict then fires the delete triggers that keep the FTS index in step
/// (the security review's L3).
pub(super) struct SecureHold<'c> {
    conn: &'c Connection,
    before: i64,
    recursive_before: i64,
}

impl<'c> SecureHold<'c> {
    /// Switches it on.
    pub(super) fn on(conn: &'c Connection) -> rusqlite::Result<Self> {
        let before: i64 = conn.query_row("PRAGMA secure_delete", [], |row| row.get(0))?;
        conn.pragma_update_and_check(None, "secure_delete", 1, |row| row.get::<_, i64>(0))?;
        let recursive_before: i64 =
            conn.query_row("PRAGMA recursive_triggers", [], |row| row.get(0))?;
        conn.pragma_update(None, "recursive_triggers", 1)?;
        Ok(Self {
            conn,
            before,
            recursive_before,
        })
    }
}

impl Drop for SecureHold<'_> {
    fn drop(&mut self) {
        let _ = self
            .conn
            .pragma_update_and_check(None, "secure_delete", self.before, |row| {
                row.get::<_, i64>(0)
            });
        let _ = self
            .conn
            .pragma_update(None, "recursive_triggers", self.recursive_before);
    }
}

/// Folds the WAL into the file and truncates it (`wal_checkpoint(TRUNCATE)`),
/// so the frames that held a forgotten row's words are gone from disk.
/// `false` when another connection's open read kept it from finishing
/// (the frames stay until the next full checkpoint; the next forget or
/// maintenance pass tries again). An in-memory database has no WAL: `true`.
pub(super) fn fold_wal(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|busy| busy == 0)
}
