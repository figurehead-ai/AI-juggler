//! `PRAGMA secure_delete` for one hold, and the WAL fold after a forget,
//! retried until it lands (the security review's L1).

use std::collections::BTreeSet;
use std::sync::Mutex;

use rusqlite::Connection;

/// The database files whose WAL fold another connection's read blocked:
/// the next write of the store or the log on that file folds it again
/// (`SecureHold`'s drop), and [`retry_pending_fold`] does so on demand
/// (maintenance's idle pass, step 3).
static PENDING_FOLDS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// `PRAGMA journal_size_limit` on every connection the store writes
/// through: 0, so each time the WAL restarts after a checkpoint it is cut
/// to nothing, and frames from before a forget can't linger past the new
/// write point (the security review's L1).
pub(super) const JOURNAL_SIZE_LIMIT: i64 = 0;

fn file_of(conn: &Connection) -> Option<String> {
    conn.path()
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
}

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
        conn.pragma_update_and_check(None, "journal_size_limit", JOURNAL_SIZE_LIMIT, |row| {
            row.get::<_, i64>(0)
        })?;
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
        // A fold an earlier forget couldn't finish is tried again now.
        let _ = retry_pending_fold(self.conn);
    }
}

/// Folds the WAL into the file and truncates it (`wal_checkpoint(TRUNCATE)`),
/// so the frames that held a forgotten row's words are gone from disk.
/// `false` when another connection's open read kept it from finishing:
/// the file is then marked, and the next write of the store or the log on
/// it, or [`retry_pending_fold`], folds it again. An in-memory database
/// has no WAL: `true`.
pub(super) fn fold_wal(conn: &Connection) -> rusqlite::Result<bool> {
    let folded = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|busy| busy == 0)?;
    if let (Some(file), Ok(mut pending)) = (file_of(conn), PENDING_FOLDS.lock()) {
        if folded {
            pending.remove(&file);
        } else {
            pending.insert(file);
        }
    }
    Ok(folded)
}

/// Folds the WAL again if an earlier fold on this database was blocked.
/// `true` when nothing is pending any more.
///
/// # Errors
///
/// The SQLite error of the checkpoint.
pub fn retry_pending_fold(conn: &Connection) -> rusqlite::Result<bool> {
    let pending = file_of(conn).is_some_and(|file| {
        PENDING_FOLDS
            .lock()
            .is_ok_and(|pending| pending.contains(&file))
    });
    if pending { fold_wal(conn) } else { Ok(true) }
}
