//! AJ-E3013 (D-116; `docs/research/desktop-pool-members.md` §4 Route K):
//! AI Juggler's connector for the Codex app (ChatGPT): one table,
//! `[mcp_servers.juggler]`, merged into that App's own `config.toml`
//! (`CODEX_HOME`, shared with the terminal `codex`), every other byte of
//! the file kept. Its tools are pre-allowed one by one, the server's
//! default left at `prompt` (D-30): the pool's level is the real check.
//!
//! The name is the one AI Juggler's own Codex launches override with `-c`
//! (`codex_bridge.rs`), so a launched session never lists the tools twice.
//! A Codex app thread can't be attested yet (`codex_peer`): until a lab
//! probe proves which thread calls, its calls are refused with the reason.

use std::fmt::Write as _;

use juggler_adapter::agent_bridge::{toml_list, toml_string};
use juggler_adapter::connector::{ConnectorInput, ConnectorPart, ConnectorPlan, DesktopConnector};
use juggler_adapter::inventory::GetIt;

/// The config file, inside the App's `CODEX_HOME`.
pub const CONFIG: &str = "config.toml";

/// The Codex app's connector.
pub struct CodexAppConnector;

impl DesktopConnector for CodexAppConnector {
    fn plan(&self, input: &ConnectorInput<'_>) -> ConnectorPlan {
        plan(input)
    }
}

/// The `[mcp_servers.<server>]` table for `input`'s edition, as TOML.
#[must_use]
pub fn table_text(input: &ConnectorInput<'_>) -> String {
    let server = input.server;
    let tools: Vec<String> = input.tools.iter().map(|tool| (*tool).to_owned()).collect();
    let mut text = format!(
        "[mcp_servers.{server}]\ncommand = {}\nargs = {}\nenv = {{ JUGGLER_AGENT_SOCKET = {} }}\nenabled_tools = {}\ndefault_tools_approval_mode = {}\n",
        toml_string(&input.agent.to_string_lossy()),
        toml_list(&["--mcp".to_owned(), "--plugin".to_owned()]),
        toml_string(&input.socket.to_string_lossy()),
        toml_list(&tools),
        toml_string("prompt"),
    );
    for tool in input.tools {
        let _ = write!(
            text,
            "\n[mcp_servers.{server}.tools.{tool}]\napproval_mode = {}\n",
            toml_string("approve")
        );
    }
    text
}

/// The connector for `input`'s edition.
#[must_use]
pub fn plan(input: &ConnectorInput<'_>) -> ConnectorPlan {
    let text = table_text(input);
    ConnectorPlan {
        parts: vec![ConnectorPart::TomlTable {
            path: CONFIG.into(),
            table: vec!["mcp_servers".to_owned(), input.server.to_owned()],
            text: text.clone(),
        }],
        picked_up: "Codex app threads get it after Settings › MCP servers › Restart, or when the app next starts. Terminal codex sessions on this folder get it too.".to_owned(),
        guide: GetIt {
            steps: vec![
                "Quit the Codex app (it rewrites config.toml while it runs).".to_owned(),
                "Open config.toml in this App's Codex folder in a text editor.".to_owned(),
                "Paste the lines below at its end and save.".to_owned(),
                "Start the Codex app again.".to_owned(),
            ],
            command: Some(text),
            open_url: None,
            needs: Vec::new(),
            then: Some("AI Juggler sees the connector in config.toml.".to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_table_names_the_bridge_and_pre_allows_each_tool() {
        let input = ConnectorInput {
            agent: Path::new("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
            socket: Path::new("/u/Library/Application Support/AI Juggler/agent/bridge.sock"),
            version: "0.4.6",
            server: "juggler",
            tools: &["whoami", "send_message"],
        };
        let text = table_text(&input);
        let parsed: toml::Table = text.parse().unwrap();
        let server = &parsed["mcp_servers"]["juggler"];
        assert_eq!(
            server["command"].as_str(),
            Some("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent")
        );
        assert_eq!(
            server["default_tools_approval_mode"].as_str(),
            Some("prompt")
        );
        assert_eq!(
            server["tools"]["send_message"]["approval_mode"].as_str(),
            Some("approve")
        );
        assert_eq!(
            server["env"].as_table().unwrap().len(),
            1,
            "the socket only"
        );
        assert!(!text.contains("TOKEN"));
        let plan = plan(&input);
        assert!(
            matches!(&plan.parts[0], ConnectorPart::TomlTable { table, .. } if table == &["mcp_servers", "juggler"])
        );
    }
}
