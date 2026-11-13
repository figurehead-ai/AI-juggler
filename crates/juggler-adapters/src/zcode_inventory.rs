//! AJ-E1605 Req.5 (AJ-E1507, AJ-E1542): what `ZCode` has set up, and
//! setting it up, so its rules, skills and MCP servers join the sync.
//!
//! Read from its own files, never by starting `ZCode` or a server. The
//! engine (0.16.9) reads all three from the user's home folder, whatever
//! tree an App runs in (research `docs/research/zcode.md`, the round-2
//! addendum; each path is `os.homedir()` or `HOME` in its own code):
//! - MCP servers from `~/.zcode/cli/config.json`, `mcp.servers` (stdio,
//!   HTTP and SSE; the engine's own spellings, `environment`, `enable`,
//!   `remote` and `http_headers`, read as it reads them);
//! - skills, `~/.zcode/skills/<name>/SKILL.md` (the shared
//!   `~/.agents/skills` is every tool's, not `ZCode`'s);
//! - its main rules file, `~/.zcode/AGENTS.md`.
//!
//! So every `ZCode` App on this computer has the same set, and setting one up
//! sets it up for all of them (each verdict says so). Setting up a server
//! adds one member to `mcp.servers` (every other byte kept; the undo takes
//! it out only while it is as it was made); a skill or the rules file is
//! created, never replaced. A secret is never written: the engine reads
//! no variables in its settings, but hands a server the environment it
//! runs in, so a server's secret is left out of its `env` and the user is
//! told to provide it there; a header's can't be, so such a server isn't
//! set up.

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo,
    Value,
};

use crate::inventory_files::children;
use crate::zcode::ZCODE_ID;

/// The user settings file, inside `ZCode`'s home tree (`~/.zcode`).
pub const CONFIG_FILE: &str = "cli/config.json";
/// The main rules file, inside the home tree.
pub const RULES_FILE: &str = "AGENTS.md";
/// The skills folder, inside the home tree.
pub const SKILLS_DIR: &str = "skills";

/// What every verdict adds: one set, in the home folder, for every App.
const EVERY_APP: &str = "for every ZCode App on this computer: ZCode reads its servers, skills and rules from your home folder";

/// Everything `ZCode`'s home tree `home_tree` (`~/.zcode`) has set up.
#[must_use]
pub fn inventory(home_tree: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let mut servers: Vec<(String, serde_json::Value)> = servers(&home_tree.join(CONFIG_FILE))
        .into_iter()
        .filter(|(_, entry)| enabled(entry))
        .collect();
    servers.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, entry) in servers {
        items.push(crate::claude_inventory::mcp_json_item(
            ZCODE_ID,
            &name,
            &normalized(&entry),
            Scope::User,
        ));
    }
    for (name, path) in children(&home_tree.join(SKILLS_DIR), true) {
        if path.join("SKILL.md").is_file() {
            items.push(item(ItemKind::Skill, &name, Detail::Files { path }));
        }
    }
    let rules = home_tree.join(RULES_FILE);
    if rules.is_file() {
        items.push(item(
            ItemKind::MainRules,
            RULES_FILE,
            Detail::Files { path: rules },
        ));
    }
    items
}

/// The `mcp.servers` members of a settings file, as written; none when
/// the file is missing or isn't an object of objects there.
fn servers(file: &Path) -> Vec<(String, serde_json::Value)> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| juggler_adapter::jsonc::parse(&text).ok())
        .and_then(|value| {
            value
                .pointer("/mcp/servers")
                .and_then(serde_json::Value::as_object)
                .cloned()
        })
        .map(|servers| {
            servers
                .into_iter()
                .filter(|(_, entry)| entry.is_object())
                .collect()
        })
        .unwrap_or_default()
}

/// The engine's own rule: `enable` or `enabled` false turns it off.
fn enabled(entry: &serde_json::Value) -> bool {
    !["enabled", "enable"]
        .iter()
        .any(|key| entry.get(key).and_then(serde_json::Value::as_bool) == Some(false))
}

/// An entry with the engine's other spellings read as it reads them
/// (0.16.9, `normalizeMcpServerConfigInput`): `environment` is `env`,
/// `remote` is `http`, `http_headers` is `headers`, and a missing `type`
/// is `stdio` with a `command`, else `http` with a `url`.
fn normalized(entry: &serde_json::Value) -> serde_json::Value {
    let mut entry = entry.clone();
    let Some(object) = entry.as_object_mut() else {
        return entry;
    };
    if !object.contains_key("env")
        && let Some(environment) = object.remove("environment")
    {
        object.insert("env".to_owned(), environment);
    }
    if !object.contains_key("headers")
        && let Some(headers) = object.remove("http_headers")
    {
        object.insert("headers".to_owned(), headers);
    }
    let kind = object.get("type").and_then(serde_json::Value::as_str);
    let kind = match kind {
        Some("remote") => Some("http"),
        Some(kind) => Some(kind),
        None if object.get("command").is_some() => Some("stdio"),
        None if object.get("url").is_some() => Some("http"),
        None => None,
    }
    .map(str::to_owned);
    if let Some(kind) = kind {
        object.insert("type".to_owned(), serde_json::Value::String(kind));
    }
    entry
}

