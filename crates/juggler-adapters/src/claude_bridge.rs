//! AJ-E1704 (D-30): Claude Code takes AI Juggler's agent tools through
//! two launch options, never through the user's own settings:
//!
//! - `--mcp-config <file>`: a per-launch file in AI Juggler's overlay
//!   folder that adds the `juggler` server. Never `--strict-mcp-config`,
//!   which would drop the user's own servers. The file holds no secret:
//!   its `env` names the bridge's variables as `${NAME}`, which Claude
//!   Code fills in from its own environment (where AI Juggler put them).
//! - `--allowedTools`: AI Juggler's own tools only, so the session never
//!   asks about them (the pool's level is the real check); every other
//!   tool asks as the user set it.
//!
//! Both go after everything else: `--mcp-config` and `--allowedTools`
//! take several values, so they must not come before the prompt.
//! Unverified until the spike on spare accounts (AJ-E1701): that a
//! server from `--mcp-config` loads without an approval prompt.
//!
//! A session in a sealed pool (P7 step 3) also leaves Claude Code's own
//! cross-session messaging: `crossSessionInbound: "refuse"` drops what
//! other sessions send it, and `SendMessage` and `ListAgents` are denied
//! (code.claude.com/docs/en/cross-session-messaging). They go in a
//! per-launch settings file holding the Account's overlay too, passed
//! last, so the launch reads the same whether Claude merges several
//! `--settings` or keeps the last.

use juggler_adapter::agent_bridge::{BridgeFile, BridgeInjection, BridgeLaunch};

/// The per-launch file's name in the launch's folder.
pub const MCP_FILE: &str = "juggler-mcp.json";
/// A sealed launch's settings file, in the same folder.
pub const SEALED_FILE: &str = "juggler-sealed-settings.json";
/// Claude Code's own tools for talking to its other sessions.
const PEER_TOOLS: [&str; 2] = ["SendMessage", "ListAgents"];

/// The settings a sealed launch gets: what `args`' own `--settings`
/// holds (a file, or JSON in place), with other sessions' messages
/// refused and the tools to reach them denied.
fn sealed_settings(args: &[String]) -> serde_json::Value {
    let given = args
        .iter()
        .rposition(|arg| arg == "--settings")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| {
            let text = if value.trim_start().starts_with('{') {
                value.clone()
            } else {
                std::fs::read_to_string(value).ok()?
            };
            serde_json::from_str::<serde_json::Value>(&text).ok()
        })
        .filter(serde_json::Value::is_object);
    let mut settings = given.unwrap_or_else(|| serde_json::json!({}));
    settings["crossSessionInbound"] = "refuse".into();
    if !settings["permissions"].is_object() {
        settings["permissions"] = serde_json::json!({});
    }
    let mut deny: Vec<serde_json::Value> = settings["permissions"]["deny"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for tool in PEER_TOOLS {
        if !deny.iter().any(|one| one == tool) {
            deny.push(tool.into());
        }
    }
    settings["permissions"]["deny"] = deny.into();
    settings
}

/// The injection for one launch with `args`.
#[must_use]
pub fn injection(bridge: &BridgeLaunch, args: &[String]) -> BridgeInjection {
    let env: serde_json::Map<String, serde_json::Value> = bridge
        .env_names
        .iter()
        .map(|name| (name.clone(), format!("${{{name}}}").into()))
        .collect();
    let config = serde_json::json!({
        "mcpServers": {
            bridge.server.clone(): {
                "type": "stdio",
                "command": bridge.program.to_string_lossy(),
                "args": ["--mcp"],
                "env": env,
            }
        }
    });
    let allowed: Vec<String> = bridge
        .tools
        .iter()
        .map(|tool| format!("mcp__{}__{tool}", bridge.server))
        .collect();
    let mut injected = vec![
        "--mcp-config".to_owned(),
        bridge.dir.join(MCP_FILE).to_string_lossy().into_owned(),
        "--allowedTools".to_owned(),
        allowed.join(","),
    ];
    let mut files = vec![BridgeFile {
        name: MCP_FILE.to_owned(),
        contents: serde_json::to_string_pretty(&config).unwrap_or_default(),
    }];
    if bridge.sealed {
        injected.extend([
            "--settings".to_owned(),
            bridge.dir.join(SEALED_FILE).to_string_lossy().into_owned(),
        ]);
        files.push(BridgeFile {
            name: SEALED_FILE.to_owned(),
            contents: serde_json::to_string_pretty(&sealed_settings(args)).unwrap_or_default(),
        });
    }
    BridgeInjection {
        at: args.len(),
        args: injected,
        files,
    }
}

