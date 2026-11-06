//! The environment types on the wire (exported to TypeScript).

use crate::features::tools::scan_wire::DiscoveredInfo;

/// One probe's result as the UI reads it (Req.4 — health rides along).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct ProbeInfo {
    /// Probe id (`s1-config-dirs` … `s10-desktop`).
    pub id: String,
    /// Human label ("Folders named .claude…").
    pub label: String,
    /// `ok` | `partial` | `denied` | `cancelled`.
    pub health: String,
    /// The reason, when not ok.
    pub reason: Option<String>,
    /// Metadata-only detail (counts, names).
    pub detail: String,
}

/// The widened scan report (AJ-E302): the candidate installs plus the
/// per-probe health, project seeds (S9), and desktop apps (S10).
#[derive(Clone, Debug, serde::Serialize)]
pub struct ScanReportInfo {
    /// Candidate setups (S1/S2/S3/S5 origins).
    pub installs: Vec<DiscoveredInfo>,
    /// All ten probes with health (Req.4).
    pub probes: Vec<ProbeInfo>,
    /// Project seed names, dirs only (S9).
    pub seeds: Vec<String>,
    /// Desktop apps present, names only (S10).
    pub desktop_apps: Vec<String>,
    /// True when the scan was cancelled partway (the UI says so —
    /// never "no setups found").
    pub cancelled: bool,
}
