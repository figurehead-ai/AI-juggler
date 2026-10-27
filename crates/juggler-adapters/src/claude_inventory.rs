//! AJ-E1542: what a Claude Code account has set up, and setting it up.
//!
//! Read from the account's own files, never by starting Claude or a
//! server:
//! - MCP servers from its state file (`.claude.json`: the user's
//!   `mcpServers`, and the project's private ones under `projects`);
//! - skills (`skills/<name>/SKILL.md`; `skills/synced/` comes from the
//!   claude.ai account), commands (`commands/*.md`), subagents
//!   (`agents/*.md`), user rules (`CLAUDE.md`, `rules/*.md`);
//! - plugins (`settings.json` `enabledPlugins`);
//! - the account's own settings (AJ-E1508): `settings.json` `permissions`
//!   and the user's own `statusLine` (never AI Juggler's capture).
//!
//! Only the keys named here are parsed out of those files; everything else
//! in them (the account, a key) is never kept. Setting up an MCP server
//! goes through Claude's own `claude mcp add-json`, so Claude writes its
//! own file. Files are copied as new files, never over existing ones.

use std::collections::BTreeMap;
use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, HowTo, Item, ItemKind, JsonEntry, McpServer, McpTransport,
    Scope, SetUp, SettingCheck, Undo, Value, secret_variable, value_of,
};
use juggler_adapter::jsonc;

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
    // claude.ai's skills sync into one folder per id they were synced
    // under, often with the same skills in each (QA P24): one item per
    // name, and none for a skill the account also has as its own files.
    let mut synced = std::collections::BTreeSet::new();
    let mut own = std::collections::BTreeSet::new();
    for (name, path) in children(&skills, true) {
        if name == "synced" {
            for (_, set) in children(&path, true) {
                synced.extend(children(&set, true).into_iter().map(|(skill, _)| skill));
            }
        } else if path.join("SKILL.md").is_file() {
            own.insert(name.clone());
            items.push(item(ItemKind::Skill, &name, Detail::Files { path }));
        }
    }
    for skill in synced.difference(&own) {
        items.push(item(ItemKind::Skill, skill, Detail::FromTheAccount));
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
    items.extend(own_settings(dir));
    items
}

/// AJ-E1508: the account's own settings, read key by key, so nothing else
/// in `settings.json` is kept: its permissions, and the user's own status
/// line (never AI Juggler's capture).
fn own_settings(dir: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let settings_file = dir.join("settings.json");
    let text = std::fs::read_to_string(&settings_file).unwrap_or_default();
    if let Some(permissions) = jsonc::value_text(&text, "permissions")
        .ok()
        .flatten()
        .filter(|value| {
            value
                .chars()
                .any(|c| !matches!(c, '{' | '}' | ' ' | '\n' | '\r' | '\t'))
        })
    {
        // Rules with a secret written in them stay behind, one by one; the
        // rest go (re-review of AJ-E1508).
        let (permissions, left_out) = without_secret_rules(&permissions);
        let (about, check) = permissions_words(&permissions);
        // Nothing left once they're out: nothing to copy.
        let check = if left_out > 0 && about.is_none() {
            SettingCheck::Cannot(
                "each of its rules has a secret written in it, and secrets are never copied: set \
                 them in the new account yourself"
                    .to_owned(),
            )
        } else {
            check
        };
        let about = match (about, left_out) {
            (about, 0) => about,
            (Some(about), n) => Some(format!("{about}; {} with a secret left out", rules(n))),
            (None, n) => Some(format!("{} with a secret left out", rules(n))),
        };
        items.push(item(
            ItemKind::Permissions,
            "permissions",
            setting(&settings_file, "permissions", permissions, about, check),
        ));
    }
    let status = jsonc::value_text(&text, "statusLine").ok().flatten();
    match crate::quota_ingest::users_own_statusline(dir, status.as_deref()) {
        crate::quota_ingest::OwnStatusLine::Own(own) => {
            let about = serde_json::from_str::<serde_json::Value>(&own)
                .ok()
                .and_then(|value| {
                    value
                        .get("command")
                        .and_then(|c| c.as_str())
                        .map(str::to_owned)
                })
                .map(|command| format!("runs {command}"));
            items.push(item(
                ItemKind::StatusLine,
                "status line",
                setting(&settings_file, "statusLine", own, about, SettingCheck::Fine),
            ));
        }
        crate::quota_ingest::OwnStatusLine::Unknown => items.push(item(
            ItemKind::StatusLine,
            "status line",
            Detail::Setting {
                file: settings_file.clone(),
                format: ConfigFormat::Json,
                entries: Vec::new(),
                about: None,
                check: SettingCheck::Cannot(
                    "AI Juggler's usage capture holds this account's status line, so its own \
                     can't be told here: set it in the new account yourself"
                        .to_owned(),
                ),
            },
        )),
        crate::quota_ingest::OwnStatusLine::None => {}
    }
    items
}

