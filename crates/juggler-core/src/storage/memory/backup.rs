//! The backup hooks for D-133's backup ruling (ADR-0037 §1.8): which rows
//! go where, the forget tombstones, and the scrub of every copy AI Juggler
//! keeps on its own.
//!
//! | Rows | Export (the user's file) | AI Juggler's own copies | Cloud backup |
//! |---|---|---|---|
//! | stated settings: a watch, or a preference naming an entity in a short label, by the user, trusted | yes | no | yes |
//! | every other record; conversations and turns; the log's sidecars | yes | no | no |
//! | the log's spine (ids and codes) | yes | yes | no |
//! | `memory_forgotten` (this Mac's tombstones) | no | no | no |
//!
//! "AI Juggler's own copies" are the backup kept before an update and the
//! copies kept before a migration or a restore: no forget made later could
//! reach them, so they hold no memory at all (going back costs memory, the
//! safe direction). Every scrub ends with the FTS index built fresh from
//! what is left, so no page separator of a removed row travels.
//!
//! Each hook is a no-op on a database without V54's tables, so the app's
//! backup calls them unconditionally.

use rusqlite::Connection;

use super::erase::{erase_subject, reindex};
use super::schema::{installed, installed_in};
use super::secure::{SecureHold, fold_wal};
use crate::error::CoreError;
use crate::storage::{open_hardened, write_transaction};

/// Where a backup goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    /// The user's export: everything but the forgotten and the tombstones.
    Local,
    /// A copy AI Juggler keeps on its own (the backup before an update):
    /// no memory, conversations or sidecars.
    OwnCopy,
    /// The encrypted cloud backup: the stated settings only, written fresh.
    Cloud,
}

/// A stated setting, the only memory that may go to the cloud (§1.8, and
/// the security review's M2 and M3): a watch the user confirmed, or a
/// preference that names an entity in a short label; by the user, trusted,
/// from a source a setting comes from.
fn stated_setting(r: &str) -> String {
    format!(
        "({r}.authority = 'user' AND {r}.untrusted = 0 AND (
             ({r}.kind = 'standing_request' AND {r}.source_kind IN ('watches_view', 'chat_turn'))
          OR ({r}.kind = 'preference' AND {r}.source_kind IN ('settings', 'chat_turn')
              AND length({r}.text) <= {max}
              AND EXISTS (SELECT 1 FROM main.memory_entities e WHERE e.record_id = {r}.id))))",
        max = super::types::PREFERENCE_MAX_CHARS
    )
}

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

