//! Process supervision and resource management (spec §5).
//!
//! Owns the lifecycle of every managed harness session: PTY spawning with
//! per-profile environments, the lifecycle event bus, process-tree tracking,
//! the stop ladder, output capture, the supervision journal for crash
//! re-adoption, and (from M2) stuck-detection heuristics and the resource
//! manager. It talks to harnesses only through `juggler_adapter` types.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod error;
pub mod group;
pub mod heuristics;
pub mod journal;
pub mod lifecycle;
pub mod orphans;
pub mod ports;
pub mod ring;
pub mod supervisor;
pub mod tree;

pub use error::SupervisorError;
pub use heuristics::{
    AttentionTransition, HeuristicEngine, HeuristicInputs, HeuristicThresholds, prompt_alive,
    quota_signature,
};
pub use journal::{AdoptedProcess, Journal};
pub use lifecycle::{ExitKind, LifecycleEvent};
pub use orphans::{
    AUTO_CLEANUP_GRACE_MS, DescendantRow, ExitSnapshot, ORPHAN_GRACE_MS, Orphan, OrphanTracker,
    OwnerState, StopOutcome, process_alive, stop_pid_ladder, verify_owner,
};
#[cfg(unix)]
pub use ports::NetstatProbe;
#[cfg(not(unix))]
pub use ports::UnsupportedProbe;
pub use ports::{
    Listener, ListenerClass, PORTS_UNKNOWN_ON_THIS_PLATFORM, PortProbe, PortProbeError, PortRow,
    WELL_KNOWN_DEV_PORTS, classify_listeners, well_known_hint,
};
pub use ring::OutputRing;
pub use supervisor::{
    LiveProcessStats, LiveStatsCollector, ProcessFacts, ProcessRow, ProcessTable, SpawnedProcess,
    Supervisor, TreeSample,
};
pub use tree::{
    LadderOutcome, LadderScope, ProcessOutcome, ladder_platform, live_details, pgid_members,
    terminate_group, verify_start_time,
};

/// Counts every `sysinfo::System` construction in this crate
/// (AJ-E504's regression gate: the reconciler tick must construct at
/// most ONE — zero in the steady state, since the collector reuses
/// its own table and queries read the published snapshot).
static SYSTEM_CONSTRUCTIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Constructs a `sysinfo::System`, bumping the construction counter
/// (the per-tick regression gate reads it — see
/// [`system_construction_count`]).
pub(crate) fn new_system() -> sysinfo::System {
    SYSTEM_CONSTRUCTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    sysinfo::System::new()
}

/// How many `sysinfo::System` objects this crate has constructed —
/// the AJ-E504 quality gate: a reconciler tick (plus its queries)
/// must add at most one; steady state adds zero.
#[must_use]
pub fn system_construction_count() -> usize {
    SYSTEM_CONSTRUCTIONS.load(std::sync::atomic::Ordering::Relaxed)
}
