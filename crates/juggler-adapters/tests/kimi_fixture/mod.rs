//! Synthetic Kimi Code homes (AJ-E1603), in the documented layout
//! (research `docs/research/kimi.md` §2, §4): `session_index.jsonl`,
//! `sessions/<workDirKey>/<id>/state.json`, `credentials/`. Every sign-in
//! is a made-up string; nothing real is read.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Kimi Code's folder key for a working folder: `wd_<slug>_<12 hex>`
/// (the real one hashes the path; any stable form reads the same here).
fn work_dir_key(cwd: &Path) -> String {
    let slug: String = cwd
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let mut hash: u64 = 1469598103934665603;
    for byte in cwd.to_string_lossy().bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(1099511628211);
    }
    format!("wd_{slug}_{:012x}", hash & 0xffff_ffff_ffff)
}

/// Writes session `id`, run in `cwd`, into `home`, as Kimi Code records
/// it; returns its record (`state.json`).
pub fn add_session(home: &Path, id: &str, cwd: &Path, title: &str) -> PathBuf {
    let dir = home.join("sessions").join(work_dir_key(cwd)).join(id);
    write(
        &dir.join("state.json"),
        &serde_json::json!({ "title": title, "lastPrompt": title }).to_string(),
    );
    write(&dir.join("agents/main/wire.jsonl"), "{\"type\":\"turn\"}\n");
    let line = serde_json::json!({
        "sessionId": id,
        "sessionDir": dir.to_string_lossy(),
        "workDir": cwd.to_string_lossy(),
    })
    .to_string();
    let index = home.join("session_index.jsonl");
    let before = std::fs::read_to_string(&index).unwrap_or_default();
    write(&index, &format!("{before}{line}\n"));
    dir.join("state.json")
}

/// The version line a session's event log opens with (protocol 1.5).
pub const WIRE_META: &str =
    r#"{"type":"metadata","protocol_version":"1.5","created_at":1792580400000}"#;

/// Appends `lines` to the event log of `agent` in the session whose record
/// is `state` (`<session>/agents/<agent>/wire.jsonl`); returns the log.
pub fn append_wire(state: &Path, agent: &str, lines: &[String]) -> PathBuf {
    let log = state
        .parent()
        .unwrap()
        .join("agents")
        .join(agent)
        .join("wire.jsonl");
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    let mut text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    if let Ok(before) = std::fs::read_to_string(&log) {
        // The fixture's placeholder line goes; a real log opens with its version.
        if before != "{\"type\":\"turn\"}\n" {
            text = before + &text;
        }
    }
    std::fs::write(&log, text).unwrap();
    log
}

/// One `usage.record` line, as Kimi Code writes it.
pub fn usage_line(model: &str, time: i64, input: u64, output: u64, cache_read: u64) -> String {
    serde_json::json!({
        "type": "usage.record",
        "agentId": "main",
        "model": model,
        "usage": {
            "inputOther": input,
            "output": output,
            "inputCacheRead": cache_read,
            "inputCacheCreation": 0
        },
        "usageScope": "turn",
        "time": time,
    })
    .to_string()
}

/// One record of `kind` with `fields`, as Kimi Code writes it.
pub fn record(kind: &str, time: i64, fields: serde_json::Value) -> String {
    let mut value = serde_json::json!({ "type": kind });
    if let serde_json::Value::Object(extra) = fields {
        value.as_object_mut().unwrap().extend(extra);
    }
    value["time"] = serde_json::json!(time);
    value.to_string()
}

/// Kimi Code's lock on session `id` in `home`, held by `pid` since
/// `started_at` (epoch ms).
pub fn lock(home: &Path, id: &str, pid: u32, started_at: i64) -> PathBuf {
    let path = home.join("locks/sessions").join(format!("{id}.lock"));
    write(
        &path,
        &serde_json::json!({
            "version": 1,
            "token": format!("{pid}:made-up"),
            "pid": pid,
            "hostname": "made-up-host",
            "startedAt": started_at,
            "clientIdentity": "kimi_code_cli",
            "clientDisplayName": "Kimi Code CLI",
        })
        .to_string(),
    );
    path
}

/// A membership (OAuth) sign-in holding `canary`.
pub fn sign_in_membership(home: &Path, canary: &str) {
    write(
        &home.join("credentials/kimi-code.json"),
        &format!(r#"{{"access_token":"{canary}","refresh_token":"{canary}"}}"#),
    );
}

/// A Kimi Platform API key's sign-in file, and no membership one.
pub fn sign_in_api_key(home: &Path) {
    write(
        &home.join("credentials/platform.json"),
        r#"{"api_key":"made-up-platform-key"}"#,
    );
}

/// A home's settings, with `canary` written in as a provider's key.
pub fn settings(home: &Path, canary: &str) {
    write(&home.join("tui.toml"), "[upgrade]\nauto_install = false\n");
    write(
        &home.join("config.toml"),
        &format!(
            "model = \"k3-256k\"\n\n[providers.platform]\ntype = \"kimi\"\napi_key = \"{canary}\"\n"
        ),
    );
    write(
        &home.join("mcp.json"),
        r#"{"mcpServers": {"files": {"command": "npx", "args": ["files-mcp"]}}}"#,
    );
    write(&home.join("AGENTS.md"), "# Rules\n");
    write(&home.join("skills/release/SKILL.md"), "# Release\n");
    write(&home.join("logs/kimi-code.log"), "started\n");
    write(&home.join("device_id"), "made-up-device");
    write(&home.join("server.token"), "made-up-server-token");
}
