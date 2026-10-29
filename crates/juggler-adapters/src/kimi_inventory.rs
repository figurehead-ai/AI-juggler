//! AJ-E1603 (AJ-E1507, AJ-E1542): what a Kimi Code home has set up, and
//! setting it up, so its rules, skills and MCP servers join the sync.
//!
//! Read from the home's own files, never by starting Kimi Code or a
//! server (research `docs/research/kimi.md` §2, §6):
//! - MCP servers from `mcp.json`, in the `mcpServers` form Claude Code
//!   uses (stdio, HTTP and SSE);
//! - skills (`skills/<name>/SKILL.md`; the shared `~/.agents/skills` is
//!   every tool's, not this account's);
//! - its main rules file, `AGENTS.md`.
//!
//! Setting up a server adds one member to `mcpServers` (every other byte
//! kept; the undo takes it out only while it is as it was made); a skill
//! or a rules file is created, never replaced. A secret is never written:
//! it becomes `${NAME}`, and the user is told to provide it.

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo,
};

use crate::inventory_files::{children, secret_names};
use crate::kimi_code::KIMI_CODE_ID;

/// The servers' file in a home.
pub const MCP_FILE: &str = "mcp.json";

/// Everything the Kimi Code home `home` has set up.
#[must_use]
pub fn inventory(home: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let servers = std::fs::read_to_string(home.join(MCP_FILE))
        .ok()
        .and_then(|text| juggler_adapter::jsonc::parse(&text).ok())
        .and_then(|value| {
            value
                .get("mcpServers")
                .and_then(serde_json::Value::as_object)
                .cloned()
        })
        .unwrap_or_default();
    let mut names: Vec<&String> = servers.keys().collect();
    names.sort();
    for name in names {
        let entry = &servers[name];
        if entry.get("disabled").and_then(serde_json::Value::as_bool) != Some(true) {
            items.push(crate::claude_inventory::mcp_json_item(
                KIMI_CODE_ID,
                name,
                entry,
                Scope::User,
            ));
        }
    }
    for (name, path) in children(&home.join("skills"), true) {
        if path.join("SKILL.md").is_file() {
            items.push(item(ItemKind::Skill, &name, Detail::Files { path }));
        }
    }
    let rules = home.join("AGENTS.md");
    if rules.is_file() {
        items.push(item(
            ItemKind::MainRules,
            "AGENTS.md",
            Detail::Files { path: rules },
        ));
    }
    items
}

/// Where a project's own server goes in Kimi Code.
const EVERY_PROJECT: &str =
    "for every project of this Kimi Code account: AI Juggler sets servers up for the account only";

/// Whether Kimi Code can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_kimi = item.tool == KIMI_CODE_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so Kimi Code can't have it".to_owned(),
        ),
        (ItemKind::McpServer, Detail::Mcp(server)) => {
            match (item.scope, mcp_verdict(&item.name, server)) {
                (Scope::ProjectPrivate, SetUp::Copy) => SetUp::Translate(EVERY_PROJECT.to_owned()),
                (Scope::ProjectPrivate, SetUp::Translate(note)) => {
                    SetUp::Translate(format!("{note}; {EVERY_PROJECT}"))
                }
                (_, verdict) => verdict,
            }
        }
        // A skill is a folder with SKILL.md in every tool that has them.
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Copy,
        (ItemKind::MainRules, Detail::Files { .. }) if from_kimi => SetUp::Copy,
        (ItemKind::MainRules, Detail::Files { .. }) => SetUp::Translate(
            "as this account's AGENTS.md (Kimi Code doesn't read @-imports)".to_owned(),
        ),
        (ItemKind::Rules, Detail::Files { .. }) => SetUp::NeedsYou(
            "Kimi Code reads one rules file, AGENTS.md: add what this one says to it yourself"
                .to_owned(),
        ),
        _ => SetUp::Impossible(format!(
            "a {} from {{tool}} has no Kimi Code equivalent in AI Juggler yet",
            item.kind.words(),
        )),
    }
}

/// Sets up `item` in the Kimi Code home `home`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the home already has; the write's error.
pub fn set_up(home: &Path, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(home, &item.name, server),
        (ItemKind::Skill, Detail::Files { path }, _) => crate::inventory_files::copy_tree(
            path,
            &home.join("skills").join(&item.name),
            str::to_owned,
        ),
        (ItemKind::MainRules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &home.join("AGENTS.md"), str::to_owned)
        }
        _ => Err(AdapterError::Unsupported(format!(
            "{} needs you to set it up",
            item.name
        ))),
    }
}

