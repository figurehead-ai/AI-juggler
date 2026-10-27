//! What a Codex conversation used, from its rollout JSONL
//! (switch-setup-items.md §4, from `codex-rs/rollout/src/policy.rs` and
//! the CLI's own strings):
//! - an MCP call: a `function_call` named `mcp__<server>__<tool>`, and in
//!   the legacy history an `mcp_tool_call_end` event's `invocation.server`
//!   (an app's calls have server `codex_apps`);
//! - a skill: a user message wrapped in `<skill>` with its `<name>` (an
//!   explicit `$name` pick), or a call that reads a `skills/<name>/SKILL.md`
//!   (a best effort: Codex keeps no other record of a skill used). The
//!   `<skills_instructions>` catalog lists every skill and is not use;
//! - a subagent: `spawn_agent`'s `agent_type`;
//! - a plugin: a `plugin://<name>@<marketplace>` mention.
//!
//! Only names are kept: never a prompt, an argument or a skill's text.

use std::collections::BTreeSet;
use std::io::BufRead as _;
use std::path::Path;

use juggler_adapter::inventory::ItemKind;

/// What the rollout at `path` used, by kind and name; `None` when it
/// can't be read.
#[must_use]
pub fn used_in(path: &Path) -> Option<Vec<(ItemKind, String)>> {
    let file = std::fs::File::open(path).ok()?;
    let mut used = BTreeSet::new();
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        if !(line.contains("function_call")
            || line.contains("mcp_tool_call_end")
            || line.contains("<skill>")
            || line.contains("plugin://"))
        {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if let Some(payload) = value.get("payload") {
            read_payload(payload, &mut used);
        }
    }
    Some(used.into_iter().collect())
}

fn read_payload(payload: &serde_json::Value, used: &mut BTreeSet<(ItemKind, String)>) {
    let text = |key: &str| payload.get(key).and_then(serde_json::Value::as_str);
    match text("type") {
        Some("function_call" | "custom_tool_call") => {
            let name = text("name").unwrap_or_default();
            let arguments = text("arguments")
                .or_else(|| text("input"))
                .unwrap_or_default();
            if let Some(found) = crate::claude_used::mcp_server(name) {
                used.insert(found);
            } else if name == "spawn_agent"
                && let Some(agent) = serde_json::from_str::<serde_json::Value>(arguments)
                    .ok()
                    .and_then(|args| {
                        args.get("agent_type")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    })
            {
                used.insert((ItemKind::Subagent, agent));
            }
            for skill in skill_reads(arguments) {
                used.insert((ItemKind::Skill, skill));
            }
        }
        Some("mcp_tool_call_end") => {
            if let Some(server) = payload
                .get("invocation")
                .and_then(|call| call.get("server"))
                .and_then(serde_json::Value::as_str)
            {
                used.insert((ItemKind::McpServer, server.to_owned()));
            }
        }
        Some("message") if text("role") == Some("user") => {
            for block in payload
                .get("content")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(text) = block.get("text").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if let Some(skill) = picked_skill(text) {
                    used.insert((ItemKind::Skill, skill));
                }
                for plugin in plugin_mentions(text) {
                    used.insert((ItemKind::Plugin, plugin));
                }
            }
        }
        _ => {}
    }
}

/// A user message that is a skill Codex injected for an explicit pick:
/// `<skill>` first, then its `<name>`.
fn picked_skill(text: &str) -> Option<String> {
    let rest = text.trim_start().strip_prefix("<skill>")?;
    let start = rest.find("<name>")? + "<name>".len();
    let end = rest[start..].find("</name>")?;
    let name = rest[start..start + end].trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// Each `skills/<name>/SKILL.md` a call's arguments name.
fn skill_reads(arguments: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = arguments;
    while let Some(at) = rest.find("/SKILL.md") {
        let before = &rest[..at];
        let name = before
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .trim_matches(['"', '\'']);
        let parent = before[..before.len() - name.len()]
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default();
        if parent == "skills" && !name.is_empty() && !name.starts_with('.') {
            found.push(name.to_owned());
        }
        rest = &rest[at + "/SKILL.md".len()..];
    }
    found
}

/// Each `plugin://<name>@<marketplace>` in a user's message, as `<name>`.
fn plugin_mentions(text: &str) -> Vec<String> {
    text.match_indices("plugin://")
        .filter_map(|(at, _)| {
            let rest = &text[at + "plugin://".len()..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '@'))
                .collect();
            let name = id.split('@').next().unwrap_or_default();
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_reads_and_picks_are_found() {
        assert_eq!(
            skill_reads(r#"{"cmd":["cat","/home/u/.agents/skills/ship-it/SKILL.md"]}"#),
            vec!["ship-it".to_owned()]
        );
        assert!(skill_reads(r#"{"cmd":"cat /x/SKILL.md"}"#).is_empty());
        assert_eq!(
            picked_skill("<skill>\n<name>ship-it</name>\n<path>/p</path>"),
            Some("ship-it".to_owned())
        );
        assert_eq!(picked_skill("<skills_instructions>…"), None);
        assert_eq!(
            plugin_mentions("use plugin://notes@market please"),
            vec!["notes".to_owned()]
        );
    }
}
