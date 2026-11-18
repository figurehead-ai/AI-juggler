//! The backup hooks for D-133's backup ruling (ADR-0037 §1.8): which rows
//! go where, the forget tombstones, and the pre-migration copy's scrub.
//!
//! | Rows | Local backup | Cloud backup |
//! |---|---|---|
//! | `preference` / `standing_request` versions by the user (the stated settings), with their entities | yes | yes |
//! | every other record; conversations and turns; the log, its sidecars, its watermarks | yes | no |
//! | `memory_forgotten` (this Mac's tombstones) | no | no |
//!
//! Each hook is a no-op on a database without V54's tables, so the app's
//! backup calls them unconditionally.

use rusqlite::Connection;

use super::schema::{installed, installed_in};
use super::secure::{SecureHold, fold_wal};
use crate::error::CoreError;
use crate::storage::{open_hardened, write_transaction};

/// Where a backup goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    /// A backup file on this Mac's side: the user's export, the copy kept
    /// before an update. Everything but the tombstones.
    Local,
    /// The encrypted cloud backup: the stated settings only.
    Cloud,
}

/// The SQL test for a stated setting (a row that may go to the cloud).
const STATED_SETTING: &str = "kind IN ('preference', 'standing_request') AND authority = 'user'";

/// What a scrub removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scrubbed {
    /// Record versions removed.
    pub records: usize,
    /// Conversations removed (their turns with them).
    pub conversations: usize,
    /// Log spine rows removed.
    pub log_rows: usize,
}

/// Scrubs a backup's database snapshot (not the live database) for
/// `dest`: forgotten subjects and the tombstones always go; for the
/// cloud, everything but the stated settings goes too. With
/// `secure_delete` on; the caller compacts the file afterwards (`VACUUM`).
///
/// # Errors
///
/// Storage failures.
pub fn scrub_snapshot(conn: &Connection, dest: Destination) -> Result<Scrubbed, CoreError> {
    if !installed(conn)? {
        return Ok(Scrubbed::default());
    }
    let _hold = SecureHold::on(conn)?;
    let tx = write_transaction(conn)?;
    let mut out = Scrubbed::default();
    tx.execute(
        "DELETE FROM ai_log_text WHERE memory_subject IN (SELECT subject_id FROM memory_forgotten)",
        [],
    )?;
    out.records += tx.execute(
        "DELETE FROM memory_records WHERE subject_id IN (SELECT subject_id FROM memory_forgotten)",
        [],
    )?;
    tx.execute("DELETE FROM memory_forgotten", [])?;
    if dest == Destination::Cloud {
        // A subject goes only when its current version is a stated
        // setting, and then only its stated versions: an earlier
        // model-noted one stays here, and the newer version's link to it
        // is cut (`supersedes … ON DELETE SET NULL`).
        out.records += tx.execute(
            &format!(
                "DELETE FROM memory_records WHERE NOT ({STATED_SETTING})
                    OR subject_key IN (SELECT subject_key FROM memory_records
                                       WHERE superseded_by IS NULL AND NOT ({STATED_SETTING}))"
            ),
            [],
        )?;
        out.conversations += tx.execute("DELETE FROM chat_conversations", [])?;
        tx.execute("DELETE FROM ai_log_text", [])?;
        // The spine goes as Clear AI activity does: each stream's
        // watermark past its newest row (never back), then the rows.
        tx.execute(
            "INSERT INTO ai_log_prune (stream, before_ms)
                 SELECT stream, MAX(at_ms) + 1 FROM ai_log WHERE true GROUP BY stream
             ON CONFLICT (stream) DO UPDATE SET before_ms = MAX(before_ms, excluded.before_ms)",
            [],
        )?;
        out.log_rows += tx.execute("DELETE FROM ai_log", [])?;
    }
    tx.commit()?;
    Ok(out)
}

/// After a restore: this Mac's tombstones (from the database the restore
/// replaced, attached as `prior`) are kept, and every subject they name is
/// dropped from what was restored, so restoring a backup made before a
/// forget doesn't bring the forgotten memory back. Returns the record
/// versions dropped.
///
/// # Errors
///
/// Storage failures.
pub fn after_restore(conn: &Connection, prior: &str) -> Result<usize, CoreError> {
    if !installed(conn)? {
        return Ok(0);
    }
    let _hold = SecureHold::on(conn)?;
    let tx = write_transaction(conn)?;
    if installed_in(&tx, prior)? {
        tx.execute(
            &format!(
                "INSERT OR IGNORE INTO main.memory_forgotten (subject_id, forgotten_ms)
                 SELECT subject_id, forgotten_ms FROM \"{}\".memory_forgotten",
                prior.replace('"', "")
            ),
            [],
        )?;
    }
    tx.execute(
        "DELETE FROM main.ai_log_text
         WHERE memory_subject IN (SELECT subject_id FROM main.memory_forgotten)",
        [],
    )?;
    let dropped = tx.execute(
        "DELETE FROM main.memory_records
         WHERE subject_id IN (SELECT subject_id FROM main.memory_forgotten)",
        [],
    )?;
    tx.commit()?;
    Ok(dropped)
}

/// Empties the memory, the conversations and the log's sidecars in a
/// pre-migration copy (`<file>.before-v<N>`) before it is flushed and
/// named (ADR-0037 §1.5): the copy is for going back to an older build,
/// and a forget made later couldn't reach it. The log's spine stays (it
/// holds no words). A copy without the tables is left as it is.
///
/// # Errors
///
/// The SQLite error when the copy can't be opened or written.
pub fn scrub_copy(path: &std::path::Path) -> rusqlite::Result<()> {
    let conn = open_hardened(path)?;
    if !installed(&conn)? {
        return Ok(());
    }
    {
        let _hold = SecureHold::on(&conn)?;
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM memory_records;
             DELETE FROM memory_forgotten;
             DELETE FROM chat_conversations;
             DELETE FROM ai_log_text;
             COMMIT;",
        )?;
    }
    fold_wal(&conn)?;
    Ok(())
}
