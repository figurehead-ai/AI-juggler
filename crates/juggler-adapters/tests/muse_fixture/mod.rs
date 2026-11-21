//! Muse Code as it leaves its files (AJ-E1608), for the adapter's tests
//! and the conformance kit: an account's session index, in the schema
//! Muse 1.4.13 wrote in a throwaway home (`sessions` as it created it), and
//! its config folder. Every value is made up.

#![allow(
    clippy::disallowed_methods,
    dead_code,
    reason = "the fixture writes an index as Muse does; each test uses part of it"
)]

use std::path::{Path, PathBuf};

/// Muse 1.4.13's own `sessions` table, as its index holds it.
const SESSIONS: &str = r#"CREATE TABLE sessions (  session_id          TEXT PRIMARY KEY,
  session_stream_id   TEXT NOT NULL,
  session_dir         TEXT NOT NULL,
  session_log_path    TEXT NOT NULL UNIQUE,
  layout              TEXT NOT NULL,
  workspace_root      TEXT,
  workspace_key       TEXT,
  provider_id         TEXT,
  model_id            TEXT,
  git_branch          TEXT,
  title               TEXT NOT NULL,
  effective_long_title TEXT,
  title_provenance TEXT CHECK(title_provenance IN ('generated','manual')),
  title_name_fingerprint TEXT,
  title_source_fingerprint TEXT,
  first_user_prompt   TEXT,
  search_text         TEXT NOT NULL,
  created_at_us       INTEGER,
  updated_at_us       INTEGER,
  prompt_count        INTEGER NOT NULL DEFAULT 0,
  status              TEXT NOT NULL,
  status_rank         INTEGER NOT NULL,
  source_fingerprint  TEXT,
  indexed_at_us       INTEGER NOT NULL,
  latest_segment_terminated INTEGER NOT NULL DEFAULT 0,
  session_name        TEXT,
  session_name_revision INTEGER CHECK(session_name_revision >= 0),
  session_name_description_dependency TEXT,
  logged_session_name TEXT,
  logged_session_name_revision INTEGER CHECK(logged_session_name_revision >= 0),
  -- 1 while the logged name is the old store name seeded at upgrade, until a derivation replaces it.
  logged_session_name_seeded INTEGER,
  msp_created_at_us   INTEGER,
  msp_updated_at_us   INTEGER,
  msp_last_activity_at_us INTEGER,
  msp_order_key_us    INTEGER,
  msp_turn_count      INTEGER,
  msp_fork_source_session_id TEXT,
  msp_fork_cut_cursor TEXT,
  msp_fork_cut_explicit INTEGER,
  msp_fork_command_id TEXT,
  msp_provider_id     TEXT,
  msp_model_id        TEXT,
  msp_title           TEXT,
  msp_first_user_prompt TEXT,
  msp_branch          TEXT,
  msp_source_fingerprint TEXT,
  msp_kind            TEXT,
  msp_parent_session_id TEXT,
  msp_created_by_client_name TEXT,
  msp_created_by_client_version TEXT,
  CHECK ((effective_long_title IS NULL) = (title_provenance IS NULL)),
  CHECK (effective_long_title IS NULL OR title_provenance='generated' OR title_name_fingerprint IS NOT NULL),
  CHECK ((session_name IS NULL) = (session_name_revision IS NULL)),
  CHECK ((logged_session_name IS NULL) = (logged_session_name_revision IS NULL)));"#;

/// The account's data folder (`<account>/.local/share/muse`), or the
/// user's own when `state_dir` is `~/.config/muse`.
pub fn data(state_dir: &Path) -> PathBuf {
    juggler_adapters::muse::data_of(state_dir)
}

/// The account's index.
pub fn index(state_dir: &Path) -> PathBuf {
    juggler_adapters::muse_sessions::index_of(&data(state_dir))
}

/// The index, made as Muse makes it (schema version `version`).
pub fn create_with(state_dir: &Path, version: &str) -> rusqlite::Connection {
    let path = index(state_dir);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
    let sessions = SESSIONS.replacen(
        "CREATE TABLE sessions",
        "CREATE TABLE IF NOT EXISTS sessions",
        1,
    );
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL); {sessions}"
    ))
    .unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta VALUES ('schema_version', ?1)",
        [version],
    )
    .unwrap();
    conn
}

/// The index in the schema read (version 1).
pub fn create(state_dir: &Path) -> rusqlite::Connection {
    create_with(state_dir, "1")
}

/// Adds root session `id`, run in `cwd`, created now, as Muse lists it.
pub fn add_session(state_dir: &Path, id: &str, cwd: &Path, title: &str) {
    add_session_at(state_dir, id, cwd, title, now_us(), None);
}

/// Adds session `id` created at `created_us`; `parent` makes it a
/// sub-agent's.
pub fn add_session_at(
    state_dir: &Path,
    id: &str,
    cwd: &Path,
    title: &str,
    created_us: i64,
    parent: Option<&str>,
) {
    let conn = create(state_dir);
    let log = data(state_dir).join(format!("sessions/2026/11/13/{id}/session.jsonl"));
    conn.execute(
        "INSERT INTO sessions (session_id, session_stream_id, session_dir, session_log_path, layout,
             workspace_root, title, first_user_prompt, search_text, created_at_us, updated_at_us,
             prompt_count, status, status_rank, indexed_at_us, msp_kind, msp_parent_session_id)
         VALUES (?1, ?1, ?2, ?3, 'session_jsonl', ?4, ?5, ?5, ?5, ?6, ?6, 1, 'valid', 0, ?6, ?7, ?8)",
        rusqlite::params![
            id,
            log.parent().unwrap().to_string_lossy(),
            log.to_string_lossy(),
            cwd.to_string_lossy(),
            title,
            created_us,
            parent.map(|_| "subagent"),
            parent,
        ],
    )
    .unwrap();
}

/// Now, in epoch µs.
pub fn now_us() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros(),
    )
    .unwrap()
}

/// A config folder's settings and skill, as Muse keeps them.
pub fn settings(config: &Path) {
    std::fs::create_dir_all(config.join("skills/release")).unwrap();
    std::fs::write(config.join("skills/release/SKILL.md"), "# Release\n").unwrap();
    std::fs::write(
        config.join("settings.json"),
        r#"{"schema_version": 1, "mcp_servers": {"files": {"transport": "stdio", "command": "npx", "args": ["files-mcp"]}}}"#,
    )
    .unwrap();
}

/// A sign-in record holding `canary` (Muse's own is a pointer to its
/// Keychain item; this one is made up).
pub fn sign_in(config: &Path, canary: &str) {
    std::fs::create_dir_all(config).unwrap();
    std::fs::write(
        config.join("auth.json"),
        format!(r#"{{"backend":"keychain","note":"{canary}"}}"#),
    )
    .unwrap();
}
