//! AJ-E1542: what a Claude Code account has set up, and setting it up.
//!
//! Read from the account's own files, never by starting Claude or a
//! server:
//! - MCP servers from its state file (`.claude.json`: the user's
//!   `mcpServers`, and the project's private ones under `projects`);
//! - skills (`skills/<name>/SKILL.md`; `skills/synced/` comes from the
//!   claude.ai account), commands (`commands/*.md`), subagents
//!   (`agents/*.md`), user rules (`CLAUDE.md`, `rules/*.md`);
//! - plugins (`settings.json` `enabledPlugins`).
//!
//! Only the keys named here are parsed out of those files; everything else
//! in them (the account, a key) is never kept. Setting up an MCP server
//! goes through Claude's own `claude mcp add-json`, so Claude writes its
//! own file. Files are copied as new files, never over existing ones.

use std::collections::BTreeMap;
use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, Detail, Item, ItemKind, JsonEntry, McpServer, McpTransport, Scope, SetUp, Undo, Value,
    secret_variable, value_of,
};

use crate::claude_code::CLAUDE_CODE_ID;
use crate::inventory_files::{children, secret_names};

/// The parts of Claude's state file the inventory reads.
#[derive(serde::Deserialize, Default)]
struct State {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    projects: BTreeMap<String, ProjectState>,
}

#[derive(serde::Deserialize, Default)]
struct ProjectState {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, serde_json::Value>,
}

/// The one part of `settings.json` the inventory reads.
#[derive(serde::Deserialize, Default)]
struct Settings {
    #[serde(default, rename = "enabledPlugins")]
    enabled_plugins: BTreeMap<String, serde_json::Value>,
}

/// Everything `dir`'s account has set up; `project` adds that project's
/// private MCP servers.
#[must_use]
pub fn inventory(dir: &Path, state_file: &Path, project: Option<&Path>) -> Vec<Item> {
    let mut items = Vec::new();
    let state: State = std::fs::read(state_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    for (name, entry) in &state.mcp_servers {
        items.push(mcp_item(name, entry, Scope::User));
    }
    if let Some(project) = project {
        let key = project.to_string_lossy();
        if let Some(found) = state.projects.get(key.as_ref()) {
            for (name, entry) in &found.mcp_servers {
                items.push(mcp_item(name, entry, Scope::ProjectPrivate));
            }
        }
    }
    let skills = dir.join("skills");
    for (name, path) in children(&skills, true) {
        if name == "synced" {
            for (_, set) in children(&path, true) {
                for (skill, _) in children(&set, true) {
                    items.push(item(ItemKind::Skill, &skill, Detail::FromTheAccount));
                }
            }
        } else if path.join("SKILL.md").is_file() {
            items.push(item(ItemKind::Skill, &name, Detail::Files { path }));
        }
    }
    for (kind, folder) in [
        (ItemKind::Command, "commands"),
        (ItemKind::Subagent, "agents"),
    ] {
        for (name, path) in children(&dir.join(folder), false) {
            if let Some(stem) = name.strip_suffix(".md") {
                items.push(item(kind, stem, Detail::Files { path }));
            }
        }
    }
    if dir.join("CLAUDE.md").is_file() {
        let path = dir.join("CLAUDE.md");
        items.push(item(
            ItemKind::MainRules,
            "CLAUDE.md",
            Detail::Files { path },
        ));
    }
    for (name, path) in children(&dir.join("rules"), false) {
        if Path::new(&name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            items.push(item(
                ItemKind::Rules,
                &format!("rules/{name}"),
                Detail::Files { path },
            ));
        }
    }
    let settings: Settings = std::fs::read(dir.join("settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    for (id, enabled) in &settings.enabled_plugins {
        if enabled.as_bool() == Some(false) {
            continue;
        }
        let detail = if id.ends_with("@synced") {
            Detail::FromTheAccount
        } else {
            Detail::Plugin { id: id.clone() }
        };
        items.push(item(ItemKind::Plugin, id, detail));
    }
    items
}

/// Whether Claude Code can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_claude = item.tool == CLAUDE_CODE_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount) => SetUp::Impossible(
            "it comes with the claude.ai account, so it can't be copied to another one".to_owned(),
        ),
        (ItemKind::McpServer, Detail::Mcp(server)) => mcp_verdict(&item.name, server),
        (ItemKind::Skill | ItemKind::Command | ItemKind::Subagent, Detail::Files { .. })
            if from_claude =>
        {
            SetUp::Copy
        }
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Copy,
        (ItemKind::Command, Detail::Files { .. }) => {
            SetUp::Translate("as a Claude command; its $1, $2… arguments become $0, $1…".to_owned())
        }
        (ItemKind::Rules | ItemKind::MainRules, Detail::Files { .. }) if from_claude => SetUp::Copy,
        (ItemKind::MainRules, Detail::Files { .. }) => {
            SetUp::Translate("as this account's CLAUDE.md".to_owned())
        }
        (ItemKind::Plugin, Detail::Plugin { id }) if from_claude => SetUp::NeedsYou(format!(
            "install it in this account with `claude plugin install {id}`"
        )),
        _ => SetUp::Impossible(format!(
            "a {} from {} has no Claude Code equivalent",
            item.kind.words(),
            item.tool
        )),
    }
}

