//! The live stats collector (AJ-E410, AJ-E2011): samples every
//! supervised process tree — CPU, memory, children, listening ports —
//! on the 2 s tick, keeps each tree's short history ring, and observes
//! the trees the app didn't start. Moved out of `supervisor.rs`
//! (AJ-E1544) so that file fits its size budget; behaviour unchanged.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use juggler_core::model::SessionId;

use crate::lifecycle::LifecycleEvent;
use crate::stats::LiveProcessStats;
use crate::supervisor::{OUTPUT_RETENTION_AFTER_EXIT_MS, Supervisor};

/// Listener-table ENUMERATION cadence (AJ-E505): every 5th tick
/// (10 s) — enumeration costs ~90 ms on a loaded box and the tick
/// budget is ≤100 ms p95; the per-tick JOIN uses the cached table and
/// query paths always probe fresh.
const PORT_PROBE_EVERY: u32 = 5;

/// One live process in a supervised tree (the Processes tab row,
/// AJ-E410). `cpu_pct` stays `None` until the second table sample.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessRow {
    /// OS process id.
    pub pid: u32,
    /// Parent pid (`None` when unreadable); the root's own parent.
    pub parent_pid: Option<u32>,
    /// Process name.
    pub name: String,
    /// CPU utilization, percent; `None` before two samples.
    pub cpu_pct: Option<f64>,
    /// Resident memory, bytes.
    pub rss_bytes: u64,
    /// Process age, ms (from the kernel's start time).
    pub age_ms: u64,
    /// Direct children of this process (within the tree).
    pub child_count: u32,
    /// Tree depth (root = 0) — the UI's indent level.
    pub depth: u8,
    /// Kernel process start time, epoch seconds (AJ-E504: the helper
    /// rows' identity fact — the ledger records THIS, never a
    /// wall-clock derivation).
    pub start_time: i64,
    /// Kernel-reported executable path (`None` when unreadable — then
    /// identity verification honestly skips the exe fact).
    pub exe: Option<String>,
    /// The full command line, joined (`None` when unreadable). The
    /// ledger's helper rows record THIS (AJ-E508: the automatic
    /// tier's role gate decides on the true command line, never a
    /// bare process name — a name like `Python` guesses nothing).
    pub cmd: Option<String>,
}

/// The reconciler's cached process table (AJ-E504 Req.1): the ppid map
/// plus per-pid identity facts, built ONCE per tick from the shared
/// `sysinfo::System`. Every query — `descendants`, orphan chain walks,
/// identity re-verification — reads this snapshot; constructing a
/// fresh table per query is the regression the construction counter
/// guards.
#[derive(Debug, Default, Clone)]
pub struct ProcessTable {
    /// Parent pid → child pids (the tree-walk edges).
    pub parents: HashMap<u32, Vec<u32>>,
    /// Pid → identity facts from the tick's refresh.
    pub facts: HashMap<u32, ProcessFacts>,
}

/// Identity facts for one pid in the reconciler's snapshot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProcessFacts {
    /// Parent pid (`None` when unreadable).
    pub ppid: Option<u32>,
    /// Process name.
    pub name: String,
    /// Kernel start time, epoch seconds.
    pub start_time: i64,
    /// Kernel-reported executable (`None` when unreadable).
    pub exe: Option<String>,
    /// Resident memory, bytes.
    pub rss_bytes: u64,
}

/// True when `pid`'s parent chain (capped walk) never reaches a
/// live Juggler-owned root — re-parented to init/launchd (`ppid` 1),
/// orphaned under a dead root, or chain-broken (AJ-E504 Req.3).
fn detached_from(
    table: &ProcessTable,
    pid: u32,
    live_roots: &std::collections::HashSet<u32>,
) -> bool {
    let mut cursor = pid;
    for _ in 0..64 {
        if live_roots.contains(&cursor) {
            return false; // still attached to a live owned root
        }
        let Some(facts) = table.facts.get(&cursor) else {
            return true; // chain left the table: broken
        };
        match facts.ppid {
            None | Some(1) => return true, // init/launchd: re-parented
            Some(parent) => cursor = parent,
        }
    }
    true // cycle guard: treat as detached, never spin
}

/// Builds the shared snapshot from a refreshed table (one pass).
fn process_table(system: &sysinfo::System) -> ProcessTable {
    let mut table = ProcessTable::default();
    for (pid, process) in system.processes() {
        let pid = pid.as_u32();
        if let Some(parent) = process.parent() {
            table.parents.entry(parent.as_u32()).or_default().push(pid);
        }
        table.facts.insert(
            pid,
            ProcessFacts {
                ppid: process.parent().map(sysinfo::Pid::as_u32),
                name: process.name().to_string_lossy().into_owned(),
                start_time: i64::try_from(process.start_time()).unwrap_or_default(),
                exe: process.exe().map(|exe| exe.to_string_lossy().into_owned()),
                rss_bytes: process.memory(),
            },
        );
    }
    table
}

/// One point of the per-tree resource ring (the I-07 sparkline source).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeSample {
    /// Sample time, epoch ms.
    pub at_ms: i64,
    /// Tree CPU%, percent; `None` before two table samples.
    pub cpu_pct: Option<f64>,
    /// Tree RSS sum, bytes.
    pub rss_bytes: u64,
}

/// Per-session collector state: the 5-minute sample ring plus the
/// freshest per-process rows.
#[derive(Debug, Clone, Default)]
pub struct TreeState {
    /// 2 s samples retained 5 min (150/tree, the performance budget).
    pub ring: std::collections::VecDeque<TreeSample>,
    /// Latest rows (root first, then descendants breadth-first); frozen
    /// at the last pre-exit tick for finished sessions (the orphan
    /// snapshot source).
    pub rows: Vec<ProcessRow>,
}

/// Ring capacity: 150 samples × 2 s = the 5-minute retention budget.
const RING_CAPACITY: usize = 150;
/// What a ring keeps, whatever its cadence (our trees are sampled every
/// 2 s, the other tools' every 4 s).
const RING_SPAN_MS: i64 = 5 * 60 * 1_000;

