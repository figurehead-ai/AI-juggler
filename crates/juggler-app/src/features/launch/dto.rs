//! The launch types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// Result of launching a session.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LaunchResult {
    /// The new session id.
    pub session_id: String,
    /// OS process id.
    pub pid: u32,
}