/// Whether `ZCode` can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_zcode = item.tool == ZCODE_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so ZCode can't have it".to_owned(),
        ),
        (ItemKind::McpServer, Detail::Mcp(server)) => mcp_verdict(server),
        // A skill is a folder with SKILL.md in every tool that has them.
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Translate(EVERY_APP.to_owned()),
        (ItemKind::MainRules, Detail::Files { .. }) if from_zcode => {
            SetUp::Translate(EVERY_APP.to_owned())
        }
        (ItemKind::MainRules, Detail::Files { .. }) => SetUp::Translate(format!(
            "as ZCode's AGENTS.md (ZCode doesn't read @-imports), {EVERY_APP}"
        )),
        (ItemKind::Rules, Detail::Files { .. }) => SetUp::NeedsYou(
            "ZCode reads one rules file, AGENTS.md: add what this one says to it yourself"
                .to_owned(),
        ),
        _ => SetUp::Impossible(format!(
            "a {} from {{tool}} has no ZCode equivalent in AI Juggler yet",
            item.kind.words(),
        )),
    }
}

/// Sets up `item` in `ZCode`'s home tree `home_tree` (`~/.zcode`).
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// it already has; the write's error.
pub fn set_up(home_tree: &Path, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(home_tree, &item.name, server),
        (ItemKind::Skill, Detail::Files { path }, _) => crate::inventory_files::copy_tree(
            path,
            &home_tree.join(SKILLS_DIR).join(&item.name),
            str::to_owned,
        ),
        (ItemKind::MainRules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &home_tree.join(RULES_FILE), str::to_owned)
        }
        _ => Err(AdapterError::Unsupported(format!(
            "{} needs you to set it up",
            item.name
        ))),
    }
}

/// The variables a server needs from the environment it runs in: its
/// `env` values that aren't written out (a secret, or a reference).
fn needed_from_environment(server: &McpServer) -> Vec<String> {
    match &server.transport {
        McpTransport::Stdio { env, .. } => env
            .iter()
            .filter(|(_, value)| !matches!(value, Value::Literal(_)))
            .map(|(key, _)| key.clone())
            .collect(),
        _ => Vec::new(),
    }
}

fn mcp_verdict(server: &McpServer) -> SetUp {
    if let Some(verdict) = server.inline_secret_verdict() {
        return verdict;
    }
    match &server.transport {
        McpTransport::Other { kind, .. } if kind != "sse" => {
            return SetUp::Impossible(format!(
                "ZCode reaches servers over stdio, HTTP and SSE, not {kind}"
            ));
        }
        McpTransport::Http { headers, .. }
            if headers
                .iter()
                .any(|(_, value)| !matches!(value, Value::Literal(_))) =>
        {
            return SetUp::Impossible(
                "ZCode reads no variables in a server's headers, and secrets are never copied: add it in ZCode yourself"
                    .to_owned(),
            );
        }
        _ => {}
    }
    if server.oauth {
        return SetUp::NeedsYou(format!(
            "sign it in afterwards in ZCode (a sign-in never moves), {EVERY_APP}"
        ));
    }
    let needed = needed_from_environment(server);
    if !needed.is_empty() {
        return SetUp::NeedsYou(format!(
            "it needs {} in your environment where ZCode runs (ZCode hands yours to the server; secrets are never copied), {EVERY_APP}",
            needed.join(", ")
        ));
    }
    SetUp::Translate(EVERY_APP.to_owned())
}

/// A server as `ZCode` writes one: only values written out; what the
/// server takes from the environment is left out of `env`.
fn zcode_entry(server: &McpServer) -> serde_json::Value {
    let literal = |pairs: &[(String, Value)]| -> serde_json::Map<String, serde_json::Value> {
        pairs
            .iter()
            .filter_map(|(key, value)| match value {
                Value::Literal(text) => {
                    Some((key.clone(), serde_json::Value::String(text.clone())))
                }
                Value::Env(_) | Value::Secret => None,
            })
            .collect()
    };
    let mut entry = match &server.transport {
        McpTransport::Stdio { command, args, .. } => serde_json::json!({
            "type": "stdio", "command": command, "args": args,
        }),
        McpTransport::Http { url, headers } => serde_json::json!({
            "type": "http", "url": url, "headers": literal(headers),
        }),
        McpTransport::Other { kind, url } => serde_json::json!({ "type": kind, "url": url }),
    };
    if let (McpTransport::Stdio { env, .. }, Some(object)) =
        (&server.transport, entry.as_object_mut())
    {
        let env = literal(env);
        if !env.is_empty() {
            object.insert("env".to_owned(), serde_json::Value::Object(env));
        }
    }
    entry
}

