//! The expert types on the wire (exported to TypeScript).

use ts_rs::TS;

/// AJ-E309 Req.5: what the [Expert setup…] panel shows for one
/// Provider — or one Account (and optionally one of its Launch
/// settings). Every path comes from AI Juggler's own state, never
/// from the caller.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ExpertSetupInfo {
    /// The Provider id.
    pub provider: String,
    /// The Account this describes; `null` = the Provider's folder
    /// (before any Account exists in this flow).
    pub account_id: Option<String>,
    /// The managed state folder (read-only display).
    pub state_dir: String,
    /// Whether that folder exists yet.
    pub exists: bool,
    /// The copyable shell line reproducing the setup (isolation env +
    /// Launch-settings overrides + the vendor's command); `null`
    /// before an Account exists or for Providers without a CLI.
    pub shell_snippet: Option<String>,
    /// Environment keys Launch-settings overrides may never set.
    pub protected_env_keys: Vec<String>,
    /// One plain sentence on what the folder is.
    pub note: String,
}