/// One setting of `settings.json`: never its text when a secret is written
/// in it (review of AJ-E1508, H1).
fn setting(
    file: &Path,
    key: &str,
    value: String,
    about: Option<String>,
    check: SettingCheck,
) -> Detail {
    let secret = crate::config_edit::strings_of(ConfigFormat::Json, &value)
        .iter()
        .any(|text| juggler_adapter::inventory::holds_secret(text));
    if secret {
        return Detail::Setting {
            file: file.to_path_buf(),
            format: ConfigFormat::Json,
            entries: Vec::new(),
            about: None,
            check: SettingCheck::Cannot(
                "a secret is written in it, and secrets are never copied: set it in the new \
                 account yourself"
                    .to_owned(),
            ),
        };
    }
    Detail::Setting {
        file: file.to_path_buf(),
        format: ConfigFormat::Json,
        entries: vec![(key.to_owned(), value)],
        about,
        check,
    }
}

/// `n` rules, in words.
fn rules(n: usize) -> String {
    if n == 1 {
        "1 rule".to_owned()
    } else {
        format!("{n} rules")
    }
}

/// A `permissions` value without the rules (and folders) that have a secret
/// written in them, and how many were left out. A value that isn't plain
/// JSON is kept as it is: the whole-value check refuses it if it holds one.
fn without_secret_rules(value: &str) -> (String, usize) {
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(value.trim()) else {
        return (value.to_owned(), 0);
    };
    let mut left_out = 0;
    if let Some(map) = parsed.as_object_mut() {
        for key in ["allow", "deny", "ask", "additionalDirectories"] {
            if let Some(list) = map.get_mut(key).and_then(serde_json::Value::as_array_mut) {
                let before = list.len();
                list.retain(|rule| {
                    !rule
                        .as_str()
                        .is_some_and(juggler_adapter::inventory::holds_secret)
                });
                left_out += before - list.len();
            }
        }
    }
    if left_out == 0 {
        return (value.to_owned(), 0);
    }
    (parsed.to_string(), left_out)
}

/// What a `permissions` value holds, in words ("2 allowed, 1 denied"), and
/// whether it turns permission checks off (review of AJ-E1508, M5): then
/// it's offered, never ticked for the user.
fn permissions_words(value: &str) -> (Option<String>, SettingCheck) {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value.trim()) else {
        return (None, SettingCheck::Fine);
    };
    let count = |key: &str| {
        parsed
            .get(key)
            .and_then(|v| v.as_array())
            .map_or(0, Vec::len)
    };
    let mut parts: Vec<String> = [
        ("allow", "allowed"),
        ("deny", "denied"),
        ("ask", "asked first"),
    ]
    .iter()
    .filter(|(key, _)| count(key) > 0)
    .map(|(key, words)| format!("{} {words}", count(key)))
    .collect();
    let mode = parsed.get("defaultMode").and_then(|m| m.as_str());
    if let Some(mode) = mode {
        parts.push(format!("mode {mode}"));
    }
    let folders = count("additionalDirectories");
    if folders > 0 {
        parts.push(format!("{folders} more folders"));
    }
    let check = if mode == Some("bypassPermissions") {
        SettingCheck::Caution(
            "it turns permission checks off (bypassPermissions): tick it only if you mean that"
                .to_owned(),
        )
    } else {
        SettingCheck::Fine
    };
    ((!parts.is_empty()).then(|| parts.join(", ")), check)
}

/// Where a claude.ai account turns its features and skills on.
pub const ACCOUNT_SETTINGS_URL: &str = "https://claude.ai/settings";

