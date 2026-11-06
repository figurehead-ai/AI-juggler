//! The cleanup types on the wire (exported to TypeScript).

// --- AJ-E506: the Cleanup Centre (read-only, Tier 0) -------------------

/// One inventory row's verification badge (AJ-E503 at query time).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct VerificationBadge {
    /// `verified` | `unverifiable` | `unknown` (no recorded facts).
    pub verdict: String,
    /// The facts behind the verdict (the expansion's check result).
    pub detail: String,
}

/// The ownership evidence chain (Req.4 — `Why does Juggler think this
/// is mine?`).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct EvidenceChain {
    /// The Instance's root pid (spawned-by).
    pub root_pid: Option<u32>,
    /// First-seen parent pid (the ppid at record time).
    pub parent_pid: Option<u32>,
    /// The row's PGID.
    pub pgid: Option<u32>,
    /// True when the PGID left the root's group (E502's flag).
    pub escaped_group: bool,
    /// The exe/start-time check result (AJ-E503 at query time).
    pub verification: VerificationBadge,
    /// First seen, epoch ms.
    pub first_seen: i64,
    /// Last seen, epoch ms.
    pub last_seen: i64,
}

/// One read-only row (the shared `ProcessRow`'s data). NO kill fields
/// exist on this DTO by construction (Req.7).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupRow {
    /// The machine that owns this row (AJ-E511) — `local_machine`
    /// on the inventory names this node; foreign rows render
    /// display-only (no kill affordance).
    pub machine_id: String,
    /// OS process id.
    pub pid: u32,
    /// Process-group id.
    pub pgid: Option<u32>,
    /// `root` | `helper`.
    pub role: String,
    /// Ledger state (`running` | `orphaned` | `unverifiable` | `exited`).
    pub state: String,
    /// The pass2 §2 label, VERBATIM — except down-period orphans,
    /// which carry the AJ-E510 label `left behind while Juggler was
    /// closed` (their root died while the app was not running).
    pub state_label: String,
    /// True for helpers of roots that died while Juggler was closed
    /// (AJ-E510 Req.2) — the distinct down-period marker.
    pub down_period: bool,
    /// Role guess in plain words ([`role_words`](crate::features::kill::role_words): "dev server",
    /// "watcher"; empty when unknown) — always labelled a guess (E504).
    pub role_guess: String,
    /// Executable path (truncated for display, full on expand).
    pub exe_path: String,
    /// Truncated command line.
    pub cmdline: String,
    /// Age from the kernel start time, ms.
    pub age_ms: Option<i64>,
    /// Live RSS bytes (frozen at last tick for exited rows).
    pub rss_bytes: Option<u64>,
    /// Live CPU%.
    pub cpu_pct: Option<f64>,
    /// Ports held (csv) — the collision evidence.
    pub ports: Option<String>,
    /// True when a sustained hog rule fired (E505).
    pub hog: bool,
    /// Which hog rule(s) fired.
    pub hog_rules: Vec<String>,
    /// The verification badge + evidence chain (expansion).
    pub evidence: EvidenceChain,
}

/// One Instance group (healthy trees collapsed by default).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupGroup {
    /// The Instance (session) id.
    pub session: String,
    /// Provider id.
    pub provider: String,
    /// Account id.
    pub account: Option<String>,
    /// Project id.
    pub project: Option<String>,
    /// Machine id (AJ-E511).
    pub machine: String,
    /// True when the tree is healthy (collapse by default).
    pub healthy: bool,
    /// The account's name ("Work Max"), when known — the group title
    /// names things, never ids.
    pub account_label: Option<String>,
    /// The project's name ("web-app"), when known.
    pub project_name: Option<String>,
    /// The user's name for the session (AJ-E1502), when it has one.
    pub label: Option<String>,
    /// The session's lifecycle status (storage vocabulary: `failed`,
    /// `stopped`, …) — how it ended; `None` without a session record.
    pub status: Option<String>,
    /// When the session started, epoch ms, when known.
    pub started_at_ms: Option<i64>,
    /// True while the session's own process runs.
    pub live: bool,
    /// When the session's own process ended, epoch ms (`None` while
    /// it runs, or when nothing recorded it).
    pub ended_at_ms: Option<i64>,
    /// The rows.
    pub rows: Vec<CleanupRow>,
}

