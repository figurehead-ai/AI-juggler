//! The pools' wire types (AJ-E1702, AJ-E1703).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::features::sessions::dto::InstanceInfo;

/// Every pool, and where the sessions the app's lists hold are: never
/// the whole history (I6). A pool's members come a page at a time
/// (`pools_members`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolsInfo {
    /// The pools: the user's, oldest first, then each project's default
    /// pool by name, "No project" last.
    pub pools: Vec<PoolInfo>,
    /// The places of the working set's sessions (what the rail holds),
    /// one row per pool a session is in, its first pool first (D-62);
    /// a session whose only place is its project's own pool as a member
    /// has no row. One past the working set (opened from History) is read
    /// alone (`pools_place`).
    pub placements: Vec<PoolMemberInfo>,
}

/// One pool.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(clippy::struct_excessive_bools)] // wire DTO: each flag is its own setting the pool page shows
pub struct PoolInfo {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// `default` | `user`.
    pub kind: String,
    /// The one project it's for, when it is.
    pub project_id: Option<String>,
    /// What agents may do here (D-28): its level and what that allows.
    pub access: AccessInfo,
    /// The user's "always allow" rules here.
    pub rules: Vec<AgentRuleInfo>,
    /// D-32: the user let it span their paired Macs (off by default).
    pub spans_macs: bool,
    /// How many projects its sessions come from: a project's default pool
    /// holds one (D-33); in a user pool more than one means the user put
    /// different projects' work together.
    pub scopes: u32,
    /// How many sessions are in it, archived ones aside: its member
    /// list's length (`pools_members`).
    pub members: u32,
    /// D-62 decision 8: it takes every project's sessions (those with no
    /// project too).
    pub scope_all: bool,
    /// Else the projects it takes (`null`: sessions with no project).
    pub scope: Vec<Option<String>>,
    /// D-62 decision 14: its sessions are in no other pool, nothing
    /// leaves it.
    pub sealed: bool,
    /// Its broad members skip sessions another pool names on their own (a
    /// project's own pool).
    pub skip_held: bool,
}

/// One member rule of a pool (D-62 decision 9): what it matches, its
/// role and its own level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolRuleInfo {
    /// Its stored ref (`work:…`, `session:…`, `app:…`, `account:…`,
    /// `tool:…`, `mac:…`, `all`).
    pub member: String,
    /// `session` | `app` | `account` | `tool` | `mac` | `everyone`.
    pub kind: String,
    /// `lead` | `member` | `reviewer` | `observer`.
    pub role: String,
    /// Its own level, else `null` (the pool's).
    pub level: Option<String>,
    /// When it joined, epoch ms.
    #[ts(type = "number")]
    pub joined_at: i64,
    /// How many sessions it brings into the pool now (archived ones
    /// aside): a broad member's group node says it.
    pub sessions: u32,
}

/// A Watched member: a session another app runs (D-62 decision 15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchedMemberInfo {
    /// Its stored ref (`conversation:<folder>/<id>`).
    pub member: String,
    /// The folder it was read from.
    pub folder: String,
    /// The tool's own conversation id.
    pub conversation_id: String,
    /// Its title, when the index has one.
    pub title: Option<String>,
    /// Its tool, when known.
    pub tool: Option<String>,
    /// Its account's label, when known.
    pub account_label: Option<String>,
    /// Its project, when known.
    pub project_id: Option<String>,
    /// Its role in the pool.
    pub role: String,
    /// The AI Juggler session that continued it, when one did (then it's
    /// a full member).
    pub continued_session: Option<String>,
}

/// One pool's talk settings (D-62 decision 13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolTalkInfo {
    /// From: `role:<role>`, or a member's stored ref.
    pub from: String,
    /// To: the same.
    pub to: String,
    /// `allowed` | `ask` | `blocked`.
    pub rule: String,
}

/// What a switch does to one of a session's pools (D-62, AJ-E3009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SwitchPoolLineInfo {
    /// The pool.
    pub pool_id: String,
    /// `stays` | `shell` | `leaves` | `joins` | `sealed`.
    pub kind: String,
    /// The line, in words.
    pub words: String,
    /// A sealed pool is entered or left: the switch asks for a confirm.
    pub confirm: bool,
}

/// Who may tell whom along one line on the canvas (D-62 decision 13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EdgeTalkInfo {
    /// The rule that applies: `allowed` | `ask` | `blocked`.
    pub rule: String,
    /// The role pair's rule (what the line follows unless it's set).
    pub pair_rule: String,
    /// The line has its own rule, overriding its pair's.
    pub own: bool,
    /// The sender's role in the pool.
    pub from_role: String,
    /// The receiver's role in the pool.
    pub to_role: String,
}

