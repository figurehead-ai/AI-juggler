//! What the UI sees of a session's runtime (AJ-E2302..E2307).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How one session runs, and its exact state when the runtime knows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RuntimeInfo {
    /// `terminal` or `driven`.
    pub kind: String,
    /// How it runs, in words ("Codex's app-server"), for a driven one.
    pub label: Option<String>,
    /// The runtime is a Beta (chosen per session).
    pub beta: bool,
    /// The exact state: `working`, `idle`, `asks_you` or `ended`, as the
    /// tool reported it (never a guess); `null` when not known.
    pub state: Option<String>,
    /// The open asks, oldest first.
    pub asks: Vec<AskInfo>,
    /// A message can be added to a running turn.
    pub can_steer: bool,
    /// Why it runs in the terminal although driven was chosen, when it does.
    pub fell_back: Option<String>,
    /// The last error the tool reported, in its words.
    pub error: Option<String>,
    /// The account's limits as this session's tool last reported them.
    pub limits: Vec<LimitWindowInfo>,
}

/// One limit window, as the tool reported it.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LimitWindowInfo {
    /// The tool's name for it.
    pub id: String,
    /// Its length in minutes, when the tool says.
    pub window_mins: Option<u32>,
    /// How much is used, 0–100.
    pub used_pct: f64,
    /// When it resets, epoch ms, when the tool says.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
}

/// One option an approval offers, in the tool's words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AskOptionInfo {
    /// What an answer names.
    pub id: String,
    /// The tool's label.
    pub label: String,
    /// `allow_once`, `allow_rule`, `deny` or `deny_and_stop`.
    pub effect: String,
    /// What the tool will remember, for `allow_rule`.
    pub rule: Option<String>,
}

/// One choice of a question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ChoiceInfo {
    /// Its label (what an answer sends).
    pub label: String,
    /// More about it.
    pub description: Option<String>,
}

/// One part of a question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QuestionPartInfo {
    /// The tool's id for it.
    pub id: String,
    /// A short header.
    pub header: String,
    /// The question.
    pub question: String,
    /// The tool's choices.
    pub choices: Vec<ChoiceInfo>,
    /// More than one may be picked.
    pub multi: bool,
    /// A free reply is allowed.
    pub free_text: bool,
    /// The reply is a secret (typed hidden, never shown again).
    pub secret: bool,
}

/// One open ask of a driven session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AskInfo {
    /// The ask's id.
    pub id: String,
    /// The session it belongs to.
    pub session_id: String,
    /// `approval` or `question`.
    pub kind: String,
    /// For an approval: `command`, `file_change`, `permissions` or `tool`.
    pub approval_kind: Option<String>,
    /// What it asks, in one line.
    pub summary: String,
    /// Why, when the tool says.
    pub reason: Option<String>,
    /// An approval's options, in the tool's order.
    pub options: Vec<AskOptionInfo>,
    /// A question's parts.
    pub parts: Vec<QuestionPartInfo>,
    /// When it was asked, epoch ms.
    #[ts(type = "number")]
    pub asked_at_ms: i64,
}

/// One line of a driven session's conversation (held in memory to show
/// it, never stored: the tool's own transcript is the record).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ConversationItemInfo {
    /// `user_message`, `agent_message`, `command`, `file_change`, `tool`,
    /// `note`.
    pub kind: String,
    /// Its text (bounded).
    pub text: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A driven session's page: its runtime, its recent conversation, the
/// text being written now.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RuntimeViewInfo {
    /// The session.
    pub session_id: String,
    /// How it runs.
    pub runtime: RuntimeInfo,
    /// The recent conversation, oldest first (at most the last 60).
    pub items: Vec<ConversationItemInfo>,
    /// What the agent is writing now.
    pub streaming: Option<String>,
    /// The model, when the tool said.
    pub model: Option<String>,
}

/// The user's answer, as the UI sends it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AnswerInput {
    /// An approval's option id.
    #[ts(optional)]
    pub option: Option<String>,
    /// A question's answers: part id → chosen labels or the free reply.
    #[ts(optional)]
    pub replies: Option<Vec<PartReplyInput>>,
}

/// One part's reply.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PartReplyInput {
    /// The part's id.
    pub part: String,
    /// The labels chosen, or the free reply.
    pub answers: Vec<String>,
}

/// The runtime choices of the profiles, and the Beta switch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RuntimeChoicesInfo {
    /// "Run without a terminal (Beta)" is on in Settings › Sessions.
    pub beta_on: bool,
    /// Per profile with a driven runtime: what it offers and what was chosen.
    pub profiles: Vec<ProfileRuntimeInfo>,
}

/// One profile's runtime choice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProfileRuntimeInfo {
    /// The profile (launch settings) id.
    pub profile_id: String,
    /// How its driven runtime runs, in words.
    pub label: String,
    /// The tool's default: `terminal` or `driven`.
    pub default: String,
    /// What the user chose, when they did.
    pub chosen: Option<String>,
    /// The driven runtime is a Beta (only per session).
    pub beta: bool,
    /// Why driven can't run with the tool installed now.
    pub unavailable: Option<String>,
}
