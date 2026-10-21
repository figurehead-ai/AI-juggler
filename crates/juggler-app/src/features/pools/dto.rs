//! The pools' wire types (AJ-E1702).

use serde::Serialize;
use ts_rs::TS;

/// Every pool, every session's place in one, and what roles may do.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PoolsInfo {
    /// The pools: "All sessions" first, then the user's, oldest first.
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
    /// What each role may do here, capability by capability.
    pub grants: Vec<GrantInfo>,
    /// How many separate groups its sessions make under D-33 (one project
    /// and one account each): in the default pool agents will see only
    /// their own group; in a user pool more than one means the user put
    /// work from different projects or accounts together.
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

/// What one capability allows, per role.
// One flag per role: the row the grants table shows.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct GrantInfo {
    /// The capability (`message`, `broadcast`, …).
    pub capability: String,
    /// Every role always has it; it can't be changed.
    pub fixed: bool,
    /// Leads may.
    pub lead: bool,
    /// Members may.
    pub member: bool,
    /// Observers may.
    pub observer: bool,
}
