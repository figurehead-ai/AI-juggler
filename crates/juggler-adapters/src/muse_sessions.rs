//! Muse Code's sessions, from its own index, read-only (AJ-E1608 R3).
//!
//! Muse keeps each session as an append-only log,
//! `<data>/sessions/YYYY/MM/DD/<uuid>/session.jsonl`, and lists them in one
//! SQLite index, `<data>/session-index.db` (WAL), where `<data>` is
//! `$XDG_DATA_HOME/muse`. The index's schema (read on 1.4.13, in a
//! throwaway home's own index): `schema_meta` (`schema_version` `1`) and a
//! `sessions` row per session with `session_id`, `session_log_path`,
//! `workspace_root`, `title`, `first_user_prompt`, `created_at_us`,
//! `updated_at_us` (epoch µs), `status`, and `msp_kind` /
//! `msp_parent_session_id` (a sub-agent's).
//!
//! The index carries no stability promise, so every read checks its format
//! first ([`format_check`]): a schema version other than the one read, or a
//! missing column, refuses the read whole, never half reads it. A session
//! AI Juggler drives is listed by Muse's own `session/list` over its
//! protocol ([`crate::muse_serve::codec`]); this reader is for every other.
//!
//! Every read goes through `common::sqlite::open_tool_store`: the
//! write-ahead log joined while Muse runs, the file opened `immutable`
//! when it doesn't, never a byte written beside it.

use std::path::{Path, PathBuf};

/// The index's file name.
pub const INDEX_FILE: &str = "session-index.db";

/// The index schema version this reader was built for.
pub const SCHEMA_VERSION: &str = "1";

/// The columns read.
const COLUMNS: &[&str] = &[
    "session_id",
    "workspace_root",
    "title",
    "first_user_prompt",
    "created_at_us",
    "updated_at_us",
    "status",
    "msp_kind",
    "msp_parent_session_id",
];

/// The index inside Muse's data folder (`$XDG_DATA_HOME/muse`).
#[must_use]
pub fn index_of(data: &Path) -> PathBuf {
    data.join(INDEX_FILE)
}

/// One Muse session, as the list and the linker need it.
pub type MuseSession = crate::common::sqlite::StoreSession;

/// Whether the index is in the format this reader knows: its schema
/// version and every column read.
///
/// # Errors
///
/// A plain message naming what differs.
pub fn format_check(conn: &rusqlite::Connection) -> Result<(), String> {
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .ok();
    if version.as_deref() != Some(SCHEMA_VERSION) {
        return Err(format!(
            "Muse Code's session index is in a format AI Juggler doesn't know (schema {}; read {SCHEMA_VERSION})",
            version.as_deref().unwrap_or("unknown")
        ));
    }
    let mut stmt = conn
        .prepare("SELECT name FROM pragma_table_info('sessions')")
        .map_err(|error| format!("muse index columns: {error}"))?;
    let have: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|error| format!("muse index columns: {error}"))?
        .filter_map(Result::ok)
        .collect();
    let missing: Vec<&str> = COLUMNS
        .iter()
        .copied()
        .filter(|column| !have.iter().any(|name| name == column))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Muse Code's session index lacks {}: its format changed",
            missing.join(", ")
        ))
    }
}

/// Every root session in the index (not a sub-agent's), newest first. Its
/// title is Muse's own, else its first prompt.
///
/// # Errors
///
/// A plain message when the index can't be opened or read, or isn't in
/// the format read ([`format_check`]).
pub fn sessions(index: &Path) -> Result<Vec<MuseSession>, String> {
    let conn = crate::common::sqlite::open_tool_store(index, "Muse Code")?;
    format_check(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT session_id,
                    COALESCE(NULLIF(title, ''), first_user_prompt, ''),
                    COALESCE(workspace_root, ''),
                    COALESCE(created_at_us, 0), COALESCE(updated_at_us, created_at_us, 0)
             FROM sessions
             WHERE (msp_kind IS NULL OR msp_kind = 'root')
               AND (msp_parent_session_id IS NULL OR msp_parent_session_id = '')
             ORDER BY updated_at_us DESC",
        )
        .map_err(|error| format!("muse session query: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MuseSession {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                created_ms: row.get::<_, i64>(3)? / 1_000,
                updated_ms: row.get::<_, i64>(4)? / 1_000,
            })
        })
        .map_err(|error| format!("muse session rows: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("muse session row: {error}")))
        .collect()
}

/// Every session's id and last change, newest first, for the watcher.
pub(crate) fn session_changes(index: &Path) -> Result<Vec<(String, i64)>, String> {
    sessions(index).map(crate::common::sqlite::changes)
}

/// The link reference for a session: `<index>#<id>`.
#[must_use]
pub fn session_ref(index: &Path, id: &str) -> PathBuf {
    crate::common::sqlite::store_ref(index, id)
}

/// The account's conversations for the index (AJ-E1529), keyed by
/// [`session_ref`]; their folders become projects. Nothing when the index
/// can't be read or its format changed. No tokens: Muse's index keeps none.
#[must_use]
pub fn stored_conversations(
    index: &Path,
) -> Vec<juggler_adapter::conversation::StoredConversation> {
    sessions(index)
        .map(|sessions| {
            sessions
                .into_iter()
                .map(|session| {
                    session.into_stored(index, juggler_core::model::TokenCounts::default())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The one session that ran in `cwd`, created at or after `since_ms`, and
/// not already linked (`taken`). Two candidates is an honest `None`.
#[must_use]
pub fn locate(
    index: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<(PathBuf, String)> {
    crate::common::sqlite::locate_one(index, cwd, since_ms, taken, sessions(index).ok()?)
}
