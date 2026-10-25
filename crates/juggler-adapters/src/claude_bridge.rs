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

use juggler_adapter::agent_bridge::{BridgeFile, BridgeInjection, BridgeLaunch};

/// The per-launch file's name in the launch's folder.
pub const MCP_FILE: &str = "juggler-mcp.json";

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
    BridgeInjection {
        at: args.len(),
        args: vec![
            "--mcp-config".to_owned(),
            bridge.dir.join(MCP_FILE).to_string_lossy().into_owned(),
            "--allowedTools".to_owned(),
            allowed.join(","),
        ],
        files: vec![BridgeFile {
            name: MCP_FILE.to_owned(),
            contents: serde_json::to_string_pretty(&config).unwrap_or_default(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::{MCP_FILE, injection};
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use std::path::PathBuf;

    fn bridge() -> BridgeLaunch {
        BridgeLaunch {
            program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            server: "juggler".into(),
            tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
            env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
            dir: PathBuf::from("/data/overlays/agent/sess-1"),
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
}
