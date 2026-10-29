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
    /// This build has the bridge program beside it.
    pub available: bool,
    /// Projects the user switched off.
    pub off_projects: Vec<String>,
    /// Profiles (account folders) the user switched off.
    pub off_profiles: Vec<String>,
}