/// Appends one sample, dropping the OLDEST past capacity or past five
/// minutes. (`truncate` would drop the newest: the ring stopped advancing
/// after 5 minutes, freezing the sparkline and the hog window on stale
/// samples.)
#[doc(hidden)]
pub fn push_ring_sample(ring: &mut std::collections::VecDeque<TreeSample>, sample: TreeSample) {
    let newest = sample.at_ms;
    ring.push_back(sample);
    while ring.len() > RING_CAPACITY
        || ring
            .front()
            .is_some_and(|oldest| newest - oldest.at_ms > RING_SPAN_MS)
    {
        ring.pop_front();
    }
}

/// Full-table discovery cadence: every 5th tick (a full scan finds NEW
/// descendants; the 2 s hot path refreshes only known pids). The first
/// three ticks are full scans too (warmup: a fast-moving fixture's
/// children must land in the table before it exits).
const FULL_SCAN_EVERY: u32 = 5;
/// Warmup full-scan ticks after collector creation.
const WARMUP_FULL_TICKS: u32 = 3;
/// Every this many ticks, one read of every process with its parent, start
/// time, CPU and memory (the listing): it finds the new processes of our
/// sessions' trees (P16) and is where the other tools' trees (the Claude
/// app, `claude` in a terminal) are measured. Reading those every tick put
/// monitoring over its budget beside a busy Claude app (290 of its
/// processes: 1.1 % of a core).
const LIST_EVERY: u32 = 2;

/// Owns the persistent process table so per-process CPU% deltas survive
/// between ticks (sysinfo computes usage between refreshes). One
/// collector serves the 2 s per-tree tick (AJ-E410), the 5 s resource
/// emission, and `instances_list`.
///
/// Refresh shape (staggered by design): the 2 s hot path refreshes ONLY
/// the known supervised pids (one targeted batch — there is exactly one
/// sampler thread, so the per-tree herd the stagger defends against
/// cannot exist by construction); every 10th tick does one full-table
/// refresh to discover newly spawned descendants.
#[derive(Debug)]
pub struct LiveStatsCollector {
    system: sysinfo::System,
    sampled: bool,
    ticks: u32,
    trees: HashMap<SessionId, TreeState>,
    stats: HashMap<SessionId, LiveProcessStats>,
    /// Per-session ledger upsert throttle (AJ-E501 Req.4: at most one
    /// batched helper upsert per 2 s per Instance).
    last_ledger_upsert: HashMap<SessionId, i64>,
    /// Consecutive ticks a running row's pid was ABSENT from the
    /// table (AJ-E504: exit-by-absence, confirmed at two ticks).
    absence: HashMap<(u32, String), u32>,
    /// Consecutive agreeing orphan-candidate ticks per helper row
    /// (AJ-E504 Req.3: two-tick confirmation kills flap).
    orphan_agree: HashMap<(u32, String), u32>,
    /// Consecutive identity-MISMATCH ticks per row (AJ-E512): a
    /// single transient sysinfo read (starved refresh, mid-exec
    /// window) must not permanently flip a healthy row to
    /// `unverifiable` — the same two-tick discipline as exit-by-
    /// absence and orphan confirmation. KILL-time verify (E503)
    /// stays single-shot fail-closed; this governs the LISTING state.
    mismatch_agree: HashMap<(u32, String), u32>,
    /// The last listener probe error text, when degraded (AJ-E505:
    /// surfaced honestly by the inventory; retried at the tick
    /// cadence — never a storm).
    ports_degraded: Option<String>,
    /// The last enumerated listener table (AJ-E505): ENUMERATION is
    /// throttled to every `PORT_PROBE_EVERY` tick — on a loaded box it
    /// costs ~90 ms and the ≤100 ms tick budget governs — while the
    /// JOIN (listeners × ledger pids) still runs every tick on the
    /// cached table. Query paths (inventory/warning) always probe
    /// fresh.
    cached_listeners: Vec<crate::ports::Listener>,
    /// Tick index of the last enumeration.
    last_port_probe_tick: u32,
    /// Roots of AI tools AI Juggler did NOT start (AJ-E1402): the Claude
    /// desktop app, `claude` in a terminal. Measured read-only; never
    /// written to the ledger, never reconciled, never signalled (D-08).
    observed_roots: Vec<ObservedRoot>,
    /// The observed trees' latest stats and rings, by root pid.
    observed: HashMap<u32, ObservedTree>,
    /// What the ticks have done so far (the monitoring budget's measure).
    work: CollectorWork,
    /// The listing (see [`LIST_EVERY`]): every process's parent, start time,
    /// CPU and memory, read every 2nd tick. Kept apart from `system`: each
    /// table's CPU deltas are over its own refresh interval.
    listing: sysinfo::System,
    /// The listing has been read twice: its CPU deltas are real.
    listing_sampled: bool,
}

/// What the collector's ticks have done: the monitoring budget (AJ-E1410)
/// is held by doing little, measured here as work rather than as CPU on a
/// machine shared with other loads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollectorWork {
    /// Ticks run.
    pub ticks: u64,
    /// Of them, full scans (every process on the Mac read).
    pub full_scans: u64,
    /// Processes of AI Juggler's own sessions read by the ticks between
    /// full scans (their trees and roots only), each pid once a tick.
    pub targeted_ours: u64,
    /// Processes of the observed tools' trees (the Claude app, `claude` in
    /// a terminal) measured, at each listing: as many as those tools run.
    pub targeted_observed: u64,
    /// Listings: reads of every process (parent, start time, CPU, memory),
    /// every 2nd tick.
    pub listings: u64,
    /// New descendants those listings found: processes a tree started
    /// since the last full scan, counted at once.
    pub discovered: u64,
}

/// One process as the running-tools scan needs it (no environment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessBrief {
    /// Process id.
    pub pid: u32,
    /// Parent pid, when known.
    pub ppid: Option<u32>,
    /// Process name.
    pub name: String,
    /// Executable path, when the OS reports it.
    pub exe: Option<String>,
    /// The command line, joined.
    pub cmd: String,
    /// Kernel start time, epoch seconds (pins a root against pid reuse).
    pub start_time: i64,
}

