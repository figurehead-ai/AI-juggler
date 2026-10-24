//! The pools' wire types (AJ-E1702, AJ-E1703).

use serde::Serialize;
use ts_rs::TS;

/// Every pool, every session's place in one, and what roles may do.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolsInfo {
    /// The pools: the user's, oldest first, then each project's default
    /// pool by name, "No project" last.
    pub pools: Vec<PoolInfo>,
    /// Each session's pool and role.
    pub members: Vec<PoolMemberInfo>,
}

/// One pool.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
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
}

/// One session's place.
#[derive(Debug, Clone, Serialize, TS)]
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
    /// Where it works: its project (and branch), else its folder's name.
    pub place: String,
    /// The tool, by its display name.
    pub tool: String,
    /// D-33: the account's label (the user's name for it, "Work Max"),
    /// never an email, a plan or any other account detail; `None` when
    /// the label is an email address.
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

/// A pool's cards, its lead first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolCards {
    /// The pool.
    pub pool_id: String,
    /// One card per member.
    pub cards: Vec<ContextCard>,
}