fn mcp_verdict(name: &str, server: &McpServer) -> SetUp {
    if let Some(verdict) = server.inline_secret_verdict() {
        return verdict;
    }
    if let McpTransport::Other { kind, .. } = &server.transport
        && kind != "sse"
    {
        return SetUp::Impossible(format!(
            "Kimi Code reaches servers over stdio, HTTP and SSE, not {kind}"
        ));
    }
    if server.oauth {
        return SetUp::NeedsYou(
            "sign it in afterwards in Kimi Code: a sign-in never moves".to_owned(),
        );
    }
    let secrets = secret_names(name, server);
    if !secrets.is_empty() {
        return SetUp::NeedsYou(format!(
            "it needs {} in your environment: secrets are never copied",
            secrets.join(", ")
        ));
    }
    SetUp::Copy
}

/// Adds `mcpServers.<name>` to `mcp.json`, every other byte kept.
fn add_mcp(home: &Path, name: &str, server: &McpServer) -> Result<Applied, AdapterError> {
    if inventory(home)
        .iter()
        .any(|item| item.kind == ItemKind::McpServer && item.name == name)
    {
        return Err(AdapterError::Unsupported(format!(
            "Kimi Code already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    let file = home.join(MCP_FILE);
    let key = crate::config_edit::json_pointer(&["mcpServers", name]);
    let value = crate::claude_inventory::claude_entry(name, server).to_string();
    crate::config_edit::set(&file, ConfigFormat::Json, &[(key.clone(), value.clone())])
        .map_err(AdapterError::Unsupported)?;
    Ok(Applied {
        summary: format!("Added the MCP server {name} to Kimi Code"),
        undo: Undo::RemoveSetting {
            file,
            format: ConfigFormat::Json,
            entries: vec![(
                key,
                crate::config_edit::fingerprint(ConfigFormat::Json, &value),
            )],
        },
        note: None,
    })
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: KIMI_CODE_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
        origin: juggler_adapter::inventory::Origin::Own,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::inventory::Value;

    #[test]
    fn a_kimi_home_lists_its_servers_skills_and_rules_and_keeps_no_secret() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join(MCP_FILE),
            r#"{"mcpServers": {
                "docs": {"url": "https://docs.example/mcp", "headers": {"Authorization": "Bearer madeup0123456789abcdef0123456789abcdef"}},
                "files": {"command": "npx", "args": ["files-mcp"], "env": {"ROOT": "/w"}},
                "off": {"command": "x", "disabled": true}
            }}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("skills/release")).unwrap();
        std::fs::write(home.path().join("skills/release/SKILL.md"), "# Release\n").unwrap();
        std::fs::write(home.path().join("AGENTS.md"), "# Rules\n").unwrap();
        let items = inventory(home.path());
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["docs", "files", "release", "AGENTS.md"]);
        assert!(items.iter().all(|item| item.tool == KIMI_CODE_ID));
        assert!(!format!("{items:?}").contains("madeup0123"));
        assert!(matches!(can_set_up(&items[0]), SetUp::NeedsYou(_)));
        assert_eq!(can_set_up(&items[1]), SetUp::Copy);
    }

    #[test]
    fn a_server_from_another_tool_is_added_beside_the_rest_and_undone() {
        let home = tempfile::tempdir().unwrap();
        let file = home.path().join(MCP_FILE);
        let before = "{\n  \"mcpServers\": {\n    \"mine\": {\"command\": \"mine\"}\n  }\n}\n";
        std::fs::write(&file, before).unwrap();
        let item = Item {
            tool: "other-tool".into(),
            kind: ItemKind::McpServer,
            name: "events".into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Stdio {
                    command: "npx".into(),
                    args: vec!["events-mcp".into()],
                    env: vec![("TOKEN".into(), Value::Secret)],
                },
                oauth: false,
            }),
            origin: juggler_adapter::inventory::Origin::Own,
        };
        let applied = set_up(home.path(), &item).unwrap();
        let names: Vec<String> = inventory(home.path()).into_iter().map(|i| i.name).collect();
        assert_eq!(names, ["events", "mine"]);
        assert!(std::fs::read_to_string(&file).unwrap().contains("${TOKEN}"));
        assert!(set_up(home.path(), &item).is_err(), "never over one it has");
        let (undo_file, format, entries) = match applied.undo {
            Undo::RemoveSetting {
                file,
                format,
                entries,
            } => Some((file, format, entries)),
            _ => None,
        }
        .unwrap();
        crate::config_edit::remove(&undo_file, format, &entries).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }
}
