//! D-106: an account's plan usage as the tool's own service reports it,
//! read on demand ([`crate::ToolQuota::plan_usage`]).
//!
//! For a tool whose limits live only with its vendor and that answers
//! for them through a local service of its own (Kimi Code's `kimi web`):
//! each window it named, as a Measured reading taken when it answered, and
//! the parts of a window it broke down. A window it left out isn't there
//! (D-06: never assumed); a reading that fails the adapter's format check
//! is an error, never a zero.

use juggler_core::quota::{QuotaSample, WindowKind};

/// One window broken down: the share of it one kind of use took (Kimi
/// Code's month, the part its coding plan used). Not a window of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanPart {
    /// The window it is a part of.
    pub window: WindowKind,
    /// What the part is, in one word (`code`).
    pub part: String,
    /// Its share of the window, percent used, 0–100.
    pub used_pct: f64,
}

/// What the tool's service said of the account's plan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanReading {
    /// Each window it named, as a Measured reading.
    pub windows: Vec<(WindowKind, QuotaSample)>,
    /// The parts of a window it broke down.
    pub parts: Vec<PlanPart>,
}
