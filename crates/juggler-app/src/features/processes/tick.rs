//! AJ-E410 Req.3: the exit snapshot and the processes tick.

use std::sync::Arc;

use juggler_core::model::{SessionId, Timestamp};
use juggler_supervisor::ExitKind;

use crate::features::cleanup::sweep::auto_cleanup_sweep;
use crate::state::AppState;

/// The exit-snapshot hook (AJ-E410 Req.3): journals the pre-exit
/// descendant rows (the durable allow-list). AJ-E2407: the in-memory
/// tracker it once fed is retired; leftovers are the ledger's. Called by the lifecycle forwarder on every Exited event
/// (clean exits are recorded too — they simply never surface orphans).
pub fn record_exit_snapshot(
    state: &Arc<AppState>,
    session: &SessionId,
    root_pid: u32,
    kind: &ExitKind,
) {
    let (rows, root_program) = snapshot_descendants(state, session, root_pid);
    let _ = root_program;
    if let Some(journal) = state.supervisor.journal_ref() {
        let _ = journal.record_descendants(session, root_pid, kind.is_success(), &rows);
    }
}

/// The frozen pre-exit rows, split into (descendants, root program name).
/// The root is dead already — its name survives only in the collector's
/// last tick while it was alive, which is exactly what this reads.
fn snapshot_descendants(
    state: &Arc<AppState>,
    session: &SessionId,
    root_pid: u32,
) -> (Vec<juggler_supervisor::DescendantRow>, String) {
    let Some(collector) = state.resources.lock().ok() else {
        return (Vec::new(), "the ended session".to_string());
    };
    let frozen = collector.rows(session).unwrap_or_default();
    let root_program = frozen
        .iter()
        .find(|row| row.pid == root_pid)
        .map_or_else(|| "the ended session".to_string(), |row| row.name.clone());
    let rows = frozen
        .into_iter()
        .filter(|row| row.pid != root_pid)
        .map(|row| juggler_supervisor::DescendantRow {
            pid: row.pid,
            name: row.name,
            rss_bytes: row.rss_bytes,
        })
        .collect();
    (rows, root_program)
}

/// The 2 s sampling tick (AJ-E410 Req.1): collector tick (targeted
/// refresh + periodic discovery), orphan grace poll with fresh liveness
/// probes, then the Tier-1 auto-cleanup sweep for opted-in Accounts
/// (60 s grace, recorded-descendant PIDs only — D-08).
pub fn processes_tick(state: &Arc<AppState>) {
    let now = Timestamp::now().millis();
    {
        let Ok(mut collector) = state.resources.lock() else {
            return;
        };
        collector.tick(&state.supervisor, now);
        // AJ-E2407: the E410 tracker's own detection (its ppid snapshot,
        // its 10 s clock) is retired: what a session left behind is the
        // reconciler's verdict in the ledger, which the Processes view
        // reads (and counts, `features::processes::helper_orphaned`).
    }
    // AJ-E1401: the machine's own signals, on the same tick (its own
    // lock — never under the collector's).
    crate::machine_health::tick(state, now);
    auto_cleanup_sweep(state, now);
}
