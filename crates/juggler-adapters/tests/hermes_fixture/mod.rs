//! Hermes Agent as it leaves its files (AJ-E1607), for the adapter's tests
//! and the conformance kit: a home's session store in the schema of the
//! MIT source read (`NousResearch/hermes-agent@422ad186`,
//! `hermes_state_common.py` `SCHEMA_SQL`, version 33: its
//! `schema_version`, `system_prompts`, `sessions` and `messages` tables as
//! written there), its settings and a made-up sign-in. Every value is made
//! up.

#![allow(
    clippy::disallowed_methods,
    dead_code,
    reason = "the fixture writes a store as Hermes does; each test uses part of it"
)]

use std::path::{Path, PathBuf};

/// Hermes' own tables, as `SCHEMA_SQL` creates them.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS system_prompts (
    hash TEXT PRIMARY KEY,
    prompt TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    runtime_revision INTEGER NOT NULL DEFAULT 0,
    runtime_generation INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL,
    created_source TEXT,
    user_id TEXT,
    session_key TEXT,
    chat_id TEXT,
    chat_type TEXT,
    thread_id TEXT,
    display_name TEXT,
    origin_json TEXT,
    expiry_finalized INTEGER DEFAULT 0,
    model TEXT,
    model_config TEXT,
    system_prompt TEXT,
    system_prompt_hash TEXT,
    parent_session_id TEXT,
    started_at REAL NOT NULL,
    ended_at REAL,
    end_reason TEXT,
    message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    cwd TEXT,
    git_branch TEXT,
    git_repo_root TEXT,
    git_metadata_generation INTEGER NOT NULL DEFAULT 0,
    billing_provider TEXT,
    billing_base_url TEXT,
    billing_mode TEXT,
    estimated_cost_usd REAL,
    actual_cost_usd REAL,
    cost_status TEXT,
    cost_source TEXT,
    pricing_version TEXT,
    title TEXT,
    title_source TEXT,
    last_activity_at REAL,
    last_activity_description TEXT,
    last_activity_provenance TEXT,
    api_call_count INTEGER DEFAULT 0,
    handoff_state TEXT,
    handoff_platform TEXT,
    handoff_error TEXT,
    compression_failure_cooldown_until REAL,
    compression_failure_error TEXT,
    compression_fallback_streak INTEGER NOT NULL DEFAULT 0,
    compression_ineffective_count INTEGER NOT NULL DEFAULT 0,
    compression_recovery_deadline REAL,
    compression_overload_streak INTEGER NOT NULL DEFAULT 0,
    profile_name TEXT,
    transport_profile TEXT,
    rewind_count INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    auto_archived INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    hidden INTEGER NOT NULL DEFAULT 0,
    last_read_at REAL,
    tool_names TEXT,
    transcript_revision INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_session_id) REFERENCES sessions(id),
    FOREIGN KEY (system_prompt_hash) REFERENCES system_prompts(hash)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT,
    tool_call_id TEXT,
    tool_calls TEXT,
    tool_name TEXT,
    effect_disposition TEXT,
    timestamp REAL NOT NULL,
    token_count INTEGER,
    finish_reason TEXT,
    reasoning TEXT,
    reasoning_content TEXT,
    reasoning_details TEXT,
    codex_reasoning_items TEXT,
    codex_message_items TEXT,
    platform_message_id TEXT,
    observed INTEGER DEFAULT 0,
    _compressed_summary INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    compacted INTEGER NOT NULL DEFAULT 0,
    api_content TEXT,
    display_kind TEXT,
    display_metadata TEXT,
    display_identity BLOB,
    display_order INTEGER,
    message_uid TEXT,
    absorbed_message_uids TEXT,
    tool_call_uids TEXT,
    tool_call_uid TEXT
);
"#;

/// The home's store.
pub fn store(home: &Path) -> PathBuf {
    juggler_adapters::hermes_sessions::store_of(home)
}

/// The store, made as Hermes makes it, at schema `version`.
pub fn create_with(home: &Path, version: i64) -> rusqlite::Connection {
    std::fs::create_dir_all(home).unwrap();
    let conn = rusqlite::Connection::open(store(home)).unwrap();
    conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    conn.execute("DELETE FROM schema_version", []).unwrap();
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [version],
    )
    .unwrap();
    conn
}

/// The store at the schema read (33).
pub fn create(home: &Path) -> rusqlite::Connection {
    create_with(home, 33)
}

/// Now, in epoch seconds (Hermes' REAL timestamps).
pub fn now_s() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

/// Adds root session `id`, run in `cwd`, started now, with one user turn.
pub fn add_session(home: &Path, id: &str, cwd: &Path, title: &str) {
    add_session_at(home, id, cwd, title, now_s(), None);
}

/// Adds session `id` started at `started_s`; `parent` makes it a
/// delegated task's. An empty `title` leaves Hermes' own title unset (its
/// first prompt names it).
pub fn add_session_at(
    home: &Path,
    id: &str,
    cwd: &Path,
    title: &str,
    started_s: f64,
    parent: Option<&str>,
) {
    let conn = create(home);
    conn.execute(
        "INSERT INTO sessions (id, source, model, started_at, last_activity_at, message_count,
             input_tokens, output_tokens, cache_read_tokens, cwd, title, parent_session_id)
         VALUES (?1, 'cli', 'nous/hermes-test', ?2, ?2, 2, 1200, 300, 800, ?3, ?4, ?5)",
        rusqlite::params![
            id,
            started_s,
            cwd.to_string_lossy(),
            (!title.is_empty()).then_some(title),
            parent,
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO messages (session_id, role, content, timestamp) VALUES (?1, 'user', ?2, ?3)",
        rusqlite::params![id, format!("first prompt of {id}"), started_s],
    )
    .unwrap();
}

