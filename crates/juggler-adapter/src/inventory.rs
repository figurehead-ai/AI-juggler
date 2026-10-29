//! AJ-E1542: what an account has set up, in one shape for every tool.
//!
//! Before a switch, AI Juggler compares what the source account has (MCP
//! servers, skills, custom commands, subagents, user rules, plugins) with
//! the target's, and offers to set up what's missing. Each adapter reads
//! its own tool's files into an [`Item`] (never by starting the tool or a
//! server), says whether it can set up an item that came from another
//! account or tool ([`SetUp`]), and does it when the user approves.
//!
//! Secrets never enter this shape: while a tool's config is read, a value
//! that looks like a secret is replaced by [`Value::Secret`] and its text
//! dropped, so nothing here can copy or show it. A value that refers to an
//! environment variable (`${TOKEN}`) is kept as a reference.

use std::path::PathBuf;

/// What kind of thing an item is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    /// An MCP server.
    McpServer,
    /// A skill (a folder with `SKILL.md`).
    Skill,
    /// A custom slash command or prompt.
    Command,
    /// A subagent definition.
    Subagent,
    /// A further rules file the tool reads for every project (Claude's
    /// `rules/*.md`).
    Rules,
    /// The account's main rules file (Claude's `CLAUDE.md`, Codex's
    /// `AGENTS.md`): one per account, whatever each tool calls it, so the
    /// two match across tools.
    MainRules,
    /// An installed plugin.
    Plugin,
    /// What the tool may do without asking: its permission rules (AJ-E1508).
    /// One per account, whatever the tool calls it.
    Permissions,
    /// The status line under the tool's input (AJ-E1508). One per account.
    StatusLine,
}

impl ItemKind {
    /// The kind in words, for a sentence ("the skill release-notes").
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::McpServer => "MCP server",
            Self::Skill => "skill",
            Self::Command => "command",
            Self::Subagent => "subagent",
            Self::Rules => "rules file",
            Self::MainRules => "main rules file",
            Self::Plugin => "plugin",
            Self::Permissions => "permissions",
            Self::StatusLine => "status line",
        }
    }

    /// One of an account's own settings (AJ-E1508): offered when an account
    /// is added, not before a switch, where the target keeps its own.
    #[must_use]
    pub const fn is_account_setting(self) -> bool {
        matches!(self, Self::Permissions | Self::StatusLine)
    }
}

/// Where an item applies.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// Every project of this account.
    User,
    /// One project, for this account only (Claude's "local" scope).
    ProjectPrivate,
}

/// One value of an MCP server's environment or headers.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Value {
    /// A plain value that isn't a secret.
    Literal(String),
    /// A reference to an environment variable (`${NAME}`), kept as is.
    Env(String),
    /// A value that looked like a secret: its text was dropped. The target
    /// gets a reference to a variable of the item's name instead, and the
    /// user is told to provide it.
    Secret,
}

/// How an MCP server is reached.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
#[serde(tag = "transport", rename_all = "kebab-case")]
pub enum McpTransport {
    /// A local program.
    Stdio {
        /// The program.
        command: String,
        /// Its arguments.
        args: Vec<String>,
        /// Its environment, by name.
        env: Vec<(String, Value)>,
    },
    /// A streamable HTTP server.
    Http {
        /// Its address.
        url: String,
        /// Its headers, by name.
        headers: Vec<(String, Value)>,
    },
    /// A transport the other tool may not have (`sse`, `ws`).
    Other {
        /// The tool's own name for it.
        kind: String,
        /// Its address, when it has one.
        url: Option<String>,
    },
}

/// An MCP server, as either tool describes it.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
pub struct McpServer {
    /// How it's reached.
    pub transport: McpTransport,
    /// Whether it signs in with OAuth (its token never moves).
    pub oauth: bool,
}

