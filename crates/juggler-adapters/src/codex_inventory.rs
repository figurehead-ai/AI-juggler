//! AJ-E1542: what a Codex home has set up, and setting it up.
//!
//! Read from the home's own files, never by starting Codex or a server:
//! - MCP servers from `config.toml` `[mcp_servers.<name>]`;
//! - skills (`~/.agents/skills/<name>/SKILL.md`, shared by every Codex
//!   home, and the home's own `skills/`, `.system` left out), prompts
//!   (`prompts/*.md`), subagents (`agents/*.toml`), rules (`AGENTS.md`);
//! - plugins (`[plugins."name@marketplace"]`), each with where it came
//!   from by its marketplace (switch-setup-items.md §4): `OpenAI`'s own
//!   (`openai-bundled`, `openai-primary-runtime`, `openai-curated`) are
//!   turned on in Codex, never copied;
//! - the home's own settings (AJ-E1508): its permissions (`approval_policy`
//!   and `sandbox_mode`) and its status line (`tui.status_line`).
//!
//! Setting up an MCP server appends one new `[mcp_servers.<name>]` table to
//! `config.toml` (the file from before the first set-up is kept as a
//! backup until the last undo, and the exact text appended is the undo
//! record); nothing already there is changed. A skill goes into
//! the shared `~/.agents/skills/`, a Claude command becomes a skill, and
//! Claude rules become the home's `AGENTS.md`, as new files only.

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::inventory::{
    Applied, ConfigFormat, Detail, GetIt, Item, ItemKind, McpServer, McpTransport, Need, Origin,
    Scope, SetUp, SettingCheck, Undo, Value, secret_variable, value_of,
};

use crate::codex::CODEX_ID;
use crate::inventory_files::{children, secret_names};

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
                items.push(Item {
                    origin: plugin_origin(id),
                    ..item(ItemKind::Plugin, id, Detail::Plugin { id: id.clone() })
                });
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
            items.push(item(
                ItemKind::MainRules,
                "AGENTS.md",
                Detail::Files { path },
            ));
            break;
        }
    }
    items.extend(own_settings(home));
    items
}

/// The marketplaces whose plugins come with the Codex app itself (local
/// `config.toml` headers and the plugin cache; switch-setup-items.md §4).
const WITH_THE_APP: [&str; 2] = ["openai-bundled", "openai-primary-runtime"];

/// The marketplace of the plugin directory `ChatGPT` and Codex share.
const CURATED: [&str; 2] = ["openai-curated", "openai-curated-remote"];

/// The plugins a user made in their `ChatGPT` workspace.
const MADE_IN_THE_WORKSPACE: &str = "created-by-me-remote";

/// Where a plugin came from, by its marketplace (`name@marketplace`).
fn plugin_origin(id: &str) -> Origin {
    let marketplace = id.rsplit_once('@').map_or("", |(_, market)| market);
    if WITH_THE_APP.contains(&marketplace) || CURATED.contains(&marketplace) {
        Origin::Provided {
            by: "OpenAI".to_owned(),
        }
    } else if marketplace == MADE_IN_THE_WORKSPACE {
        Origin::Account
    } else {
        Origin::Own
    }
}

/// How the Codex home of `{account}` gets the plugin `id`, which comes
/// from `OpenAI` or the `ChatGPT` workspace: in Codex's own `/plugins`
/// (learn.chatgpt.com/docs/plugins). No page address: no stable `ChatGPT`
/// settings address is verified (switch-setup-items.md §4, NE).
fn plugin_get_it(id: &str) -> GetIt {
    let (name, marketplace) = id.rsplit_once('@').unwrap_or((id, ""));
    let mut steps = Vec::new();
    if WITH_THE_APP.contains(&marketplace) {
        steps.push(format!(
            "Open the Codex app signed in as {{account}}: {name} comes with the app."
        ));
        steps.push(format!(
            "If it isn't on, type /plugins in Codex, find {name} and turn it on."
        ));
    } else {
        steps.push(format!(
            "In Codex on {{account}}, type /plugins, find {name} and install it."
        ));
        steps.push("If it connects an app, sign in to it when Codex asks.".to_owned());
    }
    steps.push("Start a new Codex session on {account}.".to_owned());
    GetIt {
        steps,
        command: None,
        open_url: None,
        needs: vec![Need::Admin(
            "On an Enterprise or Edu workspace, your admin may have to allow it first.".to_owned(),
        )],
        then: Some("Then choose Check again.".to_owned()),
    }
}

