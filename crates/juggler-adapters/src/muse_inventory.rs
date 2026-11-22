//! AJ-E1608 (AJ-E1507, AJ-E1542): what a Muse Code account has set up.
//!
//! Read from the account's own config folder (`$XDG_CONFIG_HOME/muse`),
//! never by starting Muse or a server (research `docs/research/muse-code.md`
//! §6):
//! - MCP servers from `settings.json`'s `mcp_servers` (each a `transport`,
//!   `stdio` with its `command`, `args` and `env`, or `streamable_http`
//!   with its `url` and `headers`), read as the `mcpServers` form Claude
//!   Code uses; a secret is never kept;
//! - skills, `skills/<name>/SKILL.md`.
//!
//! The shared `~/.agents/skills` is every tool's, not this account's, and
//! the user's Claude Code and Codex skills are kept out of an account's
//! Muse altogether ([`crate::muse`]).
//!
//! Setting up a skill copies its folder in, never over one there. An MCP
//! server is the user's to add: how Muse writes one into `settings.json`
//! isn't checked yet (AJ-E1612), so AI Juggler says what to add.

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{Applied, Detail, Item, ItemKind, Scope, SetUp};

use crate::inventory_files::{children, json_members};
use crate::muse::MUSE_ID;

/// Muse's settings, in its config folder.
const SETTINGS: &str = "settings.json";

/// Everything the Muse config folder `config` has set up.
#[must_use]
pub fn inventory(config: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    for (name, mut entry) in json_members(&config.join(SETTINGS), "mcp_servers") {
        if entry.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
            continue;
        }
        // Muse names a server's kind `transport`; the shared reader reads
        // `type`.
        if let Some(object) = entry.as_object_mut()
            && !object.contains_key("type")
            && let Some(kind) = object.get("transport").cloned()
        {
            object.insert("type".to_owned(), kind);
        }
        items.push(crate::claude_inventory::mcp_json_item(
            MUSE_ID,
            &name,
            &entry,
            Scope::User,
        ));
    }
    for (name, path) in children(&config.join("skills"), true) {
        if path.join("SKILL.md").is_file() {
            items.push(Item::own(
                MUSE_ID,
                ItemKind::Skill,
                &name,
                Detail::Files { path },
            ));
        }
    }
    items
}

/// Whether Muse Code can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so Muse Code can't have it".to_owned(),
        ),
        // A skill is a folder with SKILL.md in every tool that has them.
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Copy,
        (ItemKind::McpServer, Detail::Mcp(_)) => SetUp::NeedsYou(
            "add it to this account's Muse settings (mcp_servers) yourself: AI Juggler doesn't write Muse's servers yet"
                .to_owned(),
        ),
        (ItemKind::Rules | ItemKind::MainRules, _) => SetUp::NeedsYou(
            "Muse Code reads a project's AGENTS.md: add what this one says there yourself".to_owned(),
        ),
        _ => SetUp::Impossible(format!(
            "a {} from {{tool}} has no Muse Code equivalent in AI Juggler yet",
            item.kind.words(),
        )),
    }
}

/// Sets up `item` in the Muse config folder `config`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up here, or a skill
/// the folder already has; the copy's error.
pub fn set_up(config: &Path, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (ItemKind::Skill, Detail::Files { path }, SetUp::Copy) => {
            crate::inventory_files::copy_tree(
                path,
                &config.join("skills").join(&item.name),
                str::to_owned,
            )
        }
        (_, _, SetUp::Impossible(why) | SetUp::NeedsYou(why)) => {
            Err(AdapterError::Unsupported(why))
        }
        _ => Err(AdapterError::Unsupported(format!(
            "{} needs you to set it up",
            item.name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_muse_config_lists_its_servers_and_skills_and_keeps_no_secret() {
        let config = tempfile::tempdir().unwrap();
        std::fs::write(
            config.path().join(SETTINGS),
            r#"{"schema_version": 1, "mcp_servers": {
                "docs": {"transport": "streamable_http", "url": "https://docs.example/mcp", "headers": {"Authorization": "Bearer madeup0123456789abcdef0123456789abcdef"}},
                "files": {"transport": "stdio", "command": "npx", "args": ["files-mcp"], "env": {"ROOT": "/w"}},
                "off": {"transport": "stdio", "command": "x", "enabled": false}
            }}"#,
        )
        .unwrap();
        std::fs::create_dir_all(config.path().join("skills/release")).unwrap();
        std::fs::write(config.path().join("skills/release/SKILL.md"), "# Release\n").unwrap();
        let items = inventory(config.path());
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["docs", "files", "release"]);
        assert!(items.iter().all(|item| item.tool == MUSE_ID));
        assert!(!format!("{items:?}").contains("madeup0123"));
        assert!(matches!(
            &items[0].detail,
            Detail::Mcp(server) if matches!(server.transport, juggler_adapter::inventory::McpTransport::Http { .. })
        ));
        assert!(matches!(can_set_up(&items[1]), SetUp::NeedsYou(_)));
        assert_eq!(can_set_up(&items[2]), SetUp::Copy);
    }

    #[test]
    fn a_skill_is_copied_in_and_never_over_one_there() {
        let from = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(from.path().join("ship")).unwrap();
        std::fs::write(from.path().join("ship/SKILL.md"), "# Ship\n").unwrap();
        let skill = Item::own(
            "kimi-code",
            ItemKind::Skill,
            "ship",
            Detail::Files {
                path: from.path().join("ship"),
            },
        );
        let config = tempfile::tempdir().unwrap();
        set_up(config.path(), &skill).unwrap();
        assert!(config.path().join("skills/ship/SKILL.md").is_file());
        assert!(
            set_up(config.path(), &skill).is_err(),
            "never over one there"
        );
    }
}
