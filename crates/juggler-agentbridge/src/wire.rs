//! What the bridge and the app say to each other over the socket: one
//! JSON line each way per call. Shared by both ends, so they can't drift.

use serde::{Deserialize, Serialize};

/// The program's name, in the app bundle's `Contents/MacOS` beside AI
/// Juggler (and in the process list, where the ledger knows it as an MCP
/// server owned by its session).
pub const PROGRAM: &str = "juggler-agent";

/// The session's key, in the environment of the session AI Juggler
/// starts (never in its arguments, never in a file).
pub const TOKEN_ENV: &str = "JUGGLER_AGENT_TOKEN";

/// The app's socket for this edition, in the same environment.
pub const SOCKET_ENV: &str = "JUGGLER_AGENT_SOCKET";

/// Where this program is, for a tool without MCP to call from its shell.
pub const PROGRAM_ENV: &str = "JUGGLER_AGENT";

/// AJ-E2306: the session's report key, for its tool's hooks only (it can
/// report the session's state and nothing else; never the agent tools'
/// key). In the environment of the session AI Juggler starts.
pub const REPORT_TOKEN_ENV: &str = "JUGGLER_REPORT_TOKEN";

/// The hook events `juggler-agent hook` reports (Claude Code's names).
pub const HOOK_EVENTS: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Notification",
    "Stop",
    "StopFailure",
    "SessionEnd",
];

/// The longest summary a report carries.
pub const SUMMARY_MAX: usize = 300;

/// The MCP server's name in a tool's settings (`mcp__juggler__whoami`).
pub const SERVER_NAME: &str = "juggler";

/// AI Juggler's own tools, all of them: the ones a launch pre-allows
/// (D-30) and nothing else.
pub const TOOLS: [&str; 3] = ["whoami", "pool_roster", "set_status"];

/// The resource with the caller's pool's context cards (AJ-E1703).
pub const CARDS_URI: &str = "juggler://pool/cards";

/// The longest status line an agent may set (a line, not a note).
pub const STATUS_MAX: usize = 140;

/// The longest request the app reads.
pub const MAX_REQUEST: usize = 8 * 1024;

/// The longest answer the bridge reads.
pub const MAX_RESPONSE: usize = 512 * 1024;

/// The wire's version: a request of another is refused.
pub const VERSION: u32 = 1;

/// What an agent asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Call {
    /// Who am I: my session, my pool, my role and what the pool allows.
    Whoami,
    /// Who else is in my pool.
    PoolRoster,
    /// Set my own status line (my card's "says").
    SetStatus,
    /// My pool's context cards.
    Cards,
}

impl Call {
    /// The call a tool's name names, when it names one.
    #[must_use]
    pub fn of_tool(name: &str) -> Option<Self> {
        match name {
            "whoami" => Some(Self::Whoami),
            "pool_roster" => Some(Self::PoolRoster),
            "set_status" => Some(Self::SetStatus),
            _ => None,
        }
    }
}

/// One call, bridge to app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// [`VERSION`].
    pub v: u32,
    /// The session's key, from the bridge's own environment.
    pub token: String,
    /// What it asks for.
    pub call: Call,
    /// `set_status`'s line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// AJ-E2306: what a tool's hook reports about its own session, from the
/// fields the tool hands its hooks (never anything else).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookReport {
    /// One of [`HOOK_EVENTS`].
    pub event: String,
    /// The tool's own id for its session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_session: Option<String>,
    /// Its transcript's path, as the tool gave it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    /// The tool it uses or asks for (`Bash`, `AskUserQuestion`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// What it asks, in one bounded line (a command, a file, a question).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// A notification's type word (`permission_prompt`, `idle_prompt`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Why it stopped or ended, as a word (`rate_limit`, `prompt_input_exit`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One hook report, bridge to app (a shape of its own: never an agent's
/// call, and its key is the report key).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// [`VERSION`].
    pub v: u32,
    /// The session's report key, from the hook's own environment.
    pub token: String,
    /// What happened.
    pub hook: HookReport,
}

/// Why a call was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Above the pool's level: the user decides (D-28). Until a pending
    /// ask exists (E18), it's a refusal that says so.
    Ask,
    /// Not at all: not in the pool, or the pool is gone.
    Deny,
    /// The user switched the agent tools off for this project or profile.
    Off,
    /// The key isn't a current session's (it ended, moved on, or AI
    /// Juggler restarted and the session wasn't continued).
    Unknown,
    /// Too many calls too fast, or a request too big.
    Limit,
    /// A request the app can't read.
    Bad,
}

/// The app's answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// The call's result (`None` when refused).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Why not, when refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<Refusal>,
    /// The refusal in words, for the agent to pass on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Response {
    /// A result.
    #[must_use]
    pub fn ok(result: serde_json::Value) -> Self {
        Self {
            result: Some(result),
            refused: None,
            reason: None,
        }
    }

    /// A refusal, in words.
    #[must_use]
    pub fn refused(refusal: Refusal, reason: impl Into<String>) -> Self {
        Self {
            result: None,
            refused: Some(refusal),
            reason: Some(reason.into()),
        }
    }
}

/// A status line as it is kept: one line, no control characters, at most
/// [`STATUS_MAX`] characters (cut on a character, never mid-way).
#[must_use]
pub fn clean_status(text: &str) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    line.chars().take(STATUS_MAX).collect()
}

#[cfg(test)]
mod tests {
    use super::{Call, Request, Response, STATUS_MAX, clean_status};

    #[test]
    fn a_request_round_trips_and_refuses_strangers() {
        let request = Request {
            v: 1,
            token: "t".into(),
            call: Call::SetStatus,
            text: Some("Fixing checkout".into()),
        };
        let line = serde_json::to_string(&request).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), request);
        assert!(
            serde_json::from_str::<Request>(
                r#"{"v":1,"token":"t","call":"whoami","pool":"other"}"#
            )
            .is_err(),
            "no field picks another pool or identity"
        );
        assert!(
            serde_json::from_str::<Request>(r#"{"v":1,"token":"t","call":"set_level"}"#).is_err()
        );
        let response = Response::refused(super::Refusal::Ask, "why");
        let line = serde_json::to_string(&response).unwrap();
        assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), response);
    }

    #[test]
    fn only_the_three_tools_name_calls() {
        assert_eq!(Call::of_tool("whoami"), Some(Call::Whoami));
        assert_eq!(Call::of_tool("pool_roster"), Some(Call::PoolRoster));
        assert_eq!(Call::of_tool("set_status"), Some(Call::SetStatus));
        assert_eq!(Call::of_tool("cards"), None, "the cards are a resource");
        assert_eq!(Call::of_tool("set_level"), None);
    }

    #[test]
    fn a_status_is_one_short_line() {
        assert_eq!(
            clean_status("  Fixing\ncheckout\t now "),
            "Fixing checkout now"
        );
        assert_eq!(clean_status("\u{1b}[31mred"), "[31mred");
        let long = "é".repeat(STATUS_MAX + 9);
        assert_eq!(clean_status(&long).chars().count(), STATUS_MAX);
    }
}
