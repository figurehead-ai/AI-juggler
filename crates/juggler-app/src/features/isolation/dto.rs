//! The isolation types on the wire (exported to TypeScript).

/// AJ-E308 Req.7: one named isolation-checklist row.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct IsolationRowInfo {
    /// Stable row id (`managed_dir_private`, …).
    pub id: &'static str,
    /// The user-facing line (exactly what was checked).
    pub label: String,
    /// True when the check passed; a fail BLOCKS \[Launch\], not \[Done\].
    pub pass: bool,
    /// Evidence/note copy (the failing line names itself).
    pub detail: String,
}
