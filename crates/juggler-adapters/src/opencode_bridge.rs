//! AJ-E3002 R3: OpenCode takes AI Juggler's agent tools per launch, in
//! its inline settings (`OPENCODE_CONFIG_CONTENT`, the highest layer it
//! reads), never through the user's own config files:
//!
//! - `mcp.<server>`: a `local` server running the bridge program with
//!   `--mcp`. Its `environment` names the bridge's variables as
//!   `{env:NAME}`, which OpenCode fills in from its own environment
//!   (where AI Juggler put them), so no value is in the settings. The
//!   server would inherit them anyway: OpenCode starts a local server
//!   with its whole environment plus `environment` (`MCP.connectLocal`,
//!   1.19.5); the names keep that explicit.
//! - `permission.<server>_<tool>: "allow"` for each of AI Juggler's
//!   tools (OpenCode names an MCP tool `<server>_<tool>`), and only them:
//!   a user's `"*": "ask"` stays for everything else (OpenCode turns a
//!   plain `"permission": "ask"` into `{"*": "ask"}` per file, then
//!   merges; the later rule wins for the tools named).
//!
//! It merges into the inline settings the launch already carries (a
//! Launch-settings model), keeping them. Lab 2026-11-09 (OpenCode 1.19.5,
//! throwaway homes): `opencode mcp list` shows the server connected with
//! the key reaching it by `{env:…}`, and `opencode debug config` shows a
//! user's `"ask"` kept beside the allowed tools. Unverified until a
//! signed-in run: that a tool call then runs without a prompt.

use juggler_adapter::LaunchSpec;
use juggler_adapter::agent_bridge::{BridgeInjection, BridgeLaunch};
use serde_json::{Map, Value, json};

/// OpenCode's inline settings: the highest layer, merged over its files.
pub const CONFIG_CONTENT_ENV: &str = "OPENCODE_CONFIG_CONTENT";

