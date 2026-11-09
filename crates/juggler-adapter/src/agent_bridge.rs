//! AJ-E1704 (D-30, ADR-0020): how a launch gets AI Juggler's agent tools.
//!
//! The app says what to add ([`BridgeLaunch`]: the program, the server's
//! name, its tools, the names of the variables it reads, and a folder of
//! AI Juggler's own for this launch's files). Each adapter says how its
//! tool takes it ([`crate::ToolConfig::agent_bridge`]): launch
//! options, launch variables, and, where the tool needs one, a file for
//! that folder. Never an edit to the user's own setup: a tool that reads
//! MCP servers only from its home (Grok Build) keeps a standing entry in
//! a home AI Juggler made for the Account, and in no other
//! ([`HomeEntry`]); a launch it can't give the tools says why.
//!
//! What an injection may hold is checked for every adapter by the
//! conformance kit: no secret (the key's value is never given to the
//! adapter, only its variable's name), files only in the launch's folder,
//! a home entry only in a managed home, the original arguments kept in
//! order, and only AI Juggler's own tools named.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::launch::LaunchSpec;

/// Why a tool without [`crate::Capabilities::AGENT_BRIDGE`] gets no
/// agent tools: words for the UI and the debug log, naming no tool.
pub const NO_SERVER_AT_LAUNCH: &str = "Its tool can't add AI Juggler's agent tools at launch: it works through its shell, and can't read the pool, post or take a task.";

/// What AI Juggler adds to one launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeLaunch {
    /// The bridge program (`juggler-agent`), by its full path.
    pub program: PathBuf,
    /// The MCP server's name in the tool's settings.
    pub server: String,
    /// AI Juggler's own tools: all of them, and the only ones a launch
    /// pre-allows (D-30).
    pub tools: Vec<String>,
    /// The variables the bridge reads (its key and the app's socket), by
    /// name only: their values are in the launch's environment.
    pub env_names: Vec<String>,
    /// This launch's folder for files, in AI Juggler's own overlay folder
    /// (ADR-0018). The app writes the files there.
    pub dir: PathBuf,
    /// The Account's folder (its `state_dir`): where a tool that reads
    /// MCP servers only from its home keeps the standing entry, when AI
    /// Juggler made that home ([`HomeEntry`]). Never written otherwise.
    pub state_dir: PathBuf,
}

/// The bridge program's argument that serves it as an MCP server.
pub const SERVE_ARG: &str = "--mcp";

impl BridgeLaunch {
    /// The bridge's variables, each with a reference to it in the tool's
    /// own syntax (`refer`: `${NAME}`, `{env:NAME}`, …), never a value.
    #[must_use]
    pub fn env_refs(&self, refer: impl Fn(&str) -> String) -> Vec<(String, String)> {
        self.env_names
            .iter()
            .map(|name| (name.clone(), refer(name)))
            .collect()
    }

    /// [`Self::env_refs`] as a JSON object, for a tool's JSON settings.
    #[must_use]
    pub fn env_refs_json(
        &self,
        refer: impl Fn(&str) -> String,
    ) -> serde_json::Map<String, serde_json::Value> {
        self.env_refs(refer)
            .into_iter()
            .map(|(name, reference)| (name, serde_json::Value::String(reference)))
            .collect()
    }

    /// AI Juggler's tools by the tool's own names for them (`name` gets
    /// the server and the tool): the only ones a launch pre-allows.
    #[must_use]
    pub fn tool_names(&self, name: impl Fn(&str, &str) -> String) -> Vec<String> {
        self.tools
            .iter()
            .map(|tool| name(&self.server, tool))
            .collect()
    }
}

/// A file a tool reads its server from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeFile {
    /// Its name in [`BridgeLaunch::dir`] (a plain name, no folders).
    pub name: String,
    /// What it holds.
    pub contents: String,
}

/// The standing server entry an adapter keeps in a home AI Juggler made
/// for the Account (its marker says so), for a tool that reads MCP
/// servers only from its home. It names the bridge's variables, never
/// their values, and works harmlessly in a launch without the tools (the
/// bridge then has no key: each call says why, nothing reaches the app).
/// Kept by the adapter itself, as part of
/// [`crate::ToolConfig::agent_bridge`]; reported here so what was kept
/// can be checked and logged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeEntry {
    /// The home's settings file that holds it.
    pub file: PathBuf,
    /// The entry, as written.
    pub entry: String,
}

/// How one tool takes the bridge: arguments to insert, variables to set
/// and files to write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeInjection {
    /// Where in the launch's arguments [`Self::args`] go.
    pub at: usize,
    /// The arguments.
    pub args: Vec<String>,
    /// Launch variables to set (a tool's inline settings, merged with
    /// what the launch already held), never a secret: values only name
    /// the bridge's variables.
    pub env: Vec<(String, String)>,
    /// Files for [`BridgeLaunch::dir`].
    pub files: Vec<BridgeFile>,
    /// The standing entry kept in the Account's own managed home, if the
    /// tool needs one.
    pub home: Option<HomeEntry>,
}

impl BridgeInjection {
    /// The launch's arguments with the injection's in place.
    #[must_use]
    pub fn applied(&self, args: &[String]) -> Vec<String> {
        let at = self.at.min(args.len());
        let mut out = args[..at].to_vec();
        out.extend(self.args.iter().cloned());
        out.extend(args[at..].iter().cloned());
        out
    }

    /// Puts the injection into `spec`: its arguments in place and its
    /// variables set.
    pub fn apply(&self, spec: &mut LaunchSpec) {
        spec.args = self.applied(&spec.args);
        for (key, value) in &self.env {
            spec.env.insert(key.clone(), value.clone());
        }
    }
}

/// A string as TOML writes it (basic string, escaped): for options a
/// tool parses as TOML (`-c key=value`).
#[must_use]
pub fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A list of strings as TOML writes it.
#[must_use]
pub fn toml_list(values: &[String]) -> String {
    let items: Vec<String> = values.iter().map(|value| toml_string(value)).collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::{BridgeInjection, toml_list, toml_string};

    #[test]
    fn an_injection_goes_in_place_and_keeps_the_rest_in_order() {
        let args: Vec<String> = ["a", "--", "prompt"].map(String::from).to_vec();
        let injection = BridgeInjection {
            at: 1,
            args: vec!["-c".into(), "x=1".into()],
            ..BridgeInjection::default()
        };
        assert_eq!(injection.applied(&args), ["a", "-c", "x=1", "--", "prompt"]);
        let past = BridgeInjection {
            at: 99,
            ..injection
        };
        assert_eq!(past.applied(&args).last().map(String::as_str), Some("x=1"));
    }

    #[test]
    fn toml_strings_are_escaped() {
        assert_eq!(toml_string(r#"/A "B"\C"#), r#""/A \"B\"\\C""#);
        assert_eq!(toml_string("a\u{1}"), "\"a\\u0001\"");
        assert_eq!(toml_list(&["x".into(), "y".into()]), r#"["x","y"]"#);
    }
}
