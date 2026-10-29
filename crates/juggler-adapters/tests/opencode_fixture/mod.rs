//! Synthetic `opencode.db` files for the OpenCode tests (AJ-E1602).
//!
//! The tables carry the columns OpenCode's own schema declares
//! (`packages/core/src/session/sql.ts`, `credential/sql.ts`,
//! `account/sql.ts`, `share/sql.ts` at v1.19.3 and v2.1.2) — the ones
//! the adapter reads plus the sign-in tables a copy must empty. No real
//! database was copied; every value is made up.

#![allow(dead_code)]

use std::path::Path;

use rusqlite::{Connection, params};

/// Which OpenCode line wrote the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    V1,
    V2,
}

const SESSION_COLUMNS: &str = "
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, workspace_id TEXT,
    parent_id TEXT, slug TEXT NOT NULL, directory TEXT NOT NULL, path TEXT,
    title TEXT, version TEXT NOT NULL, share_url TEXT,
    summary_additions INTEGER, summary_deletions INTEGER, summary_files INTEGER,
    summary_diffs TEXT, metadata TEXT,
    cost REAL NOT NULL DEFAULT 0,
    tokens_input INTEGER NOT NULL DEFAULT 0, tokens_output INTEGER NOT NULL DEFAULT 0,
    tokens_reasoning INTEGER NOT NULL DEFAULT 0, tokens_cache_read INTEGER NOT NULL DEFAULT 0,
    tokens_cache_write INTEGER NOT NULL DEFAULT 0,
    revert TEXT, permission TEXT, agent TEXT, model TEXT,
    time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL,
    time_compacting INTEGER, time_archived INTEGER";

/// Creates (or opens) a database as `line` leaves it, in WAL mode as
/// OpenCode runs it.
pub fn create(path: &Path, line: Line) -> Connection {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    let sessions = match line {
        Line::V1 => "session",
        Line::V2 => "session_v2",
    };
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS project (id TEXT PRIMARY KEY, worktree TEXT);
         CREATE TABLE IF NOT EXISTS {sessions} ({SESSION_COLUMNS});
         CREATE TABLE IF NOT EXISTS message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL,
             time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS session_message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL,
             type TEXT NOT NULL, seq INTEGER NOT NULL,
             time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL, data TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS credential (id TEXT PRIMARY KEY, integration_id TEXT,
             label TEXT NOT NULL, value TEXT NOT NULL, connector_id TEXT, method_id TEXT,
             active INTEGER, time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS account (id TEXT PRIMARY KEY, email TEXT NOT NULL,
             url TEXT NOT NULL, access_token TEXT NOT NULL, refresh_token TEXT NOT NULL,
             token_expiry INTEGER, time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS account_state (id INTEGER PRIMARY KEY,
             active_account_id TEXT, active_org_id TEXT);
         CREATE TABLE IF NOT EXISTS session_share (session_id TEXT PRIMARY KEY, id TEXT NOT NULL,
             secret TEXT NOT NULL, url TEXT NOT NULL,
             time_created INTEGER NOT NULL, time_updated INTEGER NOT NULL);"
    ))
    .unwrap();
    conn
}

/// Adds a session that ran in `directory`, created at `created_ms`.
pub fn add_session(
    conn: &Connection,
    line: Line,
    id: &str,
    directory: &Path,
    parent: Option<&str>,
    created_ms: i64,
) {
    let table = match line {
        Line::V1 => "session",
        Line::V2 => "session_v2",
    };
    conn.execute(
        &format!(
            "INSERT INTO {table} (id, project_id, parent_id, slug, directory, title, version,
                 cost, tokens_input, tokens_output, time_created, time_updated)
             VALUES (?1, 'prj_1', ?2, 'calm-otter', ?3, 'Fix the build', ?4, 0.42, 1200, 300, ?5, ?5)"
        ),
        params![
            id,
            parent,
            directory.to_string_lossy(),
            if line == Line::V1 { "1.19.3" } else { "2.1.2" },
            created_ms
        ],
    )
    .unwrap();
}