/// What an item is, beyond its name.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Detail {
    /// An MCP server.
    Mcp(McpServer),
    /// One file, or a folder of files, read as text when set up elsewhere.
    Files {
        /// The file, or the folder that holds them.
        path: PathBuf,
    },
    /// A plugin, by its id (`name@marketplace`).
    Plugin {
        /// Its id.
        id: String,
    },
    /// Something the tool's account syncs itself (claude.ai skills and
    /// plugins): the account turns it on, not a copy. What the reader
    /// knows about it rides along (switch-setup-items.md §1).
    FromTheAccount {
        /// The account's own word for where it came from, kept as text
        /// (claude.ai's `manifest.json` `source`: `anthropic`,
        /// `anthropic-example`, `custom`); `None` when nothing says.
        source: Option<String>,
        /// The synced folder, only for an item that may be the user's own
        /// ([`Origin::Account`]): a copy, with consent, needs it. Never
        /// set for a provided item, which is never copied.
        files: Option<PathBuf>,
        /// What it says it needs (its `compatibility` line), for the
        /// plan- and feature-bound wording.
        requires: Vec<String>,
    },
    /// Settings in one of the tool's config files (AJ-E1508): each key and
    /// its value, as the file writes it.
    Setting {
        /// The config file.
        file: PathBuf,
        /// Its format.
        format: ConfigFormat,
        /// Each key (a dotted path in TOML) and its value's text; empty
        /// when it can't be copied (a secret's text is never kept).
        entries: Vec<(String, String)>,
        /// What it holds, in a few words ("2 allowed, 1 denied").
        about: Option<String>,
        /// Whether it can go as it is.
        check: SettingCheck,
    },
}

/// Whether a setting can be copied as it is (AJ-E1508).
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
#[serde(tag = "check", content = "note", rename_all = "kebab-case")]
pub enum SettingCheck {
    /// It can.
    Fine,
    /// It can, but the user should mean it (it turns checks off): offered,
    /// never ticked for them.
    Caution(String),
    /// It can't (a secret is written in it, or its own isn't known).
    Cannot(String),
}

/// A tool config file's format.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigFormat {
    /// JSON (comments and trailing commas allowed, as tools read it).
    Json,
    /// TOML.
    Toml,
}

/// One thing an account has set up.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
pub struct Item {
    /// The tool it belongs to (its id, as the registry names it): whether
    /// another tool can set it up depends on where it came from.
    pub tool: String,
    /// Its kind.
    pub kind: ItemKind,
    /// Its name, as the tool shows it.
    pub name: String,
    /// Where it applies.
    pub scope: Scope,
    /// What it is.
    pub detail: Detail,
    /// Where it came from, as the tool's files say.
    pub origin: Origin,
}

/// Where an item came from, as the source tool's files say
/// (switch-setup-items.md, "Item classes").
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, serde::Serialize)]
#[serde(tag = "origin", rename_all = "kebab-case")]
pub enum Origin {
    /// The user's own files on this computer: set up with consent.
    #[default]
    Own,
    /// Synced from the tool's account, and whose it is can't be told
    /// here (claude.ai's `custom`): the user's, their organization's or
    /// shared with them. Never called "yours".
    Account,
    /// Provided by the tool's maker with the account or the app: never
    /// copied, only turned on for the other account.
    Provided {
        /// Who provides it, in words (`Anthropic`, `OpenAI`).
        by: String,
    },
    /// From the account, and nothing says where from.
    Unknown,
}

/// What has to be true before an item can be turned on, each said in a
/// sentence of its own.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize)]
#[serde(tag = "need", content = "text", rename_all = "kebab-case")]
pub enum Need {
    /// A plan that includes it ("Needs a Pro or Max plan.").
    Plan(String),
    /// Someone who runs the organization has to allow it.
    Admin(String),
    /// Another product or setting has to be on first.
    Feature(String),
}

impl Need {
    /// The sentence.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Plan(text) | Self::Admin(text) | Self::Feature(text) => text,
        }
    }
}

