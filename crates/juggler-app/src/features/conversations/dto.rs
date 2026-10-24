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
    /// AJ-E2101: where it is open now, one place per process running it
    /// (open in two apps: two places, one row). Empty when it isn't open.
    pub places: Vec<PlaceInfo>,
    /// The first place's process id.
    pub pid: Option<u32>,
    /// Its state in E21's words: `asks_you`, `working`, `idle` or
    /// `done_review`; `None` when it isn't open or the tool doesn't say.
    pub state: Option<String>,
    /// When the user last had it in front in its app, epoch ms, as the app
    /// recorded it: "viewed N min ago", never "in front".
    #[ts(type = "number | null")]
    pub viewed_ms: Option<i64>,
    /// A headless run (`-p`, `exec`, an SDK), not an interactive session.
    pub background: bool,
    /// In a running app's sidebar, with no process on it now.
    pub inactive: bool,
}

/// AJ-E2101: one place a conversation is open: the app, the profile and
/// the app's instance, from the process's parent chain and its own files
/// (never its environment).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PlaceInfo {
    /// The app's name as the Dock shows it (`Claude`, `Terminal`,
    /// `ChatGPT`); empty when its parent chain names no app.
    pub app: String,
    /// The host registry's id for the app, when it knows it.
    pub app_id: Option<String>,
    /// The profile it runs in: its account's name, else its folder's.
    pub profile: Option<String>,
    /// The app's instance (its data folder's name) when the app runs
    /// several.
    pub instance: Option<String>,
    /// The process running the session.
    pub pid: u32,
    /// The place in words: `Claude · b-work`, `Terminal`.
    pub label: String,
}

/// AJ-E2102: what Go to it did, in words, and the step it reached.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GoToOutcomeInfo {
    /// What happened, for the toast: "Brought Claude · Work forward; pick
    /// the session in its sidebar".
    pub outcome: String,
    /// The step reached: `thread`, `tab`, `folder`, `app`, `pending`
    /// (still on it after 1.5 s) or `none`.
    pub reached: String,
}