/// One assistant message with its counts, as `line` stores it.
pub struct Assistant<'a> {
    pub id: &'a str,
    pub session: &'a str,
    pub created_ms: i64,
    pub provider: &'a str,
    pub model: &'a str,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cost: f64,
    pub completed: bool,
}

pub fn add_assistant(conn: &Connection, line: Line, message: &Assistant<'_>) {
    let tokens = serde_json::json!({
        "input": message.input, "output": message.output, "reasoning": 2,
        "cache": {"read": message.cache_read, "write": 0},
    });
    let mut time = serde_json::json!({"created": message.created_ms});
    if message.completed {
        time["completed"] = serde_json::json!(message.created_ms + 1_500);
    }
    match line {
        Line::V1 => {
            let data = serde_json::json!({
                "role": "assistant", "parentID": "msg_user", "mode": "build", "agent": "build",
                "modelID": message.model, "providerID": message.provider,
                "path": {"cwd": "/w", "root": "/w"},
                "cost": message.cost, "tokens": tokens, "time": time,
            });
            conn.execute(
                "INSERT INTO message (id, session_id, time_created, time_updated, data)
                 VALUES (?1, ?2, ?3, ?3, ?4)",
                params![
                    message.id,
                    message.session,
                    message.created_ms,
                    data.to_string()
                ],
            )
            .unwrap();
        }
        Line::V2 => {
            let data = serde_json::json!({
                "agent": "build",
                "model": {"id": message.model, "providerID": message.provider},
                "content": [], "cost": message.cost, "tokens": tokens, "time": time,
            });
            conn.execute(
                "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data)
                 VALUES (?1, ?2, 'assistant', (SELECT COUNT(*) FROM session_message), ?3, ?3, ?4)",
                params![message.id, message.session, message.created_ms, data.to_string()],
            )
            .unwrap();
        }
    }
}

/// A user message (never counted as usage).
pub fn add_user(conn: &Connection, line: Line, id: &str, session: &str, created_ms: i64) {
    match line {
        Line::V1 => {
            let data = serde_json::json!({"role": "user", "time": {"created": created_ms}});
            conn.execute(
                "INSERT INTO message (id, session_id, time_created, time_updated, data)
                 VALUES (?1, ?2, ?3, ?3, ?4)",
                params![id, session, created_ms, data.to_string()],
            )
            .unwrap();
        }
        Line::V2 => {
            let data = serde_json::json!({"text": "hi", "time": {"created": created_ms}});
            conn.execute(
                "INSERT INTO session_message (id, session_id, type, seq, time_created, time_updated, data)
                 VALUES (?1, ?2, 'user', (SELECT COUNT(*) FROM session_message), ?3, ?3, ?4)",
                params![id, session, created_ms, data.to_string()],
            )
            .unwrap();
        }
    }
}

/// Sign-ins in the database holding `canary`: a v2 credential, a
/// Console account, a share secret.
pub fn add_sign_ins(conn: &Connection, canary: &str, active: bool) {
    conn.execute(
        "INSERT INTO credential (id, label, value, connector_id, method_id, active, time_created, time_updated)
         VALUES ('cred_1', 'work', ?1, 'anthropic', 'api', ?2, 1, 1)",
        params![format!(r#"{{"type":"api","key":"{canary}"}}"#), active],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO account (id, email, url, access_token, refresh_token, time_created, time_updated)
         VALUES ('acc_1', 'you@example.com', 'https://opencode.ai', ?1, ?1, 1, 1)",
        [canary],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO session_share (session_id, id, secret, url, time_created, time_updated)
         VALUES ('ses_shared', 'sh_1', ?1, 'https://opncd.ai/s/1', 1, 1)",
        [canary],
    )
    .unwrap();
}

/// `auth.json` as v1 writes it: provider id → entry.
pub fn auth_json(canary: &str) -> String {
    serde_json::json!({
        "anthropic": {"type": "api", "key": canary},
        "openai": {"type": "oauth", "refresh": canary, "access": canary, "expires": 4_102_444_800_000_u64},
    })
    .to_string()
}

pub fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}
