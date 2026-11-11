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

/// AJ-E3002 R3: a launch AI Juggler starts without its agent tools (they
/// are off for its project or profile, or the launch couldn't get them).
/// A tool that keeps the bridge's server as a standing entry in its home
/// ([`HomeEntry`]) would still offer the server's tools there, and ask
/// before each call: its adapter leaves them out of this launch
/// ([`crate::ToolConfig::without_agent_tools`]). Names only: no program,
/// no key, nothing to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeOff {
    /// The MCP server's name in the tool's settings.
    pub server: String,
    /// AI Juggler's own tools: the ones a launch with them pre-allows.
    pub tools: Vec<String>,
    /// The Account's folder (its `state_dir`): where a standing entry may
    /// be, when AI Juggler made that home.
    pub state_dir: PathBuf,
    /// Why this launch goes without them, in the app's plain words ("the
    /// user switched them off for this project").
    pub why: String,
}

impl BridgeOff {
    /// What a launch without the tools tells its agent: plainly that this
    /// session wasn't started with them and why, so it doesn't try; and,
    /// for a tool that still lists the server (a standing entry it can't
    /// leave out per launch) and answers a call with its own rule string,
    /// what to tell the user instead (the rc32 walk, E3002-R3 step 12:
    /// "Denied by permission policy: deny rule on mcp matching
    /// juggler__*" was all anyone saw).
    #[must_use]
    pub fn words(&self) -> String {
        let why = self.why.trim().trim_end_matches('.');
        let why = if why.is_empty() {
            "they're off for it"
        } else {
            why
        };
        format!(
            "This session wasn't started with AI Juggler's agent tools ({why}). The MCP server \
             \"{server}\" may still be listed, but every call to its tools is refused without \
             asking: don't call them. If the user asks for them, or a call comes back denied, \
             tell the user in plain words: AI Juggler's agent tools are off for this session \
             ({why}); they can turn them on in AI Juggler, under Settings › Agent access, and \
             a session started after that gets them.",
            server = self.server
        )
    }
}

#[cfg(test)]
mod off_tests {
    use super::BridgeOff;

    #[test]
    fn a_launch_without_the_tools_says_why_and_what_to_tell_the_user() {
        let off = BridgeOff {
            server: "juggler".into(),
            tools: vec!["whoami".into()],
            state_dir: std::path::PathBuf::from("/a"),
            why: "the user switched them off for this project".into(),
        };
        let words = off.words();
        assert!(
            words.contains("wasn't started with AI Juggler's agent tools"),
            "{words}"
        );
        assert!(
            words.contains("(the user switched them off for this project)"),
            "{words}"
        );
        assert!(words.contains("tell the user in plain words"), "{words}");
        assert!(words.contains("Settings › Agent access"), "{words}");
        assert!(!words.contains("juggler__"), "no rule string: {words}");
        let unsaid = BridgeOff {
            why: String::new(),
            ..off
        };
        assert!(unsaid.words().contains("(they're off for it)"));
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