/// Adds `mcp.servers.<name>` to the settings file, every other byte kept.
fn add_mcp(home_tree: &Path, name: &str, server: &McpServer) -> Result<Applied, AdapterError> {
    let file = home_tree.join(CONFIG_FILE);
    if servers(&file).iter().any(|(existing, _)| existing == name) {
        return Err(AdapterError::Unsupported(format!(
            "ZCode already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let key = crate::config_edit::json_pointer(&["mcp", "servers", name]);
    let value = zcode_entry(server).to_string();
    crate::config_edit::set(&file, ConfigFormat::Json, &[(key.clone(), value.clone())])
        .map_err(AdapterError::Unsupported)?;
    Ok(Applied {
        summary: format!("Added the MCP server {name} to ZCode, for every ZCode App"),
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
        tool: ZCODE_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
        origin: juggler_adapter::inventory::Origin::Own,
    }
}

/// `ZCode`'s home tree for `home` (the user's home folder): `~/.zcode`.
#[must_use]
pub fn home_tree(home: &Path) -> PathBuf {
    home.join(crate::zcode::TREE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_servers_skills_and_rules_are_listed_in_its_own_spellings_and_no_secret_is_kept() {
        let home = tempfile::tempdir().unwrap();
        let tree = home.path();
        std::fs::create_dir_all(tree.join("cli")).unwrap();
        std::fs::write(
            tree.join(CONFIG_FILE),
            r#"{"mcp": {"servers": {
                "docs": {"type": "remote", "url": "https://docs.example/mcp", "http_headers": {"Authorization": "Bearer madeup0123456789abcdef0123456789abcdef"}},
                "files": {"command": "npx", "args": ["files-mcp"], "environment": {"ROOT": "/w"}},
                "off": {"command": "x", "enable": false},
                "events": {"type": "sse", "url": "https://events.example/sse"}
            }}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(tree.join("skills/release")).unwrap();
        std::fs::write(tree.join("skills/release/SKILL.md"), "# Release\n").unwrap();
        std::fs::create_dir_all(tree.join("skills/empty")).unwrap();
        std::fs::write(tree.join(RULES_FILE), "# Rules\n").unwrap();
        let items = inventory(tree);
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["docs", "events", "files", "release", "AGENTS.md"]);
        assert!(items.iter().all(|item| item.tool == ZCODE_ID));
        assert!(!format!("{items:?}").contains("madeup0123"));
        let files = match &items[2].detail {
            Detail::Mcp(server) => Some(server),
            _ => None,
        }
        .unwrap();
        assert!(matches!(
            &files.transport,
            McpTransport::Stdio { env, .. } if env == &[("ROOT".to_owned(), Value::Literal("/w".to_owned()))]
        ));
        assert!(matches!(can_set_up(&items[0]), SetUp::Impossible(_)));
        assert!(matches!(can_set_up(&items[1]), SetUp::Translate(_)));
        assert!(
            matches!(can_set_up(&items[2]), SetUp::Translate(note) if note.contains("every ZCode App"))
        );
    }

    #[test]
    fn a_server_from_another_tool_is_added_beside_the_rest_without_its_secret_and_undone() {
        let home = tempfile::tempdir().unwrap();
        let tree = home.path();
        let file = tree.join(CONFIG_FILE);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let before = "{\n  \"theme\": \"dark\",\n  \"mcp\": {\n    \"servers\": {\n      \"mine\": {\"command\": \"mine\"}\n    }\n  }\n}\n";
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
                    env: vec![
                        ("TOKEN".into(), Value::Secret),
                        ("REGION".into(), Value::Literal("eu".into())),
                    ],
                },
                oauth: false,
            }),
            origin: juggler_adapter::inventory::Origin::Own,
        };
        assert!(matches!(can_set_up(&item), SetUp::NeedsYou(note) if note.contains("TOKEN")));
        let applied = set_up(tree, &item).unwrap();
        let names: Vec<String> = inventory(tree).into_iter().map(|i| i.name).collect();
        assert_eq!(names, ["events", "mine"]);
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(
            !written.contains("TOKEN"),
            "no secret, no placeholder: {written}"
        );
        assert!(written.contains("\"REGION\""), "{written}");
        assert!(set_up(tree, &item).is_err(), "never over one it has");
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

    #[test]
    fn a_server_is_added_to_a_settings_file_that_has_none_yet() {
        let home = tempfile::tempdir().unwrap();
        let item = Item {
            tool: "other-tool".into(),
            kind: ItemKind::McpServer,
            name: "docs".into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Http {
                    url: "https://docs.example/mcp".into(),
                    headers: Vec::new(),
                },
                oauth: false,
            }),
            origin: juggler_adapter::inventory::Origin::Own,
        };
        set_up(home.path(), &item).unwrap();
        let items = inventory(home.path());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].detail, item.detail);
    }
}