/// How the user finishes an item themselves, in the target tool's own
/// words (AJ-E1543: never a bare status): turning on what comes with the
/// other account, or giving a server the secret that is never copied.
/// The steps, in order, in the app's words. Steps may name the account
/// they're for as `{account}`; the app fills in its name. Examples carry
/// placeholders (`<your GITHUB_TOKEN>`), never a value.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, serde::Serialize)]
pub struct GetIt {
    /// Short steps, in order.
    pub steps: Vec<String>,
    /// The one command the steps say to run, exactly as it's run.
    pub command: Option<String>,
    /// The page that does it (https only), opened in the browser.
    pub open_url: Option<String>,
    /// What has to be true first.
    pub needs: Vec<Need>,
    /// What happens after the last step.
    pub then: Option<String>,
}

impl GetIt {
    /// The same steps for the account named `account`.
    #[must_use]
    pub fn for_account(&self, account: &str) -> Self {
        let fill = |text: &str| text.replace("{account}", account);
        Self {
            steps: self.steps.iter().map(|step| fill(step)).collect(),
            command: self.command.clone(),
            open_url: self.open_url.clone(),
            needs: self
                .needs
                .iter()
                .map(|need| match need {
                    Need::Plan(text) => Need::Plan(fill(text)),
                    Need::Admin(text) => Need::Admin(fill(text)),
                    Need::Feature(text) => Need::Feature(fill(text)),
                })
                .collect(),
            then: self.then.as_deref().map(fill),
        }
    }
}

/// Whether a tool can set up an item that came from elsewhere.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(tag = "verdict", content = "note", rename_all = "kebab-case")]
pub enum SetUp {
    /// The same thing, copied.
    Copy,
    /// Translated into this tool's form; the note says what changes.
    Translate(String),
    /// It can be set up, but the user has to do one thing first, or after
    /// (a login, a value to fill in); the note says what.
    NeedsYou(String),
    /// It can't be set up here; the note says why.
    Impossible(String),
    /// The other account gets it by turning it on itself (a skill that
    /// comes with the account): how, step by step. Never set up by AI
    /// Juggler, unless `can_copy` says the user may copy it instead.
    TurnOn {
        /// How to get it.
        get_it: GetIt,
        /// Whether it may be copied instead, on the user's say-so: only
        /// for what may be the user's own ([`Origin::Account`]), never
        /// for a provided item.
        can_copy: bool,
    },
}

/// What setting up an item did, so it can be undone from the same place.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Applied {
    /// What was done, in a sentence.
    pub summary: String,
    /// How to undo it.
    pub undo: Undo,
    /// What the user should know about it ("left out: .env"), when
    /// anything.
    #[serde(default)]
    pub note: Option<String>,
}

