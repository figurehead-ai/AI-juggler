//! Synthetic Grok Build homes (AJ-E1604), in the documented layout
//! (research `docs/research/grok.md` §2–§4). Every sign-in is a made-up
//! string; nothing real is read.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Writes session `id`, run in `cwd`, into `home`; returns its record.
pub fn add_session(home: &Path, id: &str, cwd: &Path, title: &str) -> PathBuf {
    let dir = home
        .join("sessions")
        .join(juggler_adapters::grok_sessions::folder_name(cwd))
        .join(id);
    write(
        &dir.join("summary.json"),
        &serde_json::json!({ "title": title, "model": "grok-build-0.1" }).to_string(),
    );
    write(&dir.join("updates.jsonl"), "{\"type\":\"update\"}\n");
    write(&dir.join("signals.json"), "{}");
    dir.join("summary.json")
}

/// A browser sign-in holding `canary`.
pub fn sign_in(home: &Path, canary: &str) {
    write(
        &home.join("auth.json"),
        &format!(
            r#"{{"https://accounts.x.ai/sign-in":{{"key":"{canary}","auth_mode":"web_login","refresh_token":"{canary}","email":"made-up@example.com"}}}}"#
        ),
    );
}

/// An API key's sign-in.
pub fn sign_in_api_key(home: &Path) {
    write(
        &home.join("auth.json"),
        r#"{"xai::api_key":{"key":"made-up-key","auth_mode":"api_key"}}"#,
    );
}

/// A home's settings, with `canary` written in as a custom model's key.
pub fn settings(home: &Path, canary: &str) {
    write(&home.join("pager.toml"), "theme = \"dark\"\n");
    write(
        &home.join("config.toml"),
        &format!(
            "[mcp_servers.files]\ncommand = \"npx\"\nargs = [\"files-mcp\"]\n\n[model.mine]\nbase_url = \"https://example.invalid/v1\"\napi_key = \"{canary}\"\n"
        ),
    );
    write(&home.join("rules/style.md"), "# Style\n");
    write(&home.join("bin/grok"), "binary");
    write(&home.join("downloads/grok-macos"), "binary");
    write(
        &home.join("mcp_credentials.json"),
        &format!(r#"{{"files":{{"token":"{canary}"}}}}"#),
    );
}
