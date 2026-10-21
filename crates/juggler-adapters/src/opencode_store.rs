//! OpenCode's session store, read-only (AJ-E1602 Req.3).
//!
//! OpenCode keeps every session in one SQLite file,
//! `$XDG_DATA_HOME/opencode/opencode.db` (source: `packages/core/src/
//! database/database.ts` at v1.19.3; `packages/cli/src/database-path.ts`
//! at v2.1.2 — the same name in the same folder, upgraded in place).
//! Both lines are read:
//!
//! - **v1** (1.x): sessions in `session`, messages in `message` with the
//!   message as JSON in `data` (`role`, `modelID`, `providerID`, `cost`,
//!   `tokens {input, output, reasoning, cache {read, write}}`,
//!   `time {created, completed}` — `packages/schema/src/v1/session.ts`).
//! - **v2** (2.x): sessions in `session_v2`, messages in
//!   `session_message` (`type = 'assistant'`, `data` with `model {id,
//!   providerID}`, `cost`, `tokens`, `time` — `packages/schema/src/
//!   session-message.ts`). A v1 database upgraded by v2 may hold both;
//!   rows are joined by id, the v2 row winning.
//!
//! Every connection is `SQLITE_OPEN_READ_ONLY` with a busy timeout:
//! OpenCode writes the file in WAL mode while it runs, and this reader
//! never takes a write lock or changes a byte. Sign-ins live in the same
//! file (`credential`, `account`); no query here names those tables.
//!
//! A session has no transcript file, so its link is a reference into the
//! database: [`session_ref`] is `<db>#<session id>` — a path no file has,
//! unique per session, so readers that open transcript files find
//! nothing to misread, and "already linked" checks compare sessions.

use std::path::{Path, PathBuf};

use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

/// The database file's name in the data folder (`latest`/`beta`/`prod`
/// channels, the ones the installers ship).
pub const DB_FILE: &str = "opencode.db";

/// How long a read waits for OpenCode's own write to finish.
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// One OpenCode session, as the list and the linker need it.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeSession {
    /// OpenCode's session id (`ses_…`) — what `--session` takes.
    pub id: String,
    /// The folder the session ran in.
    pub directory: String,
    /// Its title (v2 may leave it empty).
    pub title: String,
    /// The parent session, for a sub-agent's session.
    pub parent_id: Option<String>,
    /// OpenCode's own dollar figure for the session, priced from its
    /// model list — an estimate, never a bill.
    pub cost_usd: f64,
    /// Token totals OpenCode recorded for the session.
    pub tokens: TokenCounts,
    /// Created, epoch ms.
    pub created_ms: i64,
    /// Last updated, epoch ms.
    pub updated_ms: i64,
    /// Archived in OpenCode.
    pub archived: bool,
}

/// Opens `db` read-only, waiting briefly for a writer.
///
/// While OpenCode runs, its write-ahead log sits beside the file and the
/// read joins it as any reader does. When OpenCode is closed there is no
/// log, and even a read-only open would create one (and its index) in
/// the user's folder — so the file is opened `immutable`: no log, no
/// lock, nothing written. Should OpenCode start during that read, the
/// read may fail; the next pass reads again.
///
/// # Errors
///
/// A plain message when the file can't be opened.
pub fn open_read_only(db: &Path) -> Result<Connection, String> {
    if !db.is_file() {
        return Err(format!("no OpenCode database at {}", db.display()));
    }
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = if Path::new(&wal).exists() {
        Connection::open_with_flags(db, flags)
    } else {
        Connection::open_with_flags(
            format!("file:{}?immutable=1", uri_path(db)),
            flags | OpenFlags::SQLITE_OPEN_URI,
        )
    }
    .map_err(|error| format!("opencode db open (read-only): {error}"))?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| format!("opencode db busy timeout: {error}"))?;
    Ok(conn)
}

