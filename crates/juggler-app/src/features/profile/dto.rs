//! The profile types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// A Launch settings row for the UI (AJ-E107).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LaunchRecipeInfo {
    /// Profile id.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Owning account id.
    pub account_id: String,
    /// Owning Account label (denormalized for the list).
    pub account_label: String,
    /// Provider id (wire field `harness` aliased one version; AJ-E107).
    #[serde(alias = "harness")]
    pub provider: String,
    /// Preferred model from the overlay, if any.
    pub model: Option<String>,
    /// Default working directory, if any.
    pub default_cwd: Option<String>,
    /// AJ-E1509: restart its crashed sessions — `null` follows Settings ›
    /// Sessions.
    pub restart_on_crash: Option<bool>,
}

/// Deprecated alias (one version). AJ-E107.
pub type ProfileInfo = LaunchRecipeInfo;