/// AJ-E1508: the home's own settings, key by key; never the text of one
/// with a secret written in it (review of AJ-E1508, H1).
fn own_settings(home: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let config_file = home.join("config.toml");
    let setting = |keys: &[&str]| -> Option<Detail> {
        let entries: Vec<(String, String)> = keys
            .iter()
            .filter_map(|key| {
                crate::config_edit::read(&config_file, ConfigFormat::Toml, key)
                    .map(|value| ((*key).to_owned(), value))
            })
            .collect();
        if entries.is_empty() {
            return None;
        }
        let secret = entries.iter().any(|(_, value)| {
            crate::config_edit::strings_of(ConfigFormat::Toml, value)
                .iter()
                .any(|text| juggler_adapter::inventory::holds_secret(text))
        });
        if secret {
            return Some(Detail::Setting {
                file: config_file.clone(),
                format: ConfigFormat::Toml,
                entries: Vec::new(),
                about: None,
                check: SettingCheck::Cannot(
                    "a secret is written in it, and secrets are never copied: set it in the new \
                     home yourself"
                        .to_owned(),
                ),
            });
        }
        let value = |key: &str| {
            entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.trim_matches('"').to_owned())
        };
        let about = entries
            .iter()
            .map(|(key, value)| {
                format!(
                    "{} {}",
                    key.rsplit('.').next().unwrap_or(key),
                    value.trim_matches('"')
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        // No approval and no sandbox: offered, never ticked for the user
        // (review of AJ-E1508, M5).
        // No sandbox at all: offered, never ticked for the user (re-review
        // of AJ-E1508: danger-full-access alone is enough).
        let check = if value("sandbox_mode").as_deref() == Some("danger-full-access") {
            SettingCheck::Caution(
                "it runs commands without a sandbox (danger-full-access): tick it only if you \
                 mean that"
                    .to_owned(),
            )
        } else {
            SettingCheck::Fine
        };
        Some(Detail::Setting {
            file: config_file.clone(),
            format: ConfigFormat::Toml,
            entries,
            about: Some(about),
            check,
        })
    };
    if let Some(detail) = setting(&["approval_policy", "sandbox_mode"]) {
        items.push(item(ItemKind::Permissions, "permissions", detail));
    }
    if let Some(detail) = setting(&["tui.status_line"]) {
        items.push(item(ItemKind::StatusLine, "status line", detail));
    }
    items
}

/// Where a project's own server goes in Codex.
const EVERY_PROJECT: &str =
    "for every project in this Codex home: Codex has no servers for one project only";

/// Whether Codex can set up `item`.
#[must_use]
pub fn can_set_up(item: &Item) -> SetUp {
    let from_codex = item.tool == CODEX_ID;
    match (&item.kind, &item.detail) {
        (_, Detail::FromTheAccount { .. }) => SetUp::Impossible(
            "it comes with {tool}'s own account, so Codex can't have it".to_owned(),
        ),
        (ItemKind::McpServer, Detail::Mcp(server)) => {
            let verdict = mcp_verdict(&item.name, server);
            // Codex keeps servers per home, not per project: say so before
            // a project's own server becomes everyone's (review of #132).
            match (item.scope, verdict) {
                (Scope::ProjectPrivate, SetUp::Copy) => SetUp::Translate(EVERY_PROJECT.to_owned()),
                (Scope::ProjectPrivate, SetUp::Translate(note)) => {
                    SetUp::Translate(format!("{note}; {EVERY_PROJECT}"))
                }
                (_, verdict) => verdict,
            }
        }
        // The shared skills folder is read by every Codex home on this Mac,
        // and an undo takes it away from all of them.
        (ItemKind::Skill, Detail::Files { .. }) if !from_codex => SetUp::Translate(
            "into ~/.agents/skills, which every Codex home on this Mac reads".to_owned(),
        ),
        (ItemKind::Skill, Detail::Files { .. }) => SetUp::Copy,
        (ItemKind::Permissions | ItemKind::StatusLine, Detail::Setting { check, .. })
            if from_codex =>
        {
            match check {
                SettingCheck::Fine => SetUp::Copy,
                SettingCheck::Caution(note) => SetUp::NeedsYou(note.clone()),
                SettingCheck::Cannot(note) => SetUp::Impossible(note.clone()),
            }
        }
        (ItemKind::Command | ItemKind::Subagent | ItemKind::MainRules, Detail::Files { .. })
            if from_codex =>
        {
            SetUp::Copy
        }
        (ItemKind::Command, Detail::Files { .. }) => SetUp::Translate(
            "as a Codex skill you call with $name; its arguments need a look".to_owned(),
        ),
        (ItemKind::MainRules, Detail::Files { .. }) => {
            SetUp::Translate("as this home's AGENTS.md (Codex doesn't read @-imports)".to_owned())
        }
        // Codex reads one rules file: a further one can only be added to it
        // (review of #132: each became its own AGENTS.md offer, and all but
        // the first failed).
        (ItemKind::Rules, Detail::Files { .. }) => SetUp::NeedsYou(
            "Codex reads one rules file, AGENTS.md: add what this one says to it yourself"
                .to_owned(),
        ),
        // OpenAI's own, or the workspace's: Codex installs it itself.
        (ItemKind::Plugin, Detail::Plugin { id }) if from_codex && item.origin != Origin::Own => {
            SetUp::TurnOn {
                get_it: plugin_get_it(id),
                can_copy: false,
            }
        }
        (ItemKind::Plugin, Detail::Plugin { id }) if from_codex => {
            SetUp::NeedsYou(format!("install it here with `codex plugin add {id}`"))
        }
        (ItemKind::Plugin, _) => {
            SetUp::Impossible("Claude Code plugins don't run in Codex".to_owned())
        }
        (ItemKind::Permissions | ItemKind::StatusLine, _) => SetUp::Impossible(format!(
            "the {} of {{tool}} work differently from Codex's: set them in Codex",
            item.kind.words(),
        )),
        _ => SetUp::Impossible(format!(
            "a {} from {{tool}} has no Codex equivalent yet",
            item.kind.words(),
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
        (_, _, SetUp::TurnOn { .. }) => Err(AdapterError::Unsupported(format!(
            "{} comes with the account or the app: turn it on in Codex",
            item.name
        ))),
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
        (ItemKind::MainRules, Detail::Files { path }, _) => {
            crate::inventory_files::copy_file(path, &home.join("AGENTS.md"), str::to_owned)
        }
        (ItemKind::Permissions | ItemKind::StatusLine, Detail::Setting { entries, .. }, _) => {
            if entries.is_empty() {
                return Err(AdapterError::Unsupported(
                    "there's nothing of it that can be copied".to_owned(),
                ));
            }
            let file = home.join("config.toml");
            crate::config_edit::set(&file, ConfigFormat::Toml, entries)
                .map_err(AdapterError::Unsupported)?;
            Ok(Applied {
                summary: format!("Set the {}", item.kind.words()),
                // Each value's fingerprint, never the value itself.
                undo: Undo::RemoveSetting {
                    file,
                    format: ConfigFormat::Toml,
                    entries: entries
                        .iter()
                        .map(|(key, value)| {
                            (
                                key.clone(),
                                crate::config_edit::fingerprint(ConfigFormat::Toml, value),
                            )
                        })
                        .collect(),
                },
                note: None,
            })
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
            "it needs your {}, and AI Juggler never copies a secret",
            secrets.join(" and ")
        ));
    }
    SetUp::Copy
}

/// How the user gives an MCP server that needs a secret its value on the
/// Codex account whose home is `codex_home` (as the shell reads it, `$HOME/…`;
/// empty for Codex's default `~/.codex`). Codex's own ways
/// (<https://learn.chatgpt.com/docs/extend/mcp?surface=cli>, `codex mcp
/// add … --env VAR=VALUE -- <command>`, and the `[mcp_servers.<name>.env]`
/// table; `config.toml` lives in `CODEX_HOME`, see
/// <https://learn.chatgpt.com/docs/config-file/config-sample>):
///
/// - a local server whose environment is plain values and the secret: the
///   two commands (`codex mcp remove`, as `codex mcp --help` lists it, then
///   `codex mcp add --env`), the secret a placeholder;
/// - any other local server: the lines to add to that account's
///   `config.toml`, the secret a placeholder;
/// - a server over HTTP: Codex reads its token from a variable in the
///   environment it starts in (`bearer_token_env_var`), so the line for
///   the shell's startup file — which every account on this Mac reads.
///
/// AI Juggler never writes the secret; empty for anything else.
#[must_use]
pub fn how_to_get(item: &Item, codex_home: &str) -> GetIt {
    let Detail::Mcp(server) = &item.detail else {
        return GetIt::default();
    };
    let name = &item.name;
    let secrets = secret_names(name, server);
    if server.oauth
        || secrets.is_empty()
        || !matches!(mcp_verdict(name, server), SetUp::NeedsYou(_))
    {
        return GetIt::default();
    }
    let values = secrets
        .iter()
        .map(|var| format!("<your {var}>"))
        .collect::<Vec<_>>()
        .join(" and ");
    match &server.transport {
        McpTransport::Stdio { command, args, env }
            if env.iter().all(|(_, value)| !matches!(value, Value::Env(_))) =>
        {
            add_with_env(name, codex_home, &values, command, args, env)
        }
        McpTransport::Stdio { .. } => env_lines(name, codex_home, &values, &secrets),
        _ => shell_lines(&values, &secrets),
    }
}

/// `codex mcp remove`, then `codex mcp add … --env …`, for the account.
fn add_with_env(
    name: &str,
    codex_home: &str,
    values: &str,
    command: &str,
    args: &[String],
    env: &[(String, Value)],
) -> GetIt {
    let prefix = if codex_home.is_empty() {
        String::new()
    } else {
        format!("CODEX_HOME=\"{codex_home}\" ")
    };
    let mut add = format!("{prefix}codex mcp add {}", shell_word(name));
    for (key, value) in env {
        let text = match value {
            Value::Literal(text) => text.clone(),
            _ => format!("<your {}>", secret_variable(name, key)),
        };
        add.push_str(" --env ");
        add.push_str(&shell_word(&format!("{key}={text}")));
    }
    add.push_str(" --");
    for word in std::iter::once(command).chain(args.iter().map(String::as_str)) {
        add.push(' ');
        add.push_str(&shell_word(word));
    }
    GetIt {
        steps: vec![
            "Open Terminal.".to_owned(),
            format!(
                "Paste the command below. Put your own value in place of {values} (keep the \
                 quotes around it), then press Return."
            ),
            format!(
                "It sets {name} up for this account, with the value in this account's own \
                 Codex settings. Its first line takes away a {name} set up without the value; \
                 if there isn't one, it says so, and that's fine."
            ),
            "Come back here and choose Check again.".to_owned(),
        ],
        command: Some(format!(
            "{prefix}codex mcp remove {}\n{add}",
            shell_word(name)
        )),
        ..GetIt::default()
    }
}

/// The lines for `[mcp_servers.<name>.env]` in the account's `config.toml`.
fn env_lines(name: &str, codex_home: &str, values: &str, secrets: &[String]) -> GetIt {
    let file = if codex_home.is_empty() {
        "$HOME/.codex/config.toml".to_owned()
    } else {
        format!("{codex_home}/config.toml")
    };
    let lines = std::iter::once(format!("[mcp_servers.{name}.env]"))
        .chain(
            secrets
                .iter()
                .map(|var| format!("{var} = \"<your {var}>\"")),
        )
        .collect::<Vec<_>>()
        .join("\n");
    GetIt {
        steps: vec![
            format!("Open this account's Codex settings file in a text editor: {file}"),
            format!(
                "Find the [mcp_servers.{name}.env] section (add that line under \
                 [mcp_servers.{name}] if it isn't there), and add the lines below, with your own \
                 value in place of {values}."
            ),
            format!(
                "If [mcp_servers.{name}] has an env_vars line, take {} out of it, then save the \
                 file.",
                secrets.join(" and ")
            ),
            "Come back here and choose Check again.".to_owned(),
        ],
        command: Some(lines),
        ..GetIt::default()
    }
}

/// A server over HTTP: Codex reads its token from the environment it
/// starts in, so the line for the shell's startup file.
fn shell_lines(values: &str, secrets: &[String]) -> GetIt {
    GetIt {
        steps: vec![
            format!(
                "Codex reads this server's {} from the environment it starts in, so it can't be \
                 set for one account alone.",
                secrets.join(" and ")
            ),
            "Open your shell's startup file (for zsh, ~/.zshrc) in a text editor.".to_owned(),
            format!(
                "Add the line below, with your own value in place of {values}, and save the \
                 file. Every account on this computer gets it."
            ),
            "Tick it here so it's set up on this account; sessions you start after that have \
             the value."
                .to_owned(),
        ],
        command: Some(
            secrets
                .iter()
                .map(|var| format!("export {var}=\"<your {var}>\""))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        ..GetIt::default()
    }
}

/// One word as a POSIX shell reads it: as it is when it's plain, else in
/// single quotes.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./_-".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
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
    // The backup holds the file as it was before AI Juggler's first
    // set-up: a later set-up keeps it rather than copying over it.
    let backup = home.join("config.toml.before-ai-juggler");
    if config.is_file() && !backup.exists() {
        std::fs::copy(&config, &backup)?;
    }
    if config.exists() {
        juggler_adapter::inventory::rewrite_in_place(&config, &text)?;
    } else {
        std::fs::write(&config, &text)?;
    }
    Ok(Applied {
        summary: format!("Added the MCP server {name} to Codex"),
        undo: Undo::RemoveAppended {
            file: config,
            text: appended,
            backup: backup.is_file().then_some(backup),
        },
        note: None,
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
    toml_mcp_item(CODEX_ID, name, entry)
}

/// A server in the `[mcp_servers.<name>]` TOML form Codex uses, and the
/// tools that adopted it (AJ-E1604: Grok Build), as `tool`'s item.
pub(crate) fn toml_mcp_item(tool: &str, name: &str, entry: &toml::Table) -> Item {
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
        tool: tool.to_owned(),
        kind: ItemKind::McpServer,
        name: name.to_owned(),
        scope: Scope::User,
        detail: Detail::Mcp(McpServer { transport, oauth }),
        origin: Origin::Own,
    }
}

fn item(kind: ItemKind, name: &str, detail: Detail) -> Item {
    Item {
        tool: CODEX_ID.to_owned(),
        kind,
        name: name.to_owned(),
        scope: Scope::User,
        detail,
        origin: Origin::Own,
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
                (ItemKind::MainRules, "AGENTS.md"),
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

    /// AJ-E1543: a server that needs its secret says how to give it one
    /// on that Codex account, with a placeholder, never the value.
    #[test]
    fn a_server_that_needs_a_secret_says_how_to_give_it_one() {
        let mcp = |transport| Item {
            tool: "claude-code".into(),
            kind: ItemKind::McpServer,
            name: "github".into(),
            scope: Scope::User,
            detail: Detail::Mcp(McpServer {
                transport,
                oauth: false,
            }),
            origin: Origin::Own,
        };
        let stdio = mcp(McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "gh-mcp".into()],
            env: vec![
                ("GITHUB_TOKEN".into(), Value::Secret),
                ("LOG".into(), Value::Literal("info level".into())),
            ],
        });
        let how = how_to_get(&stdio, "$HOME/.codex-personal");
        assert_eq!(
            how.command.as_deref(),
            Some(
                "CODEX_HOME=\"$HOME/.codex-personal\" codex mcp remove github\n\
                 CODEX_HOME=\"$HOME/.codex-personal\" codex mcp add github \
                 --env 'GITHUB_TOKEN=<your GITHUB_TOKEN>' --env 'LOG=info level' -- npx -y gh-mcp"
            )
        );
        assert_eq!(how.open_url, None);
        assert_eq!(
            how.steps.last().map(String::as_str),
            Some("Come back here and choose Check again.")
        );
        // Codex's own home: no CODEX_HOME at all.
        assert!(
            how_to_get(&stdio, "")
                .command
                .unwrap()
                .starts_with("codex mcp remove github\n")
        );
        // A variable it reads from the environment: the config.toml lines.
        let forwarded = mcp(McpTransport::Stdio {
            command: "gh-mcp".into(),
            args: Vec::new(),
            env: vec![
                ("GITHUB_TOKEN".into(), Value::Secret),
                ("HOME_DIR".into(), Value::Env("HOME".into())),
            ],
        });
        let how = how_to_get(&forwarded, "$HOME/.codex-personal");
        assert_eq!(
            how.command.as_deref(),
            Some("[mcp_servers.github.env]\nGITHUB_TOKEN = \"<your GITHUB_TOKEN>\"")
        );
        assert!(how.steps[0].ends_with("$HOME/.codex-personal/config.toml"));
        // Nothing to give: nothing to say.
        let plain = mcp(McpTransport::Stdio {
            command: "x".into(),
            args: Vec::new(),
            env: Vec::new(),
        });
        assert_eq!(how_to_get(&plain, ""), GetIt::default());
        assert_eq!(shell_word("it's"), "'it'\\''s'");
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
            origin: juggler_adapter::inventory::Origin::Own,
        };
        assert!(matches!(can_set_up(&item), SetUp::Impossible(_)));
    }

    #[test]
    fn a_claude_command_becomes_a_skill() {
        let skill = as_skill("review", "# Review the diff\nLook at $ARGUMENTS");
        assert!(skill.starts_with("---\nname: review\ndescription: \"Review the diff\"\n---\n"));
        assert!(skill.contains("Look at $ARGUMENTS"));
    }

    #[test]
    fn only_claudes_main_rules_file_becomes_agents_md() {
        // Review of #132: every `rules/*.md` was its own AGENTS.md offer.
        let rules = |kind: ItemKind, name: &str| Item {
            tool: "claude-code".into(),
            kind,
            name: name.into(),
            scope: Scope::User,
            detail: Detail::Files { path: "/x".into() },
            origin: juggler_adapter::inventory::Origin::Own,
        };
        assert!(matches!(
            can_set_up(&rules(ItemKind::MainRules, "CLAUDE.md")),
            SetUp::Translate(_)
        ));
        assert!(matches!(
            can_set_up(&rules(ItemKind::Rules, "rules/style.md")),
            SetUp::NeedsYou(_)
        ));
    }
}
