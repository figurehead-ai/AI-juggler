//! Hermes Agent's sessions, from its own store, read-only (AJ-E1607).
//!
//! Hermes keeps every session in one SQLite file, `$HERMES_HOME/state.db`
//! (WAL), written only by the home's gateway (`hermes_state.py`,
//! `DEFAULT_DB_PATH`; research `hermes.md` §4). Its schema, read in the
//! MIT source at `NousResearch/hermes-agent@422ad186`
//! (`hermes_state_common.py`, `SCHEMA_SQL`, `SCHEMA_VERSION = 33`): a
//! `schema_version` row and a `sessions` row per session with `id`,
//! `source` (`cli`, `tui`, `telegram`, …), `title`, `cwd`, `started_at` /
//! `last_activity_at` / `ended_at` (epoch seconds, REAL),
//! `parent_session_id` (a sub-agent's or a branch's parent), `hidden`, and
//! its token totals; `messages` holds each turn (`role`, `content`).
//!
//! Hermes migrates its schema often, so a read checks the columns it reads
//! and a schema at or past the one read ([`format_check`]): anything else
//! refuses the read whole, never half reads it. Every read goes through
//! `common::sqlite::open_tool_store`: the log joined while Hermes runs,
//! the file opened `immutable` when it doesn't, nothing written beside it.

use std::path::{Path, PathBuf};

/// The store's file name in a Hermes home.
pub const STORE_FILE: &str = "state.db";

/// The schema version this reader was built from.
pub const SCHEMA_READ: i64 = 33;

/// The `sessions` columns read.
const COLUMNS: &[&str] = &[
    "id",
    "title",
    "cwd",
    "started_at",
    "last_activity_at",
    "ended_at",
    "parent_session_id",
    "hidden",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_tokens",
];

/// The store of the Hermes home `home`.
#[must_use]
pub fn store_of(home: &Path) -> PathBuf {
    home.join(STORE_FILE)
}

/// Whether the store is in a format this reader knows: a schema version at
/// or past [`SCHEMA_READ`], and every column read.
///
/// # Errors
///
/// A plain message naming what differs.
pub fn format_check(conn: &rusqlite::Connection) -> Result<(), String> {
    let version: Option<i64> = conn
        .query_row("SELECT max(version) FROM schema_version", [], |row| {
            row.get(0)
        })
        .ok()
        .flatten();
    if version.is_none_or(|version| version < SCHEMA_READ) {
        return Err(format!(
            "Hermes Agent's session store is older than AI Juggler reads (schema {}; read {SCHEMA_READ})",
            version.map_or_else(|| "unknown".to_owned(), |version| version.to_string())
        ));
    }
    let missing: Vec<&str> = {
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('sessions')")
            .map_err(|error| format!("hermes store columns: {error}"))?;
        let have: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|error| format!("hermes store columns: {error}"))?
            .filter_map(Result::ok)
            .collect();
        COLUMNS
            .iter()
            .copied()
            .filter(|column| !have.iter().any(|name| name == column))
            .collect()
    };
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Hermes Agent's session store lacks {}: its format changed",
            missing.join(", ")
        ))
    }
}

/// One Hermes session, with its token totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HermesSession {
    /// The session as the list and the linker read it.
    pub session: crate::common::sqlite::StoreSession,
    /// Its token totals, as Hermes counted them.
    pub tokens: juggler_core::model::TokenCounts,
}

/// Seconds (Hermes' REAL epoch) as epoch milliseconds.
#[allow(
    clippy::cast_possible_truncation,
    reason = "an epoch in ms fits an i64 for any date Hermes writes"
)]
fn ms(seconds: f64) -> i64 {
    (seconds * 1_000.0).round() as i64
}