/// How to undo one set-up.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "how", rename_all = "kebab-case")]
pub enum Undo {
    /// Remove the files and folders that were created (only if they're
    /// still as they were made: `sha256` of each file).
    RemoveCreated {
        /// Each created file and its hash.
        files: Vec<(PathBuf, String)>,
        /// Folders created, removed when empty.
        dirs: Vec<PathBuf>,
    },
    /// Remove a block of text appended to a file (if it's still there,
    /// exactly).
    RemoveAppended {
        /// The file.
        file: PathBuf,
        /// The exact text appended.
        text: String,
        /// The copy kept from before the first set-up, removed by the last
        /// undo that needs it (records made before it was named carry
        /// none).
        #[serde(default)]
        backup: Option<PathBuf>,
    },
    /// Run the tool's own command that removes it (`claude mcp remove …`).
    Command {
        /// The program and its arguments.
        argv: Vec<String>,
        /// Environment for it (the account's folder).
        env: Vec<(String, String)>,
        /// Variables to clear for it (so the tool uses its default folder);
        /// empty in records made before it was kept.
        #[serde(default)]
        unset: Vec<String>,
        /// The folder it runs in (a project's own server is removed from
        /// that project); `None` in records made before it was kept.
        #[serde(default)]
        cwd: Option<PathBuf>,
        /// Run only while what was set up is as it was made; `None` in
        /// records made before it was kept.
        #[serde(default)]
        unless_changed: Option<JsonEntry>,
    },
    /// Put back a file that was replaced, from the copy kept before (only
    /// while the file is still exactly what was written). AJ-E1507.
    Restore {
        /// The file that was replaced.
        file: PathBuf,
        /// The copy of it from before, in AI Juggler's data folder.
        backup: PathBuf,
        /// The hash of what was written.
        sha256: String,
    },
    /// Take settings out of a config file again, each only while it still
    /// holds the value that was set (AJ-E1508).
    RemoveSetting {
        /// The config file.
        file: PathBuf,
        /// Its format.
        format: ConfigFormat,
        /// Each key and the fingerprint of the value that was set (its
        /// formatting aside; the value itself is never recorded).
        entries: Vec<(String, String)>,
    },
    /// D-51: take a project's copied notes out again
    /// ([`crate::memory::undo`]): the index lines added, each while it's
    /// there, and the notes made, all only while each is as it was made.
    RemoveNotes {
        /// Each note made and its hash.
        files: Vec<(PathBuf, String)>,
        /// Folders made, removed when empty.
        dirs: Vec<PathBuf>,
        /// The index lines were added to, when any were.
        index: Option<PathBuf>,
        /// The exact lines added to it.
        lines: Vec<String>,
        /// Whether the index was made by the copy (removed again when
        /// nothing else is in it).
        index_created: bool,
    },
}

/// Which file `path` is, following links: its device and inode, or its
/// canonical path (without case) where there are none. `None` when there's
/// nothing there. Two names of one file (a link, a hard link, a
/// case-insensitive disk) give the same identity.
#[must_use]
pub fn file_identity(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        Some(format!("{}:{}", meta.dev(), meta.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        std::fs::canonicalize(path)
            .ok()
            .map(|at| at.to_string_lossy().to_lowercase())
    }
}

/// Whether a configuration value looks like a secret, by its key or its
/// text. Used while a tool's config is read, so a secret's text never
/// reaches an [`Item`].
#[must_use]
pub fn looks_secret(key: &str, value: &str) -> bool {
    const KEY_WORDS: [&str; 13] = [
        "key",
        "token",
        "secret",
        "password",
        "passwd",
        "pwd",
        "auth",
        "bearer",
        "cookie",
        "session",
        "credential",
        "private",
        "dsn",
    ];
    let key = key.to_ascii_lowercase();
    if KEY_WORDS.iter().any(|word| key.contains(word)) {
        return true;
    }
    carries_secret(value) || looks_token(value)
}

/// Whether one word reads as a token by its shape: a known key prefix, or
/// a long run of letters and digits with no spaces.
fn looks_token(value: &str) -> bool {
    const PREFIXES: [&str; 10] = [
        "sk-",
        "ghp_",
        "gho_",
        "github_pat_",
        "glpat-",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "AIza",
        "eyJ",
    ];
    let value = value.trim();
    let value = value.strip_prefix("Bearer ").unwrap_or(value);
    if PREFIXES.iter().any(|prefix| value.starts_with(prefix)) {
        return true;
    }
    value.len() >= 32
        && !value.contains(char::is_whitespace)
        && !value.contains('/')
        && value.chars().filter(char::is_ascii_alphanumeric).count() * 10 >= value.len() * 9
}

/// Replaces a tool's own file with `text`, atomically, as the file it is:
/// through a symlink to the file it points at (a dotfiles-managed config
/// stays a link), and with the file's own permissions (a 0600 config
/// stays 0600). Review of #132 (M2).
///
/// # Errors
///
/// The write's, the permissions' or the rename's error; the file is
/// unchanged then.
pub fn rewrite_in_place(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let target = if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        std::fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let staged = target.with_file_name(format!(".{name}.ai-juggler-{}", std::process::id()));
    // Made with the file's permissions from the start: a 0600 config is
    // never readable by others, even for a moment (re-review of #132).
    let permissions = std::fs::metadata(&target)?.permissions();
    let written = create_with(&staged, &permissions)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(text.as_bytes())
        })
        .and_then(|()| std::fs::set_permissions(&staged, permissions))
        .and_then(|()| std::fs::rename(&staged, &target));
    if written.is_err() {
        let _ = std::fs::remove_file(&staged);
    }
    written
}

