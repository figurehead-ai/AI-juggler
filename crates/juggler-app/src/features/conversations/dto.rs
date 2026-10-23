//! The conversations feature's wire types.

/// One conversation on one of the user's accounts: started by AI Juggler
/// or not (AJ-E1529).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConversationInfo {
    /// The tool's id for it (what `--resume` takes).
    pub id: String,
    /// Its tool.
    pub tool: String,
    /// The account it's on.
    pub account_id: String,
    /// That account's name.
    pub account_label: String,
    /// The project it belongs to (worktrees fold into their repository),
    /// when AI Juggler knows it.
    pub project_id: Option<String>,
    /// The folder it ran in.
    pub cwd: Option<String>,
    /// Its title: the tool's summary, else the first thing asked.
    pub title: Option<String>,
    /// The git branch it last recorded.
    pub git_branch: Option<String>,
    /// First activity, epoch ms.
    #[ts(type = "number | null")]
    pub started_ms: Option<i64>,
    /// Last activity, epoch ms.
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// Tokens, each message counted once.
    pub tokens: Option<crate::commands::TokenCountsInfo>,
    /// A running session of its tool is on it now.
    pub live: bool,
    /// The AI Juggler session this conversation is, when AI Juggler
    /// started it (the rail shows that session instead).
    pub session_id: Option<String>,
}
