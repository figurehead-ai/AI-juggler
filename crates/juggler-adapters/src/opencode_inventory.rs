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
//! OpenCode 2 keeps them under `mcp.servers` instead, switched off by
//! `disabled: true` rather than `enabled: false` (its `mcp-servers` and
//! `migrate-v1` docs, read 2026-10-27): both forms are read, file by file.
//!
//! Setting up an MCP server adds one member to `mcp` (1.x) or to
//! `mcp.servers` (2.x) in the settings file (`opencode.jsonc` or
//! `opencode.json`, else a new `opencode.json`): the form the file already
//! uses, else the installed OpenCode's, so a file never holds both. Every
//! other byte is kept, and the undo takes it out only while it is as it
//! was made. [Inferred from the docs, on the live checklist: that 2.x
//! reads an entry of the same fields under `mcp.servers`.] A rules file is created, never replaced. A secret is never
//! written: it becomes `{env:NAME}`, and the user is told to provide it.

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo,
    Value, secret_variable, value_of,
};

use crate::inventory_files::secret_names;
use crate::opencode::{Line, OPENCODE_ID};

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

/// The form a settings file keeps its MCP servers in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// OpenCode 1: `mcp.<name>`, switched off by `enabled: false`.
    V1,
    /// OpenCode 2: `mcp.servers.<name>`, switched off by `disabled: true`
    /// (research `docs/research/capabilities/opencode.md` §A3, the 2.x
    /// docs `mcp-servers` and `migrate-v1`, read 2026-10-27).
    V2,
}

/// Whether `entry` is a server of the 1.x form: an object with a `type`.
fn is_v1_server(entry: &serde_json::Value) -> bool {
    entry.get("type").is_some_and(serde_json::Value::is_string)
}

/// The 2.x `mcp.servers` object of a settings file's `mcp`, when it is
/// one: a 1.x server may be called "servers" too, and has a `type`.
fn v2_servers(
    mcp: &serde_json::Map<String, serde_json::Value>,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    mcp.get("servers")
        .filter(|servers| !is_v1_server(servers))
        .and_then(serde_json::Value::as_object)
}

/// Every server a settings file names, each with the form it is in, and
/// whether it is switched on. A file holding both forms (2.x reads 1.x
/// fields too) lists both [whether 2.x takes a mixed `mcp` is unknown].
fn file_servers(value: &serde_json::Value) -> Vec<(String, serde_json::Value, bool)> {
    let Some(mcp) = value.get("mcp").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let flag =
        |entry: &serde_json::Value, key: &str| entry.get(key).and_then(serde_json::Value::as_bool);
    let v2 = v2_servers(mcp);
    let mut out: Vec<(String, serde_json::Value, bool)> = mcp
        .iter()
        .filter(|(name, entry)| !(v2.is_some() && *name == "servers") && is_v1_server(entry))
        .map(|(name, entry)| {
            (
                name.clone(),
                entry.clone(),
                flag(entry, "enabled") != Some(false),
            )
        })
        .collect();
    out.extend(v2.into_iter().flatten().map(|(name, entry)| {
        (
            name.clone(),
            entry.clone(),
            flag(entry, "disabled") != Some(true),
        )
    }));
    out
}

/// The form `file` keeps its servers in: 2.x when it has `mcp.servers`,
/// 1.x when it has a server of that form, `None` when it has none (or
/// isn't there).
#[must_use]
pub fn file_shape(file: &Path) -> Option<Shape> {
    let value = std::fs::read_to_string(file)
        .ok()
        .and_then(|text| juggler_adapter::jsonc::parse(&text).ok())?;
    let mcp = value.get("mcp").and_then(serde_json::Value::as_object)?;
    if v2_servers(mcp).is_some() {
        Some(Shape::V2)
    } else if mcp.values().any(is_v1_server) {
        Some(Shape::V1)
    } else {
        None
    }
}