/// How the user finishes `item` on another account themselves:
///
/// - something that comes with the claude.ai account (it can't be copied):
///   the generic steps on its settings page (the switch dialog names the
///   account);
/// - an MCP server that needs a secret: the command that sets it up for
///   that account with the secret in its environment. Claude Code's own
///   way (<https://code.claude.com/docs/en/mcp>, "Add MCP servers from
///   JSON configuration" and "MCP installation scopes"): `claude mcp
///   add-json` writes the server, with its `env` (or `headers`), into the
///   account's `.claude.json`, which is in `CLAUDE_CONFIG_DIR` when that
///   is set (`config_dir`, as the shell reads it; empty for the vendor's
///   default folder). The secret is a placeholder the user replaces.
///
/// Empty for anything else.
#[must_use]
pub fn how_to_get(item: &Item, config_dir: &str) -> HowTo {
    match &item.detail {
        Detail::FromTheAccount => HowTo {
            steps: vec![
                "Open claude.ai settings in your browser, signed in as the account you're switching to."
                    .to_owned(),
                format!("Find {} there and turn it on.", item.name),
                "Come back here and choose Check again.".to_owned(),
            ],
            command: None,
            on_account_page: true,
        },
        Detail::Mcp(server)
            if server.inline_secret_verdict().is_none()
                && !server.oauth
                && !secret_names(&item.name, server).is_empty() =>
        {
            mcp_secret_steps(&item.name, item.scope, server, config_dir)
        }
        _ => HowTo::default(),
    }
}