/// A new file, created with `permissions` where the platform allows it.
fn create_with(
    path: &std::path::Path,
    permissions: &std::fs::Permissions,
) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        options.mode(permissions.mode());
    }
    #[cfg(not(unix))]
    let _ = permissions;
    options.open(path)
}

/// One entry of a JSON file, by its path of keys, as it was when set up:
/// an undo checks it before removing anything (review of #132).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonEntry {
    /// The file.
    pub file: PathBuf,
    /// The keys down to the entry (`["mcpServers", "github"]`).
    pub keys: Vec<String>,
    /// [`JsonEntry::fingerprint`] of the entry when it was made.
    pub sha256: String,
}

impl JsonEntry {
    /// The entry's fingerprint in `file` now, `None` when it's gone or the
    /// file doesn't read.
    #[must_use]
    pub fn fingerprint(file: &std::path::Path, keys: &[String]) -> Option<String> {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
        let entry = keys.iter().try_fold(&value, |at, key| at.get(key))?;
        Some(crate::handover::sha256_hex(entry.to_string().as_bytes()))
    }

    /// Whether the entry is still exactly as it was made.
    #[must_use]
    pub fn unchanged(&self) -> bool {
        Self::fingerprint(&self.file, &self.keys).as_deref() == Some(self.sha256.as_str())
    }
}

/// Whether a `Name: value` header holds a secret.
fn header_is_secret(header: &str) -> bool {
    header
        .split_once(':')
        .is_some_and(|(name, value)| looks_secret(name.trim(), value.trim()))
}

/// Words in a name that mark its value as a secret.
const SECRET_NAME_WORDS: [&str; 9] = [
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "pwd",
    "auth",
    "sig",
    "credential",
];

/// Review of #132 (H3): a secret inside a longer text. Matches a URL with
/// a password (`postgres://user:pass@host`), and a `name=value` pair
/// whose name reads as secret: a query string (`?access_token=…`), a
/// connection string (`Password=…`) or a flag (`--api-key=…`).
#[must_use]
pub fn carries_secret(text: &str) -> bool {
    if let Some((_, rest)) = text.split_once("://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if let Some((userinfo, _)) = authority.rsplit_once('@')
            && userinfo.contains(':')
        {
            return true;
        }
        // A token as a path segment (`…/api/mcp/s/<token>/mcp`), by its
        // shape (re-review of #132).
        let path = rest.split(['?', '#']).next().unwrap_or_default();
        if path.split('/').skip(1).any(looks_token) {
            return true;
        }
    }
    text.split(['?', '&', ';', ' ', ','])
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, value)| {
            let name = name.trim_start_matches('-').to_ascii_lowercase();
            !value.trim().is_empty() && SECRET_NAME_WORDS.iter().any(|word| name.contains(word))
        })
}

