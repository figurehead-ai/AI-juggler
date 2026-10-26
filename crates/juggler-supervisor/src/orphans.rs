//! The orphan (left-behind) contract (AJ-E410, D-08): when an Instance
//! ends in any state except a clean stop, its recorded descendant set is
//! snapshotted; survivors past a grace window are counted as left
//! behind (the `leftover.detected` measure).
//!
//! This tracker DETECTS only: it holds no stop logic and sends no
//! signal. Every stop of a left-behind process goes through the one
//! verified path (AJ-E507's sheet: identity of pid + start time + exe +
//! boot, ledger ownership, audit before the signal, TERM → grace →
//! KILL), which reads the process ledger, not this tracker.

use juggler_core::model::SessionId;

/// Survivors of a non-clean exit surface after this window
/// (pass2/04 §5.1 orphan contract).
pub const ORPHAN_GRACE_MS: i64 = 10_000;

/// Tier-1 auto-cleanup grace: only Accounts the user opted in are
/// touched, and only this long after the Instance ended (D-08).
pub const AUTO_CLEANUP_GRACE_MS: i64 = 60_000;

/// One descendant recorded at snapshot time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendantRow {
    /// OS process id.
    pub pid: u32,
    /// Process name at snapshot time.
    pub name: String,
    /// Resident memory at snapshot time, bytes.
    pub rss_bytes: u64,
}

/// The descendant snapshot taken when an Instance's process exited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitSnapshot {
    /// The ended Instance.
    pub session: SessionId,
    /// The dead root process id.
    pub root_pid: u32,
    /// The dead root's program name (the parent label shown on rows).
    pub root_program: String,
    /// True for clean exits (code 0) — those never surface orphans
    /// (the contract targets every state except a clean stop).
    pub clean: bool,
    /// Snapshot time, epoch ms.
    pub taken_ms: i64,
    /// The recorded descendant rows.
    pub rows: Vec<DescendantRow>,
}

/// A confirmed orphan: a recorded descendant still alive past grace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    /// The Instance it was left behind by.
    pub session: SessionId,
    /// The dead root process id.
    pub root_pid: u32,
    /// Human label for the dead parent (its program name).
    pub parent_label: String,
    /// Orphan process id.
    pub pid: u32,
    /// Process name.
    pub name: String,
    /// Resident memory, bytes (re-read at confirmation).
    pub rss_bytes: u64,
    /// When the parent exited, epoch ms (age-since-parent-exit source).
    pub parent_exit_ms: i64,
}

/// Tracks exit snapshots → confirmed orphans. All state is in-memory;
/// the journal carries the durable record (descendant snapshots).
#[derive(Debug, Default)]
pub struct OrphanTracker {
    inner: std::sync::Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    pending: Vec<ExitSnapshot>,
    orphans: Vec<Orphan>,
}

