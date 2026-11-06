//! What a page asks the relation counts for, and what it's told.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::features::sessions::rows::SessionsCountsInfo;

/// What a count is of: the kinds of page that head lists of sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum EntityKind {
    /// Everything (History's list): its id is ignored.
    All,
    /// A project.
    Project,
    /// A tool's config folder (an `accounts` row; `SessionsQuery`'s
    /// `account_id`).
    Folder,
    /// An App (`cli:<folder>`, `app:<host>:<instance folder>`).
    App,
    /// An Account, a subscription (`acct:<vendor>:<hash>`).
    Subscription,
    /// A tool, by its adapter id.
    Tool,
    /// A Mac.
    Machine,
    /// Launch settings.
    LaunchSettings,
    /// A pool (AJ-E3104): AI Juggler's sessions only (the conversations
    /// other apps started are in none).
    Pool,
}

/// How many kinds there are.
pub const KINDS: usize = 9;

impl EntityKind {
    /// Every kind.
    pub const ALL: [Self; KINDS] = [
        Self::All,
        Self::Project,
        Self::Folder,
        Self::App,
        Self::Subscription,
        Self::Tool,
        Self::Machine,
        Self::LaunchSettings,
        Self::Pool,
    ];

    /// Its place in a row's relations.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Project => 1,
            Self::Folder => 2,
            Self::App => 3,
            Self::Subscription => 4,
            Self::Tool => 5,
            Self::Machine => 6,
            Self::LaunchSettings => 7,
            Self::Pool => 8,
        }
    }
}

/// One thing a page shows: its kind and id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct EntityRef {
    /// What it is.
    pub kind: EntityKind,
    /// Its id (any for [`EntityKind::All`]).
    pub id: String,
}

/// How many of each related thing one entity has: its sessions (AI
/// Juggler's and the ones other apps started, as its Sessions list holds
/// them), and how many different projects, folders, Apps, Accounts,
/// tools, Macs and launch settings they're on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RelationCountsInfo {
    /// What was asked for.
    pub kind: EntityKind,
    /// Its id.
    pub id: String,
    /// Its sessions: all, open, ended and archived.
    pub sessions: SessionsCountsInfo,
    /// The projects its sessions are in.
    #[ts(type = "number")]
    pub projects: u64,
    /// The folders its sessions run on.
    #[ts(type = "number")]
    pub folders: u64,
    /// The Apps its sessions are in.
    #[ts(type = "number")]
    pub apps: u64,
    /// The Accounts its sessions' folders are signed in to.
    #[ts(type = "number")]
    pub subscriptions: u64,
    /// The tools its sessions run.
    #[ts(type = "number")]
    pub tools: u64,
    /// The Macs its sessions ran on.
    #[ts(type = "number")]
    pub machines: u64,
    /// The launch settings its sessions started with (AI Juggler's only).
    #[ts(type = "number")]
    pub launch_settings: u64,
    /// The pools its sessions are in (AI Juggler's only, AJ-E3104).
    #[ts(type = "number")]
    pub pools: u64,
}

/// Which things of one kind an entity's sessions are on (AJ-E3104: a
/// page's Pools and Apps tabs list them): their ids, sorted, at most
/// [`super::MAX_ASKED`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RelatedIdsInfo {
    /// What was asked for.
    pub kind: EntityKind,
    /// Its id.
    pub id: String,
    /// The kind listed.
    pub of: EntityKind,
    /// Their ids.
    pub ids: Vec<String>,
    /// More than were listed.
    pub more: bool,
}