/// The steps and the command for an MCP server that needs a secret.
fn mcp_secret_steps(name: &str, scope: Scope, server: &McpServer, config_dir: &str) -> HowTo {
    let secrets = secret_names(name, server);
    let placeholders: Vec<String> = secrets.iter().map(|var| format!("<your {var}>")).collect();
    let scope_word = match scope {
        Scope::User => "user",
        Scope::ProjectPrivate => "local",
    };
    let prefix = if config_dir.is_empty() {
        String::new()
    } else {
        format!("CLAUDE_CONFIG_DIR=\"{config_dir}\" ")
    };
    let json = entry_with(name, server, &|var| format!("<your {var}>")).to_string();
    let command = format!(
        "{prefix}claude mcp remove {name} --scope {scope_word}\n\
         {prefix}claude mcp add-json --scope {scope_word} {name} '{json}'"
    );
    let mut steps = vec![
        if scope == Scope::ProjectPrivate {
            "Open Terminal, and go to the project's folder.".to_owned()
        } else {
            "Open Terminal.".to_owned()
        },
        format!(
            "Paste the command below. Put your own value in place of {} (keep the quotes \
             around it), then press Return.",
            placeholders.join(" and ")
        ),
        format!(
            "It sets {name} up for this account, with the value in this account's own Claude \
             Code settings. Its first line takes away a {name} set up without the value; if \
             there isn't one, it says so, and that's fine."
        ),
    ];
    steps.push("Come back here and choose Check again.".to_owned());
    HowTo {
        steps,
        command: Some(command),
        on_account_page: false,
    }
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
        (ItemKind::Permissions | ItemKind::StatusLine, Detail::Setting { check, .. })
            if from_claude =>
        {
            match check {
                SettingCheck::Fine => SetUp::Copy,
                SettingCheck::Caution(note) => SetUp::NeedsYou(note.clone()),
                SettingCheck::Cannot(note) => SetUp::Impossible(note.clone()),
            }
        }
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
        (ItemKind::Permissions | ItemKind::StatusLine, _) => SetUp::Impossible(format!(
            "the {} of {} work differently from Claude Code's: set them in Claude Code",
            item.kind.words(),
            item.tool
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
        (
            ItemKind::Permissions | ItemKind::StatusLine,
            Detail::Setting {
                file: from,
                entries,
                ..
            },
            _,
        ) => {
            if entries.is_empty() {
                return Err(AdapterError::Unsupported(
                    "there's nothing of it that can be copied".to_owned(),
                ));
            }
            let file = dir.join("settings.json");
            crate::config_edit::set(&file, ConfigFormat::Json, entries)
                .map_err(AdapterError::Unsupported)?;
            // A status line that runs a script in the other account's folder
            // keeps running it from there.
            let note = from
                .parent()
                .map(|folder| folder.to_string_lossy().into_owned())
                .filter(|folder| {
                    !folder.is_empty()
                        && entries
                            .iter()
                            .any(|(_, value)| value.contains(folder.as_str()))
                })
                .map(|_| "it runs a script from the other account's folder".to_owned());
            Ok(Applied {
                summary: format!("Set the {}", item.kind.words()),
                // Each value's fingerprint, never the value itself.
                undo: Undo::RemoveSetting {
                    file,
                    format: ConfigFormat::Json,
                    entries: entries
                        .iter()
                        .map(|(key, value)| {
                            (
                                key.clone(),
                                crate::config_edit::fingerprint(ConfigFormat::Json, value),
                            )
                        })
                        .collect(),
                },
                note,
            })
        }
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
            "it needs your {}, and AI Juggler never copies a secret",
            secrets.join(" and ")
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
    // Claude may file the project under its resolved path (`/private/tmp`
    // for `/tmp`): try that too, so the undo keeps its check.
    let resolved = folder.and_then(|folder| std::fs::canonicalize(folder).ok());
    let candidates = std::iter::once(keys.clone()).chain(resolved.map(|resolved| {
        let mut keys = keys.clone();
        keys[1] = resolved.to_string_lossy().into_owned();
        keys
    }));
    let unless_changed = candidates
        .into_iter()
        .find_map(|keys| JsonEntry::fingerprint(state_file, &keys).map(|sha256| (keys, sha256)))
        .map(|(keys, sha256)| JsonEntry {
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
    entry_with(name, server, &|var| format!("${{{var}}}"))
}

/// The entry, with each secret as `secret(variable)`: a reference for
/// the set-up, a placeholder for the user's own command.
fn entry_with(
    name: &str,
    server: &McpServer,
    secret: &dyn Fn(&str) -> String,
) -> serde_json::Value {
    let render = |key: &str, value: &Value| match value {
        Value::Literal(text) => serde_json::Value::String(text.clone()),
        Value::Env(var) => serde_json::Value::String(format!("${{{var}}}")),
        Value::Secret => serde_json::Value::String(secret(&secret_variable(name, key))),
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

        // AJ-E1543: a server that needs its secret says how to give it
        // one on that account, with a placeholder, never the value.
        let how = how_to_get(github, "$HOME/.claude-personal");
        let command = how.command.clone().unwrap();
        assert!(!format!("{how:?}").contains("ghp_live"), "never the secret");
        assert!(!how.on_account_page);
        assert!(command.starts_with(
            "CLAUDE_CONFIG_DIR=\"$HOME/.claude-personal\" claude mcp remove github --scope user\n"
        ));
        let add = command.lines().nth(1).unwrap();
        assert!(add.starts_with(
            "CLAUDE_CONFIG_DIR=\"$HOME/.claude-personal\" claude mcp add-json --scope user github '"
        ));
        let json: serde_json::Value =
            serde_json::from_str(add.split_once(" '").unwrap().1.trim_end_matches('\'')).unwrap();
        assert_eq!(json["env"]["GITHUB_TOKEN"], "<your GITHUB_TOKEN>");
        assert!(how.steps[1].contains("<your GITHUB_TOKEN>"));
        assert_eq!(
            how.steps.last().map(String::as_str),
            Some("Come back here and choose Check again.")
        );
        // The vendor's own folder: no CLAUDE_CONFIG_DIR at all.
        assert!(
            how_to_get(github, "")
                .command
                .unwrap()
                .starts_with("claude mcp remove github --scope user")
        );
        // A sign-in is no secret to paste; the account's own skill is
        // turned on on its page.
        assert_eq!(
            how_to_get(linear, "$HOME/.claude-personal"),
            HowTo::default()
        );
        assert!(how_to_get(synced, "").on_account_page);
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

    /// QA P24: claude.ai's skills, synced under two ids with the same
    /// skills in each, are one item per name; a skill the account also has
    /// as its own files is that one item.
    #[test]
    fn synced_skills_are_listed_once_each() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for set in ["org-a_user", "org-b_user"] {
            for skill in ["pdf", "docs", "release-notes"] {
                std::fs::create_dir_all(d.join("skills/synced").join(set).join(skill)).unwrap();
            }
        }
        std::fs::create_dir_all(d.join("skills/release-notes")).unwrap();
        std::fs::write(
            d.join("skills/release-notes/SKILL.md"),
            "---\nname: release-notes\n---\n",
        )
        .unwrap();
        let items = inventory(d, &d.join("state.json"), None);
        let skills: Vec<(&str, bool)> = items
            .iter()
            .filter(|i| i.kind == ItemKind::Skill)
            .map(|i| (i.name.as_str(), matches!(i.detail, Detail::FromTheAccount)))
            .collect();
        assert_eq!(
            skills,
            vec![("release-notes", false), ("docs", true), ("pdf", true)]
        );
        let mut keys: Vec<(ItemKind, &str)> =
            items.iter().map(|i| (i.kind, i.name.as_str())).collect();
        let all = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), all, "one item per kind and name");
    }
}
