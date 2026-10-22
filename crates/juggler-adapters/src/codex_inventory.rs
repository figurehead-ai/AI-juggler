//! AJ-E1542: what a Codex home has set up, and setting it up.
//!
//! Read from the home's own files, never by starting Codex or a server:
//! - MCP servers from `config.toml` `[mcp_servers.<name>]`;
//! - skills (`~/.agents/skills/<name>/SKILL.md`, shared by every Codex
//!   home, and the home's own `skills/`, `.system` left out), prompts
//!   (`prompts/*.md`), subagents (`agents/*.toml`), rules (`AGENTS.md`);
//! - plugins (`[plugins."name@marketplace"]`).
//!
//! Setting up an MCP server appends one new `[mcp_servers.<name>]` table to
//! `config.toml` (a backup is kept first, and the exact text appended is
//! the undo record); nothing already there is changed. A skill goes into
//! the shared `~/.agents/skills/`, a Claude command becomes a skill, and
//! Claude rules become the home's `AGENTS.md`, as new files only.

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, Detail, Item, ItemKind, McpServer, McpTransport, Scope, SetUp, Undo, Value,
    secret_variable, value_of,
};

use crate::claude_inventory::{children, kind_word, secret_names};
use crate::codex::CODEX_ID;

/// Everything the Codex home `home` has set up. `user_home` is the user's
/// home folder (for the shared `~/.agents/skills`).
#[must_use]
pub fn inventory(home: &Path, user_home: Option<&Path>) -> Vec<Item> {
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
                items.push(mcp_item(name, entry));
            }
        }
    }
    if let Some(plugins) = config.get("plugins").and_then(toml::Value::as_table) {
        for (id, entry) in plugins {
            if entry.get("enabled").and_then(toml::Value::as_bool) != Some(false) {
                items.push(item(
                    ItemKind::Plugin,
                    id,
                    Detail::Plugin { id: id.clone() },
                ));
            }
        }
    }
    let skill_dirs = user_home
        .map(|user| user.join(".agents").join("skills"))
        .into_iter()
        .chain(std::iter::once(home.join("skills")));
    for skills in skill_dirs {
        for (name, path) in children(&skills, true) {
            if path.join("SKILL.md").is_file()
                && !items
                    .iter()
                    .any(|i| i.kind == ItemKind::Skill && i.name == name)
            {
                items.push(item(ItemKind::Skill, &name, Detail::Files { path }));
            }
        }
    }
    for (name, path) in children(&home.join("prompts"), false) {
        if let Some(stem) = name.strip_suffix(".md") {
            items.push(item(ItemKind::Command, stem, Detail::Files { path }));
        }
    }
    for (name, path) in children(&home.join("agents"), false) {
        if let Some(stem) = name.strip_suffix(".toml") {
            items.push(item(ItemKind::Subagent, stem, Detail::Files { path }));
        }
    }
    for rules in ["AGENTS.override.md", "AGENTS.md"] {
        let path = home.join(rules);
        if path.is_file() {
            items.push(item(ItemKind::Rules, "AGENTS.md", Detail::Files { path }));
            break;
        }
    }
    items
}

/// Whether Codex can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_codex = item.tool == CODEX_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount) => SetUp::Impossible(
            "it comes with the claude.ai account, so Codex can't have it".to_owned(),
        ),
        (ItemKind::McpServer, Detail::Mcp(server)) => mcp_verdict(&item.name, server),
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Copy,
        (ItemKind::Command | ItemKind::Subagent | ItemKind::Rules, Detail::Files { .. })
            if from_codex =>
        {
            SetUp::Copy
        }
        (ItemKind::Command, Detail::Files { .. }) => SetUp::Translate(
            "as a Codex skill you call with $name; its arguments need a look".to_owned(),
        ),
        (ItemKind::Rules, Detail::Files { .. }) => {
            SetUp::Translate("as this home's AGENTS.md (Codex doesn't read @-imports)".to_owned())
        }
        (ItemKind::Plugin, Detail::Plugin { id }) if from_codex => {
            SetUp::NeedsYou(format!("install it here with `codex plugin add {id}`"))
        }
        (ItemKind::Plugin, _) => {
            SetUp::Impossible("Claude Code plugins don't run in Codex".to_owned())
        }
        _ => SetUp::Impossible(format!(
            "a {} from {} has no Codex equivalent yet",
            kind_word(item.kind),
            item.tool
        )),
    }
}

