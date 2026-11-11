//! The providers types on the wire (exported to TypeScript).

/// One Step-0 row (AJ-E301 Req.2): `Installed vX.Y` / `Not installed`
/// / `Installed (vX, unverified)`.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProviderInstallInfo {
    /// Provider id (the wizard's `provider` parameter).
    pub provider: String,
    /// Display name ("Claude Code").
    pub display_name: String,
    /// Vendor install page.
    pub install_url: String,
    /// `installed` | `not_installed` | `unverified` | `probe_failed` |
    /// `found` (found from its files, its version not asked: this run
    /// starts no tool, the read-only test bridge).
    pub status: String,
    /// The reported version, when a binary was found.
    pub version: Option<String>,
    /// Resolved binary path, when found.
    pub binary: Option<String>,
    /// Discovery notes (drift details), verbatim.
    pub notes: Vec<String>,
    /// AJ-E306 flow (c): the verified version range(s) — the card's
    /// badge; empty renders "unverified" honestly.
    pub verified_against: Vec<String>,
    /// `terminal` or `desktop` (the kind badge, Req.3).
    #[ts(type = "\"terminal\" | \"desktop\"")]
    pub kind: String,
    /// The single-instance desktop limit, stated BEFORE selection
    /// (Req.3) — the adapter's isolation claim verbatim.
    pub single_instance_note: Option<String>,
}

/// What a registered Provider supports, from the registry alone — no
/// install probe, so it is cheap to ask on every screen that needs it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProviderCapabilityInfo {
    /// Provider id.
    pub provider: String,
    /// ADR-0018: its setups can be used where they are (the adapter
    /// adopts a folder without writing to it).
    pub can_use_in_place: bool,
    /// It reports its own usage limits (the 5-hour and weekly windows);
    /// a tool that doesn't shows no limit rows (0.0.11 walk, Q3).
    pub reports_limits: bool,
}
