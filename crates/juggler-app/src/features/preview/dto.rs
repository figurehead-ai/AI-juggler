//! The preview types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// Wire payload for one preview frame (AJ-E415): a base64 PNG of the
/// Instance's window plus its capture time (frame age) and sequence.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PreviewFrameInfo {
    /// Owning Instance id.
    pub session_id: String,
    /// Base64-encoded PNG bytes of the captured window.
    pub png_b64: String,
    /// Capture time, epoch milliseconds (the panel labels frame age).
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Monotonic frame sequence.
    #[ts(type = "number")]
    pub seq: u64,
}

/// Wire payload for a preview phase change (AJ-E415): the phase name
/// (`live|paused|denied|unsupported`) and the copy-exact panel text.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PreviewStateInfo {
    /// Owning Instance id.
    pub session_id: String,
    /// `live` | `paused` | `denied` | `unsupported`.
    pub phase: String,
    /// Copy-exact text the panel renders for the phase (empty for live).
    pub text: String,
}