/// Review of AJ-E1508 (H1, and its re-review): whether free text, a status
/// line's command or a permission rule, holds a secret anywhere in it,
/// read word by word:
/// - a word shaped like a token, or like a base64 secret (`/` and `+` in
///   it), on its own or as the value of `name=value` or `name:value`;
/// - a URL with a password or a token in its path ([`carries_secret`]);
/// - `Bearer x` and `Basic x`;
/// - a header whose name names a secret, and its value (`x-api-key: …`);
/// - a flag whose name names a secret, and its value (`--api-key abc`,
///   `--password=abc`), and `-u user:password`.
///
/// A `$VARIABLE` is a reference, not a secret. Names match by their parts,
/// so `--author` isn't `auth`.
#[must_use]
pub fn holds_secret(text: &str) -> bool {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || "'\"`()[]{},;".contains(c))
        .filter(|word| !word.is_empty())
        .collect();
    let value = |word: &str| !word.is_empty() && !word.starts_with('$');
    let shaped = |word: &str| looks_token(word) || looks_base64_secret(word);
    for (at, word) in words.iter().enumerate() {
        let next = words.get(at + 1).copied().filter(|next| value(next));
        if word.contains("://") && carries_secret(word) {
            return true;
        }
        if word
            .split(['=', ':'])
            .any(|part| value(part) && shaped(part))
        {
            return true;
        }
        if (word.eq_ignore_ascii_case("bearer") || word.eq_ignore_ascii_case("basic"))
            && next.is_some()
        {
            return true;
        }
        // A header name ends in `:`; a flag starts with `-`.
        // Its value, past a scheme (`Authorization: Bearer $TOKEN` holds a
        // reference, not a secret).
        if let Some(name) = word.strip_suffix(':')
            && secret_name(name)
        {
            let scheme = next.is_some_and(|next| {
                next.eq_ignore_ascii_case("bearer") || next.eq_ignore_ascii_case("basic")
            });
            let held = if scheme {
                words.get(at + 2).copied().filter(|after| value(after))
            } else {
                next
            };
            if held.is_some() {
                return true;
            }
        }
        if let Some(flag) = word.strip_prefix('-') {
            let flag = flag.trim_start_matches('-');
            match flag.split_once('=') {
                Some((name, rest)) if secret_name(name) && value(rest) => return true,
                None if secret_name(flag) && next.is_some_and(|next| !next.starts_with('-')) => {
                    return true;
                }
                _ => {}
            }
            if matches!(flag, "u" | "user")
                && next
                    .and_then(|next| next.split_once(':'))
                    .is_some_and(|(user, password)| !user.is_empty() && value(password))
            {
                return true;
            }
        }
        // `name=value` outside a flag: an environment variable's.
        if let Some((name, rest)) = word.split_once('=')
            && !name.starts_with('-')
            && secret_name(name)
            && value(rest)
        {
            return true;
        }
    }
    false
}

/// Whether a name (a flag's, a header's, a variable's) names a secret: one
/// of its parts, split at `-`, `_` and `.`, is a word for one.
fn secret_name(name: &str) -> bool {
    const PARTS: [&str; 13] = [
        "key",
        "apikey",
        "token",
        "secret",
        "password",
        "passwd",
        "pwd",
        "auth",
        "authorization",
        "credential",
        "credentials",
        "bearer",
        "pat",
    ];
    name.to_ascii_lowercase()
        .split(['-', '_', '.'])
        .any(|part| PARTS.contains(&part))
}