/// OpenCode's name for an MCP server's tool (`MCP.tools`: both parts
/// with anything but letters, digits, `_` and `-` made `_`).
#[must_use]
pub fn tool_name(server: &str, tool: &str) -> String {
    let clean = |text: &str| {
        text.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    format!("{}_{}", clean(server), clean(tool))
}

/// `settings` (inline JSON, or none) with the bridge's server and its
/// tools' permissions added; everything else kept.
fn merged(existing: Option<&String>, bridge: &BridgeLaunch) -> Value {
    let mut settings = existing
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let environment: Map<String, Value> = bridge
        .env_names
        .iter()
        .map(|name| (name.clone(), Value::String(format!("{{env:{name}}}"))))
        .collect();
    let server = json!({
        "type": "local",
        "command": [bridge.program.to_string_lossy(), "--mcp"],
        "environment": environment,
        "enabled": true,
    });
    if !settings["mcp"].is_object() {
        settings["mcp"] = json!({});
    }
    settings["mcp"][&bridge.server] = server;
    // A plain action is OpenCode's shorthand for every tool: keep it as
    // that rule, then add ours after it.
    let permission = match settings.get("permission") {
        Some(Value::String(action)) => json!({ "*": action }),
        Some(Value::Object(rules)) => Value::Object(rules.clone()),
        _ => json!({}),
    };
    settings["permission"] = permission;
    for tool in &bridge.tools {
        settings["permission"][tool_name(&bridge.server, tool)] = json!("allow");
    }
    settings
}

/// The injection for one launch.
#[must_use]
pub fn injection(bridge: &BridgeLaunch, launch: &LaunchSpec) -> BridgeInjection {
    let settings = merged(launch.env.get(CONFIG_CONTENT_ENV), bridge);
    BridgeInjection {
        at: launch.args.len(),
        env: vec![(CONFIG_CONTENT_ENV.to_owned(), settings.to_string())],
        ..BridgeInjection::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{CONFIG_CONTENT_ENV, injection, tool_name};
    use juggler_adapter::LaunchSpec;
    use juggler_adapter::agent_bridge::BridgeLaunch;
    use serde_json::{Value, json};
    use std::path::PathBuf;

    fn bridge() -> BridgeLaunch {
        BridgeLaunch {
            program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            server: "juggler".into(),
            tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
            env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
            dir: PathBuf::from("/data/overlays/agent/sess-1"),
            state_dir: PathBuf::from("/data/accounts/a"),
        }
    }

    fn launch(content: Option<&str>) -> LaunchSpec {
        let mut spec = LaunchSpec {
            program: PathBuf::from("/opt/homebrew/bin/opencode"),
            args: vec!["--prompt=- fix it".into()],
            env: std::collections::BTreeMap::new(),
            cwd: PathBuf::from("/p"),
            needs_pty: true,
        };
        spec.env
            .insert("JUGGLER_AGENT_TOKEN".into(), "secret-key-value".into());
        if let Some(content) = content {
            spec.env.insert(CONFIG_CONTENT_ENV.into(), content.into());
        }
        spec
    }

    fn settings_of(spec: &LaunchSpec) -> Value {
        serde_json::from_str(&spec.env[CONFIG_CONTENT_ENV]).unwrap()
    }

    #[test]
    fn the_server_and_only_its_tools_go_in_the_inline_settings() {
        let mut spec = launch(None);
        let injection = injection(&bridge(), &spec);
        assert!(injection.args.is_empty() && injection.files.is_empty());
        assert!(injection.home.is_none());
        injection.apply(&mut spec);
        assert_eq!(spec.args, ["--prompt=- fix it"]);
        assert_eq!(
            settings_of(&spec),
            json!({
                "mcp": {"juggler": {
                    "type": "local",
                    "command": ["/Applications/AI Juggler.app/Contents/MacOS/juggler-agent", "--mcp"],
                    "environment": {
                        "JUGGLER_AGENT_TOKEN": "{env:JUGGLER_AGENT_TOKEN}",
                        "JUGGLER_AGENT_SOCKET": "{env:JUGGLER_AGENT_SOCKET}",
                    },
                    "enabled": true,
                }},
                "permission": {
                    "juggler_whoami": "allow",
                    "juggler_pool_roster": "allow",
                    "juggler_set_status": "allow",
                },
            })
        );
        // Never a value from the environment, and no wildcard.
        let text = &spec.env[CONFIG_CONTENT_ENV];
        assert!(!text.contains("secret-key-value") && !text.contains('*'));
    }

    #[test]
    fn a_launch_settings_model_and_other_inline_settings_are_kept() {
        let mut spec = launch(Some(
            r#"{"model":"anthropic/claude-sonnet","mcp":{"docs":{"type":"remote","url":"https://d"}},"permission":{"bash":"ask"}}"#,
        ));
        injection(&bridge(), &spec).apply(&mut spec);
        let settings = settings_of(&spec);
        assert_eq!(settings["model"], "anthropic/claude-sonnet");
        assert_eq!(settings["mcp"]["docs"]["url"], "https://d");
        assert_eq!(settings["mcp"]["juggler"]["type"], "local");
        assert_eq!(settings["permission"]["bash"], "ask");
        assert_eq!(settings["permission"]["juggler_whoami"], "allow");
    }

    #[test]
    fn a_plain_permission_stays_the_rule_for_every_other_tool() {
        let mut spec = launch(Some(r#"{"permission":"ask"}"#));
        injection(&bridge(), &spec).apply(&mut spec);
        let settings = settings_of(&spec);
        let rules: Vec<(&String, &Value)> =
            settings["permission"].as_object().unwrap().iter().collect();
        assert_eq!(rules[0], (&"*".to_owned(), &json!("ask")));
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn tool_names_follow_opencode() {
        assert_eq!(tool_name("juggler", "task_get"), "juggler_task_get");
        assert_eq!(tool_name("a.b", "c d"), "a_b_c_d");
    }
}
