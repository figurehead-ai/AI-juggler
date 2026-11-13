//! Synthetic `ZCode` trees for the `ZCode` tests (AJ-E1605): the session
//! store (`cli/db/db.sqlite`) with the columns engine 0.16.9's own schema
//! has for the tables the adapter reads (research `docs/research/zcode.md`
//! § C), made in WAL mode and closed as the engine leaves it, and a
//! sign-in file. No real store was copied; every value is made up.

#![allow(
    clippy::disallowed_methods,
    dead_code,
    reason = "the fixture writes a store as the tool does; each test uses part of it"
)]

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

/// A made-up Z.ai app bundle (`ZCode.app`, `dev.zcode.app`) in `dir`, with
/// an engine file; returns the engine's path.
pub fn app(dir: &Path, bundle_id: &str) -> PathBuf {
    let app = dir.join("ZCode.app");
    write(
        &app.join("Contents/Info.plist"),
        &format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n  \
             <key>CFBundleName</key>\n  <string>ZCode</string>\n  \
             <key>CFBundleIdentifier</key>\n  <string>{bundle_id}</string>\n</dict>\n</plist>\n"
        ),
    );
    let engine = app.join("Contents/Resources/glm/zcode.cjs");
    write(&engine, "#!/usr/bin/env node\n\"use strict\";\n");
    engine
}

pub fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The tree of a managed App's folder.
pub fn tree(state_dir: &Path) -> PathBuf {
    juggler_adapters::zcode::tree_of(state_dir)
}

/// The store of an App's folder.
pub fn db(state_dir: &Path) -> PathBuf {
    juggler_adapters::zcode::db_path_of(state_dir)
}

/// Opens (or creates) the store of `state_dir` with `ZCode`'s tables, in
/// WAL mode as the engine runs it.
pub fn create(state_dir: &Path) -> Connection {
    let path = db(state_dir);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session (
             id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, directory TEXT NOT NULL,
             path TEXT, title TEXT, version TEXT, time_created INTEGER NOT NULL,
             time_updated INTEGER NOT NULL, time_compacting INTEGER, time_archived INTEGER,
             task_type TEXT, trace_id TEXT);
         CREATE TABLE IF NOT EXISTS model_usage (
             id TEXT PRIMARY KEY, session_id TEXT, provider_id TEXT, model_id TEXT NOT NULL,
             status TEXT, started_at INTEGER NOT NULL, input_tokens INTEGER NOT NULL DEFAULT 0,
             output_tokens INTEGER NOT NULL DEFAULT 0, reasoning_tokens INTEGER NOT NULL DEFAULT 0,
             cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
             cache_read_input_tokens INTEGER NOT NULL DEFAULT 0,
             computed_total_tokens INTEGER, duration_ms INTEGER);",
    )
    .unwrap();
    conn
}

/// One session row as `ZCode` writes it.
pub struct Row<'a> {
    pub id: &'a str,
    pub directory: &'a Path,
    pub title: &'a str,
    pub parent: Option<&'a str>,
    pub task_type: &'a str,
    pub archived: Option<i64>,
    pub created_ms: i64,
}

impl<'a> Row<'a> {
    /// A plain interactive session.
    pub fn plain(id: &'a str, directory: &'a Path, created_ms: i64) -> Self {
        Self {
            id,
            directory,
            title: "Fix the cart",
            parent: None,
            task_type: "interactive",
            archived: None,
            created_ms,
        }
    }
}

pub fn add(conn: &Connection, row: &Row<'_>) {
    conn.execute(
        "INSERT INTO session (id, project_id, parent_id, directory, title, version,
             time_created, time_updated, time_archived, task_type)
         VALUES (?1, 'prj_kit', ?2, ?3, ?4, '0.16.9', ?5, ?6, ?7, ?8)",
        params![
            row.id,
            row.parent,
            row.directory.to_string_lossy(),
            row.title,
            row.created_ms,
            row.created_ms + 1_000,
            row.archived,
            row.task_type,
        ],
    )
    .unwrap();
}

/// Adds an interactive session run in `cwd` now, and a sub-agent's
/// session under it in the same folder; closes the store.
pub fn add_session(state_dir: &Path, id: &str, cwd: &Path) {
    let conn = create(state_dir);
    let now = now_ms();
    add(&conn, &Row::plain(id, cwd, now));
    add(
        &conn,
        &Row {
            parent: Some(id),
            ..Row::plain(&format!("{id}_sub"), cwd, now)
        },
    );
}

/// A `model_usage` row.
pub fn usage(conn: &Connection, id: &str, model: &str, status: &str, at: i64, input: i64) {
    conn.execute(
        "INSERT INTO model_usage (id, session_id, provider_id, model_id, status, started_at,
             input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens)
         VALUES (?1, 'sess_use', 'zai', ?2, ?3, ?4, ?5, 120, 0, 30)",
        params![id, model, status, at, input],
    )
    .unwrap();
}

/// A sign-in file holding `canary` (made up).
pub fn sign_in(tree: &Path, canary: &str) {
    write(
        &tree.join("v2/credentials.json"),
        &format!(r#"{{"oauth:zai:access_token":"{canary}"}}"#),
    );
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