/// `path` for a `file:` URI: every byte outside the unreserved set and
/// `/` percent-encoded (spaces, `#` and `?` included).
fn uri_path(path: &Path) -> String {
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

/// Whether `conn` has a table named `name`.
pub(crate) fn has_table(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

fn count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

/// Every session in `db`, newest first (v1 and v2 tables joined).
///
/// # Errors
///
/// A plain message when the file can't be opened or holds no session
/// table (not an OpenCode database).
pub fn sessions(db: &Path) -> Result<Vec<OpenCodeSession>, String> {
    let conn = open_read_only(db)?;
    let tables: Vec<&str> = ["session_v2", "session"]
        .into_iter()
        .filter(|table| has_table(&conn, table))
        .collect();
    if tables.is_empty() {
        return Err("no OpenCode session table in this database".into());
    }
    let mut out: Vec<OpenCodeSession> = Vec::new();
    for table in tables {
        // The table name is one of two literals above, never input.
        let sql = format!(
            "SELECT id, directory, COALESCE(title, ''), parent_id, cost,
                    tokens_input, tokens_output, tokens_reasoning,
                    tokens_cache_read, tokens_cache_write,
                    time_created, time_updated, time_archived
             FROM {table}"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| format!("opencode {table} query: {error}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(OpenCodeSession {
                    id: row.get(0)?,
                    directory: row.get(1)?,
                    title: row.get(2)?,
                    parent_id: row.get(3)?,
                    cost_usd: row.get(4)?,
                    tokens: TokenCounts {
                        input: count(row.get(5)?),
                        output: count(row.get(6)?),
                        reasoning: count(row.get(7)?),
                        cache_read: count(row.get(8)?),
                        cache_write: count(row.get(9)?),
                    },
                    created_ms: row.get(10)?,
                    updated_ms: row.get(11)?,
                    archived: row.get::<_, Option<i64>>(12)?.is_some(),
                })
            })
            .map_err(|error| format!("opencode {table} rows: {error}"))?;
        for row in rows {
            let session = row.map_err(|error| format!("opencode {table} row: {error}"))?;
            if !out.iter().any(|known| known.id == session.id) {
                out.push(session);
            }
        }
    }
    out.sort_by_key(|session| std::cmp::Reverse(session.updated_ms));
    Ok(out)
}

/// The link reference for a session: `<db>#<id>` (see module docs).
#[must_use]
pub fn session_ref(db: &Path, id: &str) -> PathBuf {
    let mut text = db.as_os_str().to_owned();
    text.push("#");
    text.push(id);
    PathBuf::from(text)
}

/// The session id inside a [`session_ref`], if `path` is one.
#[must_use]
pub fn session_of_ref(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    let (db, id) = text.rsplit_once('#')?;
    (db.ends_with(DB_FILE) && !id.is_empty()).then(|| id.to_owned())
}

/// The one top-level session that ran in `cwd`, created at or after
/// `since_ms`, and not already linked (`taken`). Two candidates is an
/// honest `None`, never a guess.
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
        .filter(|session| session.parent_id.is_none())
        .filter(|session| session.created_ms >= since_ms.saturating_sub(2_000))
        .filter(|session| crate::claude_code::same_folder(&session.directory, cwd))
        .map(|session| (session_ref(db, &session.id), session.id))
        .filter(|(reference, _)| !taken.contains(reference));
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// A JSON number as a token count (OpenCode stores finite numbers).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "counts are whole, non-negative and far below 2^53"
)]
fn tokens_of(value: &serde_json::Value) -> u64 {
    value.as_f64().map_or(0, |n| n.max(0.0) as u64)
}

/// A JSON string field's text.
fn text(value: Option<&serde_json::Value>) -> Option<&str> {
    value.and_then(serde_json::Value::as_str)
}

/// Reads one assistant message's JSON into an event. `None` for a
/// message still running (no completion time: its counts may still
/// grow) or one without tokens.
fn event_of(
    account: &str,
    session: String,
    id: &str,
    created_column: i64,
    data: &str,
) -> Option<UsageEvent> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    let time = value.get("time")?;
    time.get("completed")?.as_f64()?;
    let tokens = value.get("tokens")?;
    let cache = tokens.get("cache");
    let counts = TokenCounts {
        input: tokens_of(tokens.get("input")?),
        output: tokens_of(tokens.get("output")?),
        reasoning: tokens.get("reasoning").map_or(0, tokens_of),
        cache_read: cache.and_then(|c| c.get("read")).map_or(0, tokens_of),
        cache_write: cache.and_then(|c| c.get("write")).map_or(0, tokens_of),
    };
    if counts == TokenCounts::default() {
        return None;
    }
    // v1: modelID/providerID; v2: model {id, providerID}.
    let model_id = text(value.get("modelID")).or_else(|| text(value.pointer("/model/id")));
    let provider =
        text(value.get("providerID")).or_else(|| text(value.pointer("/model/providerID")));
    let model = model_id.map(|model| match provider {
        Some(provider) => format!("{provider}/{model}"),
        None => model.to_owned(),
    });
    let ts = time
        .get("created")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(created_column);
    Some(UsageEvent {
        account: account.to_owned(),
        session: Some(session),
        ts,
        model,
        tokens: counts,
        credits: None,
        // OpenCode's price from its model list: an estimate.
        usd_est: value
            .get("cost")
            .and_then(serde_json::Value::as_f64)
            .filter(|cost| *cost > 0.0),
        source: UsageSource::Sqlite,
        // The counts are the provider's, as OpenCode recorded them.
        confidence: Confidence::Measured,
        msg_uuid: Some(format!("opencode:{id}")),
    })
}

/// Token events for every completed assistant message updated at or
/// after `since_ms` (0 for all), keyed `opencode:<message id>` so a
/// message read again — or seen in both v1 and v2 tables — counts once.
///
/// # Errors
///
/// A plain message when the file can't be opened or read.
pub fn usage_events(account: &str, db: &Path, since_ms: i64) -> Result<Vec<UsageEvent>, String> {
    let conn = open_read_only(db)?;
    let mut events: Vec<UsageEvent> = Vec::new();
    let queries = [
        (
            "session_message",
            "SELECT id, session_id, time_created, data FROM session_message
             WHERE time_updated >= ?1 AND type = 'assistant'",
        ),
        (
            "message",
            "SELECT id, session_id, time_created, data FROM message
             WHERE time_updated >= ?1 AND json_extract(data, '$.role') = 'assistant'",
        ),
    ];
    for (table, sql) in queries {
        if !has_table(&conn, table) {
            continue;
        }
        let mut stmt = conn
            .prepare(sql)
            .map_err(|error| format!("opencode {table} query: {error}"))?;
        let rows = stmt
            .query_map([since_ms], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| format!("opencode {table} rows: {error}"))?;
        for row in rows {
            let (id, session, created, data) =
                row.map_err(|error| format!("opencode {table} row: {error}"))?;
            let key = format!("opencode:{id}");
            if events
                .iter()
                .any(|event| event.msg_uuid.as_deref() == Some(key.as_str()))
            {
                continue;
            }
            if let Some(event) = event_of(account, session, &id, created, &data) {
                events.push(event);
            }
        }
    }
    Ok(events)
}
