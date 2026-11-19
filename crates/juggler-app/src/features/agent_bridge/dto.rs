//! The agent bridge's wire type for the app's own window.

use serde::Serialize;
use ts_rs::TS;

/// D-30: whether sessions AI Juggler starts get the agent tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AgentBridgeInfo {
    /// The one-time question was answered.
    pub asked: bool,
    /// The answer was yes (or the switch is on).
    pub on: bool,
    /// This build has the bridge program beside it, safe to run.
    pub available: bool,
    /// AJ-E3013: why not, when it isn't: `missing` (nothing to run beside
    /// AI Juggler) | `unsafe_location` (it's there, in a folder another
    /// user owns or can write, so it's never run). Optional on the wire,
    /// so the window's own literals stay as they are.
    #[ts(optional = nullable)]
    pub unavailable: Option<String>,
    /// With `unsafe_location`: the folder that failed the check, the home
    /// folder written `~`.
    #[ts(optional = nullable)]
    pub unsafe_folder: Option<String>,
    /// Projects the user switched off.
    pub off_projects: Vec<String>,
    /// Profiles (account folders) the user switched off.
    pub off_profiles: Vec<String>,
}
