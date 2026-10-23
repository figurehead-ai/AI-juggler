//! A supervised tree's live sums (AJ-E401 read model).

/// Live process statistics for one supervised Instance (AJ-E401 read
/// model). Sums cover the root process plus its descendants; CPU% is
/// only reported once the collector has two table samples (a single
/// refresh cannot measure a delta).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LiveProcessStats {
    /// Descendant process count (the root itself is not counted).
    pub child_count: u32,
    /// CPU utilization summed over the tree, percent; `None` until the
    /// second table refresh (never a fabricated 0.0).
    pub cpu_pct: Option<f64>,
    /// Resident memory summed over the tree, bytes.
    pub rss_bytes: u64,
}

impl LiveProcessStats {
    /// Definitive zeros for a session whose process has exited — nothing
    /// can be running or resident after exit.
    #[must_use]
    pub fn dead() -> Self {
        Self {
            child_count: 0,
            cpu_pct: Some(0.0),
            rss_bytes: 0,
        }
    }

    /// These sums when something was measured: a running process always
    /// has resident memory, so zero means its process has exited (or has
    /// no sample yet) — never "under 1 MB" (AJ-E1535).
    #[must_use]
    pub fn measured(self) -> Option<Self> {
        (self.rss_bytes > 0).then_some(self)
    }
}
