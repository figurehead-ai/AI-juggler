//! Bot's wire types (D-136): a found row with its source, one turn of the
//! conversation, and a page of the stream. Every sentence is the window's
//! (`ui/src/lib/bot-words.ts`); these carry ids, states, numbers and the
//! records' own names.

use serde::{Deserialize, Serialize};

/// What kind of record a row is: its link opens that page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotRowKind {
    /// A session (its page).
    Session,
    /// A task (its pool's board).
    Task,
    /// An Account (its page).
    Account,
}

/// A row's state, from the owning module's facts. Never merged: each row
/// says its own (the lead is built from these, one clause per state).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotState {
    /// The session asks you.
    AsksYou,
    /// It looks stuck.
    Stuck,
    /// It reached its limit.
    Limit,
    /// It stopped with an error.
    Failed,
    /// The task is blocked.
    Blocked,
    /// The task waits for a review.
    InReview,
    /// It is working (a task: in progress).
    Working,
    /// The session finished.
    Finished,
    /// The session was ended (stopped, handed over).
    Ended,
    /// The task is done.
    Done,
    /// The task is ready to pick up.
    Ready,
    /// The task isn't ready yet.
    Todo,
    /// The task was dropped.
    Cancelled,
}

impl BotState {
    /// Its attention level (D-117): 2 needs you, 1 worth a look, 0 calm.
    #[must_use]
    pub const fn level(self) -> u8 {
        match self {
            Self::AsksYou | Self::Stuck | Self::Limit => 2,
            Self::Failed | Self::Blocked | Self::InReview => 1,
            _ => 0,
        }
    }

    /// Its stored code (history's then-state).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AsksYou => "asks_you",
            Self::Stuck => "stuck",
            Self::Limit => "limit",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::InReview => "in_review",
            Self::Working => "working",
            Self::Finished => "finished",
            Self::Ended => "ended",
            Self::Done => "done",
            Self::Ready => "ready",
            Self::Todo => "todo",
            Self::Cancelled => "cancelled",
        }
    }
}

/// How many matched records are of one kind in one state: the lead
/// sentence counts every match with these, not only the rows shown (the
/// 0.5 walk's F-04: "8 sessions finished" over six rows and "7 more").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotTally {
    /// The records' kind.
    pub kind: BotRowKind,
    /// Their state when asked (an Account has none).
    pub state: Option<BotState>,
    /// How many.
    pub n: u32,
}

/// One found record: its source (kind and id: the link is built from
/// these, never from text), its name, its state and AI Juggler's facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotRowInfo {
    /// What it is.
    pub kind: BotRowKind,
    /// Its id.
    pub id: String,
    /// Its name (agent-written names are guarded: an order is removed).
    pub title: String,
    /// The name held an order and was left out.
    #[serde(default)]
    pub name_hidden: bool,
    /// Its state, when it has one (an Account has none).
    pub state: Option<BotState>,
    /// When its state began, or its last activity.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// When it started (a session).
    #[ts(type = "number | null")]
    pub started_ms: Option<i64>,
    /// Its tool (a session), for its App's name and mark.
    pub provider: Option<String>,
    /// Its App's id (a session).
    pub app_id: Option<String>,
    /// Its folder's or profile's label (a session).
    pub app_label: Option<String>,
    /// A desktop app's session.
    #[serde(default)]
    pub desktop: bool,
    /// Its project's name.
    pub project: Option<String>,
    /// Its project's id.
    pub project_id: Option<String>,
    /// Its pool's name (a task).
    pub pool: Option<String>,
    /// Its pool's id (a task).
    pub pool_id: Option<String>,
    /// Who holds it (a task).
    pub holder: Option<String>,
    /// Files changed and not committed (a session).
    pub files_changed: Option<u32>,
    /// Its branch (a session).
    pub branch: Option<String>,
    /// The share of its window used (an Account), percent.
    pub used_percent: Option<u32>,
    /// Which window that is: `five_hour` or `weekly`.
    pub window: Option<String>,
    /// When that window resets.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// How many tasks block it (a task).
    pub blocked_by: Option<u32>,
    /// The agent's own words, quoted data after the guard (only while
    /// Last messages is on).
    pub quote: Option<String>,
    /// History: its state when the question was asked, when that differs.
    pub then_state: Option<BotState>,
    /// History: it no longer exists.
    #[serde(default)]
    pub gone: bool,
}

/// What a question was asked about (D-136 point 3): the chip "About:
/// api ×". It rides on the question and travels by id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotScope {
    /// `session`, `app`, `project`, `pool` or `account`.
    pub kind: BotScopeKind,
    /// Its id.
    pub id: String,
    /// Its name, as the chip shows it.
    pub name: String,
    /// An App's sessions, by id (the window knows which are the App's).
    #[serde(default)]
    pub sessions: Vec<String>,
}

/// A scope's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotScopeKind {
    /// One session.
    Session,
    /// Every session in an App.
    App,
    /// Every session and task in a project.
    Project,
    /// Every task in a pool.
    Pool,
    /// One Account.
    Account,
}

/// How a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotOutcome {
    /// The found rows (the look-up view).
    Rows,
    /// A written answer was shown above the rows (Beta).
    Answered,
    /// "I don't know".
    DontKnow,
    /// Outside what Bot answers.
    OutOfScope,
    /// What Bot can do.
    Help,
    /// A memory kept ("Remember …").
    Remembered,
    /// A memory refused (it looked like a secret).
    Refused,
    /// A watch set from a confirm card.
    WatchSet,
}

