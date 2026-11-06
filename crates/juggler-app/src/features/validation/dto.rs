//! The validation types on the wire (exported to TypeScript).

/// One Settings → Beta matrix row (the same data the badges derive
/// from — one source, no drift).
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
// A flat table row for one panel: independent facts about one pair, not
// a state machine.
#[allow(
    clippy::struct_excessive_bools,
    reason = "wire DTO: each flag is its own fact the matrix shows"
)]
pub struct ValidationCellInfo {
    /// Source side (a provider id, or one through another service).
    pub from: String,
    /// Target side.
    pub to: String,
    /// The source's tool.
    pub from_tool: String,
    /// The target's tool.
    pub to_tool: String,
    /// AJ-E1601: the source runs its tool through another service.
    pub from_service: bool,
    /// AJ-E1601: the target runs its tool through another service.
    pub to_service: bool,
    /// The switch mechanism at the ceiling (transplant/replay/pack-only).
    pub mechanism: String,
    /// The researched ceiling (E203), wire name.
    pub researched_ceiling: String,
    /// The advertised ceiling after validation gating (a FAILED live
    /// cell lowers it one level until fixed).
    pub advertised: String,
    /// The wire badge ("Full story, fresh start · Beta" / "Same conversation, another app · Proven").
    pub badge: String,
    /// Validated = real-CI row green AND a passing live cell.
    pub validated: bool,
    /// The pair's shape is proven over the fake harness in CI.
    pub mechanism_green: bool,
    /// Both sides' REAL adapters have green fixture rows.
    pub ci_green: bool,
    /// The recorded live cell, when one exists (dates + verdict +
    /// hashes — never secrets).
    pub dogfood: Option<DogfoodCellInfo>,
    /// One-line status in plain words ("Automatic tests pass with the
    /// real tools; not tried by hand yet").
    pub status: String,
    /// Honest notes (blocks, pending adapters), verbatim.
    pub notes: Vec<String>,
}

/// The recorded live cell as the UI reads it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DogfoodCellInfo {
    /// ISO date the cell ran.
    pub at: String,
    /// Project shape.
    pub project_shape: String,
    /// pass | fail.
    pub verdict: String,
    /// sha256 of the source transcript before the switch.
    pub source_sha256: String,
    /// sha256 of the target transcript after the resume.
    pub target_sha256: String,
    /// The continuity note (pass) or the failure reason (fail).
    pub note: String,
}
