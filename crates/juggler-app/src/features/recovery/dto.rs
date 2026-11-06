//! The recovery types on the wire (exported to TypeScript).

/// The one-time recovery summary (AJ-E510 Req.3): present exactly
/// once per crash — the second relaunch (no new findings) re-shows
/// nothing.
#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub struct RecoverySummaryInfo {
    /// The exact template text.
    pub summary: String,
    /// `{a}` — Instances re-adopted (stoppable through the E507 path).
    pub adopted: usize,
    /// `{b}` — helpers left behind while Juggler was closed.
    pub left_behind: usize,
    /// `{c}` — processes that could not be verified (left alone).
    pub unverifiable: usize,
}