/// Sets up `item` in the Claude account at `dir`. `claude` is the Claude
/// Code program, and `config_env` the `CLAUDE_CONFIG_DIR` value for `dir`
/// (empty to leave it unset, for the vendor's default folder).
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for what can't be set up, or something
/// the account already has; the copy's or the command's error.
pub fn set_up(
    dir: &Path,
    claude: &Path,
    config_env: &str,
    state_file: &Path,
    project: Option<&Path>,
    item: &Item,
) -> Result<Applied, AdapterError> {
    match (&item.kind, &item.detail, can_set_up(item)) {
        (_, _, SetUp::Impossible(why)) => Err(AdapterError::Unsupported(why)),
        (ItemKind::McpServer, Detail::Mcp(server), _) => add_mcp(
            claude, config_env, state_file, project, &item.name, item.scope, server,
        ),
        (ItemKind::Skill, Detail::Files { path }, _) => {
            crate::inventory_files::copy_tree(path, &dir.join("skills").join(&item.name), |text| {
                text.to_owned()
            })
        }
        (ItemKind::Command | ItemKind::Subagent, Detail::Files { path }, _) => {
            let folder = if item.kind == ItemKind::Command {
                "commands"
            } else {
                "agents"
            };
            let renumber = item.tool != CLAUDE_CODE_ID && item.kind == ItemKind::Command;
            crate::inventory_files::copy_file(
                path,
                &dir.join(folder).join(format!("{}.md", item.name)),
                |text| {
                    if renumber {
                        renumber_arguments(text)
                    } else {
                        text.to_owned()
                    }
                },
            )
        }
        (ItemKind::Rules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &dir.join(&item.name), str::to_owned)
        }
        (ItemKind::MainRules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &dir.join("CLAUDE.md"), str::to_owned)
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
    let secrets: Vec<String> = secret_names(name, server);
    if server.oauth {
        return SetUp::NeedsYou(format!(
            "sign it in afterwards with `claude mcp login {name}`: a sign-in never moves"
        ));
    }
    if !secrets.is_empty() {
        return SetUp::NeedsYou(format!(
            "it needs {} in your environment: secrets are never copied",
            secrets.join(", ")
        ));
    }
    SetUp::Copy
}

