//! The Subagents tab's wire types (AJ-E1520: 0.2, the list; 0.3, the
//! views, groups, tree and a subagent's own page).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::commands::TokenCountsInfo;
use crate::paging::Page;

/// What the tab asks for: a view, a search, a type, an order, and which
/// groups the user opened or folded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentsQuery {
    /// `running`, `attention` (failed, stopped, unknown), `done` (every
    /// one not running) or `all` (the default).
    #[serde(default)]
    pub view: Option<String>,
    /// Words to find in a subagent's task, prompt, result, type or files;
    /// `type:<word>` and `file:<word>` narrow to one.
    #[serde(default)]
    pub search: Option<String>,
    /// Only this type (a [`SubagentKindInfo::kind`]).
    #[serde(default)]
    pub kind: Option<String>,
    /// `newest` (the default), `tokens` (most first) or `longest`.
    #[serde(default)]
    pub sort: Option<String>,
    /// Groups the user opened (a group's `key`).
    #[serde(default)]
    pub open: Vec<String>,
    /// Groups the user folded.
    #[serde(default)]
    pub closed: Vec<String>,
}

/// A session's subagents (or a project's), a page of rows at a time, or
/// why there are none.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentsInfo {
    /// `listed`; `unsupported` (its tool keeps no record AI Juggler
    /// reads: `reason` says why); `none` (no record of the session on
    /// this Mac); `unreadable` (its record couldn't be read just now).
    pub state: String,
    /// Why they aren't listed (`unsupported`, `unreadable`).
    pub reason: Option<String>,
    /// This page of rows: each group (the message that started them),
    /// then its subagents as a tree, in the order asked.
    pub page: Page<SubagentRowInfo>,
    /// Counts over all of them (whatever the view).
    pub summary: SubagentsSummaryInfo,
    /// Each type and how many, for the Type filter.
    pub kinds: Vec<SubagentKindInfo>,
    /// How many match the view, search and type.
    pub matched: u32,
    /// How many are running now (the summary's, kept for the rail's line).
    pub running: u32,
    /// A very long session is still being read: what's found so far is
    /// listed; ask again for the rest.
    pub reading: bool,
    /// Whether the session is open now (a subagent the records never saw
    /// end is unknown once it isn't).
    pub live: bool,
    /// Why a subagent's conversation can't be opened, when its tool keeps
    /// none AI Juggler reads; `null` when it can.
    pub conversations_note: Option<String>,
    /// The tool's name ("Claude Code"), for "Not recorded by …".
    pub tool: String,
}

/// Counts over all of a session's subagents.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentsSummaryInfo {
    /// Every subagent.
    pub total: u32,
    /// Running now (only while the session is open).
    pub running: u32,
    /// Ended with an error.
    pub failed: u32,
    /// Stopped before it finished.
    pub stopped: u32,
    /// How it ended isn't on record.
    pub unknown: u32,
    /// Failed, stopped or unknown: the Needs attention view.
    pub attention: u32,
    /// Their tokens together (`null`: not recorded per subagent).
    #[ts(type = "number | null")]
    pub tokens: Option<u64>,
    /// Their share of all the session's tokens, 0–100 (`null`: not
    /// known).
    pub share_pct: Option<f64>,
    /// `in_session` (part of the session's total: a share, never added
    /// on top), `separate` (apart from the parent's) or `none`.
    pub counted: String,
}

/// One type of subagent and how many there are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentKindInfo {
    /// The type as the tool records it.
    pub kind: String,
    /// How many.
    pub count: u32,
}

/// One row of the tab's tree: a group or a subagent.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentRowInfo {
    /// Unique in the list (`g:…` a group, `a:…` a subagent).
    pub key: String,
    /// `group` or `agent`.
    pub row: String,
    /// The group, on a group row.
    pub group: Option<SubagentGroupInfo>,
    /// The subagent, on an agent row.
    pub agent: Option<SubagentInfo>,
    /// For each level above this row's: whether a line runs down past it
    /// (that ancestor has a later sibling shown): the tree's `│`.
    pub guides: Vec<bool>,
    /// The last of its siblings shown (`└` rather than `├`).
    pub last: bool,
    /// Shown only because a subagent under it matches (dimmed).
    pub context: bool,
}

/// The subagents one message started.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentGroupInfo {
    /// Stable for the message (what `open` and `closed` name).
    pub key: String,
    /// The message's first line, redacted (`null`: not recorded).
    pub asked: Option<String>,
    /// When it was sent (else the first subagent's start), epoch ms.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// Its subagents (theirs too).
    pub count: u32,
    /// Running now.
    pub running: u32,
    /// Failed.
    pub failed: u32,
    /// Their tokens together.
    #[ts(type = "number | null")]
    pub tokens: Option<u64>,
    /// Open (its subagents listed after it).
    pub open: bool,
    /// The session it is in (a project's view), else `null`.
    pub session: Option<SubagentSessionInfo>,
}

/// The session a subagent is in (a project's view).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentSessionInfo {
    /// Its account.
    pub account_id: String,
    /// The tool's id for its conversation.
    pub conversation_id: String,
    /// Its title, when it has one.
    pub title: Option<String>,
}