/// Every root session in the store (not a sub-agent's or a branch's, not
/// one Hermes hides), newest first. Its title is Hermes' own, else its
/// first prompt.
///
/// # Errors
///
/// A plain message when the store can't be opened or read, or isn't in
/// the format read ([`format_check`]).
pub fn sessions(store: &Path) -> Result<Vec<HermesSession>, String> {
    let conn = crate::common::sqlite::open_tool_store(store, "Hermes Agent")?;
    format_check(&conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id,
                    COALESCE(NULLIF(s.title, ''),
                             (SELECT m.content FROM messages m
                              WHERE m.session_id = s.id AND m.role = 'user'
                              ORDER BY m.id LIMIT 1), ''),
                    COALESCE(s.cwd, ''),
                    s.started_at,
                    COALESCE(s.last_activity_at, s.ended_at, s.started_at),
                    COALESCE(s.input_tokens, 0), COALESCE(s.output_tokens, 0),
                    COALESCE(s.cache_read_tokens, 0), COALESCE(s.cache_write_tokens, 0),
                    COALESCE(s.reasoning_tokens, 0)
             FROM sessions s
             WHERE (s.parent_session_id IS NULL OR s.parent_session_id = '')
               AND COALESCE(s.hidden, 0) = 0
             ORDER BY COALESCE(s.last_activity_at, s.ended_at, s.started_at) DESC",
        )
        .map_err(|error| format!("hermes session query: {error}"))?;
    let count = |row: &rusqlite::Row<'_>, at: usize| -> rusqlite::Result<u64> {
        Ok(u64::try_from(row.get::<_, i64>(at)?).unwrap_or(0))
    };
    let rows = stmt
        .query_map([], |row| {
            Ok(HermesSession {
                session: crate::common::sqlite::StoreSession {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    directory: row.get(2)?,
                    created_ms: ms(row.get(3)?),
                    updated_ms: ms(row.get(4)?),
                },
                tokens: juggler_core::model::TokenCounts {
                    input: count(row, 5)?,
                    output: count(row, 6)?,
                    cache_read: count(row, 7)?,
                    cache_write: count(row, 8)?,
                    reasoning: count(row, 9)?,
                },
            })
        })
        .map_err(|error| format!("hermes session rows: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("hermes session row: {error}")))
        .collect()
}

/// Every session's id and last change, newest first, for the watcher.
pub(crate) fn session_changes(store: &Path) -> Result<Vec<(String, i64)>, String> {
    sessions(store)
        .map(|found| crate::common::sqlite::changes(found.into_iter().map(|one| one.session)))
}

/// The link reference for a session: `<store>#<id>`.
#[must_use]
pub fn session_ref(store: &Path, id: &str) -> PathBuf {
    crate::common::sqlite::store_ref(store, id)
}

/// The account's conversations (AJ-E1529), keyed by [`session_ref`], each
/// with Hermes' own token totals; their folders become projects. Nothing
/// when the store can't be read or its format changed.
#[must_use]
pub fn stored_conversations(
    store: &Path,
) -> Vec<juggler_adapter::conversation::StoredConversation> {
    sessions(store)
        .map(|found| {
            found
                .into_iter()
                .map(|one| one.session.into_stored(store, one.tokens))
                .collect()
        })
        .unwrap_or_default()
}

/// The launch's session in the Hermes home `home`: the one that ran in
/// `cwd`, started at or after `since_ms` and not already linked (`taken`),
/// as its link `<store>#<id>`. Two candidates is an honest `None`.
#[must_use]
pub fn launched(
    home: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<juggler_core::model::TranscriptRef> {
    let (path, id) = locate(&store_of(home), cwd, since_ms, taken)?;
    Some(juggler_core::model::TranscriptRef {
        native_id: Some(id),
        path,
    })
}

/// The one session that ran in `cwd`, created at or after `since_ms`, and
/// not already linked (`taken`). Two candidates is an honest `None`.
#[must_use]
pub fn locate(
    store: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<(PathBuf, String)> {
    let found = sessions(store).ok()?;
    crate::common::sqlite::locate_one(
        store,
        cwd,
        since_ms,
        taken,
        found.into_iter().map(|one| one.session),
    )
}
