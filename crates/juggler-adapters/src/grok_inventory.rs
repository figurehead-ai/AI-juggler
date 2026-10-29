//! AJ-E1604 (AJ-E1507, AJ-E1542): what a Grok Build home has set up, and
//! setting it up, so its rules and MCP servers join the sync.
//!
//! Read from the home's own files, never by starting Grok or a server
//! (research `docs/research/grok.md` §6):
//! - MCP servers from `config.toml`'s `[mcp_servers.<name>]` (a `command`
//!   with its `args` and `env`, or a `url`), read as Codex's form;
//! - user rules, `rules/*.md`.
//!
//! Setting up a local server adds one `mcp_servers.<name>` to
//! `config.toml` (every other byte kept; the undo takes it out only while
//! it is as it was made), a secret as `${NAME}`. A remote server with
//! headers isn't set up (how Grok takes headers isn't documented); a
//! rules file is created, never replaced.

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo,
    Value, secret_variable,
};

use crate::grok_build::GROK_BUILD_ID;
use crate::inventory_files::{children, secret_names};

/// Everything the Grok Build home `home` has set up.
#[must_use]
pub fn inventory(home: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let config: toml::Table = std::fs::read_to_string(home.join("config.toml"))
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    if let Some(servers) = config.get("mcp_servers").and_then(toml::Value::as_table) {
        for (name, entry) in servers {
            if let Some(entry) = entry.as_table()
                && entry.get("enabled").and_then(toml::Value::as_bool) != Some(false)
            {
                items.push(crate::codex_inventory::toml_mcp_item(
                    GROK_BUILD_ID,
                    name,
                    entry,
                ));
            }
        }
    }
    for (name, path) in children(&home.join("rules"), false) {
        if let Some(stem) = name.strip_suffix(".md") {
            items.push(Item {
                tool: GROK_BUILD_ID.to_owned(),
                kind: ItemKind::Rules,
                name: stem.to_owned(),
                scope: Scope::User,
                detail: Detail::Files { path },
                origin: juggler_adapter::inventory::Origin::Own,
            });
        }
    }
    items
}

/// Where a project's own server goes in Grok Build.
const EVERY_PROJECT: &str =
    "for every project of this Grok account: AI Juggler sets servers up for the account only";

/// Whether Grok Build can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_grok = item.tool == GROK_BUILD_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so Grok can't have it".to_owned(),
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
        (ItemKind::Rules, Detail::Files { .. }) if from_grok => SetUp::Copy,
        (ItemKind::Rules | ItemKind::MainRules, Detail::Files { .. }) => SetUp::Translate(
            "as a rules file in this account's rules folder (Grok doesn't read @-imports)"
                .to_owned(),
        ),
        _ => SetUp::Impossible(format!(
            "a {} from {{tool}} has no Grok equivalent in AI Juggler yet",
            item.kind.words(),
        )),
    }
}