/// A home's settings, as Hermes keeps them.
pub fn settings(home: &Path) {
    std::fs::create_dir_all(home.join("skills/release")).unwrap();
    std::fs::write(home.join("skills/release/SKILL.md"), "# Release\n").unwrap();
    std::fs::write(home.join("config.yaml"), "model:\n  provider: nous\n").unwrap();
    std::fs::write(home.join("SOUL.md"), "You are a careful coder.\n").unwrap();
}

/// A provider sign-in holding `canary` (made up), with a key in `.env`.
pub fn sign_in(home: &Path, canary: &str) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::write(
        home.join("auth.json"),
        format!(r#"{{"providers":{{"nous":{{"access_token":"{canary}"}}}}}}"#),
    )
    .unwrap();
    std::fs::write(home.join(".env"), format!("OPENROUTER_API_KEY={canary}\n")).unwrap();
}

/// A stand-in for `hermes` that does what the source says Hermes does with
/// the files this adapter keeps it from (`hermes_guard.rs`'s docs), so a
/// test can watch it: it picks its home (`$HERMES_HOME`, else
/// `~/.hermes`); loads the home's `.env`, then the policy folder's
/// (`$HERMES_MANAGED_DIR`), each overriding what came before; borrows and
/// refreshes the Codex CLI's and Claude Code's sign-ins unless a policy or
/// the home's config sets `adopt_external_logins: false`, and writes its
/// MCP server into the Codex home either way (its Codex runtime). Then:
/// `--version` prints its banner; `usage --json` prints the documented
/// example (exit 1 without a sign-in); `gateway run` records itself as
/// Hermes does and forks a worker; `gateway stop` ends the recorded one.
/// It writes what it saw to `<home>/.stub-saw`.
pub const STUB: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "Hermes Agent v0.22.3 (2026.9.24)"; exit 0; fi
home="${HERMES_HOME:-$HOME/.hermes}"
mkdir -p "$home"
load() { if [ -f "$1" ]; then set -a; . "$1"; set +a; fi; }
load "$home/.env"
policy=""
if [ -n "$HERMES_MANAGED_DIR" ] && [ -d "$HERMES_MANAGED_DIR" ]; then policy="$HERMES_MANAGED_DIR"; load "$policy/.env"; fi
adopt=true
if [ -n "$policy" ] && grep -q "adopt_external_logins: false" "$policy/config.yaml" 2>/dev/null; then adopt=false; fi
if grep -q "adopt_external_logins: false" "$home/config.yaml" 2>/dev/null; then adopt=false; fi
codex="${CODEX_HOME:-$HOME/.codex}"
claude="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
if [ "$adopt" = true ]; then
  if [ -f "$codex/auth.json" ]; then echo '{"refreshed":"by-hermes"}' > "$codex/auth.json"; fi
  if [ -f "$claude/.credentials.json" ]; then echo '{"refreshed":"by-hermes"}' > "$claude/.credentials.json"; fi
fi
if [ -d "$codex" ]; then echo '[mcp_servers.hermes-tools]' >> "$codex/config.toml"; fi
printf 'adopt=%s\ncodex=%s\nclaude=%s\nlocks=%s\nshared=%s\nargs=%s\n' "$adopt" "$codex" "$claude" "$HERMES_GATEWAY_LOCK_DIR" "$HERMES_SHARED_AUTH_DIR" "$*" > "$home/.stub-saw"
case "$1 $2" in
  "usage --json")
    if [ ! -s "$home/auth.json" ]; then echo "No account usage available" >&2; exit 1; fi
    echo '{"provider": "openai-codex", "source": "usage_api", "title": "Account limits", "plan": "Plus", "fetched_at": "2026-09-19T07:58:55+00:00", "windows": [{"label": "Session", "used_percent": 37.0, "resets_at": "2026-09-19T21:00:00+00:00", "detail": null}, {"label": "Weekly", "used_percent": 12.5, "resets_at": "2026-09-25T09:00:00+00:00", "detail": null}], "details": [], "unavailable_reason": null}'
    exit 0 ;;
  "gateway run")
    sh -c 'echo $$ > "$1/.worker.pid"; exec sleep 60' worker "$home" &
    w=$!
    trap 'kill $w 2>/dev/null; rm -f "$home/gateway.pid"; exit 0' TERM
    printf '{"pid": %s, "kind": "hermes-gateway", "argv": ["hermes", "gateway", "run"], "start_time": 1}' "$$" > "$home/gateway.pid"
    wait
    exit 0 ;;
  "gateway stop")
    if [ -f "$home/gateway.pid" ]; then
      pid=$(sed 's/.*"pid": \([0-9]*\).*/\1/' "$home/gateway.pid")
      kill "$pid" 2>/dev/null; rm -f "$home/gateway.pid"
    fi
    exit 0 ;;
esac
exit 0
"#;

/// Writes [`STUB`] at `path`, runnable.
pub fn stub(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, STUB).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