/// One AI tool AI Juggler did not start, measured over its whole process
/// tree (AJ-E1402). Read-only by construction: it has no session, no
/// ledger row and no stop path.
#[derive(Debug, Clone, Default)]
pub struct ObservedTree {
    /// Tree sums (root + descendants).
    pub stats: LiveProcessStats,
    /// 2 s samples, 5 minutes (the heavy rule's sustained window).
    pub ring: std::collections::VecDeque<TreeSample>,
    /// Every pid of the tree at the last tick: all of them are refreshed
    /// on every tick, so each process's CPU delta covers the same
    /// interval as the refresh it's divided by (`sysinfo` divides a delta
    /// since the process's OWN last refresh by the time since the last
    /// refresh call — a helper refreshed only on full scans read ~5× high
    /// and then froze).
    pub pids: Vec<u32>,
}

/// A root to observe, pinned to the process the scan saw: a pid reused by
/// another process (different start time) is dropped, never measured as
/// the tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedRoot {
    /// The root pid.
    pub pid: u32,
    /// Its kernel start time (epoch seconds) when the scan saw it; `None`
    /// when unknown (then only liveness is checked).
    pub start_time: Option<i64>,
}

impl Default for LiveStatsCollector {
    fn default() -> Self {
        // The ONE shared table (AJ-E504): counted like every other
        // construction — the tick itself adds zero more.
        Self {
            system: crate::new_system(),
            sampled: false,
            ticks: 0,
            trees: HashMap::new(),
            stats: HashMap::new(),
            last_ledger_upsert: HashMap::new(),
            absence: HashMap::new(),
            orphan_agree: HashMap::new(),
            mismatch_agree: HashMap::new(),
            ports_degraded: None,
            cached_listeners: Vec::new(),
            last_port_probe_tick: 0,
            observed_roots: Vec::new(),
            observed: HashMap::new(),
            work: CollectorWork::default(),
            listing: sysinfo::System::new(),
            listing_sampled: false,
        }
    }
}

impl LiveStatsCollector {
    /// Creates a collector with an empty (not yet refreshed) table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One 2 s sampling tick (AJ-E410): targeted refresh of known pids
    /// (full scan every 5th tick for discovery),
    /// recompute per-tree rows + stats, append one ring sample per live
    /// tree. Finished sessions keep their last rows frozen (the exit
    /// snapshot source) and report definitive dead stats.
    ///
    /// AJ-E504: the SAME tick is the reconciler — it builds the ppid
    /// map ONCE (the shared `ProcessTable`), publishes it for
    /// query paths, re-verifies candidate ledger rows against the
    /// table (no per-row probes), detects orphans after grace +
    /// two agreeing ticks, patches helper exits by absence, and
    /// evicts finished `ChildHandle`s past the retention window.
    pub fn tick(&mut self, supervisor: &Supervisor, now_ms: i64) {
        self.ticks = self.ticks.saturating_add(1);
        let full = self.ticks <= WARMUP_FULL_TICKS || self.ticks.is_multiple_of(FULL_SCAN_EVERY);
        let roots = supervisor.live_roots();
        self.work.ticks += 1;
        if full {
            self.work.full_scans += 1;
            self.refresh_all();
        } else {
            // A tree's rows include its root: each pid is read once.
            let mut known: Vec<u32> = self
                .trees
                .values()
                .flat_map(|tree| tree.rows.iter().map(|row| row.pid))
                .chain(roots.iter().map(|(_, pid, _)| *pid))
                .collect();
            known.sort_unstable();
            known.dedup();
            self.refresh_pids(&known);
            self.work.targeted_ours += known.len() as u64;
        }
        // Every 2nd tick, the listing: our trees' new processes (P16: a
        // build fanning out under `claude` counts within 4 s, not up to 10 s
        // later) and the other tools' trees, from one read. Not on a full
        // scan's tick (it already read everything), except while warming
        // up, so they show from the start: the other tools are read every
        // 4 s, 8 s across a full scan.
        let listed =
            self.ticks.is_multiple_of(LIST_EVERY) && (!full || self.ticks <= WARMUP_FULL_TICKS);
        if listed {
            self.refresh_listing();
            if !full {
                let ours_now: Vec<u32> = roots
                    .iter()
                    .filter(|(_, _, finished)| !finished)
                    .map(|(_, pid, _)| *pid)
                    .collect();
                let fresh = self.new_descendants(&ours_now);
                if !fresh.is_empty() {
                    self.work.discovered += fresh.len() as u64;
                    self.refresh_pids(&fresh);
                }
            }
        }
        // ONE table per tick (Req.1): parents + identity facts from
        // the single shared refresh — every downstream reader uses it.
        let table = std::sync::Arc::new(process_table(&self.system));
        self.rebuild_trees(&roots, &table.parents, now_ms);
        if listed {
            self.rebuild_observed(now_ms);
        }
        supervisor.publish_table(std::sync::Arc::clone(&table));
        self.ledger_upserts(supervisor, now_ms);
        // ONE candidates read per tick feeds BOTH the reconciler and
        // the listener join (a second identical query measurably
        // stalls under disk pressure — the tick budget governs).
        let candidates = Self::fetch_candidates(supervisor);
        self.reconcile(supervisor, &table, &candidates, now_ms);
        self.join_ports(supervisor, &table, &candidates, now_ms);
        Self::sweep_finished(supervisor, now_ms);
        let sampled = self.sampled;
        self.sampled = true;
        if !sampled {
            // First-ever tick: CPU deltas do not exist yet — honest
            // absence (None), never a zero that would read as "idle".
            for tree in self.trees.values_mut() {
                if let Some(last) = tree.ring.back_mut() {
                    last.cpu_pct = None;
                }
                for row in &mut tree.rows {
                    row.cpu_pct = None;
                }
            }
            for stat in self.stats.values_mut() {
                if stat.cpu_pct == Some(0.0_f64) {
                    stat.cpu_pct = None;
                }
            }
        }
    }

