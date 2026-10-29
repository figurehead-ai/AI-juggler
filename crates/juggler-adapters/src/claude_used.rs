//! What a Claude Code conversation used, from its JSONL transcript (the
//! CLI's and the desktop app's Code tab alike; switch-setup-items.md §3):
//! - a `Skill` tool call (`input.skill`), and the `invoked_skills`
//!   attachment Claude Code writes after one;
//! - a command or skill the user typed (`<command-name>/name</command-name>`
//!   in a user line; synced skills as `anthropic-skills:<name>`, a
//!   plugin's as `<plugin>:<name>`);
//! - an MCP tool call (`mcp__<server>__<tool>`; a plugin's server is
//!   `mcp__plugin_<plugin>_<server>__<tool>`);
//! - a subagent (`Task` or `Agent`, `input.subagent_type`).
//!
//! Only names are kept: never a prompt, an argument or a skill's text.
//! The file is read line by line, and a line that can't hold any of these
//! isn't parsed.

use std::collections::BTreeSet;
use std::io::BufRead as _;
use std::path::Path;

use juggler_adapter::inventory::ItemKind;

/// What the transcript at `path` used, by kind and name; `None` when it
/// can't be read.
#[must_use]
pub fn used_in(path: &Path) -> Option<Vec<(ItemKind, String)>> {
    let file = std::fs::File::open(path).ok()?;
    let mut used = BTreeSet::new();
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        if !(line.contains("tool_use")
            || line.contains("<command-name>")
            || line.contains("invoked_skills"))
        {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        read_line(&value, &mut used);
    }
    Some(used.into_iter().collect())
}

fn read_line(value: &serde_json::Value, used: &mut BTreeSet<(ItemKind, String)>) {
    let kind = value.get("type").and_then(serde_json::Value::as_str);
    // The attachment, at the top or under `attachment`.
    for holder in [Some(value), value.get("attachment")].into_iter().flatten() {
        if holder.get("type").and_then(serde_json::Value::as_str) == Some("invoked_skills") {
            for skill in holder
                .get("skills")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(name) = skill.get("name").and_then(serde_json::Value::as_str) {
                    add_skill(name, used);
                }
            }
        }
    }
    let content = value
        .get("message")
        .and_then(|message| message.get("content"));
    match kind {
        Some("assistant") => {
            for block in content
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                if block.get("type").and_then(serde_json::Value::as_str) == Some("tool_use") {
                    tool_use(block, used);
                }
            }
        }
        Some("user") => {
            let mut texts = Vec::new();
            match content {
                Some(serde_json::Value::String(text)) => texts.push(text.as_str()),
                Some(serde_json::Value::Array(blocks)) => texts.extend(
                    blocks
                        .iter()
                        .filter_map(|block| block.get("text").and_then(serde_json::Value::as_str)),
                ),
                _ => {}
            }
            for text in texts {
                for name in command_names(text) {
                    add_typed(name, used);
                }
            }
        }
        _ => {}
    }
}

fn tool_use(block: &serde_json::Value, used: &mut BTreeSet<(ItemKind, String)>) {
    let name = block
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let input = block.get("input");
    let field = |key: &str| {
        input
            .and_then(|input| input.get(key))
            .and_then(serde_json::Value::as_str)
    };
    match name {
        "Skill" => {
            if let Some(skill) = field("skill") {
                add_skill(skill, used);
            }
        }
        "Task" | "Agent" => {
            if let Some(agent) = field("subagent_type") {
                let (plugin, agent) = split_plugin(agent);
                if let Some(plugin) = plugin {
                    used.insert((ItemKind::Plugin, plugin.to_owned()));
                }
                used.insert((ItemKind::Subagent, agent.to_owned()));
            }
        }
        _ => {
            if let Some((kind, name)) = mcp_server(name) {
                used.insert((kind, name));
            }
        }
    }
}

/// `mcp__<server>__<tool>` as the server; a plugin's
/// (`mcp__plugin_<plugin>_<server>__<tool>`) as its plugin.
pub(crate) fn mcp_server(tool: &str) -> Option<(ItemKind, String)> {
    let rest = tool.strip_prefix("mcp__")?;
    let (server, _) = rest.split_once("__")?;
    if server.is_empty() {
        return None;
    }
    if let Some(plugin) = server.strip_prefix("plugin_") {
        let plugin = plugin.split_once('_').map_or(plugin, |(plugin, _)| plugin);
        return Some((ItemKind::Plugin, plugin.to_owned()));
    }
    Some((ItemKind::McpServer, server.to_owned()))
}

/// A skill by the name it was called with: a synced one's
/// `anthropic-skills:` dropped, a plugin's counted for its plugin too.
fn add_skill(name: &str, used: &mut BTreeSet<(ItemKind, String)>) {
    let (plugin, skill) = split_plugin(name);
    if let Some(plugin) = plugin {
        used.insert((ItemKind::Plugin, plugin.to_owned()));
    }
    used.insert((ItemKind::Skill, skill.to_owned()));
}

/// What the user typed as `/name`: a command or a skill, either.
fn add_typed(name: &str, used: &mut BTreeSet<(ItemKind, String)>) {
    add_skill(name, used);
    let (_, command) = split_plugin(name);
    used.insert((ItemKind::Command, command.to_owned()));
}

/// `anthropic-skills:docs` is `docs`; `notes:save` is the plugin `notes`'s
/// `save`.
fn split_plugin(name: &str) -> (Option<&str>, &str) {
    match name.split_once(':') {
        Some(("anthropic-skills", skill)) => (None, skill),
        Some((plugin, skill)) if !plugin.is_empty() && !skill.is_empty() => (Some(plugin), skill),
        _ => (None, name),
    }
}

/// Each `<command-name>/name</command-name>` in `text`, as Claude Code's
/// own scanner reads it (`[\w:-]+`).
fn command_names(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("<command-name>") {
        rest = &rest[at + "<command-name>".len()..];
        let Some(end) = rest.find("</command-name>") else {
            break;
        };
        let name = rest[..end].trim().trim_start_matches('/');
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '-'))
        {
            names.push(name);
        }
        rest = &rest[end..];
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_read_from_each_shape() {
        assert_eq!(
            mcp_server("mcp__github__create_issue"),
            Some((ItemKind::McpServer, "github".to_owned()))
        );
        assert_eq!(
            mcp_server("mcp__plugin_notes_store__save"),
            Some((ItemKind::Plugin, "notes".to_owned()))
        );
        assert_eq!(mcp_server("Bash"), None);
        assert_eq!(
            command_names("<command-name>/anthropic-skills:docs</command-name> x"),
            vec!["anthropic-skills:docs"]
        );
        assert_eq!(split_plugin("anthropic-skills:docs"), (None, "docs"));
        assert_eq!(split_plugin("notes:save"), (Some("notes"), "save"));
    }

    #[test]
    fn an_unreadable_transcript_says_nothing() {
        assert_eq!(used_in(Path::new("/nonexistent/transcript.jsonl")), None);
    }
}
