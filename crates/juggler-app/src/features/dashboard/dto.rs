//! The dashboard types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// Dashboard counts.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StatsInfo {
    /// Managed accounts.
    #[ts(type = "number")]
    pub accounts: u64,
    /// Configured profiles.
    #[ts(type = "number")]
    pub profiles: u64,
    /// Sessions not in a terminal state.
    #[ts(type = "number")]
    pub active_sessions: u64,
    /// AJ-E501: a configured process ledger failed — cleanup is
    /// read-only until restart (the degraded banner's data).
    pub ledger_degraded: bool,
}
