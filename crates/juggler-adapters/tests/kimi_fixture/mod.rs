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
