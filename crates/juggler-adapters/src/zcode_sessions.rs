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
/// can't be read. Each carries its tokens, summed from its own
/// `model_usage` rows (Req.4); a session without any shows none.
#[must_use]
pub fn stored_conversations(db: &Path) -> Vec<juggler_adapter::conversation::StoredConversation> {
    let Ok(sessions) = sessions(db) else {
        return Vec::new();
    };
    let mut totals = crate::zcode_usage::session_totals(db).unwrap_or_default();
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
                    tokens: totals.remove(&session.id).unwrap_or_default(),
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

/// One turn of a session as a handover reads it: who spoke and what they
/// said (its text parts, joined), oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZCodeTurn {
    /// `user` or `assistant`.
    pub role: &'static str,
    /// The turn's text.
    pub text: String,
    /// When the message was created, epoch ms.
    pub at_ms: i64,
}

/// The turns of session `id` (AJ-E1605 Req.6), read-only from its
/// `message` and `part` rows (engine 0.16.9: a message's `data` is a JSON
/// object with its `role`; a part's is one with its `type`, and a text
/// part's `text`). Tool calls, reasoning and the engine's own synthetic
/// text are left out; a message with no text is no turn.
///
/// # Errors
///
/// A plain message when the store can't be read, or when a row isn't in
/// the shape read on 0.16.9 (a message without a role, a text part
/// without its text): a changed format is refused whole, never half read.
pub fn turns(db: &Path, id: &str) -> Result<Vec<ZCodeTurn>, String> {
    let conn = crate::common::sqlite::open_tool_store(db, "ZCode")?;
    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.time_created, m.data, p.data
             FROM message m LEFT JOIN part p ON p.message_id = m.id
             WHERE m.session_id = ?1
             ORDER BY m.time_created, m.rowid, p.time_created, p.rowid",
        )
        .map_err(|error| format!("zcode message query: {error}"))?;
    let rows = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|error| format!("zcode message rows: {error}"))?;
    let changed =
        |what: &str| format!("ZCode's {what} isn't in the shape AI Juggler reads (engine 0.16.9)");
    let mut out: Vec<ZCodeTurn> = Vec::new();
    let mut current: Option<String> = None;
    for row in rows {
        let (message, at_ms, data, part) = row.map_err(|error| format!("zcode row: {error}"))?;
        let message_data: serde_json::Value =
            serde_json::from_str(&data).map_err(|_| changed("message record"))?;
        let role = match message_data.get("role").and_then(serde_json::Value::as_str) {
            Some("user") => "user",
            Some("assistant") => "assistant",
            // Another speaker (the engine's own notes): no turn of the chat.
            Some(_) => continue,
            None => return Err(changed("message record")),
        };
        let text = match part {
            None => None,
            Some(part) => {
                let part: serde_json::Value =
                    serde_json::from_str(&part).map_err(|_| changed("message part"))?;
                match part.get("type").and_then(serde_json::Value::as_str) {
                    Some("text") => {
                        let text = part
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| changed("text part"))?;
                        let synthetic = part.get("synthetic").and_then(serde_json::Value::as_bool)
                            == Some(true);
                        (!synthetic && !text.trim().is_empty()).then(|| text.to_owned())
                    }
                    Some(_) => None,
                    None => return Err(changed("message part")),
                }
            }
        };
        let Some(text) = text else {
            continue;
        };
        match out.last_mut() {
            Some(last) if current.as_deref() == Some(message.as_str()) => {
                last.text.push_str("\n\n");
                last.text.push_str(&text);
            }
            _ => {
                out.push(ZCodeTurn { role, text, at_ms });
                current = Some(message);
            }
        }
    }
    Ok(out)
}

/// [`turns`] for a session's link (`<db>#<id>`); `None` for a link that
/// isn't one, or a store that can't be read in the shape read on 0.16.9.
#[must_use]
pub fn turns_of_ref(reference: &Path) -> Option<Vec<ZCodeTurn>> {
    let (db, id) = crate::common::sqlite::store_ref_parts(reference, DB_FILE)?;
    turns(&db, &id).ok()
}