impl BotOutcome {
    /// The stored code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Rows => "rows",
            Self::Answered => "answered",
            Self::DontKnow => "dont_know",
            Self::OutOfScope => "out_of_scope",
            Self::Help => "help",
            Self::Remembered => "remembered",
            Self::Refused => "refused",
            Self::WatchSet => "watch_set",
        }
    }

    /// The outcome a stored code names.
    #[must_use]
    pub fn parse(code: &str) -> Self {
        match code {
            "answered" => Self::Answered,
            "dont_know" => Self::DontKnow,
            "out_of_scope" => Self::OutOfScope,
            "help" => Self::Help,
            "remembered" => Self::Remembered,
            "refused" => Self::Refused,
            "watch_set" => Self::WatchSet,
            _ => Self::Rows,
        }
    }
}

/// Why Bot doesn't know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotUnknown {
    /// Nothing in AI Juggler matched.
    NothingFound,
    /// AI Juggler doesn't record how long tests take.
    TestDurations,
    /// AI Juggler doesn't record test results for a session.
    TestResults,
    /// Bot doesn't look up costs (Usage has them).
    Costs,
    /// AI Juggler doesn't record who wrote what.
    Authors,
}

/// The nearest screen that answers it (no AI needed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotShortcut {
    /// Needs you, in the rail.
    NeedsMe,
    /// Home, filtered to Finished.
    FinishedToday,
    /// Home's Runway column.
    Runway,
    /// Open now.
    Running,
    /// Usage.
    Usage,
}

/// What a turn looked at (the "Looked at" line), from the scope's kind
/// and name and the look-up's own reach, never from model text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotLooked {
    /// `everything` (every session, task and pool), `accounts`, `scope`
    /// (the chip's), `subject` (the record the question named).
    pub reach: String,
    /// The scope or the subject, by kind and name.
    pub scope: Option<BotScope>,
    /// Memories used (the receipt's count).
    #[serde(default)]
    pub memories: u32,
}

/// A written answer (Beta, ADR-0034 §6.5): shown only when every name,
/// number and state in it was found in the rows it cites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotWritten {
    /// Its sentences, each with the rows it cites (1-based).
    pub parts: Vec<BotWrittenPart>,
    /// The model's display name (from the `models` section).
    pub model_name: String,
    /// On this Mac (always, in 0.5).
    pub on_this_mac: bool,
}

/// One sentence of a written answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotWrittenPart {
    /// The words.
    pub text: String,
    /// The rows it rests on, 1-based.
    pub cites: Vec<u32>,
}

/// A memory used or kept (the echo, the "Looked at" receipt).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotMemoryInfo {
    /// Its subject key (Forget targets it).
    pub key: String,
    /// `alias`, `focus` or `note`.
    pub slot: String,
    /// The user's words.
    pub text: String,
    /// The name of what it is about (an alias's record).
    pub about: Option<String>,
    /// The kind of that record (`session`, `app`, `project`, `pool`).
    pub about_kind: Option<String>,
    /// When it was said.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// How many times it was used.
    pub used: u32,
    /// Earlier versions kept.
    pub versions: u32,
}

/// One turn of the conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotTurnInfo {
    /// Its id.
    pub id: String,
    /// When it was asked.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The user's words.
    pub question: String,
    /// The question's language (the lead's words follow it).
    pub lang: BotLang,
    /// How it ended.
    pub outcome: BotOutcome,
    /// The chip it was asked with.
    pub scope: Option<BotScope>,
    /// The found rows, at most eight.
    pub rows: Vec<BotRowInfo>,
    /// How many matched in all ("and 4 more").
    pub total: u32,
    /// Every match counted by kind and state, for the lead (empty on a
    /// turn kept before 0.5.0's fix: the lead then counts the rows).
    #[serde(default)]
    pub tallies: Vec<BotTally>,
    /// Why Bot doesn't know, and the nearest record or screen.
    pub unknown: Option<BotUnknown>,
    /// The nearest screen for "I don't know" and the no-model shortcuts.
    pub shortcut: Option<BotShortcut>,
    /// What it looked at.
    pub looked: BotLooked,
    /// The written answer (Beta), when one passed the check.
    pub written: Option<BotWritten>,
    /// The memory kept or used.
    pub memory: Option<BotMemoryInfo>,
    /// The memories used, for "Looked at".
    #[serde(default)]
    pub used: Vec<BotMemoryInfo>,
    /// Rows read now (a past turn shows how it stands now).
    pub live: bool,
    /// Forget cleared its words.
    pub redacted: bool,
    /// The user said "This is wrong" about its written answer.
    #[serde(default)]
    pub flagged: bool,
}

/// The languages the words cover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum BotLang {
    /// English.
    #[default]
    En,
    /// German.
    De,
}

/// Where a New topic divider goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotTopicInfo {
    /// Its id.
    pub id: String,
    /// When it began.
    #[ts(type = "number")]
    pub started_ms: i64,
    /// The very first topic: no divider.
    pub first: bool,
}

/// One page of the conversation, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotPageInfo {
    /// The turns, newest first.
    pub turns: Vec<BotTurnInfo>,
    /// The topics that begin within the page.
    pub topics: Vec<BotTopicInfo>,
    /// Older turns remain.
    pub more: bool,
}

/// What the clear sheet says goes and what stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotCountInfo {
    /// Questions in the conversation.
    pub questions: u32,
    /// The oldest one's time.
    #[ts(type = "number | null")]
    pub since_ms: Option<i64>,
    /// The user's notes and names (they stay).
    pub memories: u32,
    /// Watches (they stay).
    pub watches: u32,
}

/// A turn's position in the stream (search: "1 of 5").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotFoundInfo {
    /// The turn's id.
    pub id: String,
    /// When it was asked.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// The question as asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BotAskInput {
    /// The user's words.
    pub question: String,
    /// The chip, when one rides on it.
    pub scope: Option<BotScope>,
}
