//! AJ-E1602 Req.5 (AJ-E1507, AJ-E1542): what an OpenCode account has set
//! up, and setting it up, so its rules and MCP servers join the sync.
//!
//! Read from the account's settings folder, never by starting OpenCode or
//! a server (research `docs/research/opencode.md` §2, §6):
//! - MCP servers from the `mcp` object of its settings files
//!   (`config.json`, `opencode.json`, `opencode.jsonc`, read in that order,
//!   a later one winning, as OpenCode merges them [Inferred from the
//!   v1.19.3 source; on the live checklist]): `type: "local"` (a command
//!   list and its `environment`) or `type: "remote"` (a `url` and its
//!   `headers`). A `{env:NAME}` value is a reference, kept as one.
//! - its main rules file, `AGENTS.md` in the same folder.
//!
//! Setting up an MCP server adds one member to `mcp` in the settings file
//! (`opencode.jsonc` or `opencode.json`, else a new `opencode.json`),
//! every other byte kept, and the undo takes it out only while it is as
//! it was made. A rules file is created, never replaced. A secret is never
//! written: it becomes `{env:NAME}`, and the user is told to provide it.

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo,
    Value, secret_variable, value_of,
};

use crate::inventory_files::secret_names;
use crate::opencode::OPENCODE_ID;

/// OpenCode's settings files, in the order it reads them.
pub const SETTINGS_FILES: &[&str] = &["config.json", "opencode.json", "opencode.jsonc"];

/// The settings file a set-up writes in `config`: the JSON-with-comments
/// one when it's there, else the plain one (made when missing).
#[must_use]
pub fn settings_file(config: &Path) -> PathBuf {
    let jsonc = config.join("opencode.jsonc");
    if jsonc.is_file() {
        jsonc
    } else {
        config.join("opencode.json")
    }
}

/// Everything the OpenCode settings folder `config` has set up.
#[must_use]
pub fn inventory(config: &Path) -> Vec<Item> {
    let mut servers: Vec<(String, serde_json::Value)> = Vec::new();
    for name in SETTINGS_FILES {
        let Some(mcp) = std::fs::read_to_string(config.join(name))
            .ok()
            .and_then(|text| juggler_adapter::jsonc::parse(&text).ok())
            .and_then(|value| {
                value
                    .get("mcp")
                    .and_then(serde_json::Value::as_object)
                    .cloned()
            })
        else {
            continue;
        };
        for (server, entry) in mcp {
            servers.retain(|(known, _)| *known != server);
            servers.push((server, entry));
        }
    }
    // By name: the same list whichever order the files hold them in.
    servers.sort_by(|a, b| a.0.cmp(&b.0));
    let mut items: Vec<Item> = servers
        .iter()
        .filter(|(_, entry)| {
            entry.get("enabled").and_then(serde_json::Value::as_bool) != Some(false)
        })
        .filter_map(|(name, entry)| mcp_item(name, entry))
        .collect();
    let rules = config.join("AGENTS.md");
    if rules.is_file() {
        items.push(item(
            ItemKind::MainRules,
            "AGENTS.md",
            Detail::Files { path: rules },
        ));
    }
    items
}

/// Where a project's own server goes in OpenCode.
const EVERY_PROJECT: &str =
    "for every project of this OpenCode account: AI Juggler sets servers up for the account only";

/// Whether OpenCode can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_opencode = item.tool == OPENCODE_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount) => SetUp::Impossible(
            "it comes with the other tool's own account, so OpenCode can't have it".to_owned(),
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
        (ItemKind::MainRules, Detail::Files { .. }) if from_opencode => SetUp::Copy,
        (ItemKind::MainRules, Detail::Files { .. }) => SetUp::Translate(
            "as this account's AGENTS.md (OpenCode doesn't read @-imports)".to_owned(),
        ),
        (ItemKind::Rules, Detail::Files { .. }) => SetUp::NeedsYou(
            "OpenCode reads one rules file, AGENTS.md: add what this one says to it yourself"
                .to_owned(),
        ),
        _ => SetUp::Impossible(format!(
            "a {} from {} has no OpenCode equivalent in AI Juggler yet",
            item.kind.words(),
            item.tool
        )),
    }
}

