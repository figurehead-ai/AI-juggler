//! Projects as a feature module (ADR-0019 §3). Their wire type lives
//! here; the commands move from `commands.rs` with AJ-E2017.

pub mod git;
pub mod icon;
pub mod mark;

/// A tracked project for the UI — the juggling anchor (ADR-0007).
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProjectInfo {
    /// Project id.
    pub id: String,
    /// Canonical root directory.
    pub root: String,
    /// Display name.
    pub name: String,
    /// Last activity, epoch milliseconds (when known).
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// The instance currently managing this project, when live.
    pub active_instance: Option<crate::commands::InstanceInfo>,
    /// Account labels that have ever run in this project (AI Juggler runs
    /// plus imported pre-existing history).
    pub accounts_used: Vec<String>,
    /// Total instances run through AI Juggler in this project.
    #[ts(type = "number")]
    pub instance_count: u64,
    /// Sessions discovered in imported transcripts (pre-AI-Juggler work).
    #[ts(type = "number")]
    pub history_sessions: u64,
    /// When the user last worked here outside AI Juggler (the newest
    /// imported transcript's write time), epoch ms.
    #[ts(type = "number | null")]
    pub history_last_used_ms: Option<i64>,
    /// The repository this folder is a checkout of (its `origin` remote,
    /// normalised: `github.com/owner/name`), so clones group together.
    pub repo: Option<String>,
    /// True when the root directory no longer exists on disk (AJ-E408
    /// Overview danger banner + `Relocate…`).
    pub missing: bool,
    /// AJ-E1501: the account this project's new sessions start with,
    /// when the user picked one.
    #[ts(optional = nullable)]
    pub default_account_id: Option<String>,
    /// AJ-E1502: pinned to the top of the rail.
    #[ts(optional)]
    pub pinned: Option<bool>,
    /// AJ-E1502: forgotten — hidden from the rail and New session until
    /// it is used again.
    #[ts(optional)]
    pub forgotten: Option<bool>,
    /// AJ-E1532: its mark — its own icon, or letters on a colour — the
    /// same on every screen. Set once its repository is known.
    #[ts(optional)]
    pub mark: Option<mark::ProjectMarkInfo>,
}