/// Whether one word reads as a base64 secret (an AWS secret key holds `/`
/// and `+`, which [`looks_token`] doesn't take): long, of base64's letters
/// only, mixed case and digits, and not a path.
fn looks_base64_secret(word: &str) -> bool {
    word.len() >= 32
        && !word.starts_with('/')
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='))
        && word.chars().any(|c| c.is_ascii_uppercase())
        && word.chars().any(|c| c.is_ascii_lowercase())
        && word.chars().any(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod holds_secret_tests {
    use super::holds_secret;

    #[test]
    fn secrets_in_commands_and_rules_are_found() {
        for text in [
            "GITHUB_TOKEN=ghp_abcdef123456 ~/bin/line",
            "Bash(curl -H 'Authorization: Bearer abc')",
            "Bash(curl -H 'Authorization: Basic dXNlcjpwYXNz')",
            "Bash(curl -H 'x-api-key: 0d5c1a6e-1f7b-4c1e-9a0a-1b2c3d4e5f60')",
            "mytool --api-key abc123 --fast",
            "mytool --password=hunter2",
            "curl -u alice:hunter2 https://example.com",
            "aws s3 cp --secret wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY x",
            "export AWS_SECRET=wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "psql postgres://me:hunter2@db/app",
        ] {
            assert!(holds_secret(text), "{text}");
        }
    }

    #[test]
    fn ordinary_rules_and_references_are_not_secrets() {
        for text in [
            "Bash(npm test)",
            "Bash(git commit --author=me@example.com:*)",
            "Bash(curl -H \"Authorization: Bearer $TOKEN\" https://api.example.com)",
            "Read(/Users/you/Dev/project/node_modules/some-package/lib/index.js)",
            "mytool --api-key $API_KEY",
            "~/bin/statusline --theme dark",
            "Bash(git log --oneline -5)",
        ] {
            assert!(!holds_secret(text), "{text}");
        }
    }
}

impl McpServer {
    /// The verdict every tool gives a server with a secret written into
    /// its arguments or address: it can't be set up elsewhere, so the
    /// secret never leaves this account (not in the target's config, a
    /// command line, or an undo record).
    #[must_use]
    pub fn inline_secret_verdict(&self) -> Option<SetUp> {
        self.secret_inline().then(|| {
            SetUp::Impossible(
                "a secret is written into its arguments or address, and secrets are never \
                 copied: set it up on the other account yourself"
                    .to_owned(),
            )
        })
    }

    /// Whether a secret is written into its arguments or its address,
    /// where no variable can stand in for it in every tool: such a server
    /// can't be set up elsewhere (its secret would travel with it).
    #[must_use]
    pub fn secret_inline(&self) -> bool {
        match &self.transport {
            McpTransport::Stdio { args, .. } => args.iter().enumerate().any(|(at, arg)| {
                // `--api-key sk-…`: a secret-named flag takes the next word.
                let flag = arg.starts_with('-')
                    && !arg.contains('=')
                    && SECRET_NAME_WORDS
                        .iter()
                        .any(|word| arg.to_ascii_lowercase().contains(word))
                    && args.get(at + 1).is_some_and(|next| !next.starts_with('-'));
                // `--header "Authorization: Bearer …"` (mcp-remote and the
                // like): the next word is a header, checked as one.
                let header = matches!(arg.as_str(), "--header" | "-H")
                    && args.get(at + 1).is_some_and(|next| header_is_secret(next));
                let inline_header = arg.strip_prefix("--header=").is_some_and(header_is_secret);
                flag || header || inline_header || looks_secret("", arg)
            }),
            McpTransport::Http { url, .. } => carries_secret(url),
            McpTransport::Other { url, .. } => url.as_deref().is_some_and(carries_secret),
        }
    }
}

/// A value from a tool's config, as [`Value`]: a whole `${NAME}` reference
/// is kept as one, a secret-looking literal is dropped.
#[must_use]
pub fn value_of(key: &str, text: &str) -> Value {
    let trimmed = text.trim();
    if let Some(name) = trimmed
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    {
        return Value::Env(name.to_owned());
    }
    if looks_secret(key, trimmed) {
        Value::Secret
    } else {
        Value::Literal(text.to_owned())
    }
}

/// The environment variable a dropped secret becomes on the target:
/// the key itself when it's a plain name (`GITHUB_TOKEN`), else a name
/// made from the server and the key.
#[must_use]
pub fn secret_variable(server: &str, key: &str) -> String {
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    if plain {
        return key.to_owned();
    }
    let mut name: String = format!("{server}_{key}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    while name.contains("__") {
        name = name.replace("__", "_");
    }
    name.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_known_by_their_key_or_their_shape() {
        assert!(looks_secret("GITHUB_TOKEN", "anything"));
        assert!(looks_secret("Authorization", "Bearer abc"));
        assert!(looks_secret("X", "sk-ant-api03-abcdef"));
        assert!(looks_secret(
            "X",
            "ghp_0123456789abcdefABCDEF0123456789abcd"
        ));
        assert!(looks_secret("X", "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8"));
        assert!(!looks_secret("LOG_LEVEL", "debug"));
        assert!(!looks_secret("ROOT", "/Users/you/Dev/shop"));
        assert!(!looks_secret("URL", "https://example.com/mcp"));
    }

    #[test]
    fn a_reference_stays_a_reference_and_a_secret_is_dropped() {
        assert_eq!(
            value_of("GITHUB_TOKEN", "${GITHUB_TOKEN}"),
            Value::Env("GITHUB_TOKEN".into())
        );
        assert_eq!(value_of("GITHUB_TOKEN", "ghp_live_value"), Value::Secret);
        assert_eq!(
            value_of("LOG_LEVEL", "debug"),
            Value::Literal("debug".into())
        );
        // A partial reference is text, judged like any other.
        assert_eq!(
            value_of("PREFIX", "x-${A}"),
            Value::Literal("x-${A}".into())
        );
    }

    #[test]
    fn a_dropped_secret_gets_a_variable_name() {
        assert_eq!(secret_variable("github", "GITHUB_TOKEN"), "GITHUB_TOKEN");
        assert_eq!(
            secret_variable("linear-mcp", "Authorization"),
            "LINEAR_MCP_AUTHORIZATION"
        );
    }

    fn stdio(args: &[&str]) -> McpServer {
        McpServer {
            transport: McpTransport::Stdio {
                command: "server".into(),
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                env: Vec::new(),
            },
            oauth: false,
        }
    }

    #[test]
    fn a_secret_inside_a_longer_text_is_found() {
        // Review of #132 (H3): these passed as plain values.
        assert!(looks_secret(
            "DATABASE_URL",
            "postgres://app:hunter2@db.local/app"
        ));
        assert!(carries_secret(
            "https://api.example.com/mcp?access_token=abc123"
        ));
        assert!(carries_secret("Server=db;User Id=app;Password=hunter2;"));
        assert!(carries_secret("--api-key=abc123"));
        // Not secrets: a URL with a user and no password, a plain query.
        assert!(!carries_secret("https://git@github.com/org/repo"));
        assert!(!carries_secret(
            "https://example.com/mcp?region=eu&limit=10"
        ));
        assert!(!looks_secret("LOG_LEVEL", "info"));
    }

    #[test]
    fn a_server_with_a_secret_in_its_arguments_or_address_cant_move() {
        assert!(stdio(&["--api-key", "abc123"]).secret_inline());
        assert!(stdio(&["--token=abc123"]).secret_inline());
        assert!(stdio(&["serve", "sk-live-0123456789abcdef"]).secret_inline());
        assert!(!stdio(&["--port", "8080", "--verbose"]).secret_inline());
        assert!(!stdio(&["--api-key-file", "--verbose"]).secret_inline());
        let http = |url: &str| McpServer {
            transport: McpTransport::Http {
                url: url.into(),
                headers: Vec::new(),
            },
            oauth: false,
        };
        assert!(http("https://u:p@mcp.example.com").secret_inline());
        // Re-review of #132: a header passed as an argument, and a token
        // as a URL path segment.
        assert!(
            stdio(&[
                "mcp-remote",
                "https://x.example/mcp",
                "--header",
                "Authorization: Bearer abc123"
            ])
            .secret_inline()
        );
        assert!(stdio(&["--header=X-Api-Key: abc123"]).secret_inline());
        assert!(!stdio(&["--header", "Accept: text/event-stream"]).secret_inline());
        assert!(
            http("https://mcp.zapier.com/api/mcp/s/aB3dE5fG7hJ9kL1mN3pQ5rS7tU9vW1xY3zA5/mcp")
                .secret_inline()
        );
        assert!(
            stdio(&[
                "mcp-remote",
                "https://mcp.zapier.com/api/mcp/s/aB3dE5fG7hJ9kL1mN3pQ5rS7tU9vW1xY3zA5/mcp"
            ])
            .secret_inline()
        );
        assert!(!http("https://mcp.linear.app/v1/sse").secret_inline());
        assert!(!http("https://mcp.example.com/sse").secret_inline());
        assert!(matches!(
            stdio(&["--api-key", "abc123"]).inline_secret_verdict(),
            Some(SetUp::Impossible(_))
        ));
    }
}
