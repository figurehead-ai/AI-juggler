//! AJ-E3013 (D-116, D-119; `docs/research/desktop-pool-members.md` §2
//! Route A): AI Juggler's connector for a Claude app profile's Code tab.
//!
//! One folder in the profile's own Claude Code folder,
//! `<CLAUDE_CONFIG_DIR>/skills/ai-juggler/`, which Claude Code loads as the
//! plugin `ai-juggler@skills-dir` with no approval of its own (the user's
//! consent is AI Juggler's card):
//! - `.claude-plugin/plugin.json`: the bridge as its MCP server
//!   (`<agent> --mcp --plugin`, the edition's socket in its `env`: a path,
//!   not a secret) and the inbox hooks (`SessionStart`, `UserPromptSubmit`:
//!   `<agent> hook <event> --connector`);
//! - `skills/pool-tasks/SKILL.md`: the published plugin's own skill.
//!
//! Nothing is merged into a file Claude Code or the user writes
//! (`.claude.json`, `settings.json`): Remove deletes the files it wrote.

use juggler_adapter::adapter::shell_quote;
use juggler_adapter::connector::{ConnectorInput, ConnectorPart, ConnectorPlan, DesktopConnector};
use juggler_adapter::inventory::GetIt;

/// The connector's folder, inside the profile's Claude Code folder.
pub const FOLDER: &str = "skills/ai-juggler";

/// The plugin's name (`ai-juggler@skills-dir`; its tools are
/// `mcp__plugin_ai-juggler_ai-juggler__<tool>`).
pub const PLUGIN: &str = "ai-juggler";

/// The hook events the connector adds (the inbox note, D-116).
pub const HOOK_EVENTS: [&str; 2] = ["SessionStart", "UserPromptSubmit"];

/// The skill the published plugin carries, shipped the same.
const POOL_TASKS: &str =
    include_str!("../../../plugins/claude-code/ai-juggler/skills/pool-tasks/SKILL.md");

/// How long a hook may take, seconds (it answers in milliseconds, and
/// gives up on AI Juggler after two seconds).
const HOOK_TIMEOUT: u64 = 5;

/// The Claude app's connector.
pub struct ClaudeAppConnector;

impl DesktopConnector for ClaudeAppConnector {
    fn plan(&self, input: &ConnectorInput<'_>) -> ConnectorPlan {
        plan(input)
    }
}