/// `claude mcp add-json -s <scope> -- <name> '<json>'`, for the account
/// (and in the project's folder, for a server private to it). The undo
/// runs in the same folder, and only while the entry Claude wrote in
/// `state_file` is as it was made.
fn add_mcp(
    claude: &Path,
    config_env: &str,
    state_file: &Path,
    project: Option<&Path>,
    name: &str,
    scope: Scope,
    server: &McpServer,
) -> Result<Applied, AdapterError> {
    let scope_word = match scope {
        Scope::User => "user",
        Scope::ProjectPrivate => "local",
    };
    // A project's own server belongs to that folder; without it, Claude
    // would file it under whatever folder this runs in.
    let folder = match scope {
        Scope::User => None,
        Scope::ProjectPrivate => Some(project.ok_or_else(|| {
            AdapterError::Unsupported(format!(
                "{name} belongs to a project, and the project's folder isn't known"
            ))
        })?),
    };
    let json = claude_entry(name, server);
    let env = if config_env.is_empty() {
        Vec::new()
    } else {
        vec![("CLAUDE_CONFIG_DIR".to_owned(), config_env.to_owned())]
    };
    let mut command = std::process::Command::new(claude);
    command
        .args([
            "mcp",
            "add-json",
            "-s",
            scope_word,
            "--",
            name,
            &json.to_string(),
        ])
        .stdin(std::process::Stdio::null());
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    if config_env.is_empty() {
        command.env_remove("CLAUDE_CONFIG_DIR");
    }
    for (key, value) in &env {
        command.env(key, value);
    }
    let output = command
        .output()
        .map_err(|error| AdapterError::Unsupported(error.to_string()))?;
    if !output.status.success() {
        return Err(AdapterError::Unsupported(format!(
            "Claude didn't add {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let keys: Vec<String> = match folder {
        None => vec!["mcpServers".into(), name.into()],
        Some(folder) => vec![
            "projects".into(),
            folder.to_string_lossy().into_owned(),
            "mcpServers".into(),
            name.into(),
        ],
    };
    let unless_changed = JsonEntry::fingerprint(state_file, &keys).map(|sha256| JsonEntry {
        file: state_file.to_path_buf(),
        keys,
        sha256,
    });
    Ok(Applied {
        summary: format!("Added the MCP server {name} to this account"),
        undo: Undo::Command {
            argv: vec![
                claude.to_string_lossy().into_owned(),
                "mcp".into(),
                "remove".into(),
                "-s".into(),
                scope_word.into(),
                // A name can't be read as an option (review of #132).
                "--".into(),
                name.into(),
            ],
            env,
            unset: if config_env.is_empty() {
                vec!["CLAUDE_CONFIG_DIR".to_owned()]
            } else {
                Vec::new()
            },
            cwd: folder.map(Path::to_path_buf),
            unless_changed,
        },
        note: None,
    })
}

/// An MCP server in Claude's JSON form; a dropped secret becomes a
/// `${VARIABLE}` reference.
pub(crate) fn claude_entry(name: &str, server: &McpServer) -> serde_json::Value {
    let render = |key: &str, value: &Value| match value {
        Value::Literal(text) => serde_json::Value::String(text.clone()),
        Value::Env(var) => serde_json::Value::String(format!("${{{var}}}")),
        Value::Secret => serde_json::Value::String(format!("${{{}}}", secret_variable(name, key))),
    };
    let map = |pairs: &[(String, Value)]| -> serde_json::Map<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.clone(), render(key, value)))
            .collect()
    };
    match &server.transport {
        McpTransport::Stdio { command, args, env } => serde_json::json!({
            "type": "stdio", "command": command, "args": args, "env": map(env),
        }),
        McpTransport::Http { url, headers } => serde_json::json!({
            "type": "http", "url": url, "headers": map(headers),
        }),
        McpTransport::Other { kind, url } => serde_json::json!({ "type": kind, "url": url }),
    }
}