/// A pool's rules: its members by kind and its talk settings.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolRulesInfo {
    /// The pool.
    pub pool_id: String,
    /// Its member rules, most specific first.
    pub members: Vec<PoolRuleInfo>,
    /// Its talk settings (anything unset is allowed).
    pub talk: Vec<PoolTalkInfo>,
}

/// One pool a session is in, and how (its Pools tab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionPoolInfo {
    /// The pool.
    pub pool_id: String,
    /// Its name on screen.
    pub pool_name: String,
    /// The session's role there.
    pub role: String,
    /// How it got in: `session` | `app` | `account` | `tool` | `mac` |
    /// `everyone`.
    pub via: String,
    /// The member that brought it in (its stored ref).
    pub member: String,
    /// Its own level there, when the member carries one.
    pub level: Option<String>,
    /// The pool is sealed.
    pub sealed: bool,
}

/// A pool a member matched the session in that doesn't hold it, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionSkipInfo {
    /// The pool.
    pub pool_id: String,
    /// Its name on screen.
    pub pool_name: String,
    /// `out_of_scope` | `sealed_elsewhere` | `held_elsewhere`.
    pub reason: String,
    /// The sealed pool, or the pool that names it.
    pub by_pool: Option<String>,
    /// That pool's name on screen.
    pub by_pool_name: Option<String>,
    /// The reason in words, with the step that would let it in.
    pub words: String,
}

/// Every pool a session is in, how it got into each, and the pools that
/// left it out (D-62; AJ-E3004 req. 3).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SessionPoolsInfo {
    /// The session.
    pub session_id: String,
    /// Its pools, the first the one it's shown in first.
    pub pools: Vec<SessionPoolInfo>,
    /// The pools that left it out.
    pub skipped: Vec<SessionSkipInfo>,
    /// Its sign-in can't be read: Account members can't match it.
    pub account_unknown: bool,
}

/// One session's place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolMemberInfo {
    /// The session.
    pub session_id: String,
    /// Its pool.
    pub pool_id: String,
    /// `lead` | `member` | `observer`.
    pub role: String,
}

/// A pool's access level (D-28), set by the user in the app only.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccessInfo {
    /// `read` | `work_together` | `full` | `custom`.
    pub level: String,
    /// The actions the level allows without an ask (rules aside), in the
    /// catalog's order.
    pub actions: Vec<String>,
    /// D-29: hops a delivered thread may take before delivery pauses and
    /// asks (default 5).
    pub hop_limit: u32,
    /// D-29: the user stopped all delivery into this pool's sessions.
    pub delivery_stopped: bool,
}

/// One "always allow" rule the user made.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AgentRuleInfo {
    /// Its id (to take it back).
    #[ts(type = "number")]
    pub id: i64,
    /// The pool it's for.
    pub pool_id: String,
    /// The action it allows.
    pub action: String,
    /// When it was made, epoch ms.
    #[ts(type = "number")]
    pub created_at: i64,
    /// Who made it (always `user`).
    pub made_by: String,
}

/// One thing a card says, or why it can't (D-06: never a guess). Exactly
/// one of the two is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CardField {
    /// What AI Juggler knows.
    pub value: Option<String>,
    /// Why it doesn't, in words ("Unavailable — …").
    pub unavailable: Option<String>,
}

impl CardField {
    /// A known value.
    #[must_use]
    pub fn known(value: impl Into<String>) -> Self {
        Self {
            value: Some(value.into()),
            unavailable: None,
        }
    }

    /// Unknown, and why.
    #[must_use]
    pub fn unavailable(why: &str) -> Self {
        Self {
            value: None,
            unavailable: Some(format!("Unavailable — {why}")),
        }
    }
}

/// AJ-E1703: what one member of a pool is working on, built only from
/// what AI Juggler knows. Of its account it carries only the label (D-33):
/// the same card is what an agent will be shown (AJ-E1704).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ContextCard {
    /// The session.
    pub session_id: String,
    /// Its name: the user's label, else what it's about, else its branch
    /// or folder.
    pub name: String,
    /// Where it works, as the rows say it (D-36): project · app ·
    /// profile, its folder's name standing in for a project.
    pub place: String,
    /// The tool, by its display name.
    pub tool: String,
    /// D-33, D-35: its profile's label (the user's name for it, "Work
    /// Max"), never an email, a plan or any other account detail; `None`
    /// when the label is an email address.
    pub account: Option<String>,
    /// Its state (`running`, `waiting_user`, `stuck`, `ended`, …).
    pub state: String,
    /// Its role in the pool.
    pub role: String,
    /// The task it has claimed (E19).
    pub task: CardField,
    /// What it says it's doing, in its own words: the step its own task
    /// list has in progress. Shown as said by the agent, never as a fact.
    pub said: CardField,
    /// When it was last active, epoch ms.
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// The files it changed most recently, newest first (the last two
    /// parts of each path).
    pub files: Vec<String>,
    /// Why `files` is empty when it's not known to be empty.
    pub files_unavailable: Option<String>,
}