/// Sets up `item` in the Grok Build home `home`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the home already has; the write's error.
pub fn set_up(home: &Path, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(home, &item.name, server),
        (ItemKind::Rules | ItemKind::MainRules, Detail::Files { path }, _) => {
            let name = item.name.strip_suffix(".md").unwrap_or(&item.name);
            crate::inventory_files::copy_file(
                path,
                &home.join("rules").join(format!("{name}.md")),
                str::to_owned,
            )
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
    match &server.transport {
        McpTransport::Other { kind, .. } => {
            return SetUp::Impossible(format!(
                "Grok reaches servers by a command or a streamable HTTP address, not {kind}"
            ));
        }
        McpTransport::Http { headers, .. } if !headers.is_empty() => {
            return SetUp::Impossible(
                "how Grok takes a server's headers isn't documented: add it in Grok yourself"
                    .to_owned(),
            );
        }
        _ => {}
    }
    if server.oauth {
        return SetUp::NeedsYou("sign it in afterwards in Grok: a sign-in never moves".to_owned());
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

/// A server as an inline TOML table: a command with its arguments and
/// environment (a secret as `${NAME}`), or an address.
fn grok_entry(name: &str, server: &McpServer) -> String {
    let mut table = toml_edit::InlineTable::new();
    match &server.transport {
        McpTransport::Stdio { command, args, env } => {
            table.insert("command", command.as_str().into());
            if !args.is_empty() {
                let mut list = toml_edit::Array::new();
                for arg in args {
                    list.push(arg.as_str());
                }
                table.insert("args", toml_edit::Value::Array(list));
            }
            if !env.is_empty() {
                let mut vars = toml_edit::InlineTable::new();
                for (key, value) in env {
                    let text = match value {
                        Value::Literal(text) => text.clone(),
                        Value::Env(var) => format!("${{{var}}}"),
                        Value::Secret => format!("${{{}}}", secret_variable(name, key)),
                    };
                    vars.insert(key.as_str(), text.as_str().into());
                }
                table.insert("env", toml_edit::Value::InlineTable(vars));
            }
        }
        McpTransport::Http { url, .. } | McpTransport::Other { url: Some(url), .. } => {
            table.insert("url", url.as_str().into());
        }
        McpTransport::Other { .. } => {}
    }
    toml_edit::Value::InlineTable(table).to_string()
}

/// Adds `mcp_servers.<name>` to `config.toml`, every other byte kept.
fn add_mcp(home: &Path, name: &str, server: &McpServer) -> Result<Applied, AdapterError> {
    if name.contains('.') {
        return Err(AdapterError::Unsupported(format!(
            "{name} has a dot in its name, which Grok's settings would read as two names"
        )));
    }
    if inventory(home)
        .iter()
        .any(|item| item.kind == ItemKind::McpServer && item.name == name)
    {
        return Err(AdapterError::Unsupported(format!(
            "Grok already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    let file = home.join("config.toml");
    let key = format!("mcp_servers.{name}");
    let value = grok_entry(name, server);
    crate::config_edit::set(&file, ConfigFormat::Toml, &[(key.clone(), value.clone())])
        .map_err(AdapterError::Unsupported)?;
    Ok(Applied {
        summary: format!("Added the MCP server {name} to Grok"),
        undo: Undo::RemoveSetting {
            file,
            format: ConfigFormat::Toml,
            entries: vec![(
                key,
                crate::config_edit::fingerprint(ConfigFormat::Toml, &value),
            )],
        },
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grok_home_lists_its_servers_and_rules_and_keeps_no_secret() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("config.toml"),
            "[mcp_servers.files]\ncommand = \"npx\"\nargs = [\"files-mcp\"]\nenv = { TOKEN = \"madeup0123456789abcdef0123456789abcdef\" }\n\n[mcp_servers.docs]\nurl = \"https://docs.example/mcp\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("rules")).unwrap();
        std::fs::write(home.path().join("rules/style.md"), "# Style\n").unwrap();
        let items = inventory(home.path());
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["docs", "files", "style"]);
        assert!(items.iter().all(|item| item.tool == GROK_BUILD_ID));
        assert!(!format!("{items:?}").contains("madeup0123"));
        assert_eq!(can_set_up(&items[0]), SetUp::Copy);
        assert!(matches!(can_set_up(&items[1]), SetUp::NeedsYou(_)));
    }

    #[test]
    fn a_server_from_another_tool_is_added_beside_the_rest_and_undone() {
        let home = tempfile::tempdir().unwrap();
        let file = home.path().join("config.toml");
        let before = "# mine\nmodel = \"grok-build-0.1\"\n";
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
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.starts_with(before) && text.contains("${TOKEN}"),
            "{text}"
        );
        let names: Vec<String> = inventory(home.path()).into_iter().map(|i| i.name).collect();
        assert_eq!(names, ["events"]);
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
        assert!(inventory(home.path()).is_empty());
    }
}