    /// One batched helper upsert per live session per 2 s (AJ-E501
    /// Req.4): rows come from the freshly rebuilt trees; the root's
    /// own row was patched at spawn and is owned by the exit patch.
    /// Per helper it also records the LIVE process-group id and flags
    /// `escaped_group` when it left the root's group (AJ-E502 Req.4)
    /// — one `getpgid` per helper riding the existing tick, zero new
    /// polling; the flag is sticky in the store (MAX), never un-set.
    fn ledger_upserts(&mut self, supervisor: &Supervisor, now_ms: i64) {
        let Some(ledger) = supervisor.ledger.clone() else {
            return;
        };
        let contexts = supervisor
            .spawn_contexts
            .lock()
            .map(|contexts| contexts.clone())
            .unwrap_or_default();
        let boot_id = ledger.boot_id();
        for (session, tree) in &self.trees {
            let Some(root_row) = tree.rows.first() else {
                continue;
            };
            let last = self.last_ledger_upsert.get(session).copied().unwrap_or(0);
            if now_ms - last < 2_000 {
                continue;
            }
            let Some(ctx) = contexts.get(session) else {
                continue;
            };
            let root_pgid = supervisor.root_pgid(session);
            let mut helpers = Vec::new();
            for row in tree.rows.iter().skip(1) {
                // AJ-E504: the row carries the KERNEL facts from the
                // shared table — the ledger records the real start
                // time and real exe (the identity facts, E503), never
                // a wall-clock derivation or a bare process name.
                let started_at = (row.start_time > 0).then_some(row.start_time);
                // Containment probe: a helper that itself `setsid`'d has
                // a live PGID ≠ the root's. Probe failure (process gone
                // this tick) claims NOTHING — the sticky store flag owns
                // the verdict, and absence stays absence.
                let live_pgid = crate::group::pgid_of(row.pid).ok();
                let escaped_group = match (root_pgid, live_pgid) {
                    (Some(root), Some(live)) => live != root,
                    _ => false,
                };
                // AJ-E508: the helper row records the TRUE command
                // line (the automatic tier's role gate reads it; a
                // bare process name — `Python`, `node` — guesses
                // nothing and must never gate a kill).
                let cmdline = row.cmd.clone().unwrap_or_else(|| row.name.clone());
                helpers.push(juggler_core::storage::LedgerRow {
                    pid: Some(row.pid),
                    pgid: live_pgid,
                    started_at,
                    exe_path: row.exe.clone().unwrap_or_else(|| row.name.clone()),
                    cmdline_hash: {
                        use std::hash::{Hash, Hasher as _};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        cmdline.hash(&mut hasher);
                        format!("cmd:{}", hasher.finish())
                    },
                    cmdline_trunc: cmdline,
                    boot_id: boot_id.clone(),
                    session_id: session.as_str().to_string(),
                    instance_id: None,
                    provider: ctx.provider.clone(),
                    account_id: ctx.account_id.clone(),
                    project_id: ctx.project_id.clone(),
                    machine_id: ctx.machine.as_str().to_string(),
                    root_pid: Some(root_row.pid),
                    parent_pid: row.parent_pid,
                    depth: 1,
                    role: juggler_core::storage::ROLE_HELPER.to_string(),
                    state: String::new(),
                    listens_on: None,
                    cwd: String::new(),
                    peak_rss: Some(i64::try_from(row.rss_bytes).unwrap_or(0)),
                    peak_cpu: row.cpu_pct,
                    exit_kind: None,
                    reaped_by_us: None,
                    escaped_group,
                    down_period: false,
                    first_seen: now_ms,
                    last_seen: now_ms,
                });
            }
            if helpers.is_empty() {
                // Nothing to write this tick; keep the throttle open so
                // the first helper appearance is not delayed by 2 s.
                continue;
            }
            if ledger.upsert_helpers(session, &helpers).is_err() {
                supervisor
                    .ledger_degraded
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                eprintln!("ai-juggler: process ledger degraded at helper upsert");
            }
            self.last_ledger_upsert.insert(session.clone(), now_ms);
        }
    }

    /// The tick's candidate rows: every `running|orphaned` row on
    /// every machine with spawn contexts — fetched ONCE per tick and
    /// shared by the reconciler and the listener join (a second
    /// identical query measurably stalls under disk pressure).
    fn fetch_candidates(supervisor: &Supervisor) -> Vec<juggler_core::storage::LedgerRow> {
        let Some(ledger) = supervisor.ledger.clone() else {
            return Vec::new();
        };
        let contexts = supervisor
            .spawn_contexts
            .lock()
            .map(|contexts| contexts.clone())
            .unwrap_or_default();
        let machines: Vec<juggler_core::model::MachineId> = {
            let mut seen = std::collections::HashSet::new();
            contexts
                .values()
                .map(|ctx| ctx.machine.clone())
                .filter(|machine| seen.insert(machine.as_str().to_string()))
                .collect()
        };
        let mut candidates = Vec::new();
        for machine in &machines {
            if let Ok(rows) = ledger.candidates(machine) {
                candidates.extend(rows);
            } else {
                supervisor
                    .ledger_degraded
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                return Vec::new(); // read failed: this tick reconciles nothing
            }
        }
        candidates
    }