/// One member of a pool as the pool page and sheet list it: its session's
/// row (as the rail has it) and its card.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolMemberRow {
    /// The session.
    pub session: InstanceInfo,
    /// What it's working on, and its role.
    pub card: ContextCard,
}

/// A pool's cards, its lead first (what an agent is shown, AJ-E1704).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolCards {
    /// The pool.
    pub pool_id: String,
    /// One card per member.
    pub cards: Vec<ContextCard>,
}

/// 0.3 (redesign §5): what a pool's canvas draws besides its members —
/// where the user put them, who talks to whom, what waits on the user and
/// the pool's newest activity. Read for the members the canvas shows,
/// never the whole history (I6).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolCanvasInfo {
    /// The pool.
    pub pool_id: String,
    /// Where the user put each member; one with none is laid out by the
    /// app.
    pub positions: Vec<NodePositionInfo>,
    /// Messages and task hand-offs between two members since `since_ms`.
    pub edges: Vec<CanvasEdgeInfo>,
    /// Agents' asks waiting on the user, each on the member that asked.
    pub asks: Vec<NodeAskInfo>,
    /// The pool's newest messages and task events, newest first.
    pub stream: Vec<StreamLineInfo>,
    /// Edges count from here, epoch ms.
    #[ts(type = "number")]
    pub since_ms: i64,
}

/// Where the user put one member on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NodePositionInfo {
    /// The member's session (as the canvas shows it).
    pub session_id: String,
    /// Across, in canvas units.
    pub x: f64,
    /// Down, in canvas units.
    pub y: f64,
}

/// Who talked to whom: one direction between two members.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CanvasEdgeInfo {
    /// The sender (or the one who gave the task).
    pub from_session_id: String,
    /// The recipient (or the task's holder).
    pub to_session_id: String,
    /// Messages sent this way.
    pub messages: u32,
    /// Tasks handed over this way that the recipient still holds.
    pub tasks: u32,
    /// The newest of them, epoch ms.
    #[ts(type = "number")]
    pub last_at_ms: i64,
    /// The newest message's thread (the conversation to open).
    pub thread_id: Option<String>,
    /// Messages this way the recipient hasn't got yet that the user can
    /// act on.
    pub pending: Vec<PendingInfo>,
}

/// A message that waits: on the user's click to go in, or on their answer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PendingInfo {
    /// The message.
    pub message_id: String,
    /// Its thread.
    pub thread_id: String,
    /// Its subject.
    pub subject: String,
    /// The recipient as AI Juggler keeps it (what Deliver names).
    pub recipient: String,
    /// `waiting` (in a session run here; Deliver puts it in) | `hop_limit`
    /// (its chain reached the pool's hop limit: Let it continue or Don't
    /// deliver) | `asked` (above what the pool allows: answered on the
    /// sender's node).
    pub kind: String,
    /// The held ask that answers it, for `hop_limit` and `asked`.
    pub ask_id: Option<String>,
    /// When it was sent, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// An agent's ask that waits on the user (D-28), on the member that asked.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NodeAskInfo {
    /// The member that asked; `None` when it isn't on the canvas.
    pub session_id: Option<String>,
    /// `message` (answered with `messages_answer`) | `task`
    /// (`tasks_answer`).
    pub source: String,
    /// The ask's id (what the answer names).
    pub id: String,
    /// In one line.
    pub title: String,
    /// Offer "Always allow in this pool" (from the second approval).
    pub offer_always: bool,
    /// Since when, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// One line of the pool's live stream, in words.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StreamLineInfo {
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// What happened ("Planner gave «Round tax» to Cart totals").
    pub words: String,
    /// `message` | `task`.
    pub source: String,
    /// The member who did it, when it's on the canvas.
    pub session_id: Option<String>,
    /// The member it went to, when one did and it's on the canvas.
    pub other_session_id: Option<String>,
}

/// One pool's card on the Pools screen: where its open members sit on its
/// canvas, and how many of its tasks are in progress.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolPreviewInfo {
    /// The pool.
    pub pool_id: String,
    /// Where the user put its members that are in the working set (what
    /// the rail holds).
    pub positions: Vec<NodePositionInfo>,
    /// Its tasks in progress.
    pub moving: u32,
}