/// Everything the OpenCode settings folder `config` has set up: MCP
/// servers in either form, a later file winning by name.
#[must_use]
pub fn inventory(config: &Path) -> Vec<Item> {
    let mut servers: Vec<(String, serde_json::Value, bool)> = Vec::new();
    for name in SETTINGS_FILES {
        let Some(value) = std::fs::read_to_string(config.join(name))
            .ok()
            .and_then(|text| juggler_adapter::jsonc::parse(&text).ok())
        else {
            continue;
        };
        for (server, entry, on) in file_servers(&value) {
            servers.retain(|(known, _, _)| *known != server);
            servers.push((server, entry, on));
        }
    }
    // By name: the same list whichever order the files hold them in.
    servers.sort_by(|a, b| a.0.cmp(&b.0));
    let mut items: Vec<Item> = servers
        .iter()
        .filter(|(_, _, on)| *on)
        .filter_map(|(name, entry, _)| mcp_item(name, entry))
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
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so OpenCode can't have it".to_owned(),
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
            "a {} from {{tool}} has no OpenCode equivalent in AI Juggler yet",
            item.kind.words(),
        )),
    }
}

/// Sets up `item` in the OpenCode settings folder `config`, for the
/// OpenCode `line` installed (`None` when it can't be told).
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the account already has; the write's error.
pub fn set_up(config: &Path, item: &Item, line: Option<Line>) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(config, &item.name, server, line),
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

/// The form a server is added in to `file`: the one the file already
/// uses, so a file never holds both; for a file with none, the installed
/// OpenCode's (1.x's when it can't be told). A file in 2.x's form with
/// OpenCode 1 installed takes nothing: 1.x wouldn't read it.
fn shape_for(file: &Path, line: Option<Line>) -> Result<Shape, AdapterError> {
    match (file_shape(file), line) {
        (Some(Shape::V2), Some(Line::V1)) => Err(AdapterError::Unsupported(
            "this OpenCode settings file is in OpenCode 2's form, but OpenCode 1 is installed, \
             so a server added to it wouldn't be read"
                .to_owned(),
        )),
        (Some(shape), _) => Ok(shape),
        (None, Some(Line::V2)) => Ok(Shape::V2),
        (None, _) => Ok(Shape::V1),
    }
}