    /// The reconciler pass (AJ-E504 Req.2/3): re-verifies candidate
    /// rows against the tick's OWN table (zero per-row probes),
    /// patches helper exits by two-tick absence, and flags orphans
    /// only after grace + two consecutive agreeing ticks. Single-row
    /// state patches only — helper bulk writes stay in the batched
    /// upsert transaction (AJ-E501's contract).
    #[allow(clippy::too_many_lines)] // one pass per tick; splitting would hide the state machine
    fn reconcile(
        &mut self,
        supervisor: &Supervisor,
        table: &ProcessTable,
        candidates: &[juggler_core::storage::LedgerRow],
        now_ms: i64,
    ) {
        use juggler_core::storage::STATE_RUNNING;

        let Some(ledger) = supervisor.ledger.clone() else {
            return;
        };
        let contexts = supervisor
            .spawn_contexts
            .lock()
            .map(|contexts| contexts.clone())
            .unwrap_or_default();
        let boot_id = ledger.boot_id();

        // Live-owned roots: unfinished handles + running ROOT rows.
        let mut live_roots: std::collections::HashSet<u32> = supervisor
            .live_roots()
            .into_iter()
            .filter(|(_, _, finished)| !finished)
            .map(|(_, pid, _)| pid)
            .collect();
        for row in candidates {
            if row.role == "root"
                && row.state == STATE_RUNNING
                && let Some(pid) = row.pid
            {
                live_roots.insert(pid);
            }
        }

        // Group candidate rows per session (root-exit facts come from
        // the session's full row set — the root row may be `exited`).
        let mut by_session: HashMap<String, Vec<juggler_core::storage::LedgerRow>> = HashMap::new();
        for row in candidates {
            by_session
                .entry(row.session_id.clone())
                .or_default()
                .push(row.clone());
        }

        for (session_id, rows) in by_session {
            let session = SessionId::new(session_id.clone());
            let Some(ctx) = contexts.get(&session) else {
                continue;
            };
            let grace_ms = i64::try_from(supervisor.orphan_grace_for(&ctx.provider))
                .unwrap_or(i64::MAX)
                .saturating_mul(1000);
            let full_rows = ledger.session_rows(&session).unwrap_or_default();
            let root_row = full_rows.iter().find(|row| row.role == "root");
            let root_exit_ms = root_row
                .filter(|row| row.state == "exited")
                .map(|row| row.last_seen);

            for row in rows {
                let Some(pid) = row.pid else { continue };
                let key = (pid, row.boot_id.clone());
                let Some(facts) = table.facts.get(&pid) else {
                    // Absent from the tick's table: one agreeing tick
                    // short of an exit patch; confirmed at two.
                    let ticks = self.absence.entry(key).and_modify(|n| *n += 1).or_insert(1);
                    if *ticks >= 2 {
                        self.absence.remove(&(pid, row.boot_id.clone()));
                        self.orphan_agree.remove(&(pid, row.boot_id.clone()));
                        let _ = ledger.mark_exited(pid, &row.boot_id, "unknown", false);
                    }
                    continue;
                };
                self.absence.remove(&key);

                // (c) boot gate first (FMEA C8): a record from another
                // boot is permanently unverifiable.
                if row.boot_id != boot_id {
                    let _ = ledger.mark_unverifiable(pid, &row.boot_id);
                    self.orphan_agree.remove(&key);
                    continue;
                }
                // Table-based re-verification (Req.2): the recorded
                // identity facts vs THIS tick's refresh. Rows without
                // recorded facts (legacy, unpatched) are skipped —
                // honest absence, never a flip on missing data. The
                // leg that fired is named in the flip log — the next
                // flake diagnoses itself from the runner output.
                let mut start_mismatch = false;
                if let Some(recorded_start) = row.started_at
                    && recorded_start > 0
                    && recorded_start != facts.start_time
                {
                    start_mismatch = true;
                }
                // The exe fact is KERNEL-vs-kernel — only HELPER rows
                // carry a kernel-recorded exe (the upsert writes it);
                // ROOT rows record the launch program, which the
                // kernel may legitimately alias or replace across
                // exec (macOS /bin/sh → bash), so roots verify below
                // against the handle's kernel-recorded identity.
                // Inside the exec-settle window (AJ-E508,
                // `EXEC_SETTLE_MS`) a young helper's image is
                // legitimately in transition: discovery can catch the
                // fork BEFORE the exec (recorded exe = the wrapper),
                // and the upsert's refinement — throttled to 2 s —
                // races THIS verdict, which ticks every pass; a flip
                // here would freeze the row `unverifiable` before the
                // correction lands (the refinement only touches
                // `running` rows). The exe leg waits out the window.
                // The start time never legitimately changes — that
                // leg (the true PID-reuse signal) never waits.
                let mut exe_mismatch = false;
                if row.role == "helper"
                    && now_ms.saturating_sub(row.first_seen)
                        >= juggler_core::storage::EXEC_SETTLE_MS
                    && let Some(live_exe) = facts.exe.as_deref()
                    && row.exe_path.starts_with('/')
                    && row.exe_path != live_exe
                {
                    exe_mismatch = true;
                }
                let mut root_mismatch = false;
                let mut mismatch = start_mismatch || exe_mismatch;
                if row.role == "root" && !mismatch {
                    // Roots verify against the kernel-recorded handle
                    // identity (exe moves legitimately across exec;
                    // the handle's settled record is the truth).
                    if let Some(identity) = supervisor.root_identity(&session)
                        && identity.start_time != facts.start_time
                    {
                        root_mismatch = true;
                        mismatch = true;
                    }
                }
                if mismatch {
                    // AJ-E512: two CONSECUTIVE mismatching ticks before
                    // the durable flip — one transient read (starved
                    // sysinfo refresh, mid-exec window) stays a
                    // warning, not a verdict. A clean tick clears it.
                    let agrees = self
                        .mismatch_agree
                        .entry(key.clone())
                        .and_modify(|n| *n += 1)
                        .or_insert(1);
                    if *agrees >= 2 {
                        self.mismatch_agree.remove(&(pid, row.boot_id.clone()));
                        let _ = ledger.mark_unverifiable(pid, &row.boot_id);
                        self.orphan_agree.remove(&key);
                        eprintln!(
                            "ai-juggler: pid {pid} failed re-verification \
                             (start-time={start_mismatch}, exe={exe_mismatch}, \
                             root-identity={root_mismatch}, \
                             age={}ms) — flipped unverifiable",
                            now_ms.saturating_sub(row.first_seen)
                        );
                    }
                    continue;
                }
                self.mismatch_agree.remove(&key);

                // Orphan detection (Req.3): running helpers of an
                // EXITED root, past grace, whose parent chain no
                // longer reaches a live Juggler-owned root — two
                // consecutive agreeing ticks.
                if row.role == "helper"
                    && row.state == STATE_RUNNING
                    && let Some(root_exit) = root_exit_ms
                    && now_ms - root_exit >= grace_ms
                    && detached_from(table, pid, &live_roots)
                {
                    let ticks = self
                        .orphan_agree
                        .entry(key)
                        .and_modify(|n| *n += 1)
                        .or_insert(1);
                    if *ticks >= 2 {
                        self.orphan_agree.remove(&(pid, row.boot_id.clone()));
                        let _ = ledger.mark_orphaned(pid, &row.boot_id);
                        let _ = supervisor.events_tx.send(LifecycleEvent::HelperOrphaned {
                            session: session.clone(),
                            pid,
                        });
                    }
                } else {
                    // Not an orphan candidate this tick: flap control.
                    self.orphan_agree.remove(&key);
                }
            }
        }
    }

