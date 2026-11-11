//! The desktop types on the wire (exported to TypeScript).

use serde::Serialize;
use ts_rs::TS;

/// Result of a desktop launch (AJ-E413 Req.1): the tracked Instance id,
/// the OS pid, and the bundle identifier when the platform has one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DesktopLaunchInfo {
    /// The new (desktop-kind) Instance id.
    pub session_id: String,
    /// OS process id.
    pub pid: u32,
    /// Bundle identifier (macOS), when known.
    pub bundle_id: Option<String>,
    /// The per-Provider isolation claim, shown verbatim (Req.1): e.g.
    /// `Single-account app — Juggler tracks, cannot isolate`.
    pub isolation_label: String,
}

/// A focus attempt's honest result (AJ-E414 Req.5): the copy-exact
/// outcome label, whether the Accessibility grant was missing (the
/// attempt degraded to app activation), and the window title the
/// attempt targeted (after first-sight adoption / re-discovery).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FocusOutcomeInfo {
    /// `Focused window` | `App opened — window not found` |
    /// `App not running — Relaunch?` | `macOS didn't bring the app
    /// forward…` (it isn't in front) | platform-unavailable note.
    pub outcome: String,
    /// True when tiers (b)/(c) were impossible without the
    /// Accessibility permission — the UI shows its explainer.
    pub permission_missing: bool,
    /// The window title this attempt targeted, once known.
    pub window_title: Option<String>,
}
