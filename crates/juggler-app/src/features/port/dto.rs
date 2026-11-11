//! The port types on the wire (exported to TypeScript).

/// One port-inventory row (the API DTO; `NotManaged` rows carry
/// `killable: false` and NO stop/kill command ids — the F3 guard).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PortRowInfo {
    /// The listening port (the headline).
    pub port: u16,
    /// The owning process id (the evidence).
    pub pid: u32,
    /// `managed-orphan` | `managed-running` | `re-adopted` |
    /// `not-managed`.
    pub class: String,
    /// Owning Instance, when managed.
    pub session: Option<String>,
    /// Provider id, when managed.
    pub provider: Option<String>,
    /// Account id, when managed.
    pub account: Option<String>,
    /// Project id, when managed.
    pub project: Option<String>,
    /// Listener age from the kernel process start, ms.
    #[ts(type = "number | null")]
    pub listener_age_ms: Option<i64>,
    /// Process display name, when managed.
    pub name: Option<String>,
    /// The well-known dev-port hint (a LABEL, never identity).
    pub hint: Option<String>,
    /// False exactly for `not-managed` rows: no kill affordance, ever.
    pub killable: bool,
}

/// One sustained-breach hog fact (Req.5 — the Cleanup Centre's input).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct HogFactInfo {
    /// The hog pid (helper for RSS rules, the root for tree rules).
    pub pid: u32,
    /// Owning Instance.
    pub session: String,
    /// Provider id.
    pub provider: String,
    /// Live (or last-seen) RSS, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// Live CPU%, when sampled.
    pub cpu_pct: Option<f64>,
    /// Which rule fired: `helper-rss>2GiB` | `tree-cpu>90%` |
    /// `tree-rss>p95(provider)`.
    pub rule: String,
}

/// The inventory response: rows + the honest probe note + hog facts.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PortInventoryInfo {
    /// Rows sorted by port.
    pub rows: Vec<PortRowInfo>,
    /// `Some` when the platform has no probe or the OS call failed —
    /// the honest note, never an empty map presented as truth.
    pub note: Option<String>,
    /// Sustained-breach hogs (60 s window).
    pub hogs: Vec<HogFactInfo>,
}

/// The pre-launch collision warning (advisor informs, never blocks).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PortWarningInfo {
    /// The contested port.
    pub port: u16,
    /// The orphan holding it.
    pub pid: u32,
    /// The Instance label (`Provider (id-prefix)`).
    pub instance: String,
    /// How long the holder has lived, ms.
    #[ts(type = "number")]
    pub age_ms: i64,
    /// The copy-exact warning (Req.4).
    pub message: String,
}