    /// The listener join (AJ-E505 Req.2): the JOIN runs every tick on
    /// the cached listener table; ENUMERATION is throttled to every
    /// [`PORT_PROBE_EVERY`] ticks (its ~90 ms loaded-box cost would
    /// breach the ≤100 ms tick budget — the budget governs; ports
    /// change rarely and query paths probe fresh anyway). A probe
    /// failure degrades until the next enumeration attempt (the
    /// throttle is the retry bound — never a storm).
    fn join_ports(
        &mut self,
        supervisor: &Supervisor,
        table: &ProcessTable,
        candidates: &[juggler_core::storage::LedgerRow],
        now_ms: i64,
    ) {
        #[cfg(unix)]
        use crate::ports::NetstatProbe;
        use crate::ports::PortProbe as _;
        let Some(ledger) = supervisor.ledger.clone() else {
            return;
        };
        if self.ticks.saturating_sub(self.last_port_probe_tick) >= PORT_PROBE_EVERY {
            self.last_port_probe_tick = self.ticks;
            #[cfg(unix)]
            let probe = NetstatProbe;
            #[cfg(not(unix))]
            let probe = crate::ports::UnsupportedProbe;
            match probe.listeners() {
                Ok(listeners) => {
                    self.cached_listeners = listeners;
                    self.ports_degraded = None;
                }
                Err(error) => {
                    self.ports_degraded = Some(error.to_string());
                    return;
                }
            }
        }
        let listeners = self.cached_listeners.clone();
        let boot_id = ledger.boot_id();
        let by_pid: HashMap<u32, Vec<u16>> = {
            let mut map: HashMap<u32, Vec<u16>> = HashMap::new();
            for listener in listeners {
                map.entry(listener.pid).or_default().push(listener.port);
            }
            map
        };
        // Candidate rows + live roots: every pid the ledger could
        // still own a listener for (the reconciler's shared read).
        let mut pids: Vec<(u32, String)> = supervisor
            .live_roots()
            .into_iter()
            .filter(|(_, _, finished)| !finished)
            .map(|(_, pid, _)| (pid, boot_id.clone()))
            .collect();
        // The reconciler's candidates (fetched once per tick).
        for row in candidates {
            if let Some(pid) = row.pid {
                pids.push((pid, row.boot_id.clone()));
            }
        }
        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (pid, row_boot) in pids {
            if !seen.insert(pid) {
                continue;
            }
            // Only pids PRESENT in the listener table get a write;
            // absent pids keep their stored value (no clearing churn).
            if table.facts.contains_key(&pid)
                && let Some(ports) = by_pid.get(&pid).filter(|ports| !ports.is_empty())
            {
                let csv = ports
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                entries.push((pid, row_boot, csv));
            }
        }
        if entries.is_empty() {
            return;
        }
        let refs: Vec<(u32, &str, &str)> = entries
            .iter()
            .map(|(pid, boot, ports)| (*pid, boot.as_str(), ports.as_str()))
            .collect();
        if ledger.apply_ports(&refs).is_err() {
            supervisor
                .ledger_degraded
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let _ = now_ms;
    }

    /// Evicts finished `ChildHandle`s past the output-retention
    /// window (AJ-E504 Req.4, F-6): stopped Instances leave no
    /// phantom Running rows and rings do not grow for the process
    /// lifetime. The ledger keeps the durable history.
    fn sweep_finished(supervisor: &Supervisor, now_ms: i64) {
        let Ok(mut children) = supervisor.children.lock() else {
            return;
        };
        let expired: Vec<SessionId> = children
            .iter()
            .filter(|(_, handle)| {
                let exit_at = handle.exit_at_ms.load(Ordering::Relaxed);
                exit_at > 0
                    && handle.finished.load(Ordering::Acquire)
                    && now_ms.saturating_sub(exit_at) > OUTPUT_RETENTION_AFTER_EXIT_MS
            })
            .map(|(session, _)| session.clone())
            .collect();
        for session in expired {
            children.remove(&session);
        }
    }

    fn refresh_all(&mut self) {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory()
                .with_exe(sysinfo::UpdateKind::Always)
                .with_cmd(sysinfo::UpdateKind::Always),
        );
    }