impl OrphanTracker {
    /// An empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an exit snapshot. Every exit is recorded (the journal
    /// mirrors it for audit); only non-clean ones can produce orphans.
    pub fn record_exit(&self, snapshot: ExitSnapshot) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.pending.push(snapshot);
        }
    }

    /// Advances grace windows: pending snapshots past
    /// [`ORPHAN_GRACE_MS`] are confirmed against live process state and
    /// surface as orphans (clean exits are dropped). `lookup` returns
    /// `(name, rss)` for a live non-zombie pid, `None` when gone.
    /// Returns the newly confirmed orphans.
    pub fn poll<F>(&self, now_ms: i64, lookup: F) -> Vec<Orphan>
    where
        F: Fn(u32) -> Option<(String, u64)>,
    {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };
        // Confirmed orphans that have gone (stopped through the
        // verified path, or exited) stop being watched.
        inner.orphans.retain(|orphan| lookup(orphan.pid).is_some());
        let mut confirmed = Vec::new();
        let mut still_pending = Vec::new();
        for snapshot in inner.pending.drain(..) {
            if now_ms - snapshot.taken_ms < ORPHAN_GRACE_MS {
                still_pending.push(snapshot);
                continue;
            }
            if snapshot.clean || snapshot.rows.is_empty() {
                continue; // clean exits never surface; empty snapshots drop
            }
            for row in &snapshot.rows {
                if let Some((name, rss_bytes)) = lookup(row.pid) {
                    confirmed.push(Orphan {
                        session: snapshot.session.clone(),
                        root_pid: snapshot.root_pid,
                        parent_label: snapshot.root_program.clone(),
                        pid: row.pid,
                        name,
                        rss_bytes,
                        parent_exit_ms: snapshot.taken_ms,
                    });
                }
            }
            // The snapshot is consumed: survivors became orphans, the
            // rest are gone. Orphans that go later drop on the next poll.
        }
        inner.pending = still_pending;
        inner.orphans.extend(confirmed.clone());
        confirmed
    }

    /// Confirmed orphans for one Instance (row order: as confirmed).
    #[must_use]
    pub fn orphans_of(&self, session: &SessionId) -> Vec<Orphan> {
        self.inner
            .lock()
            .map(|inner| {
                inner
                    .orphans
                    .iter()
                    .filter(|orphan| orphan.session == *session)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every pid the tracker still watches (pending snapshot rows +
    /// confirmed orphans) — the liveness-probe set for tick time.
    #[must_use]
    pub fn tracked_pids(&self) -> Vec<u32> {
        self.inner
            .lock()
            .map(|inner| {
                inner
                    .pending
                    .iter()
                    .flat_map(|snapshot| snapshot.rows.iter().map(|row| row.pid))
                    .chain(inner.orphans.iter().map(|orphan| orphan.pid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All confirmed orphans across Instances (the Cleanup-Centre feed).
    #[must_use]
    pub fn all_orphans(&self) -> Vec<Orphan> {
        self.inner
            .lock()
            .map(|inner| inner.orphans.clone())
            .unwrap_or_default()
    }
}

/// Live, non-zombie process lookup via a targeted sysinfo refresh:
/// returns `(name, rss)` or `None`. One pid — one probe.
#[must_use]
pub fn process_alive(pid: u32) -> Option<(String, u64)> {
    let system = fresh_table(&[pid]);
    system
        .process(sysinfo::Pid::from_u32(pid))
        .filter(|process| !zombie(process))
        .map(|process| {
            (
                process.name().to_string_lossy().into_owned(),
                process.memory(),
            )
        })
}

/// Is this process a zombie/dead entry (not killable, not consuming)?
fn zombie(process: &sysinfo::Process) -> bool {
    matches!(
        process.status(),
        sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead
    )
}

/// A one-shot sysinfo table refreshed for exactly these pids.
fn fresh_table(pids: &[u32]) -> sysinfo::System {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let mut system = crate::new_system();
    let targets: Vec<sysinfo::Pid> = pids
        .iter()
        .map(|pid| sysinfo::Pid::from_u32(*pid))
        .collect();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&targets),
        true,
        ProcessRefreshKind::nothing().without_tasks().with_memory(),
    );
    system
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str) -> SessionId {
        SessionId::new(id)
    }

    fn snapshot(session: &SessionId, clean: bool, pids: &[u32], taken_ms: i64) -> ExitSnapshot {
        ExitSnapshot {
            session: session.clone(),
            root_pid: 100,
            root_program: "fake-harness".into(),
            clean,
            taken_ms,
            rows: pids
                .iter()
                .map(|pid| DescendantRow {
                    pid: *pid,
                    name: format!("proc-{pid}"),
                    rss_bytes: 1024,
                })
                .collect(),
        }
    }

    #[test]
    fn grace_constants_match_the_contract() {
        assert_eq!(ORPHAN_GRACE_MS, 10_000);
        assert_eq!(AUTO_CLEANUP_GRACE_MS, 60_000);
    }

    #[test]
    fn non_clean_exit_survivors_surface_after_grace() {
        let tracker = OrphanTracker::new();
        let s = session("s1");
        tracker.record_exit(snapshot(&s, false, &[11, 12], 1_000));
        // Before grace: nothing (12 is "gone").
        let confirmed = tracker.poll(6_000, |pid| (pid == 11).then(|| ("sleep".into(), 2048)));
        assert!(confirmed.is_empty());
        assert!(tracker.orphans_of(&s).is_empty());
        // Past grace: 11 alive -> orphan; 12 gone -> dropped.
        let confirmed = tracker.poll(11_500, |pid| (pid == 11).then(|| ("sleep".into(), 2048)));
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].pid, 11);
        assert_eq!(confirmed[0].parent_label, "fake-harness");
        assert_eq!(confirmed[0].parent_exit_ms, 1_000);
        assert_eq!(tracker.orphans_of(&s).len(), 1);
    }

    #[test]
    fn clean_exit_never_surfaces_orphans() {
        let tracker = OrphanTracker::new();
        let s = session("s2");
        tracker.record_exit(snapshot(&s, true, &[21], 1_000));
        let confirmed = tracker.poll(60_000, |pid| (pid == 21).then(|| ("sleep".into(), 1)));
        assert!(confirmed.is_empty());
        assert!(tracker.orphans_of(&s).is_empty());
    }

    #[test]
    fn gone_orphans_stop_being_watched() {
        let tracker = OrphanTracker::new();
        let s = session("s3");
        tracker.record_exit(snapshot(&s, false, &[31], 1_000));
        tracker.poll(11_500, |pid| (pid == 31).then(|| ("sleep".into(), 1)));
        assert_eq!(tracker.tracked_pids(), vec![31]);
        // Stopped through the verified path (or exited): the next poll
        // drops it, so the tick never probes a dead pid again.
        tracker.poll(14_000, |_| None);
        assert!(tracker.orphans_of(&s).is_empty());
        assert!(tracker.tracked_pids().is_empty());
    }
}
