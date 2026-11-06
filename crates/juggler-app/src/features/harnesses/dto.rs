//! The harnesses types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// A Provider the app can manage (ADR-0008; AJ-E107).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProviderInfo {
    /// Provider id, e.g. `claude-code`.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// Deprecated alias (one version). AJ-E107.
pub type HarnessInfo = ProviderInfo;
