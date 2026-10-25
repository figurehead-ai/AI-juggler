//! The fake tool's MCP client (AJ-E1704): what a real tool does with a
//! `--mcp-config` file, done the plain way so tests can watch it.
//!
//! It reads each server's `command`, `args` and `env` (filling `${NAME}`
//! from its own environment, as Claude Code documents), starts the server
//! with that `env` and nothing else of its own (no variable leaks in by
//! inheritance: the file must be enough), speaks MCP over its stdio, and
//! writes each request, its reply and how long it took to the log, one
//! JSON line each. Then it closes the server's stdin, as a tool does when
//! its session ends.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

/// What the fake asks, in order.
const REQUESTS: &[&str] = &[
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fake-harness","version":"0.1.0"}}}"#,
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"whoami","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"set_status","arguments":{"text":"Writing the fake's turns"}}}"#,
    r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"pool_roster","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":6,"method":"resources/read","params":{"uri":"juggler://pool/cards"}}"#,
];

/// `${NAME}` from this process's environment (empty when unset).
fn expand(value: &str) -> String {
    match value
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        Some(name) => std::env::var(name).unwrap_or_default(),
        None => value.to_owned(),
    }
}

/// Runs every server in `config`, logging to `log`.
/// `FAKE_HARNESS_MCP_DELAY_MS` waits before the first request: a real
/// tool's model calls a tool well after its launch is recorded, where the
/// fake would otherwise call in the same millisecond.
pub fn run(config: &Path, log: &Path) -> Result<(), String> {
    if let Some(ms) = std::env::var("FAKE_HARNESS_MCP_DELAY_MS")
        .ok()
        .and_then(|ms| ms.trim().parse::<u64>().ok())
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
    let text = std::fs::read_to_string(config).map_err(|e| format!("{}: {e}", config.display()))?;
    let config: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let servers = config["mcpServers"].as_object().ok_or("no mcpServers")?;
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|e| e.to_string())?;
    for (name, server) in servers {
        let command = server["command"].as_str().ok_or("no command")?;
        let mut child = Command::new(command);
        child.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            child.env("PATH", path);
        }
        for arg in server["args"].as_array().into_iter().flatten() {
            child.arg(arg.as_str().unwrap_or_default());
        }
        for (key, value) in server["env"].as_object().into_iter().flatten() {
            child.env(key, expand(value.as_str().unwrap_or_default()));
        }
        let mut child = child
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("{command}: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        let mut stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        for request in REQUESTS {
            let started = Instant::now();
            writeln!(stdin, "{request}").map_err(|e| e.to_string())?;
            let notification = !request.contains("\"id\"");
            let mut reply = String::new();
            if !notification {
                stdout.read_line(&mut reply).map_err(|e| e.to_string())?;
            }
            let entry = serde_json::json!({
                "server": name,
                "request": serde_json::from_str::<serde_json::Value>(request).unwrap_or_default(),
                "reply": serde_json::from_str::<serde_json::Value>(reply.trim()).unwrap_or_default(),
                "micros": u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
            });
            writeln!(out, "{entry}").map_err(|e| e.to_string())?;
        }
        drop(stdin);
        let _ = child.wait();
    }
    Ok(())
}