/// One Cleanup Centre history row (AJ-E509: the audit log's read
/// shape, newest first).
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct CleanupHistoryInfo {
    /// The audit history (7 days), newest first.
    pub rows: Vec<juggler_core::storage::AuditOutcomeInfo>,
    /// The copy-exact empty note when there are no records.
    pub empty_note: Option<String>,
}

/// The Cleanup Centre response.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default)]
pub struct CleanupInventoryInfo {
    /// Groups matching the filter.
    pub groups: Vec<CleanupGroup>,
    /// The executing node's machine id (AJ-E511): rows whose
    /// `machine_id` equals this are `This Mac`; anything else is a
    /// future-synced foreign row — display-only.
    pub local_machine: String,
    /// Instances fully clean (the empty-state copy's N).
    pub clean_instances: usize,
    /// True when the ledger is degraded (AJ-E501 banner + read-only).
    pub ledger_degraded: bool,
    /// The History filter's audit rows (AJ-E509 Req.4; `None` for
    /// every other filter).
    pub history: Option<CleanupHistoryInfo>,
}

/// One live Tier-2 notice (AJ-E508 Req.1): the crash-orphan's Undo
/// window. Exactly one per candidate; `keep` (Undo / Keep running)
/// dismisses it forever for that pid.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Tier2Notice {
    /// The candidate pid.
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
    /// Truncated command line (the notification names the process).
    pub cmdline_trunc: String,
    /// When the notice was shown, epoch ms.
    pub shown_at_ms: i64,
    /// The window's end, epoch ms (≥30 s after `shown_at_ms`; no
    /// signal before it).
    pub deadline_ms: i64,
    /// True once the user chose Undo / Keep running.
    pub keep: bool,
}

/// One live Tier-2 notice as the UI reads it.
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct Tier2NoticeInfo {
    /// The candidate pid.
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
    /// Truncated command line.
    pub cmdline_trunc: String,
    /// The window's end, epoch ms.
    #[ts(type = "number")]
    pub deadline_ms: i64,
}

/// One passive Cleanup Centre alert (AJ-E512 Req.3). `role=status`
/// surfaces in the UI — never a focus-stealing dialog.
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CleanupAlertInfo {
    /// `hog` (resource alert, staged-sheet eligible) or `idle` (the
    /// once-per-orphan nudge).
    pub kind: String,
    /// The exact copy (byte-asserted in tests; one language with the
    /// Rust suite).
    pub text: String,
    /// The helper pid the alert names (0 when unresolvable).
    pub pid: u32,
    /// The Instance it belonged to.
    pub session: String,
}

/// The passive alert set: hog alerts (exact template) + idle-orphan
/// nudges (fired at most ONCE per orphan — the once-flag is persisted
/// the moment the nudge is computed).
#[derive(Clone, Debug, serde::Serialize, PartialEq, Default, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CleanupAlertsInfo {
    /// Hog alerts first (actionable), then idle nudges.
    pub alerts: Vec<CleanupAlertInfo>,
}

/// The cleanup automation's status (the centre's banners + the
/// Settings surface read THIS — one truth).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CleanupStatusInfo {
    /// True while `Pause automatic cleanup` is on.
    pub paused: bool,
    /// True while the 24 h crash suspension is active.
    pub suspended: bool,
    /// The suspension's deadline, epoch ms (when `suspended`).
    #[ts(type = "number | null")]
    pub suspended_until_ms: Option<i64>,
    /// The live Tier-2 notices (open Undo windows).
    pub tier2_pending: Vec<Tier2NoticeInfo>,
}