#[cfg(test)]
mod tests {
    use super::{MCP_FILE, SEALED_FILE, injection};
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use std::path::PathBuf;

    fn bridge() -> BridgeLaunch {
        BridgeLaunch {
            program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            server: "juggler".into(),
            tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
            env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
            dir: PathBuf::from("/data/overlays/agent/sess-1"),
            sealed: false,
        }
    }

    #[test]
    fn adds_the_server_by_file_and_pre_allows_only_its_tools() {
        let args: Vec<String> = vec![
            "fix the \"- bullet\"".into(),
            "--add-dir".into(),
            "/h".into(),
        ];
        let injection = injection(&bridge(), &args);
        let applied = injection.applied(&args);
        assert_eq!(
            &applied[..3],
            &args[..],
            "the launch's own arguments first, unchanged"
        );
        assert_eq!(
            &applied[3..],
            [
                "--mcp-config",
                "/data/overlays/agent/sess-1/juggler-mcp.json",
                "--allowedTools",
                "mcp__juggler__whoami,mcp__juggler__pool_roster,mcp__juggler__set_status",
            ]
        );
        assert!(!applied.iter().any(|arg| arg.contains("strict-mcp-config")));
        let file = &injection.files[0];
        assert_eq!(file.name, MCP_FILE);
        let config: serde_json::Value = serde_json::from_str(&file.contents).unwrap();
        let server = &config["mcpServers"]["juggler"];
        assert_eq!(server["args"], serde_json::json!(["--mcp"]));
        assert_eq!(
            server["env"]["JUGGLER_AGENT_TOKEN"],
            "${JUGGLER_AGENT_TOKEN}"
        );
        assert_eq!(
            server["command"],
            "/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"
        );
    }

    /// P7 step 3: a sealed launch refuses other sessions' messages and
    /// can't send its own, and keeps the Account's overlay with it.
    #[test]
    fn a_sealed_launch_leaves_claudes_own_channel_and_keeps_the_overlay() {
        let mut bridge = bridge();
        let args: Vec<String> = vec![
            "--settings".into(),
            r#"{"statusLine":{"type":"command","command":"wrap"},"permissions":{"deny":["WebFetch"]}}"#.into(),
            "fix it".into(),
        ];
        let plain = injection(&bridge, &args);
        assert!(!plain.args.contains(&"--settings".to_owned()));
        bridge.sealed = true;
        let sealed = injection(&bridge, &args);
        let at = sealed.args.iter().position(|a| a == "--settings").unwrap();
        assert!(sealed.args[at + 1].ends_with(SEALED_FILE));
        let file = sealed.files.iter().find(|f| f.name == SEALED_FILE).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&file.contents).unwrap();
        assert_eq!(settings["crossSessionInbound"], "refuse");
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(["WebFetch", "SendMessage", "ListAgents"])
        );
        assert_eq!(
            settings["statusLine"]["command"], "wrap",
            "the overlay kept"
        );
        // Its own settings come after the overlay's.
        let applied = sealed.applied(&args);
        let last = applied.iter().rposition(|a| a == "--settings").unwrap();
        assert!(applied[last + 1].ends_with(SEALED_FILE));
    }
}