/// Erases every subject this database's tombstones name (a copy made
/// before a forget, or restored from one). Returns the versions dropped.
fn drop_tombstoned(conn: &Connection) -> rusqlite::Result<usize> {
    let subjects: Vec<String> = conn
        .prepare(
            "SELECT DISTINCT r.subject_id FROM main.memory_records r
             JOIN main.memory_forgotten f ON f.subject_id = r.subject_id",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut versions = 0;
    for subject in &subjects {
        versions += erase_subject(conn, subject)?.versions;
    }
    // A sidecar about a forgotten subject whose record is long gone.
    conn.execute(
        "DELETE FROM main.ai_log_text
         WHERE memory_subject IN (SELECT subject_id FROM main.memory_forgotten)",
        [],
    )?;
    Ok(versions)
}

/// Empties the memory, the conversations, the links and the sidecars
/// (each table by name: the cascades aren't relied on).
fn empty_memory(conn: &Connection) -> rusqlite::Result<usize> {
    let records = conn.execute("DELETE FROM memory_records", [])?;
    conn.execute_batch(
        "DELETE FROM memory_entities;
         DELETE FROM memory_forgotten;
         DELETE FROM chat_turn_memory;
         DELETE FROM chat_turns;
         DELETE FROM chat_conversations;
         DELETE FROM ai_log_text;",
    )?;
    Ok(records)
}

/// Scrubs a backup's database snapshot (not the live database) for
/// `dest`, with `secure_delete` on; the caller compacts the file
/// afterwards (`VACUUM`), so the copy is rebuilt from what is left.
///
/// - Every destination: forgotten subjects and the tombstones go, and the
///   FTS index is built fresh from the rows left.
/// - [`Destination::OwnCopy`]: no memory at all.
/// - [`Destination::Cloud`]: the stated settings are read out, every
///   memory table is emptied, and they are written back as new rows into
///   an empty index: the cloud copy's memory is a filtered export, never
///   what the source file held.
///
/// # Errors
///
/// Storage failures.
pub fn scrub_snapshot(conn: &Connection, dest: Destination) -> Result<Scrubbed, CoreError> {
    if !installed(conn)? {
        return Ok(Scrubbed::default());
    }
    // The stated settings are held in memory while the tables are emptied,
    // never in a temporary file.
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    let _hold = SecureHold::on(conn)?;
    let tx = write_transaction(conn)?;
    let mut out = Scrubbed::default();
    out.records += drop_tombstoned(&tx)?;
    tx.execute("DELETE FROM memory_forgotten", [])?;
    match dest {
        Destination::Local => {}
        Destination::OwnCopy => {
            out.conversations += count(&tx, "chat_conversations")?;
            out.records += empty_memory(&tx)?;
        }
        Destination::Cloud => {
            tx.execute_batch(&format!(
                "CREATE TEMP TABLE cloud_records AS
                     SELECT * FROM main.memory_records r WHERE {stated}
                       AND r.subject_key NOT IN (
                           SELECT c.subject_key FROM main.memory_records c
                           WHERE c.superseded_by IS NULL AND NOT {current});
                 CREATE TEMP TABLE cloud_entities AS
                     SELECT * FROM main.memory_entities
                     WHERE record_id IN (SELECT id FROM temp.cloud_records);
                 UPDATE temp.cloud_records SET supersedes = NULL
                     WHERE supersedes NOT IN (SELECT id FROM temp.cloud_records);",
                stated = stated_setting("r"),
                current = stated_setting("c"),
            ))?;
            let kept = count(&tx, "temp.cloud_records")?;
            out.conversations += count(&tx, "chat_conversations")?;
            out.records += empty_memory(&tx)?.saturating_sub(kept);
            // The spine goes as Clear AI activity does: each stream's
            // watermark past its newest row (never back), then the rows.
            tx.execute(
                "INSERT INTO ai_log_prune (stream, before_ms)
                     SELECT stream, MAX(at_ms) + 1 FROM ai_log WHERE true GROUP BY stream
                 ON CONFLICT (stream) DO UPDATE SET before_ms = MAX(before_ms, excluded.before_ms)",
                [],
            )?;
            out.log_rows += tx.execute("DELETE FROM ai_log", [])?;
            tx.execute(
                "INSERT INTO memory_fts(memory_fts) VALUES ('delete-all')",
                [],
            )?;
            tx.execute_batch(
                "INSERT INTO main.memory_records SELECT * FROM temp.cloud_records;
                 INSERT INTO main.memory_entities SELECT * FROM temp.cloud_entities;
                 DROP TABLE temp.cloud_records;
                 DROP TABLE temp.cloud_entities;",
            )?;
        }
    }
    reindex(&tx)?;
    tx.commit()?;
    Ok(out)
}

fn count(conn: &Connection, table: &str) -> rusqlite::Result<usize> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|n| usize::try_from(n).unwrap_or(0))
}

/// Applies this Mac's tombstones to a database about to be restored, or
/// just restored: the tombstones of the database it replaces (attached as
/// `prior`) are kept, and every subject they name is erased from it (its
/// turns redacted too), so restoring a backup made before a forget doesn't
/// bring the forgotten memory back. Then the index is rebuilt and the
/// WAL folded. Returns the record versions dropped.
///
/// # Errors
///
/// Storage failures.
pub fn after_restore(conn: &Connection, prior: &str) -> Result<usize, CoreError> {
    if !installed(conn)? {
        return Ok(0);
    }
    let dropped = {
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
        let dropped = drop_tombstoned(&tx)?;
        if dropped > 0 {
            reindex(&tx)?;
        }
        tx.commit()?;
        dropped
    };
    fold_wal(conn)?;
    Ok(dropped)
}

/// Empties the memory, the conversations and the log's sidecars in a copy
/// AI Juggler keeps on its own (`<file>.before-v<N>`, a before-restore
/// copy) before it is kept (ADR-0037 §1.5): a forget made later couldn't
/// reach it. The log's spine stays (it holds no words). A copy without the
/// tables is left as it is.
///
/// # Errors
///
/// The SQLite error when the copy can't be opened or written.
pub fn scrub_copy(path: &std::path::Path) -> rusqlite::Result<()> {
    let conn = open_hardened(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    if !installed(&conn)? {
        return Ok(());
    }
    {
        let _hold = SecureHold::on(&conn)?;
        let tx =
            rusqlite::Transaction::new_unchecked(&conn, rusqlite::TransactionBehavior::Immediate)?;
        empty_memory(&tx)?;
        reindex(&tx)?;
        tx.commit()?;
    }
    fold_wal(&conn)?;
    Ok(())
}