/// Adds `mcp.<name>` (1.x) or `mcp.servers.<name>` (2.x) to the settings
/// file, every other byte kept.
fn add_mcp(
    config: &Path,
    name: &str,
    server: &McpServer,
    line: Option<Line>,
) -> Result<Applied, AdapterError> {
    if inventory(config)
        .iter()
        .any(|item| item.kind == ItemKind::McpServer && item.name == name)
    {
        return Err(AdapterError::Unsupported(format!(
            "OpenCode already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    let file = settings_file(config);
    // The entry itself is the same in both forms (`type`, `command`,
    // `environment`, `url`, `headers`); only where it goes differs.
    let key = match shape_for(&file, line)? {
        Shape::V1 => crate::config_edit::json_pointer(&["mcp", name]),
        Shape::V2 => crate::config_edit::json_pointer(&["mcp", "servers", name]),
    };
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
        origin: juggler_adapter::inventory::Origin::Own,
    })
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: OPENCODE_ID.to_owned(),
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
            origin: juggler_adapter::inventory::Origin::Own,
        };
        assert!(matches!(can_set_up(&item), SetUp::NeedsYou(_)));
        let applied = set_up(dir.path(), &item, Some(Line::V1)).unwrap();
        let now = juggler_adapter::jsonc::parse(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            now["mcp"]["files"],
            serde_json::json!({"type": "local", "command": ["npx", "files-mcp"],
                               "environment": {"API_KEY": "{env:API_KEY}"}})
        );
        assert_eq!(now["theme"], "system");
        // Never over one it has.
        assert!(set_up(dir.path(), &item, Some(Line::V1)).is_err());
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
            origin: juggler_adapter::inventory::Origin::Own,
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

    fn names(config: &Path) -> Vec<String> {
        inventory(config)
            .into_iter()
            .filter(|item| item.kind == ItemKind::McpServer)
            .map(|item| item.name)
            .collect()
    }

    fn files_server(name: &str) -> Item {
        Item {
            tool: "other-tool".into(),
            kind: ItemKind::McpServer,
            name: name.into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Stdio {
                    command: "npx".into(),
                    args: vec!["files-mcp".into()],
                    env: Vec::new(),
                },
                oauth: false,
            }),
            origin: juggler_adapter::inventory::Origin::Own,
        }
    }

    fn read(file: &Path) -> serde_json::Value {
        juggler_adapter::jsonc::parse(&std::fs::read_to_string(file).unwrap()).unwrap()
    }

    #[test]
    fn an_opencode_2_config_lists_its_servers_and_leaves_out_a_disabled_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("opencode.jsonc"),
            r#"{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "timeout": {"startup": 45000},
    "servers": {
      "github": {"type": "local", "command": ["npx", "-y", "gh-mcp"],
                 "environment": {"GITHUB_TOKEN": "{env:GITHUB_TOKEN}"}},
      "context7": {"type": "remote", "url": "https://mcp.example.com/mcp", "oauth": false,
                   "headers": {"CONTEXT7_API_KEY": "{env:CONTEXT7_API_KEY}"}},
      "off": {"type": "local", "command": ["x"], "disabled": true}
    }
  }
}"#,
        )
        .unwrap();
        assert_eq!(names(dir.path()), ["context7", "github"]);
        assert_eq!(
            file_shape(&dir.path().join("opencode.jsonc")),
            Some(Shape::V2)
        );
        let github = inventory(dir.path())
            .into_iter()
            .find(|item| item.name == "github")
            .unwrap();
        assert_eq!(
            github.detail,
            Detail::Mcp(McpServer {
                transport: McpTransport::Stdio {
                    command: "npx".into(),
                    args: vec!["-y".into(), "gh-mcp".into()],
                    env: vec![("GITHUB_TOKEN".into(), Value::Env("GITHUB_TOKEN".into()))],
                },
                oauth: false,
            })
        );
    }

    #[test]
    fn a_1x_server_named_servers_stays_a_1x_server_and_both_forms_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("opencode.json");
        std::fs::write(
            &file,
            r#"{"mcp": {"servers": {"type": "local", "command": ["srv"]}, "docs": {"type": "remote", "url": "https://d/mcp"}}}"#,
        )
        .unwrap();
        assert_eq!(names(dir.path()), ["docs", "servers"]);
        assert_eq!(file_shape(&file), Some(Shape::V1));
        // A file someone wrote in both forms lists both.
        std::fs::write(
            &file,
            r#"{"mcp": {"old": {"type": "local", "command": ["o"], "enabled": true}, "servers": {"new": {"type": "local", "command": ["n"]}}}}"#,
        )
        .unwrap();
        assert_eq!(names(dir.path()), ["new", "old"]);
        assert_eq!(file_shape(&file), Some(Shape::V2));
    }

    #[test]
    fn a_new_server_takes_the_files_form_else_the_installed_opencodes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("opencode.json");
        // No servers yet, OpenCode 2 installed: the 2.x form.
        std::fs::write(&file, "{\n  \"theme\": \"system\"\n}\n").unwrap();
        let applied = set_up(dir.path(), &files_server("files"), Some(Line::V2)).unwrap();
        let now = read(&file);
        assert_eq!(
            now["mcp"]["servers"]["files"],
            serde_json::json!({"type": "local", "command": ["npx", "files-mcp"]})
        );
        assert!(
            now["mcp"].get("files").is_none(),
            "a 1.x entry was written too"
        );
        assert_eq!(names(dir.path()), ["files"]);
        // Its undo takes out what it added, nothing else.
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
        assert!(names(dir.path()).is_empty());
        assert_eq!(read(&file)["theme"], "system");

        // A file already in the 1.x form stays in it, whatever is installed.
        std::fs::write(
            &file,
            r#"{"mcp": {"docs": {"type": "remote", "url": "https://d/mcp"}}}"#,
        )
        .unwrap();
        set_up(dir.path(), &files_server("files"), Some(Line::V2)).unwrap();
        let now = read(&file);
        assert!(now["mcp"]["files"].is_object());
        assert!(
            now["mcp"].get("servers").is_none(),
            "a 2.x entry was mixed in"
        );

        // A file in the 2.x form stays in it when the version can't be told.
        std::fs::write(&file, r#"{"mcp": {"servers": {}}}"#).unwrap();
        set_up(dir.path(), &files_server("files"), None).unwrap();
        assert!(read(&file)["mcp"]["servers"]["files"].is_object());

        // No servers and no version known: the 1.x form, as before.
        std::fs::write(&file, "{}").unwrap();
        set_up(dir.path(), &files_server("files"), None).unwrap();
        assert!(read(&file)["mcp"]["files"].is_object());
    }

    #[test]
    fn a_2x_file_takes_nothing_while_opencode_1_is_installed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("opencode.json");
        let before =
            r#"{"mcp": {"servers": {"docs": {"type": "remote", "url": "https://d/mcp"}}}}"#;
        std::fs::write(&file, before).unwrap();
        assert!(matches!(
            set_up(dir.path(), &files_server("files"), Some(Line::V1)),
            Err(AdapterError::Unsupported(_))
        ));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }
}