/// Sets up `item` in the OpenCode settings folder `config`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the account already has; the write's error.
pub fn set_up(config: &Path, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(config, &item.name, server),
        (ItemKind::MainRules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &config.join("AGENTS.md"), str::to_owned)
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
    if let McpTransport::Other { kind, .. } = &server.transport {
        return SetUp::Impossible(format!(
            "OpenCode's remote servers use streamable HTTP, not {kind}; if it also offers that, \
             add that address"
        ));
    }
    if server.oauth {
        return SetUp::NeedsYou(format!(
            "sign it in afterwards with `opencode mcp auth {name}`: a sign-in never moves"
        ));
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

/// Adds `mcp.<name>` to the settings file, every other byte kept.
fn add_mcp(config: &Path, name: &str, server: &McpServer) -> Result<Applied, AdapterError> {
    if inventory(config)
        .iter()
        .any(|item| item.kind == ItemKind::McpServer && item.name == name)
    {
        return Err(AdapterError::Unsupported(format!(
            "OpenCode already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    let file = settings_file(config);
    let key = crate::config_edit::json_pointer(&["mcp", name]);
    let value = opencode_entry(name, server).to_string();
    crate::config_edit::set(&file, ConfigFormat::Json, &[(key.clone(), value.clone())])
        .map_err(AdapterError::Unsupported)?;
    Ok(Applied {
        summary: format!("Added the MCP server {name} to OpenCode"),
        // The value's fingerprint, never the value itself.
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

/// A server in OpenCode's form; a dropped secret becomes `{env:NAME}`.
#[must_use]
pub fn opencode_entry(name: &str, server: &McpServer) -> serde_json::Value {
    let render = |key: &str, value: &Value| match value {
        Value::Literal(text) => serde_json::Value::String(text.clone()),
        Value::Env(var) => serde_json::Value::String(format!("{{env:{var}}}")),
        Value::Secret => {
            serde_json::Value::String(format!("{{env:{}}}", secret_variable(name, key)))
        }
    };
    let map = |pairs: &[(String, Value)]| -> serde_json::Map<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.clone(), render(key, value)))
            .collect()
    };
    match &server.transport {
        McpTransport::Stdio { command, args, env } => {
            let mut words = vec![command.clone()];
            words.extend(args.iter().cloned());
            let mut entry = serde_json::json!({ "type": "local", "command": words });
            if !env.is_empty() {
                entry["environment"] = serde_json::Value::Object(map(env));
            }
            entry
        }
        McpTransport::Http { url, headers } => {
            let mut entry = serde_json::json!({ "type": "remote", "url": url });
            if !headers.is_empty() {
                entry["headers"] = serde_json::Value::Object(map(headers));
            }
            entry
        }
        McpTransport::Other { kind, url } => serde_json::json!({ "type": kind, "url": url }),
    }
}

/// A value from OpenCode's settings: a whole `{env:NAME}` is a reference.
fn opencode_value(key: &str, text: &str) -> Value {
    let trimmed = text.trim();
    match trimmed
        .strip_prefix("{env:")
        .and_then(|rest| rest.strip_suffix('}'))
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }) {
        Some(name) => Value::Env(name.to_owned()),
        None => value_of(key, text),
    }
}

fn mcp_item(name: &str, entry: &serde_json::Value) -> Option<Item> {
    let pairs = |key: &str| -> Vec<(String, Value)> {
        entry
            .get(key)
            .and_then(serde_json::Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .map(|(k, v)| (k.clone(), opencode_value(k, v.as_str().unwrap_or_default())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let transport = match entry.get("type").and_then(serde_json::Value::as_str)? {
        "local" => {
            let mut words = entry
                .get("command")?
                .as_array()?
                .iter()
                .filter_map(|word| word.as_str().map(str::to_owned));
            McpTransport::Stdio {
                command: words.next()?,
                args: words.collect(),
                env: pairs("environment"),
            }
        }
        "remote" => McpTransport::Http {
            url: entry.get("url")?.as_str()?.to_owned(),
            headers: pairs("headers"),
        },
        other => McpTransport::Other {
            kind: other.to_owned(),
            url: entry
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        },
    };
    Some(Item {
        tool: OPENCODE_ID.to_owned(),
        kind: ItemKind::McpServer,
        name: name.to_owned(),
        scope: Scope::User,
        detail: Detail::Mcp(McpServer {
            transport,
            // Set-up OAuth (`oauth: {…}`); `false` turns it off.
            oauth: entry.get("oauth").is_some_and(serde_json::Value::is_object),
        }),
    })
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: OPENCODE_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opencode_account_lists_its_servers_and_rules_and_keeps_no_secret() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("opencode.jsonc"),
            r#"{
  // mine
  "mcp": {
    "github": {"type": "local", "command": ["npx", "-y", "gh-mcp"],
               "environment": {"GITHUB_TOKEN": "{env:GITHUB_TOKEN}", "MODE": "ro"}},
    "docs": {"type": "remote", "url": "https://docs.example/mcp",
             "headers": {"Authorization": "Bearer madeup0123456789abcdef0123456789abcdef"}},
    "off": {"type": "local", "command": ["x"], "enabled": false},
  },
}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "# Rules\n").unwrap();
        let items = inventory(dir.path());
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["docs", "github", "AGENTS.md"]);
        let github = match &items[1].detail {
            Detail::Mcp(server) => Some(server),
            _ => None,
        }
        .unwrap();
        assert_eq!(
            github.transport,
            McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "gh-mcp".into()],
                env: vec![
                    ("GITHUB_TOKEN".into(), Value::Env("GITHUB_TOKEN".into())),
                    ("MODE".into(), Value::Literal("ro".into())),
                ],
            }
        );
        let shown = format!("{items:?}");
        assert!(
            !shown.contains("madeup0123"),
            "a secret reached the inventory"
        );
        assert_eq!(
            can_set_up(&items[0]),
            SetUp::NeedsYou(
                "it needs DOCS_AUTHORIZATION in your environment: secrets are never copied"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_server_from_another_tool_is_added_beside_the_rest_and_undone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("opencode.json");
        let before = "{\n  \"theme\": \"system\"\n}\n";
        std::fs::write(&file, before).unwrap();
        let item = Item {
            tool: "other-tool".into(),
            kind: ItemKind::McpServer,
            name: "files".into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Stdio {
                    command: "npx".into(),
                    args: vec!["files-mcp".into()],
                    env: vec![("API_KEY".into(), Value::Secret)],
                },
                oauth: false,
            }),
        };
        assert!(matches!(can_set_up(&item), SetUp::NeedsYou(_)));
        let applied = set_up(dir.path(), &item).unwrap();
        let now = juggler_adapter::jsonc::parse(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            now["mcp"]["files"],
            serde_json::json!({"type": "local", "command": ["npx", "files-mcp"],
                               "environment": {"API_KEY": "{env:API_KEY}"}})
        );
        assert_eq!(now["theme"], "system");
        // Never over one it has.
        assert!(set_up(dir.path(), &item).is_err());
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
        let after =
            juggler_adapter::jsonc::parse(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(after, serde_json::json!({"theme": "system", "mcp": {}}));
    }

    #[test]
    fn rules_from_another_tool_become_agents_md_and_sse_cant_come() {
        let rules = Item {
            tool: "other-tool".into(),
            kind: ItemKind::MainRules,
            name: "CLAUDE.md".into(),
            scope: Scope::User,
            detail: Detail::Files {
                path: "/x/CLAUDE.md".into(),
            },
        };
        assert!(matches!(can_set_up(&rules), SetUp::Translate(_)));
        let sse = Item {
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Other {
                    kind: "sse".into(),
                    url: Some("https://x/sse".into()),
                },
                oauth: false,
            }),
            kind: ItemKind::McpServer,
            ..rules
        };
        assert!(matches!(can_set_up(&sse), SetUp::Impossible(_)));
    }
}
