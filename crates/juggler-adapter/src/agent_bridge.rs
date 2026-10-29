//! AJ-E1704 (D-30, ADR-0020): how a launch gets AI Juggler's agent tools.
//!
//! The app says what to add ([`BridgeLaunch`]: the program, the server's
//! name, its tools, the names of the variables it reads, and a folder of
//! AI Juggler's own for this launch's files). Each adapter says how its
//! tool takes it ([`crate::HarnessAdapter::agent_bridge`]): launch
//! options and, where the tool needs one, a file for that folder. Never
//! an edit to the user's own setup.
//!
//! What an injection may hold is checked for every adapter by the
//! conformance kit: no secret (the key's value is never given to the
//! adapter, only its variable's name), files only in the launch's folder,
//! the original arguments kept in order, and only AI Juggler's own tools
//! named.

use std::fmt::Write as _;
use std::path::PathBuf;

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
}

/// A file a tool reads its server from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeFile {
    /// Its name in [`BridgeLaunch::dir`] (a plain name, no folders).
    pub name: String,
    /// What it holds.
    pub contents: String,
}

/// How one tool takes the bridge: arguments to insert and files to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeInjection {
    /// Where in the launch's arguments [`Self::args`] go.
    pub at: usize,
    /// The arguments.
    pub args: Vec<String>,
    /// Files for [`BridgeLaunch::dir`].
    pub files: Vec<BridgeFile>,
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
            files: Vec::new(),
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
