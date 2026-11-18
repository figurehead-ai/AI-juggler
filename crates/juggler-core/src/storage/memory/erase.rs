//! Erasing one subject everywhere its words are (ADR-0037 §1.5), shared by
//! forget, the backup scrubs and a restore's tombstone pass. Runs inside
//! the caller's transaction and `SecureHold`.

use rusqlite::{Connection, params};

/// A conversation's title when no turn is left to take one from.
pub const NEUTRAL_TITLE: &str = "Earlier conversation";
/// The most characters of a turn's question a title takes.
const TITLE_CHARS: i64 = 80;

/// What erasing one subject removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Erased {
    pub(super) versions: usize,
    pub(super) sidecars: usize,
    pub(super) turns: usize,
    pub(super) titles: usize,
}

/// Erases the subject `subject_id`:
/// 1. links the turns its versions came from (`source_kind = chat_turn`);
/// 2. erases the log sidecars about it: by its subject id, by an event
///    naming one of its versions, and by an event naming one of its turns;
/// 3. redacts those turns' words (`question` emptied, `shown` cleared,
///    marked `redacted`) and renames every conversation whose title came
///    from one of them, from its first turn left, else [`NEUTRAL_TITLE`];
/// 4. deletes the links and every version (entities by cascade, index
///    entries by trigger).
pub(super) fn erase_subject(conn: &Connection, subject_id: &str) -> rusqlite::Result<Erased> {
    let mut out = Erased::default();
    conn.execute(
        "INSERT OR IGNORE INTO chat_turn_memory (turn_id, subject_id)
         SELECT t.id, r.subject_id FROM memory_records r JOIN chat_turns t ON t.id = r.source_id
         WHERE r.subject_id = ?1 AND r.source_kind = 'chat_turn'",
        [subject_id],
    )?;
    out.sidecars = conn.execute(
        "DELETE FROM ai_log_text WHERE memory_subject = ?1
            OR seq IN (SELECT l.seq FROM ai_log l JOIN memory_records r
                         ON l.subject_kind = 'memory' AND l.subject_id = r.id
                       WHERE r.subject_id = ?1)
            OR seq IN (SELECT l.seq FROM ai_log l JOIN chat_turn_memory m
                         ON l.subject_kind = 'turn' AND l.subject_id = m.turn_id
                       WHERE m.subject_id = ?1)",
        [subject_id],
    )?;
    out.turns = conn.execute(
        "UPDATE chat_turns SET question = '', shown = NULL, redacted = 1
         WHERE redacted = 0
           AND id IN (SELECT turn_id FROM chat_turn_memory WHERE subject_id = ?1)",
        [subject_id],
    )?;
    out.titles = conn.execute(
        "UPDATE chat_conversations AS c SET
             title = COALESCE(
                 (SELECT substr(t.question, 1, ?2) FROM chat_turns t
                  WHERE t.conversation_id = c.id AND t.redacted = 0 ORDER BY t.seq LIMIT 1),
                 ?3),
             title_turn = (SELECT t.id FROM chat_turns t
                           WHERE t.conversation_id = c.id AND t.redacted = 0
                           ORDER BY t.seq LIMIT 1)
         WHERE c.title_turn IN (SELECT turn_id FROM chat_turn_memory WHERE subject_id = ?1)",
        params![subject_id, TITLE_CHARS, NEUTRAL_TITLE],
    )?;
    conn.execute(
        "DELETE FROM chat_turn_memory WHERE subject_id = ?1",
        [subject_id],
    )?;
    out.versions = conn.execute(
        "DELETE FROM memory_records WHERE subject_id = ?1",
        [subject_id],
    )?;
    Ok(out)
}

/// Rewrites the FTS index into one segment (`optimize`). `secure-delete`
/// removes a deleted row's entries from the leaves but leaves the page
/// separators in `memory_fts_idx`, each the prefix of a page's first term
/// (the security review's H1); a rewrite makes new separators from the
/// terms left, and the old pages are freed zeroed (`SecureHold`).
pub(super) fn optimize_index(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO memory_fts(memory_fts) VALUES ('optimize')", [])
        .map(drop)
}

/// Drops the whole index and builds it fresh from the current, trusted
/// rows (a backup's copy: nothing of the source file's index travels).
pub(super) fn reindex(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "INSERT INTO memory_fts(memory_fts) VALUES ('delete-all');
         INSERT INTO memory_fts(rowid, text)
             SELECT seq, text FROM memory_records
             WHERE superseded_by IS NULL AND untrusted = 0;",
    )
}