fn mcp_item(name: &str, entry: &serde_json::Value, scope: Scope) -> Item {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let pairs = |key: &str| -> Vec<(String, Value)> {
        entry
            .get(key)
            .and_then(serde_json::Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .map(|(k, v)| (k.clone(), value_of(k, v.as_str().unwrap_or_default())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let kind = text("type").unwrap_or_else(|| {
        if entry.get("command").is_some() {
            "stdio".into()
        } else {
            "http".into()
        }
    });
    let transport = match kind.as_str() {
        "stdio" => McpTransport::Stdio {
            command: text("command").unwrap_or_default(),
            args: entry
                .get("args")
                .and_then(serde_json::Value::as_array)
                .map(|args| {
                    args.iter()
                        .filter_map(|a| a.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            env: pairs("env"),
        },
        "http" | "streamable-http" | "streamable_http" => McpTransport::Http {
            url: text("url").unwrap_or_default(),
            headers: pairs("headers"),
        },
        other => McpTransport::Other {
            kind: other.to_owned(),
            url: text("url"),
        },
    };
    Item {
        tool: CLAUDE_CODE_ID.to_owned(),
        kind: ItemKind::McpServer,
        name: name.to_owned(),
        scope,
        detail: Detail::Mcp(McpServer {
            transport,
            oauth: entry.get("oauth").is_some(),
        }),
    }
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: CLAUDE_CODE_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
    }
}

/// A Codex prompt's `$1…$9` (1-based) as a Claude command's `$0…$8`.
fn renumber_arguments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$'
            && let Some(digit) = chars.peek().copied().filter(|d| ('1'..='9').contains(d))
        {
            chars.next();
            out.push('$');
            out.push(char::from(
                u8::try_from(u32::from(digit) - 1).unwrap_or(b'0'),
            ));
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_claude_account_lists_what_it_has_and_keeps_no_secret() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let state = d.join(".claude.json");
        std::fs::write(
            &state,
            serde_json::json!({
                "oauthAccount": {"emailAddress": "you@example.com"},
                "mcpServers": {
                    "github": {"type": "stdio", "command": "npx", "args": ["-y", "gh-mcp"],
                               "env": {"GITHUB_TOKEN": "ghp_live_secret_value_0123456789", "LOG": "info"}},
                    "linear": {"type": "http", "url": "https://mcp.linear.app/mcp", "oauth": {}}
                },
                "projects": {"/Users/you/Dev/shop": {"mcpServers": {
                    "db": {"command": "pg-mcp", "env": {"DATABASE_URL": "${DATABASE_URL}"}}
                }}}
            })
            .to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(d.join("skills/release-notes")).unwrap();
        std::fs::write(
            d.join("skills/release-notes/SKILL.md"),
            "---\nname: release-notes\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(d.join("skills/synced/abc/team-style")).unwrap();
        std::fs::create_dir_all(d.join("commands")).unwrap();
        std::fs::write(d.join("commands/review.md"), "Review $0").unwrap();
        std::fs::write(d.join("CLAUDE.md"), "Be brief.").unwrap();
        std::fs::write(
            d.join("settings.json"),
            r#"{"apiKey":"sk-should-never-be-read","enabledPlugins":{"tidy@market":true,"team@synced":true}}"#,
        )
        .unwrap();

        let items = inventory(d, &state, Some(Path::new("/Users/you/Dev/shop")));
        let names: Vec<(ItemKind, &str)> =
            items.iter().map(|i| (i.kind, i.name.as_str())).collect();
        assert!(names.contains(&(ItemKind::McpServer, "github")));
        assert!(names.contains(&(ItemKind::McpServer, "linear")));
        assert!(names.contains(&(ItemKind::McpServer, "db")));
        assert!(names.contains(&(ItemKind::Skill, "release-notes")));
        assert!(names.contains(&(ItemKind::Skill, "team-style")));
        assert!(names.contains(&(ItemKind::Command, "review")));
        assert!(names.contains(&(ItemKind::MainRules, "CLAUDE.md")));
        assert!(names.contains(&(ItemKind::Plugin, "tidy@market")));
        let everything = format!("{items:?}");
        assert!(
            !everything.contains("ghp_live"),
            "a secret's text is never kept"
        );
        assert!(
            !everything.contains("sk-should"),
            "nothing else in settings.json is read"
        );
        assert!(!everything.contains("you@example.com"));

        let github = items.iter().find(|i| i.name == "github").unwrap();
        assert!(
            matches!(can_set_up(github), SetUp::NeedsYou(note) if note.contains("GITHUB_TOKEN"))
        );
        let linear = items.iter().find(|i| i.name == "linear").unwrap();
        assert!(
            matches!(can_set_up(linear), SetUp::NeedsYou(note) if note.contains("claude mcp login linear"))
        );
        let synced = items.iter().find(|i| i.name == "team-style").unwrap();
        assert!(matches!(can_set_up(synced), SetUp::Impossible(_)));
        let entry = claude_entry(
            "github",
            match &github.detail {
                Detail::Mcp(s) => s,
                _ => unreachable!(),
            },
        );
        assert_eq!(entry["env"]["GITHUB_TOKEN"], "${GITHUB_TOKEN}");
        assert_eq!(entry["env"]["LOG"], "info");
    }

    #[test]
    fn a_codex_prompt_becomes_a_claude_command() {
        assert_eq!(
            renumber_arguments("Fix $1 then $2; cost $$ stays"),
            "Fix $0 then $1; cost $$ stays"
        );
    }
}