/// Sets up `item` in the Codex home `home`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the home already has; the write's error.
pub fn set_up(home: &Path, user_home: Option<&Path>, item: &Item) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(home, &item.name, server),
        (ItemKind::Skill, Detail::Files { path }, _) => {
            let skills = user_home
                .map(|user| user.join(".agents").join("skills"))
                .ok_or_else(|| AdapterError::Unsupported("no home folder".to_owned()))?;
            crate::inventory_files::copy_tree(path, &skills.join(&item.name), str::to_owned)
        }
        (ItemKind::Command, Detail::Files { path }, _) if item.tool != CODEX_ID => {
            // A Claude command becomes a skill: SKILL.md with its name.
            let skills = user_home
                .map(|user| user.join(".agents").join("skills"))
                .ok_or_else(|| AdapterError::Unsupported("no home folder".to_owned()))?;
            let name = item.name.clone();
            crate::inventory_files::copy_file(
                path,
                &skills.join(&item.name).join("SKILL.md"),
                move |text| as_skill(&name, text),
            )
        }
        (ItemKind::Command, Detail::Files { path }, _) => crate::inventory_files::copy_file(
            path,
            &home.join("prompts").join(format!("{}.md", item.name)),
            str::to_owned,
        ),
        (ItemKind::Subagent, Detail::Files { path }, _) => crate::inventory_files::copy_file(
            path,
            &home.join("agents").join(format!("{}.toml", item.name)),
            str::to_owned,
        ),
        (ItemKind::Rules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &home.join("AGENTS.md"), str::to_owned)
        }
        _ => Err(AdapterError::Unsupported(format!(
            "{} needs you to set it up",
            item.name
        ))),
    }
}