/// The connector for `input`'s edition.
#[must_use]
pub fn plan(input: &ConnectorInput<'_>) -> ConnectorPlan {
    let agent = input.agent.to_string_lossy();
    let socket = input.socket.to_string_lossy();
    let hook = |event: &str| {
        serde_json::json!([{ "hooks": [{
            "type": "command",
            // L9: AI Juggler moved or removed: the hook says nothing and
            // succeeds, never a "command not found" in the session.
            "command": format!(
                "[ -x {agent} ] || exit 0; JUGGLER_AGENT_SOCKET={socket} {agent} hook {event} --connector",
                agent = shell_quote(&agent),
                socket = shell_quote(&socket),
            ),
            "timeout": HOOK_TIMEOUT,
        }]}])
    };
    let hooks: serde_json::Map<String, serde_json::Value> = HOOK_EVENTS
        .iter()
        .map(|event| ((*event).to_owned(), hook(event)))
        .collect();
    let manifest = serde_json::json!({
        "name": PLUGIN,
        "version": input.version,
        "description": "AI Juggler's pool connector: this profile's sessions read and send their pool's messages, shared docs and tasks while AI Juggler runs. Added by AI Juggler; remove it there.",
        "author": { "name": "Figurehead" },
        "homepage": "https://github.com/figurehead-ai/AI-juggler",
        "license": "MIT",
        "mcpServers": {
            PLUGIN: {
                "command": agent,
                "args": ["--mcp", "--plugin"],
                "env": { "JUGGLER_AGENT_SOCKET": socket },
            }
        },
        "hooks": hooks,
    });
    let mut manifest_text = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    manifest_text.push('\n');
    ConnectorPlan {
        parts: vec![
            ConnectorPart::File {
                path: format!("{FOLDER}/.claude-plugin/plugin.json").into(),
                text: manifest_text,
            },
            ConnectorPart::File {
                path: format!("{FOLDER}/skills/pool-tasks/SKILL.md").into(),
                text: POOL_TASKS.to_owned(),
            },
        ],
        picked_up: "New Code-tab sessions of this profile get it when they start; sessions open now get it when they next start.".to_owned(),
        guide: GetIt {
            steps: vec![
                "In the Claude app, open a Code session in this profile.".to_owned(),
                "Click + below the prompt, then Add plugins.".to_owned(),
                "Add the marketplace figurehead-ai/AI-juggler, then install ai-juggler (for your user).".to_owned(),
                "Start a new Code session: AI Juggler sees it join the pool.".to_owned(),
            ],
            command: Some("/plugin marketplace add figurehead-ai/AI-juggler".to_owned()),
            open_url: None,
            needs: Vec::new(),
            then: Some("The session's first call to AI Juggler's tools shows it here as connected.".to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn input() -> ConnectorPlan {
        plan(&ConnectorInput {
            agent: Path::new("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            socket: Path::new("/Users/u/Library/Application Support/AI Juggler/agent/bridge.sock"),
            version: "0.4.6",
            server: "juggler",
            tools: &["whoami"],
        })
    }

    #[test]
    fn the_plugin_names_the_bridge_by_its_path_and_holds_no_secret() {
        let plan = input();
        let ConnectorPart::File { path, text } = &plan.parts[0] else {
            unreachable!("a file");
        };
        assert_eq!(
            path,
            Path::new("skills/ai-juggler/.claude-plugin/plugin.json")
        );
        let manifest: serde_json::Value = serde_json::from_str(text).unwrap();
        let server = &manifest["mcpServers"]["ai-juggler"];
        assert_eq!(
            server["command"],
            "/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"
        );
        assert_eq!(server["args"], serde_json::json!(["--mcp", "--plugin"]));
        assert_eq!(
            server["env"].as_object().unwrap().len(),
            1,
            "the socket only"
        );
        let hook = manifest["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert_eq!(
            hook,
            "[ -x '/Applications/AI Juggler.app/Contents/MacOS/juggler-agent' ] || exit 0; JUGGLER_AGENT_SOCKET='/Users/u/Library/Application Support/AI Juggler/agent/bridge.sock' '/Applications/AI Juggler.app/Contents/MacOS/juggler-agent' hook UserPromptSubmit --connector"
        );
        assert!(manifest["hooks"]["SessionStart"].is_array());
        for part in &plan.parts {
            assert!(juggler_adapter::connector::stays_inside(part.path()));
            let ConnectorPart::File { text, .. } = part else {
                unreachable!("files only");
            };
            assert!(!text.contains("TOKEN"), "no key in any file");
        }
        assert!(plan.parts[1].path().ends_with("skills/pool-tasks/SKILL.md"));
    }

    /// L9: with AI Juggler moved or removed, each hook in the written
    /// plugin.json exits 0 and prints nothing.
    #[cfg(unix)]
    #[test]
    fn a_hook_is_silent_and_succeeds_when_the_agent_is_gone() {
        use std::io::Write as _;
        let root = tempfile::tempdir().unwrap();
        let gone = root
            .path()
            .join("AI Juggler.app/Contents/MacOS/juggler-agent");
        let plan = plan(&ConnectorInput {
            agent: &gone,
            socket: &root.path().join("agent/bridge.sock"),
            version: "0.4.6",
            server: "juggler",
            tools: &["whoami"],
        });
        let ConnectorPart::File { text, .. } = &plan.parts[0] else {
            unreachable!("a file");
        };
        let manifest: serde_json::Value = serde_json::from_str(text).unwrap();
        for event in HOOK_EVENTS {
            let command = manifest["hooks"][event][0]["hooks"][0]["command"]
                .as_str()
                .unwrap();
            let mut child = std::process::Command::new("/bin/sh")
                .args(["-c", command])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            // The hook input, as Claude Code sends it. The hook may exit
            // before reading it (it does at once when the program is
            // gone), so a closed pipe is the case under test; any other
            // write error isn't.
            let sent = child
                .stdin
                .take()
                .unwrap()
                .write_all(br#"{"session_id":"s"}"#);
            if let Err(error) = sent {
                assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe, "{event}");
            }
            let out = child.wait_with_output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{event}");
            assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{event}");
        }
    }
}
