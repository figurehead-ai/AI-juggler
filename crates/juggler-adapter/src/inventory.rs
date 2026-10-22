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
    /// Instructions the tool reads for every project (`CLAUDE.md`,
    /// `AGENTS.md`, `rules/`).
    Rules,
    /// An installed plugin.
    Plugin,
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
            Self::Plugin => "plugin",
        }
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
    /// connectors): it can't be set up by copying.
    FromTheAccount,
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
}

/// What setting up an item did, so it can be undone from the same place.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Applied {
    /// What was done, in a sentence.
    pub summary: String,
    /// How to undo it.
    pub undo: Undo,
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
    },
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
    let key = key.to_ascii_lowercase();
    if KEY_WORDS.iter().any(|word| key.contains(word)) {
        return true;
    }
    let value = value.trim();
    let value = value.strip_prefix("Bearer ").unwrap_or(value);
    if PREFIXES.iter().any(|prefix| value.starts_with(prefix)) {
        return true;
    }
    // A long run of letters and digits with no spaces reads as a token.
    value.len() >= 32
        && !value.contains(char::is_whitespace)
        && !value.contains('/')
        && value.chars().filter(char::is_ascii_alphanumeric).count() * 10 >= value.len() * 9
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
}
