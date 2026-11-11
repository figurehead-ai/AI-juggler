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

use juggler_adapter::agent_bridge::{BridgeFile, BridgeInjection, BridgeLaunch, SERVE_ARG};

/// The per-launch file's name in the launch's folder.
pub const MCP_FILE: &str = "juggler-mcp.json";
/// A sealed launch's settings file, in the same folder.
pub const SEALED_FILE: &str = "juggler-sealed-settings.json";
/// Claude Code's own tools for talking to its other sessions.
const PEER_TOOLS: [&str; 2] = ["SendMessage", "ListAgents"];

/// What one `--settings` value holds: a file, or JSON in place.
fn settings_value(value: &str) -> Option<serde_json::Value> {
    let text = if value.trim_start().starts_with('{') {
        value.to_owned()
    } else {
        std::fs::read_to_string(value).ok()?
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .filter(serde_json::Value::is_object)
}

/// `later` over `earlier`: objects merged key by key, lists joined
/// without repeats (hooks run, rules deny, as both said), anything else
/// the later one's.
fn merge(earlier: &mut serde_json::Value, later: serde_json::Value) {
    match (earlier, later) {
        (serde_json::Value::Object(into), serde_json::Value::Object(from)) => {
            for (key, value) in from {
                match into.get_mut(&key) {
                    Some(slot) => merge(slot, value),
                    None => {
                        into.insert(key, value);
                    }
                }
            }
        }
        (serde_json::Value::Array(into), serde_json::Value::Array(from)) => {
            for value in from {
                if !into.contains(&value) {
                    into.push(value);
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

/// The settings a sealed launch gets: every `--settings` in `args`, in
/// order (the Account's overlay, the hooks), with other sessions'
/// messages refused and the tools to reach them denied.
fn sealed_settings(args: &[String]) -> serde_json::Value {
    let mut settings = serde_json::json!({});
    for (at, arg) in args.iter().enumerate() {
        if arg == "--settings"
            && let Some(value) = args.get(at + 1).and_then(|value| settings_value(value))
        {
            merge(&mut settings, value);
        }
    }
    merge(
        &mut settings,
        serde_json::json!({
            "crossSessionInbound": "refuse",
            "permissions": { "deny": PEER_TOOLS },
        }),
    );
    // Refuse, whatever an earlier one said.
    settings["crossSessionInbound"] = "refuse".into();
    settings
}

/// A sealed launch (P7 step 3): its settings file, passed last.
#[must_use]
pub fn sealed(dir: &std::path::Path, args: &[String]) -> BridgeInjection {
    BridgeInjection {
        at: args.len(),
        args: vec![
            "--settings".to_owned(),
            dir.join(SEALED_FILE).to_string_lossy().into_owned(),
        ],
        files: vec![BridgeFile {
            name: SEALED_FILE.to_owned(),
            contents: serde_json::to_string_pretty(&sealed_settings(args)).unwrap_or_default(),
        }],
        ..BridgeInjection::default()
    }
}

/// The injection for one launch with `args`.
#[must_use]
pub fn injection(bridge: &BridgeLaunch, args: &[String]) -> BridgeInjection {
    let env = bridge.env_refs_json(|name| format!("${{{name}}}"));
    let config = serde_json::json!({
        "mcpServers": {
            bridge.server.clone(): {
                "type": "stdio",
                "command": bridge.program.to_string_lossy(),
                "args": [SERVE_ARG],
                "env": env,
            }
        }
    });
    let allowed = bridge.tool_names(|server, tool| format!("mcp__{server}__{tool}"));
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
        ..BridgeInjection::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{MCP_FILE, SEALED_FILE, injection};
    use juggler_adapter::agent_bridge::BridgeLaunch;

    fn bridge() -> BridgeLaunch {
        crate::common::bridge_sample::sample(std::path::Path::new("/data/accounts/a"))
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
    /// can't send its own, and holds every earlier `--settings` (the
    /// Account's overlay, the hooks) so it reads the same whether Claude
    /// merges them or keeps the last.
    #[test]
    fn a_sealed_launch_leaves_claudes_own_channel_and_keeps_every_settings() {
        let args: Vec<String> = vec![
            "--settings".into(),
            r#"{"statusLine":{"type":"command","command":"wrap"},"permissions":{"deny":["WebFetch"]}}"#.into(),
            "fix it".into(),
            "--settings".into(),
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"report"}]}]},"crossSessionInbound":"accept"}"#.into(),
        ];
        let sealed = super::sealed(std::path::Path::new("/data/overlays/sealed/s1"), &args);
        assert_eq!(sealed.at, args.len(), "last");
        assert!(sealed.args[1].ends_with(SEALED_FILE));
        let settings: serde_json::Value = serde_json::from_str(&sealed.files[0].contents).unwrap();
        assert_eq!(settings["crossSessionInbound"], "refuse");
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(["WebFetch", "SendMessage", "ListAgents"])
        );
        assert_eq!(
            settings["statusLine"]["command"], "wrap",
            "the overlay kept"
        );
        assert_eq!(
            settings["hooks"]["Stop"][0]["hooks"][0]["command"], "report",
            "the hooks kept"
        );
        // The agent tools' own injection adds no settings.
        assert!(
            !injection(&bridge(), &args)
                .args
                .contains(&"--settings".to_owned())
        );
    }
}