/// One subagent, as the tab shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentInfo {
    /// The tool's id for it (unique in the session).
    pub id: String,
    /// Its type or name, when the tool records one.
    pub kind: Option<String>,
    /// What it was asked to do, redacted (D-14), when recorded.
    pub task: Option<String>,
    /// `running`, `done`, `failed`, `stopped` or `unknown` (D-06: never a
    /// guess).
    pub status: String,
    /// When it started, epoch ms.
    #[ts(type = "number | null")]
    pub started_ms: Option<i64>,
    /// When it ended, epoch ms (only once ended).
    #[ts(type = "number | null")]
    pub ended_ms: Option<i64>,
    /// How long it ran (or has run), ms.
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    /// When its records last changed, epoch ms.
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// The subagent that started it; `null` when the session did.
    pub parent: Option<String>,
    /// 1 when the session started it, 2 for one of its subagents', ….
    pub depth: u32,
    /// Whether it ran in the background, when the tool records it.
    pub background: Option<bool>,
    /// Whether its own conversation can be opened.
    pub has_conversation: bool,
    /// Its tokens with those of the subagents it started.
    #[ts(type = "number | null")]
    pub tokens: Option<u64>,
    /// Its own tokens.
    #[ts(type = "number | null")]
    pub own_tokens: Option<u64>,
    /// The model it ran on, as the tool names it.
    pub model: Option<String>,
    /// How many subagents it started.
    pub children: u32,
    /// The session it is in (a project's view), else `null`.
    pub session: Option<SubagentSessionInfo>,
}

/// One subagent's own page.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentDetailInfo {
    /// `shown`; `none` (no such subagent, or no record of the session);
    /// `unsupported`; `unreadable`.
    pub state: String,
    /// Why it isn't shown.
    pub reason: Option<String>,
    /// The subagent.
    pub agent: Option<SubagentInfo>,
    /// The subagents above it, the session's first.
    pub chain: Vec<SubagentCrumbInfo>,
    /// The prompt it was given, redacted (`null`: not recorded, or last
    /// messages are off).
    pub prompt: Option<String>,
    /// The start of its answer, redacted (markdown).
    pub result: Option<String>,
    /// Why it failed, in one line.
    pub failure: Option<String>,
    /// Last messages are off: no prompt or result is read or shown.
    pub messages_off: bool,
    /// The user's message the turn that started it (or its first
    /// ancestor) began with, redacted.
    pub asked: Option<String>,
    /// Its tool calls by kind (`null`: not recorded).
    pub tools: Option<SubagentToolsInfo>,
    /// Lines its edits added and removed.
    pub lines: Option<SubagentLinesInfo>,
    /// The files it edited, most-edited first (at most 200).
    pub files: Vec<String>,
    /// How many files it edited in all.
    pub files_total: u32,
    /// The subagents it started (at most 50).
    pub children: Vec<SubagentInfo>,
    /// How many it started.
    pub children_total: u32,
    /// Its own tokens by kind (`null`: not recorded).
    pub tokens: Option<TokenCountsInfo>,
    /// How its tokens relate to the session's (`in_session`, `separate`,
    /// `none`).
    pub counted: String,
    /// An estimate of what its tokens cost (USD), from the price list.
    pub cost_usd: Option<f64>,
    /// Its tokens (with its subagents') as a share of the session's,
    /// 0–100.
    pub share_pct: Option<f64>,
    /// Processes it probably started (matched from its commands).
    pub processes: Vec<SubagentProcessInfo>,
    /// The Processes view's id for the session (to open it there).
    pub process_session: Option<String>,
    /// How many commands it ran (what processes are matched by).
    pub commands: u32,
    /// How to stop it in the tool itself (AI Juggler can't stop one
    /// subagent inside a session), when the tool has a way.
    pub stop_hint: Option<String>,
    /// The worktree it ran in, and its branch.
    pub worktree: Option<String>,
    /// That worktree's branch.
    pub branch: Option<String>,
    /// The user stopped it, when recorded.
    pub stopped_by_user: Option<bool>,
    /// The tool's name.
    pub tool: String,
}

/// A subagent above another, for the breadcrumbs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentCrumbInfo {
    /// Its id.
    pub id: String,
    /// Its task, redacted, else its type.
    pub title: Option<String>,
}

/// Tool calls by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentToolsInfo {
    /// Every call.
    pub total: u32,
    /// Files read.
    pub reads: u32,
    /// Searches.
    pub searches: u32,
    /// Commands run.
    pub commands: u32,
    /// Files edited or written.
    pub edits: u32,
    /// Anything else.
    pub other: u32,
}

/// Lines added and removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentLinesInfo {
    #[ts(type = "number")]
    /// Added.
    pub added: u64,
    #[ts(type = "number")]
    /// Removed.
    pub removed: u64,
}

/// A process a subagent probably started.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubagentProcessInfo {
    /// The Processes view's row key (to select it there).
    pub key: String,
    /// Its process id.
    pub pid: u32,
    /// Its program's name.
    pub name: String,
    /// The command it was matched by, redacted.
    pub command: String,
    #[ts(type = "number")]
    /// Its memory now, bytes.
    pub rss_bytes: u64,
    /// Its CPU now, percent of one core.
    pub cpu_pct: Option<f64>,
    /// Still running.
    pub alive: bool,
}
