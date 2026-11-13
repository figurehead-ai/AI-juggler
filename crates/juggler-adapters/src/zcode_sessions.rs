//! `ZCode`'s sessions, from its own store, read-only (AJ-E1605 Req.3).
//!
//! `ZCode`'s engine keeps every session in one SQLite file,
//! `<tree>/cli/db/db.sqlite` (the engine's `storage.sessionDbPath`; the
//! schema read on engine 0.16.9, research `docs/research/zcode.md` § C).
//! A `session` row has `id` (`sess_…`, what `--resume` takes), `title`,
//! `directory` (the folder it ran in), `parent_id` (a sub-agent's or a
//! fork's parent), `task_type`, and `time_created`, `time_updated`,
//! `time_archived` in epoch ms. The sessions listed are the ones `ZCode`'s
//! own resume list shows: interactive, not archived, and not a sub-agent's.
//!
//! Every read goes through `common::sqlite::open_tool_store`: the
//! write-ahead log joined while `ZCode` runs, the file opened `immutable`
//! when it doesn't, never a byte written beside it. No query names a
//! table that holds a sign-in.
//!
//! A session has no transcript file, so its link is a reference into the
//! store, `<db>#<id>` (`common::sqlite::store_ref`), as for `OpenCode`.

use std::path::{Path, PathBuf};

/// The store's file name.
pub const DB_FILE: &str = "db.sqlite";

/// The store inside a `ZCode` tree (the `.zcode` folder).
#[must_use]
pub fn db_of(tree: &Path) -> PathBuf {
    tree.join("cli").join("db").join(DB_FILE)
}

/// One `ZCode` session, as the list and the linker need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZCodeSession {
    /// `ZCode`'s session id (`sess_…`).
    pub id: String,
    /// Its title (may be empty).
    pub title: String,
    /// The folder it ran in.
    pub directory: String,
    /// Created, epoch ms.
    pub created_ms: i64,
    /// Last updated, epoch ms.
    pub updated_ms: i64,
}

/// Every session `ZCode`'s own list shows (interactive, not archived, not
/// a sub-agent's), newest first.
///
/// # Errors
///
/// A plain message when the store can't be opened or read.
pub fn sessions(db: &Path) -> Result<Vec<ZCodeSession>, String> {
    let conn = crate::common::sqlite::open_tool_store(db, "ZCode")?;
    let mut stmt = conn
        .prepare(
            "SELECT id, COALESCE(title, ''), COALESCE(directory, ''),
                    time_created, time_updated
             FROM session
             WHERE task_type = 'interactive'
               AND time_archived IS NULL
               AND (parent_id IS NULL OR parent_id = '')
             ORDER BY time_updated DESC",
        )
        .map_err(|error| format!("zcode session query: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ZCodeSession {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                created_ms: row.get(3)?,
                updated_ms: row.get(4)?,
            })
        })
        .map_err(|error| format!("zcode session rows: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("zcode session row: {error}")))
        .collect()
}

/// Every session's id and last change, newest first, for the watcher.
pub(crate) fn session_changes(db: &Path) -> Result<Vec<(String, i64)>, String> {
    Ok(sessions(db)?
        .into_iter()
        .map(|session| (session.id, session.updated_ms))
        .collect())
}

/// The link reference for a session: `<db>#<id>`.
#[must_use]
pub fn session_ref(db: &Path, id: &str) -> PathBuf {
    crate::common::sqlite::store_ref(db, id)
}

/// The account's conversations for the index (AJ-E1529), keyed by
/// [`session_ref`]; their folders become projects. Nothing when the store
/// can't be read. Tokens aren't read per session yet (Req.4), so they are
/// unknown here, never claimed.
#[must_use]
pub fn stored_conversations(db: &Path) -> Vec<juggler_adapter::conversation::StoredConversation> {
    let Ok(sessions) = sessions(db) else {
        return Vec::new();
    };
    sessions
        .into_iter()
        .map(
            |session| juggler_adapter::conversation::StoredConversation {
                key: session_ref(db, &session.id),
                changed_ms: session.updated_ms,
                summary: juggler_adapter::conversation::ConversationSummary {
                    cwd: (!session.directory.is_empty()).then(|| PathBuf::from(&session.directory)),
                    git_branch: None,
                    started_in: None,
                    by_script: false,
                    title: juggler_adapter::conversation::prompt_title(&session.title, 120),
                    opening: juggler_adapter::conversation::title_from(&session.title, 120),
                    started_ms: Some(session.created_ms),
                    last_ms: Some(session.updated_ms),
                    tokens: juggler_core::model::TokenCounts::default(),
                    id: session.id,
                },
            },
        )
        .collect()
}

/// The one session that ran in `cwd`, created at or after `since_ms`, and
/// not already linked (`taken`). Two candidates is an honest `None`, never
/// a guess.
#[must_use]
pub fn locate(
    db: &Path,
    cwd: &Path,
    since_ms: i64,
    taken: &[PathBuf],
) -> Option<(PathBuf, String)> {
    let mut found = sessions(db)
        .ok()?
        .into_iter()
        .filter(|session| session.created_ms >= since_ms.saturating_sub(2_000))
        .filter(|session| crate::claude_code::same_folder(&session.directory, cwd))
        .map(|session| (session_ref(db, &session.id), session.id))
        .filter(|(reference, _)| !taken.contains(reference));
    let first = found.next()?;
    found.next().is_none().then_some(first)
}