    /// Reads the listing: every process with its parent, start time, CPU
    /// and memory (never the exe or command line, which feed the ledger).
    fn refresh_listing(&mut self) {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        self.work.listings += 1;
        self.listing.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory(),
        );
    }

    /// Every descendant of `roots` that the main table doesn't hold yet,
    /// from the listing. A pid the table holds for a process that has since
    /// ended and been reused (another start time) is new too.
    fn new_descendants(&self, roots: &[u32]) -> Vec<u32> {
        if roots.is_empty() {
            return Vec::new();
        }
        let children = parents_map(&self.listing);
        let mut fresh = Vec::new();
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut queue: Vec<u32> = roots.to_vec();
        while let Some(pid) = queue.pop() {
            if !seen.insert(pid) {
                continue;
            }
            let Some(listed) = self.listing.process(sysinfo::Pid::from_u32(pid)) else {
                continue; // gone since: nothing to read, no children
            };
            let held = self.system.process(sysinfo::Pid::from_u32(pid));
            if held.is_none_or(|held| held.start_time() != listed.start_time()) {
                fresh.push(pid);
            }
            if let Some(kids) = children.get(&pid) {
                queue.extend(kids);
            }
        }
        fresh
    }

    /// Targeted refresh of specific pids (orphan liveness probes).
    ///
    /// Each pid is listed once: sysinfo drops a process it is asked to
    /// refresh twice in one call (the second visit sees its "updated"
    /// mark already consumed), and a root also appears among its own
    /// tree's rows.
    pub fn refresh_pids(&mut self, pids: &[u32]) {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        let mut targets: Vec<sysinfo::Pid> = pids
            .iter()
            .map(|pid| sysinfo::Pid::from_u32(*pid))
            .collect();
        targets.sort_unstable();
        targets.dedup();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&targets),
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory()
                .with_exe(sysinfo::UpdateKind::Always)
                .with_cmd(sysinfo::UpdateKind::Always),
        );
    }

    /// Recomputes every tree's rows/stats/ring from the current table.
    fn rebuild_trees(
        &mut self,
        roots: &[(SessionId, u32, bool)],
        parents: &HashMap<u32, Vec<u32>>,
        now_ms: i64,
    ) {
        // Drop state for sessions the supervisor no longer knows.
        let live_ids: Vec<SessionId> = roots.iter().map(|(id, _, _)| id.clone()).collect();
        self.trees.retain(|id, _| live_ids.contains(id));
        self.stats.retain(|id, _| live_ids.contains(id));
        for (session, root, finished) in roots {
            let finished = *finished;
            let tree = self.trees.entry(session.clone()).or_default();
            if finished {
                // Frozen: keep the last pre-exit rows (orphan snapshot
                // source) and report definitive dead stats.
                self.stats
                    .insert((*session).clone(), LiveProcessStats::dead());
                continue;
            }
            // The root died but the supervisor hasn't marked the session
            // finished yet: its helpers have been reparented, so a
            // rebuild from the dead root would lose them — and freeze
            // that (the hog guard and the orphan snapshot read these
            // rows). Keep the last live rows until it is finished.
            let root_gone = self
                .system
                .process(sysinfo::Pid::from_u32(*root))
                .is_none_or(|process| process.status() == sysinfo::ProcessStatus::Zombie);
            if root_gone && !tree.rows.is_empty() {
                self.stats
                    .insert((*session).clone(), LiveProcessStats::dead());
                continue;
            }
            let (rows, stats) = tree_rows(&self.system, parents, *root);
            tree.rows = rows;
            push_ring_sample(
                &mut tree.ring,
                TreeSample {
                    at_ms: now_ms,
                    cpu_pct: stats.cpu_pct,
                    rss_bytes: stats.rss_bytes,
                },
            );
            self.stats.insert((*session).clone(), stats);
        }
    }

    /// Sets the roots of AI tools AI Juggler did not start (AJ-E1402).
    /// A root inside a supervised tree is ignored at the next tick (no
    /// double count); trees for roots no longer listed are dropped.
    pub fn set_observed_roots(&mut self, roots: Vec<ObservedRoot>) {
        self.observed
            .retain(|root, _| roots.iter().any(|pinned| pinned.pid == *root));
        self.observed_roots = roots;
    }

    /// What the ticks have done so far.
    #[must_use]
    pub fn work(&self) -> CollectorWork {
        self.work
    }

    /// The observed trees as of the last tick, by root pid.
    #[must_use]
    pub fn observed_snapshot(&self) -> HashMap<u32, ObservedTree> {
        self.observed.clone()
    }

    /// Every process from the collector's table — pid, parent, name,
    /// executable, command line; never the environment (AJ-E1402: the
    /// running-tools scan classifies from this instead of a second full
    /// refresh). A watched tree's new processes appear within two ticks
    /// (P16); any other new process at the next full scan (≤ 10 s).
    #[must_use]
    pub fn process_briefs(&self) -> Vec<ProcessBrief> {
        self.system
            .processes()
            .values()
            .map(|process| ProcessBrief {
                pid: process.pid().as_u32(),
                ppid: process.parent().map(sysinfo::Pid::as_u32),
                name: process.name().to_string_lossy().into_owned(),
                exe: process.exe().map(|exe| exe.to_string_lossy().into_owned()),
                cmd: process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
                start_time: i64::try_from(process.start_time()).unwrap_or_default(),
            })
            .collect()
    }

    /// The other tools' trees, measured from the listing.
    fn rebuild_observed(&mut self, now_ms: i64) {
        let parents = parents_map(&self.listing);
        // Excluded: every supervised pid and AI Juggler itself (a dev
        // build may run under a terminal `claude`; its sessions are ours).
        let supervised: std::collections::HashSet<u32> = self
            .trees
            .values()
            .flat_map(|tree| tree.rows.iter().map(|row| row.pid))
            .chain(std::iter::once(std::process::id()))
            .collect();
        let roots = self.observed_roots.clone();
        let sampled = self.listing_sampled;
        self.listing_sampled = true;
        let mut reused = Vec::new();
        for pinned in roots {
            let root = pinned.pid;
            let process = self.listing.process(sysinfo::Pid::from_u32(root));
            let alive =
                process.is_some_and(|process| process.status() != sysinfo::ProcessStatus::Zombie);
            // The pid now names another process: never measure it as the
            // tool (and stop watching it until the next scan).
            let same = process.is_none_or(|process| {
                pinned.start_time.is_none_or(|started| {
                    i64::try_from(process.start_time()).is_ok_and(|now| now == started)
                })
            });
            if !same {
                reused.push(root);
            }
            if supervised.contains(&root) || !alive || !same {
                self.observed.remove(&root);
                continue;
            }
            let (rows, _) = tree_rows(&self.listing, &parents, root);
            self.work.targeted_observed += rows.len() as u64;
            // A supervised subtree never also counts as observed.
            let mut stats = observed_sums(&rows, &supervised);
            if !sampled {
                stats.cpu_pct = None; // no delta yet — never a fake idle
            }
            let tree = self.observed.entry(root).or_default();
            tree.stats = stats;
            tree.pids = rows.iter().map(|row| row.pid).collect();
            push_ring_sample(
                &mut tree.ring,
                TreeSample {
                    at_ms: now_ms,
                    cpu_pct: stats.cpu_pct,
                    rss_bytes: stats.rss_bytes,
                },
            );
        }
        self.observed_roots
            .retain(|pinned| !reused.contains(&pinned.pid));
    }

    /// The listener-probe degrade note for the LAST tick, when the
    /// platform has no probe or the OS call failed (AJ-E505: the
    /// inventory shows `ports unknown on this platform` honestly —
    /// never an empty map presented as truth).
    #[must_use]
    pub fn ports_note(&self) -> Option<String> {
        self.ports_degraded.clone()
    }

    /// Cached batched stats from the last tick (no refresh — the read
    /// model must not trigger table refreshes per row, AJ-E401).
    #[must_use]
    pub fn stats_snapshot(&self) -> HashMap<SessionId, LiveProcessStats> {
        self.stats.clone()
    }

    /// The latest per-process rows for one session (frozen at the last
    /// pre-exit tick for finished sessions).
    #[must_use]
    pub fn rows(&self, session: &SessionId) -> Option<Vec<ProcessRow>> {
        self.trees.get(session).map(|tree| tree.rows.clone())
    }

    /// The 5-minute sample ring for one session (the I-07 sparkline
    /// source), oldest first.
    #[must_use]
    pub fn ring(&self, session: &SessionId) -> Vec<TreeSample> {
        self.trees
            .get(session)
            .map(|tree| tree.ring.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Live, non-zombie lookup against the current table: `(name, rss)`
    /// or `None` when the pid is gone/zombie.
    #[must_use]
    pub fn lookup(&self, pid: u32) -> Option<(String, u64)> {
        self.system
            .process(sysinfo::Pid::from_u32(pid))
            .filter(|process| {
                !matches!(
                    process.status(),
                    sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead
                )
            })
            .map(|process| {
                (
                    process.name().to_string_lossy().into_owned(),
                    process.memory(),
                )
            })
    }

    /// Refreshes the process table once and returns batched stats for
    /// every supervised session (compat API for one-shot callers; the
    /// steady state is [`LiveStatsCollector::tick`]).
    pub fn refresh(&mut self, supervisor: &Supervisor) -> HashMap<SessionId, LiveProcessStats> {
        let now = juggler_core::model::Timestamp::now().millis();
        self.tick(supervisor, now);
        self.stats.clone()
    }
}

/// ppid → children map over the current table.
pub(crate) fn parents_map(system: &sysinfo::System) -> HashMap<u32, Vec<u32>> {
    let mut parents: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            parents
                .entry(parent.as_u32())
                .or_default()
                .push(pid.as_u32());
        }
    }
    parents
}

