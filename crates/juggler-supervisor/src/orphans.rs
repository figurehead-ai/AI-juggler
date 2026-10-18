//! The orphan (left-behind) contract (AJ-E410, D-08): when an Instance
//! ends in any state except a clean stop, its recorded descendant set is
//! snapshotted; survivors past a grace window surface as
//! `Orphaned - left behind (N)` rows with verified stop actions.
//!
//! Ownership is **ppid-map + journal only** — never process groups and
//! never name-based matching. A PID that was not recorded as a
//! descendant of the ended Instance is untouchable in every code path
//! (fail closed). PID-identity hardening (start-time/argv verification
//! against pid reuse) is the E5 cleanup epic's seam
//! (`docs/SPEC.md` §5.2).

use std::collections::HashMap;
use std::time::Duration;

use juggler_core::model::SessionId;

/// Survivors of a non-clean exit surface after this window
/// (pass2/04 §5.1 orphan contract).
pub const ORPHAN_GRACE_MS: i64 = 10_000;

/// Tier-1 auto-cleanup grace: only Accounts the user opted in are
/// touched, and only this long after the Instance ended (D-08).
pub const AUTO_CLEANUP_GRACE_MS: i64 = 60_000;

/// One descendant recorded at snapshot time (the allow-list row).
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
    /// The recorded descendant rows (the kill allow-list).
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

/// Outcome of the staged TERM → grace → KILL ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The process is gone. `escalated` records whether SIGKILL was
    /// needed (TERM alone sufficed when false).
    Gone {
        /// True when the ladder had to escalate to SIGKILL.
        escalated: bool,
    },
    /// Still alive after SIGKILL (e.g. an unreapable zombie) — the UI
    /// renders `still alive - escalated to SIGKILL`.
    StillAlive,
    /// The PID is not in the recorded descendant set — refused, fail
    /// closed. Name-based or unrecorded kills never happen.
    RefusedNotRecorded,
}

impl StopOutcome {
    /// The UI state string for this outcome.
    #[must_use]
    pub fn state_label(&self) -> &'static str {
        match self {
            Self::Gone { .. } => "gone",
            Self::StillAlive => "still alive - escalated to SIGKILL",
            Self::RefusedNotRecorded => "refused - not a recorded descendant",
        }
    }
}

/// Owner verification verdict for one PID (the `Verify owner` action).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerState {
    /// The process no longer exists.
    Gone,
    /// Alive; `ppid` re-read now (1/launchd = re-parented orphan).
    Alive {
        /// Current parent pid, when readable.
        ppid: Option<u32>,
    },
}

/// Tracks exit snapshots → confirmed orphans, plus per-PID stop states
/// for the UI's re-read display. All state is in-memory; the journal
/// carries the durable record (descendant snapshots + kill audit).
#[derive(Debug, Default)]
pub struct OrphanTracker {
    inner: std::sync::Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    pending: Vec<ExitSnapshot>,
    orphans: Vec<Orphan>,
    /// Last stop outcome per pid — the row's verification state.
    stops: HashMap<u32, StopOutcome>,
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
            // rest are gone. Orphans already stopped are removed from
            // `orphans` when their stop outcome lands (record_stop).
        }
        inner.pending = still_pending;
        inner.orphans.extend(confirmed.clone());
        confirmed
    }

    /// True when `pid` was recorded as a descendant of `session`'s exit
    /// snapshot or is a live orphan of it — the kill allow-list.
    #[must_use]
    pub fn allows(&self, session: &SessionId, pid: u32) -> bool {
        let Ok(inner) = self.inner.lock() else {
            return false;
        };
        let in_pending = inner.pending.iter().any(|snapshot| {
            snapshot.session == *session && snapshot.rows.iter().any(|row| row.pid == pid)
        });
        let in_orphans = inner
            .orphans
            .iter()
            .any(|orphan| orphan.session == *session && orphan.pid == pid);
        in_pending || in_orphans
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

    /// Records a stop outcome for a pid and drops it from the live
    /// orphan rows when gone (the UI keeps the state string).
    pub fn record_stop(&self, pid: u32, outcome: StopOutcome) {
        if let Ok(mut inner) = self.inner.lock() {
            if matches!(outcome, StopOutcome::Gone { .. }) {
                inner.orphans.retain(|orphan| orphan.pid != pid);
            }
            inner.stops.insert(pid, outcome);
        }
    }

    /// The last stop state for a pid, if any (the row's verification
    /// display source).
    #[must_use]
    pub fn stop_state(&self, pid: u32) -> Option<StopOutcome> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.stops.get(&pid).cloned())
    }
}