fn mcp_verdict(name: &str, server: &McpServer) -> SetUp {
    if let McpTransport::Other { kind, .. } = &server.transport {
        return SetUp::Impossible(format!(
            "Codex can't reach a {kind} server; if it also offers streamable HTTP, add that address"
        ));
    }
    if server.oauth {
        return SetUp::NeedsYou(format!(
            "sign it in afterwards with `codex mcp login {name}`: a sign-in never moves"
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

/// Appends `[mcp_servers.<name>]` to `config.toml`, after a backup.
fn add_mcp(home: &Path, name: &str, server: &McpServer) -> Result<Applied, AdapterError> {
    let config = home.join("config.toml");
    let existing = std::fs::read_to_string(&config).unwrap_or_default();
    let parsed: toml::Table = existing.parse().unwrap_or_default();
    if parsed
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .is_some_and(|servers| servers.contains_key(name))
    {
        return Err(AdapterError::Unsupported(format!(
            "Codex already has an MCP server named {name}, so nothing was replaced"
        )));
    }
    let block = codex_block(name, server);
    let mut text = existing.clone();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let appended = format!("\n{block}");
    text.push_str(&appended);
    // The result must still be TOML Codex reads, with the new server in it.
    let check: toml::Table = text.parse().map_err(|error| {
        AdapterError::Unsupported(format!("the change wouldn't be valid TOML: {error}"))
    })?;
    if check
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .and_then(|servers| servers.get(name))
        .is_none()
    {
        return Err(AdapterError::Unsupported(
            "the change didn't add the server".to_owned(),
        ));
    }
    if config.is_file() {
        std::fs::copy(&config, home.join("config.toml.before-ai-juggler"))?;
    }
    let staged = home.join(format!(".config.toml.ai-juggler-{}", std::process::id()));
    std::fs::write(&staged, &text)?;
    std::fs::rename(&staged, &config)?;
    Ok(Applied {
        summary: format!("Added the MCP server {name} to Codex"),
        undo: Undo::RemoveAppended {
            file: config,
            text: appended,
        },
    })
}

/// A server as a `[mcp_servers.<name>]` table; a dropped secret becomes a
/// variable Codex forwards (`env_vars`, `bearer_token_env_var`,
/// `env_http_headers`).
pub(crate) fn codex_block(name: &str, server: &McpServer) -> String {
    let mut table = toml::Table::new();
    let string = |s: &str| toml::Value::String(s.to_owned());
    match &server.transport {
        McpTransport::Stdio { command, args, env } => {
            table.insert("command".into(), string(command));
            if !args.is_empty() {
                table.insert(
                    "args".into(),
                    toml::Value::Array(args.iter().map(|a| string(a)).collect()),
                );
            }
            let mut literal = toml::Table::new();
            let mut forwarded = Vec::new();
            for (key, value) in env {
                match value {
                    Value::Literal(text) => {
                        literal.insert(key.clone(), string(text));
                    }
                    // Codex forwards a variable by name.
                    Value::Env(var) if var == key => forwarded.push(string(key)),
                    Value::Env(var) => {
                        literal.insert(key.clone(), string(&format!("${{{var}}}")));
                    }
                    Value::Secret => forwarded.push(string(&secret_variable(name, key))),
                }
            }
            if !literal.is_empty() {
                table.insert("env".into(), toml::Value::Table(literal));
            }
            if !forwarded.is_empty() {
                table.insert("env_vars".into(), toml::Value::Array(forwarded));
            }
        }
        McpTransport::Http { url, headers } => {
            table.insert("url".into(), string(url));
            let mut literal = toml::Table::new();
            let mut from_env = toml::Table::new();
            for (key, value) in headers {
                let variable = match value {
                    Value::Literal(text) => {
                        literal.insert(key.clone(), string(text));
                        continue;
                    }
                    Value::Env(var) => var.clone(),
                    Value::Secret => secret_variable(name, key),
                };
                if key.eq_ignore_ascii_case("authorization") {
                    table.insert("bearer_token_env_var".into(), string(&variable));
                } else {
                    from_env.insert(key.clone(), string(&variable));
                }
            }
            if !literal.is_empty() {
                table.insert("http_headers".into(), toml::Value::Table(literal));
            }
            if !from_env.is_empty() {
                table.insert("env_http_headers".into(), toml::Value::Table(from_env));
            }
        }
        McpTransport::Other { .. } => {}
    }
    let mut servers = toml::Table::new();
    servers.insert(name.to_owned(), toml::Value::Table(table));
    let mut root = toml::Table::new();
    root.insert("mcp_servers".into(), toml::Value::Table(servers));
    toml::to_string(&root).unwrap_or_default()
}

fn mcp_item(name: &str, entry: &toml::Table) -> Item {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let table_pairs = |key: &str| -> Vec<(String, Value)> {
        entry
            .get(key)
            .and_then(toml::Value::as_table)
            .map(|table| {
                table
                    .iter()
                    .map(|(k, v)| (k.clone(), value_of(k, v.as_str().unwrap_or_default())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let transport = if let Some(url) = text("url") {
        let mut headers = table_pairs("http_headers");
        if let Some(var) = text("bearer_token_env_var") {
            headers.push(("Authorization".to_owned(), Value::Env(var)));
        }
        if let Some(table) = entry
            .get("env_http_headers")
            .and_then(toml::Value::as_table)
        {
            for (key, var) in table {
                if let Some(var) = var.as_str() {
                    headers.push((key.clone(), Value::Env(var.to_owned())));
                }
            }
        }
        McpTransport::Http { url, headers }
    } else {
        let mut env = table_pairs("env");
        if let Some(names) = entry.get("env_vars").and_then(toml::Value::as_array) {
            for name in names.iter().filter_map(toml::Value::as_str) {
                env.push((name.to_owned(), Value::Env(name.to_owned())));
            }
        }
        McpTransport::Stdio {
            command: text("command").unwrap_or_default(),
            args: entry
                .get("args")
                .and_then(toml::Value::as_array)
                .map(|args| {
                    args.iter()
                        .filter_map(|a| a.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            env,
        }
    };
    let oauth = text("auth").as_deref() == Some("oauth") || entry.get("oauth").is_some();
    Item {
        tool: CODEX_ID.to_owned(),
        kind: ItemKind::McpServer,
        name: name.to_owned(),
        scope: Scope::User,
        detail: Detail::Mcp(McpServer { transport, oauth }),
    }
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: CODEX_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
    }
}

/// A Claude command as a Codex skill: frontmatter with its name and a
/// description from its first line, then the command's text.
fn as_skill(name: &str, text: &str) -> String {
    let body = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(_, body)| body))
        .unwrap_or(text);
    let description = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("A command brought over from Claude Code")
        .trim_start_matches('#')
        .trim()
        .replace('"', "'");
    format!("---\nname: {name}\ndescription: \"{description}\"\n---\n\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_codex_home_lists_what_it_has() {
        let home = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("config.toml"),
            r#"model = "gpt-6"
[mcp_servers.github]
command = "npx"
args = ["-y", "gh-mcp"]
env_vars = ["GITHUB_TOKEN"]

[mcp_servers.docs]
url = "https://docs.example.com/mcp"
bearer_token_env_var = "DOCS_TOKEN"

[mcp_servers.off]
command = "x"
enabled = false
"#,
        )
        .unwrap();
        std::fs::create_dir_all(user.path().join(".agents/skills/ship-it")).unwrap();
        std::fs::write(
            user.path().join(".agents/skills/ship-it/SKILL.md"),
            "---\nname: ship-it\n---\n",
        )
        .unwrap();
        std::fs::write(home.path().join("AGENTS.md"), "Be brief.").unwrap();
        let items = inventory(home.path(), Some(user.path()));
        let names: Vec<(ItemKind, &str)> =
            items.iter().map(|i| (i.kind, i.name.as_str())).collect();
        assert_eq!(
            names,
            vec![
                (ItemKind::McpServer, "docs"),
                (ItemKind::McpServer, "github"),
                (ItemKind::Skill, "ship-it"),
                (ItemKind::Rules, "AGENTS.md"),
            ]
        );
    }

    #[test]
    fn a_claude_server_with_a_secret_arrives_as_a_forwarded_variable() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), "model = \"gpt-6\"\n").unwrap();
        let server = McpServer {
            transport: McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "gh-mcp".into()],
                env: vec![
                    ("GITHUB_TOKEN".into(), Value::Secret),
                    ("LOG".into(), Value::Literal("info".into())),
                ],
            },
            oauth: false,
        };
        let applied = add_mcp(home.path(), "github", &server).unwrap();
        let text = std::fs::read_to_string(home.path().join("config.toml")).unwrap();
        assert!(
            text.starts_with("model = \"gpt-6\"\n"),
            "what was there is unchanged"
        );
        let table: toml::Table = text.parse().unwrap();
        let github = &table["mcp_servers"]["github"];
        assert_eq!(github["command"].as_str(), Some("npx"));
        assert_eq!(github["env_vars"][0].as_str(), Some("GITHUB_TOKEN"));
        assert_eq!(github["env"]["LOG"].as_str(), Some("info"));
        assert!(home.path().join("config.toml.before-ai-juggler").is_file());
        assert!(matches!(applied.undo, Undo::RemoveAppended { .. }));
        assert!(
            add_mcp(home.path(), "github", &server).is_err(),
            "never replaces one that's there"
        );
    }

    #[test]
    fn an_sse_server_cant_go_to_codex() {
        let item = Item {
            tool: "claude-code".into(),
            kind: ItemKind::McpServer,
            name: "old".into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport: McpTransport::Other {
                    kind: "sse".into(),
                    url: Some("https://x/sse".into()),
                },
                oauth: false,
            }),
        };
        assert!(matches!(can_set_up(&item), SetUp::Impossible(_)));
    }

    #[test]
    fn a_claude_command_becomes_a_skill() {
        let skill = as_skill("review", "# Review the diff\nLook at $ARGUMENTS");
        assert!(skill.starts_with("---\nname: review\ndescription: \"Review the diff\"\n---\n"));
        assert!(skill.contains("Look at $ARGUMENTS"));
    }
}