/// Pure (AJ-E1402): an observed tree's sums with every excluded pid and
/// its whole subtree pruned. `rows` is breadth-first (parents before
/// children), as [`tree_rows`] returns them.
#[doc(hidden)]
#[must_use]
pub fn observed_sums<S: std::hash::BuildHasher>(
    rows: &[ProcessRow],
    excluded: &std::collections::HashSet<u32, S>,
) -> LiveProcessStats {
    let mut pruned: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut stats = LiveProcessStats {
        child_count: 0,
        cpu_pct: Some(0.0),
        rss_bytes: 0,
    };
    for (index, row) in rows.iter().enumerate() {
        let under_pruned = row
            .parent_pid
            .is_some_and(|parent| pruned.contains(&parent));
        if excluded.contains(&row.pid) || under_pruned {
            pruned.insert(row.pid);
            continue;
        }
        stats.rss_bytes += row.rss_bytes;
        stats.cpu_pct = stats.cpu_pct.map(|sum| sum + row.cpu_pct.unwrap_or(0.0));
        if index > 0 {
            stats.child_count += 1;
        }
    }
    stats
}

/// Per-process rows (root first, breadth-first over descendants) plus
/// the tree aggregate, read from a table the caller already refreshed.
fn tree_rows(
    system: &sysinfo::System,
    parents: &HashMap<u32, Vec<u32>>,
    root: u32,
) -> (Vec<ProcessRow>, LiveProcessStats) {
    let mut rows = Vec::new();
    let mut cpu = 0.0_f64;
    let mut rss = 0_u64;
    let mut child_count = 0_u32;
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<(u32, u8)> =
        std::collections::VecDeque::from([(root, 0)]);
    while let Some((pid, depth)) = queue.pop_front() {
        if !seen.insert(pid) {
            continue; // ppid cycles must not loop forever
        }
        let children = parents.get(&pid).cloned().unwrap_or_default();
        let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
            queue.extend(children.into_iter().map(|child| (child, depth + 1)));
            continue;
        };
        let age_ms = process.run_time().saturating_mul(1000);
        rows.push(ProcessRow {
            pid,
            parent_pid: process.parent().map(sysinfo::Pid::as_u32),
            name: process.name().to_string_lossy().into_owned(),
            cpu_pct: Some(f64::from(process.cpu_usage())),
            rss_bytes: process.memory(),
            age_ms,
            child_count: u32::try_from(children.len()).unwrap_or(u32::MAX),
            depth,
            start_time: i64::try_from(process.start_time()).unwrap_or_default(),
            exe: process.exe().map(|exe| exe.to_string_lossy().into_owned()),
            cmd: (!process.cmd().is_empty()).then(|| {
                process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
        });
        cpu += f64::from(process.cpu_usage());
        rss += process.memory();
        if pid != root {
            child_count += 1;
        }
        queue.extend(children.into_iter().map(|child| (child, depth + 1)));
    }
    (
        rows,
        LiveProcessStats {
            child_count,
            cpu_pct: Some(cpu),
            rss_bytes: rss,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{LiveStatsCollector, RING_CAPACITY, RING_SPAN_MS, TreeSample, push_ring_sample};
    use crate::supervisor::Supervisor;

    fn sample(at_ms: i64) -> TreeSample {
        TreeSample {
            at_ms,
            cpu_pct: None,
            rss_bytes: 0,
        }
    }

    /// AJ-E1544: the module stands on its own. The collector's old path
    /// (`supervisor::LiveStatsCollector`) is the same type.
    #[test]
    fn the_old_path_names_the_same_collector() {
        let collector: crate::supervisor::LiveStatsCollector = LiveStatsCollector::new();
        assert_eq!(collector.work(), LiveStatsCollector::default().work());
    }

    #[test]
    fn an_idle_supervisor_has_nothing_to_report() {
        let supervisor = Supervisor::default();
        let mut collector = LiveStatsCollector::new();
        collector.tick(&supervisor, 1_000);
        assert!(collector.stats_snapshot().is_empty());
        assert!(collector.refresh(&supervisor).is_empty());
        assert!(collector.observed_snapshot().is_empty());
    }

    #[test]
    fn the_ring_keeps_five_minutes_and_at_most_its_capacity() {
        let mut ring = std::collections::VecDeque::new();
        for n in 0..i64::try_from(RING_CAPACITY).unwrap_or(150) + 10 {
            push_ring_sample(&mut ring, sample(n));
        }
        assert_eq!(ring.len(), RING_CAPACITY);
        push_ring_sample(&mut ring, sample(RING_SPAN_MS + 1_000));
        assert!(
            ring.iter()
                .all(|kept| RING_SPAN_MS + 1_000 - kept.at_ms <= RING_SPAN_MS),
            "older than five minutes dropped"
        );
    }
}