/// The staged stop ladder for one PID: SIGTERM, wait `term_grace`,
/// then SIGKILL. Liveness is injected so tests stay hermetic; the
/// production checker is [`process_alive`]. `still_alive_after_kill_ms`
/// bounds the post-KILL re-read window.
///
/// The caller MUST have verified the PID against the recorded
/// descendant set first (fail closed — this function signals, it does
/// not authorize).
pub fn stop_pid_ladder<F>(pid: u32, term_grace: Duration, alive: F) -> StopOutcome
where
    F: Fn(u32) -> bool,
{
    send_signal(pid, &Signal::Term);
    let deadline = std::time::Instant::now() + term_grace;
    while std::time::Instant::now() < deadline {
        if !alive(pid) {
            return StopOutcome::Gone { escalated: false };
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    send_signal(pid, &Signal::Kill);
    // Post-KILL re-read: bounded wait for the process to disappear.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if !alive(pid) {
            return StopOutcome::Gone { escalated: true };
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    StopOutcome::StillAlive
}

enum Signal {
    Term,
    Kill,
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: &Signal) {
    use nix::sys::signal::{Signal as NixSignal, kill};
    use nix::unistd::Pid;
    let Some(pid) = i32::try_from(pid).ok().map(Pid::from_raw) else {
        return;
    };
    let signal = match signal {
        Signal::Term => NixSignal::SIGTERM,
        Signal::Kill => NixSignal::SIGKILL,
    };
    let _ = kill(pid, Some(signal));
}

#[cfg(not(unix))]
fn send_signal(pid: u32, signal: &Signal) {
    // Windows seam (E5, Job objects): TERM does not exist; KILL maps to
    // a hard terminate. Until then, non-unix builds no-op.
    let _ = (pid, signal);
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

/// Owner verification: re-reads the process's current parent.
#[must_use]
pub fn verify_owner(pid: u32) -> OwnerState {
    let system = fresh_table(&[pid]);
    match system.process(sysinfo::Pid::from_u32(pid)) {
        None => OwnerState::Gone,
        Some(process) if zombie(process) => OwnerState::Gone,
        Some(process) => OwnerState::Alive {
            ppid: process.parent().map(sysinfo::Pid::as_u32),
        },
    }
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
        ProcessRefreshKind::nothing().with_memory(),
    );
    system
}

// Owner-state display helper.
impl OwnerState {
    /// Human note for the UI.
    #[must_use]
    pub fn note(&self) -> String {
        match self {
            Self::Gone => "process is gone".to_string(),
            Self::Alive { ppid: Some(1 | 0) } => {
                "alive — re-parented to init/launchd (its parent exited)".to_string()
            }
            Self::Alive { ppid: Some(parent) } => {
                format!("alive — parent pid {parent}")
            }
            Self::Alive { ppid: None } => "alive — parent not readable".to_string(),
        }
    }
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
    fn allow_list_only_covers_recorded_descendants() {
        let tracker = OrphanTracker::new();
        let s = session("s3");
        tracker.record_exit(snapshot(&s, false, &[31], 1_000));
        tracker.poll(60_000, |pid| (pid == 31).then(|| ("sleep".into(), 1)));
        assert!(tracker.allows(&s, 31));
        assert!(!tracker.allows(&s, 999)); // unrelated pid: refused
        let other = session("other");
        assert!(!tracker.allows(&other, 31)); // right pid, wrong session
    }

    #[test]
    fn stop_states_display_and_drop_gone_rows() {
        let tracker = OrphanTracker::new();
        tracker.record_stop(41, StopOutcome::Gone { escalated: false });
        assert_eq!(tracker.stop_state(41).unwrap().state_label(), "gone");
        tracker.record_stop(42, StopOutcome::StillAlive);
        assert_eq!(
            tracker.stop_state(42).unwrap().state_label(),
            "still alive - escalated to SIGKILL"
        );
        assert_eq!(
            StopOutcome::RefusedNotRecorded.state_label(),
            "refused - not a recorded descendant"
        );
    }
}
