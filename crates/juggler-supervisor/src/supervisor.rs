//! The process supervisor: spawns harness sessions (PTY or piped), captures
//! output, runs the stop ladder, and journals lifecycle for crash re-adoption
//! (spec §5.1, §5.3).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapter::LaunchSpec;
use juggler_core::model::{SessionId, Timestamp};

use crate::error::SupervisorError;
use crate::group;
use crate::journal::Journal;
use crate::lifecycle::{ExitKind, LifecycleEvent};
use crate::ring::OutputRing;

/// Default output retention per session (spec §1 footprint budget).
const DEFAULT_RING_CAPACITY: usize = 512 * 1024;

/// Default orphan grace, seconds (AJ-E504 Req.3): how long a helper
/// survives its Instance's dead root before the reconciler will even
/// CONSIDER flagging it — the false-alert budget (brief build flicker
/// must never surface). Per-Provider tunable via
/// [`Supervisor::set_orphan_grace`]; never lowered for demos.
pub const DEFAULT_ORPHAN_GRACE_SECS: u64 = 30;

/// Listener-table ENUMERATION cadence (AJ-E505): every 5th tick
/// (10 s) — enumeration costs ~90 ms on a loaded box and the tick
/// budget is ≤100 ms p95; the per-tick JOIN uses the cached table and
/// query paths always probe fresh.
const PORT_PROBE_EVERY: u32 = 5;

/// How long a finished session's `ChildHandle` stays in the children
/// map after exit (AJ-E504 Req.4, F-6): the window the UI may still
/// read the retained output tail; after it the handle (and its ring)
/// is evicted so stopped Instances leave no phantom rows and rings do
/// not live for the process lifetime.
pub const OUTPUT_RETENTION_AFTER_EXIT_MS: i64 = 120_000;

/// A successfully spawned supervised process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnedProcess {
    /// The session this process runs for.
    pub session: SessionId,
    /// OS process id.
    pub pid: u32,
    /// The child's process-group id (AJ-E502): one `getpgid` at spawn;
    /// `None` where the platform has no groups — the ownership
    /// identity later kill paths target (never a guessed group).
    pub pgid: Option<u32>,
    /// The spawn-time identity (AJ-E503): fork-time OS start time +
    /// kernel-reported exe + boot id. `None` only when the probe
    /// never landed — verification then fails CLOSED. The settled
    /// post-`exec` identity arrives via the journal's
    /// `identity_settled` entry and the handle.
    pub identity: Option<crate::tree::ProcessIdentity>,
}

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
}

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

/// Full-table discovery cadence: every 5th tick (a full scan finds NEW
/// descendants; the 2 s hot path refreshes only known pids). The first
/// three ticks are full scans too (warmup: a fast-moving fixture's
/// children must land in the table before it exits).
const FULL_SCAN_EVERY: u32 = 5;
/// Warmup full-scan ticks after collector creation.
const WARMUP_FULL_TICKS: u32 = 3;

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
        if full {
            self.refresh_all();
        } else {
            let known: Vec<u32> = self
                .trees
                .values()
                .flat_map(|tree| tree.rows.iter().map(|row| row.pid))
                .chain(roots.iter().map(|(_, pid, _)| *pid))
                .collect();
            self.refresh_pids(&known);
            // Targeted refresh keeps parent edges of known pids fresh;
            // brand-new children appear at the next full scan.
        }
        // ONE table per tick (Req.1): parents + identity facts from
        // the single shared refresh — every downstream reader uses it.
        let table = std::sync::Arc::new(process_table(&self.system));
        self.rebuild_trees(&roots, &table.parents, now_ms);
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
                .with_cpu()
                .with_memory()
                .with_exe(sysinfo::UpdateKind::Always)
                .with_cmd(sysinfo::UpdateKind::Always),
        );
    }

    /// Targeted refresh of specific pids (orphan liveness probes).
    pub fn refresh_pids(&mut self, pids: &[u32]) {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        let targets: Vec<sysinfo::Pid> = pids
            .iter()
            .map(|pid| sysinfo::Pid::from_u32(*pid))
            .collect();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&targets),
            true,
            ProcessRefreshKind::nothing()
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
            let (rows, stats) = tree_rows(&self.system, parents, *root);
            tree.rows = rows;
            tree.ring.push_back(TreeSample {
                at_ms: now_ms,
                cpu_pct: stats.cpu_pct,
                rss_bytes: stats.rss_bytes,
            });
            tree.ring.truncate(RING_CAPACITY);
            self.stats.insert((*session).clone(), stats);
        }
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
fn parents_map(system: &sysinfo::System) -> HashMap<u32, Vec<u32>> {
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

/// Caller-supplied handle extras (PTY writer + pre-created output
/// tracking atomic) — keeps `register` under the argument budget.
struct ChildExtras {
    pty: Option<Arc<Mutex<PtyHandle>>>,
    last_output_ms: Arc<std::sync::atomic::AtomicI64>,
    /// The child's process-group id (AJ-E502): the deterministic kill
    /// target; `None` where the platform has no groups (honest).
    pgid: Option<u32>,
    /// The spawn-time identity (AJ-E503); refined by the settle
    /// thread after wrappers exec their interpreter.
    identity: Option<crate::tree::ProcessIdentity>,
    /// Exit wall-clock (epoch ms, 0 = live) — shared with the wait
    /// thread so the eviction sweep (AJ-E504 Req.4) can time it.
    exit_at_ms: Arc<AtomicI64>,
}

/// One re-adopted root (the recovery summary's `{a}` source).
#[derive(Debug, Clone)]
pub struct AdoptionRecord {
    /// The Instance it belonged to.
    pub session: SessionId,
    /// The verified root pid.
    pub pid: u32,
    /// The program name (display).
    pub program: String,
}

/// The startup recovery report (AJ-E510 Req.3): the one-time summary's
/// exact counts.
#[derive(Debug, Default)]
pub struct RecoveryReport {
    /// Verified live roots, handles rebuilt (stoppable).
    pub adopted: Vec<AdoptionRecord>,
    /// Live helpers whose roots died while Juggler was closed.
    pub left_behind: Vec<u32>,
    /// Alive rows that failed verification — listed, never killed.
    pub unverifiable: Vec<u32>,
}

/// The row's identity facts as the E503 record (the verify input).
fn row_identity(row: &juggler_core::storage::LedgerRow) -> crate::tree::ProcessIdentity {
    crate::tree::ProcessIdentity {
        pid: row.pid.unwrap_or(0),
        pgid: row.pgid,
        start_time: row.started_at.unwrap_or_default(),
        exe: std::path::PathBuf::from(&row.exe_path),
        boot_id: row.boot_id.clone(),
    }
}

/// The root row's identity for the start-time-only liveness probe
/// (E504's exec-aliasing rule: a root's exe moves legitimately).
fn root_identity(root: &juggler_core::storage::LedgerRow) -> crate::tree::ProcessIdentity {
    row_identity(root)
}

struct ChildHandle {
    pid: u32,
    /// The child's process-group id (AJ-E502, the ownership identity).
    pgid: Option<u32>,
    /// The recorded identity (AJ-E503) — the verify-before-kill
    /// input for kill paths (E507/E508) and the reconciler (E504).
    /// Behind a mutex: the settle thread refines it after `exec`.
    identity: Mutex<Option<crate::tree::ProcessIdentity>>,
    /// When the process exited (epoch ms; 0 = live) — the eviction
    /// timer (AJ-E504 Req.4).
    exit_at_ms: Arc<AtomicI64>,
    ring: Arc<Mutex<OutputRing>>,
    finished: Arc<AtomicBool>,
    /// Last output arrival (epoch ms, 0 = none) — feeds the Stuck rule
    /// and event-driven recovery (AJ-E405).
    last_output_ms: Arc<std::sync::atomic::AtomicI64>,
    force_kill: Arc<dyn Fn() + Send + Sync>,
    /// Live PTY master for interactive sessions: keyboard input and
    /// resize. `None` for piped (headless) sessions.
    pty: Option<Arc<Mutex<PtyHandle>>>,
}

/// The write/resize side of a supervised PTY.
struct PtyHandle {
    writer: Box<dyn std::io::Write + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
}

/// Owns supervised sessions for one AI Juggler process.
///
/// Children **survive** the supervisor (and the app) by design: dropping it
/// neither signals nor waits. Re-adoption after a restart happens via
/// [`Journal::adopt_orphans`].
pub struct Supervisor {
    journal: Option<Journal>,
    /// Persistent spawn ledger (AJ-E501): intent rows before spawn,
    /// helper upserts per tick — survives app exit and reboot. `None`
    /// = unconfigured (no degraded flag; journal still records roots).
    ledger: Option<crate::tree::SharedLedger>,
    /// Set when a CONFIGURED ledger fails: spawns never fail because
    /// of it — cleanup turns read-only until restart (the banner).
    ledger_degraded: std::sync::atomic::AtomicBool,
    children: Mutex<HashMap<SessionId, Arc<ChildHandle>>>,
    /// Sessions stopped via [`Supervisor::stop`] — PTY children surface
    /// user kills as plain non-zero exit codes, so the stop intent is the
    /// only reliable "not a failure" signal.
    user_stopped: Mutex<Vec<SessionId>>,
    events_tx: std::sync::mpsc::Sender<LifecycleEvent>,
    events_rx: Mutex<std::sync::mpsc::Receiver<LifecycleEvent>>,
    /// Spawn contexts (AJ-E501): helper rows inherit the root's
    /// provider/account/project/machine; kept for the ledger tick.
    spawn_contexts: Mutex<HashMap<SessionId, crate::tree::SpawnContext>>,
    /// Sessions re-adopted by crash recovery (AJ-E510): the
    /// `Running — re-adopted` marker set. These ARE supervised again
    /// (stoppable through the E507 path) — the old heuristic "adopted
    /// = journal-live minus supervised" could not say that.
    adopted: Mutex<HashMap<SessionId, Arc<ChildHandle>>>,
    /// The reconciler's published process table (AJ-E504 Req.1):
    /// queries (`descendants`, chain walks) read this cached
    /// snapshot — never a fresh per-call scan.
    table: Mutex<Option<std::sync::Arc<ProcessTable>>>,
    /// Orphan grace per Provider, seconds (AJ-E504 Req.3). The `""`
    /// key overrides the default; lookup falls back to
    /// [`DEFAULT_ORPHAN_GRACE_SECS`].
    orphan_grace_secs: Mutex<HashMap<String, u64>>,
    /// Sessions with fresh output since the last drain (throttled per
    /// session) — the recovery signal (AJ-E405 Req.5, <300 ms).
    activity_tx: std::sync::mpsc::SyncSender<SessionId>,
    activity_rx: Mutex<std::sync::mpsc::Receiver<SessionId>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        let (events_tx, events_rx) = std::sync::mpsc::channel();
        let (activity_tx, activity_rx) = std::sync::mpsc::sync_channel(256);
        Self {
            journal: None,
            ledger: None,
            ledger_degraded: std::sync::atomic::AtomicBool::new(false),
            spawn_contexts: Mutex::new(HashMap::new()),
            children: Mutex::new(HashMap::new()),
            adopted: Mutex::new(HashMap::new()),
            user_stopped: Mutex::new(Vec::new()),
            table: Mutex::new(None),
            orphan_grace_secs: Mutex::new(HashMap::new()),
            events_tx,
            events_rx: Mutex::new(events_rx),
            activity_tx,
            activity_rx: Mutex::new(activity_rx),
        }
    }
}

impl Supervisor {
    /// Creates a supervisor without a journal (no crash re-adoption).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a supervisor that journals lifecycle to `journal_dir`.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::Journal`] when the journal cannot be
    /// opened.
    pub fn with_journal(journal_dir: &Path) -> Result<Self, SupervisorError> {
        Ok(Self {
            journal: Some(Journal::open(journal_dir.join("supervision.jsonl"))?),
            ..Self::default()
        })
    }

    /// Attaches the persistent spawn ledger (AJ-E501). Without it the
    /// supervisor still runs — the ledger is additive.
    #[must_use]
    pub fn with_ledger(mut self, ledger: crate::tree::SharedLedger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    /// True when a configured ledger failed (Req.3): cleanup is
    /// read-only until restart — surfaced as the degraded banner.
    #[must_use]
    pub fn ledger_degraded(&self) -> bool {
        self.ledger_degraded
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Best-effort ledger write: a failure degrades health, never the
    /// spawn (Req.3).
    fn ledger_write(
        &self,
        write: impl FnOnce(&crate::tree::SharedLedger) -> Result<(), crate::tree::LedgerError>,
    ) {
        if let Some(ledger) = &self.ledger
            && let Err(error) = write(ledger)
        {
            self.ledger_degraded
                .store(true, std::sync::atomic::Ordering::SeqCst);
            eprintln!("ai-juggler: process ledger degraded: {error}");
        }
    }

    /// Builds the Intent row for a spawn context.
    fn intent_row(
        ctx: &crate::tree::SpawnContext,
        session: &SessionId,
        program: &std::path::Path,
        args: &[String],
        cwd: &std::path::Path,
        boot_id: &str,
    ) -> juggler_core::storage::LedgerRow {
        let (cmdline_hash, cmdline_trunc) = crate::tree::cmdline_fields(program, args);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
        juggler_core::storage::LedgerRow {
            pid: None,
            pgid: None,
            started_at: None,
            exe_path: program.to_string_lossy().into_owned(),
            cmdline_hash,
            cmdline_trunc,
            boot_id: boot_id.to_string(),
            session_id: session.as_str().to_string(),
            instance_id: None,
            provider: ctx.provider.clone(),
            account_id: ctx.account_id.clone(),
            project_id: ctx.project_id.clone(),
            machine_id: ctx.machine.as_str().to_string(),
            root_pid: None,
            parent_pid: None,
            depth: 0,
            role: ctx.role.to_string(),
            state: String::new(),
            listens_on: None,
            cwd: cwd.to_string_lossy().into_owned(),
            peak_rss: None,
            peak_cpu: None,
            exit_kind: None,
            reaped_by_us: None,
            escaped_group: false,
            down_period: false,
            first_seen: now,
            last_seen: now,
        }
    }

    /// The full spawn-time identity probe (AJ-E503): fork-time OS
    /// start time + kernel-reported exe, one targeted refresh with a
    /// short retry for table lag on a just-forked pid. `None` means
    /// the probe never landed — the caller records NO identity and
    /// verification later fails CLOSED, never open.
    fn os_probed(pid: u32) -> Option<crate::tree::ProbedProcess> {
        use crate::tree::{IdentityProbe as _, OsIdentityProbe};
        let probe = OsIdentityProbe;
        for attempt in 0..5 {
            if let Some(facts) = probe.probe(pid) {
                return Some(facts);
            }
            if attempt < 4 {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        None
    }

    /// The machine's boot identity (epoch seconds as text) — the
    /// §6.2 boot gate's current value.
    fn boot_id_string() -> String {
        use crate::tree::{IdentityProbe as _, OsIdentityProbe};
        OsIdentityProbe.boot_id()
    }

    /// Builds the spawn-time identity for a just-forked child.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn spawn_identity(pid: u32, pgid: Option<u32>) -> Option<crate::tree::ProcessIdentity> {
        Self::os_probed(pid).map(|facts| crate::tree::ProcessIdentity {
            pid,
            pgid,
            start_time: facts.start_time,
            exe: facts.exe,
            boot_id: Self::boot_id_string(),
        })
    }

    /// AJ-E503 settle thread: script wrappers `exec` their
    /// interpreter AFTER our spawn-time probe, so the kernel exe
    /// legitimately moves — re-probe once the image settles and
    /// record the authoritative identity (handle + journal, so
    /// adoption verifies against the truth). Crash-safe by omission:
    /// the entry simply never happens if we die first. A start-time
    /// disagreement between the two probes means the pid was
    /// recycled mid-settle — the record is left untouched (fail
    /// closed).
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn settle_identity(&self, session: &SessionId, pid: u32, handle: Arc<ChildHandle>) {
        let journal = self.journal.clone();
        let session = session.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            let Some(facts) = Self::os_probed(pid) else {
                return;
            };
            let pgid = handle.pgid;
            let next = crate::tree::ProcessIdentity {
                pid,
                pgid,
                start_time: facts.start_time,
                exe: facts.exe,
                boot_id: Self::boot_id_string(),
            };
            if let Ok(mut identity) = handle.identity.lock() {
                if let Some(current) = identity.as_ref()
                    && current.start_time != next.start_time
                {
                    return;
                }
                *identity = Some(next.clone());
            }
            if let Some(journal) = journal {
                let _ = journal.record_identity_settled(&session, pid, &next);
            }
        });
    }

    /// Spawns one session per the launch spec. Interactive specs get a real
    /// PTY; headless specs run with piped output.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::Spawn`] when the process cannot start.
    pub fn spawn(
        &self,
        session: &SessionId,
        spec: &LaunchSpec,
        ctx: &crate::tree::SpawnContext,
    ) -> Result<SpawnedProcess, SupervisorError> {
        // Intent BEFORE spawn (AJ-E501 Req.3): the ownership record
        // exists even if the spawn or the app dies mid-breath. Ledger
        // failure degrades health, never the spawn.
        let boot_id = self
            .ledger
            .as_ref()
            .map_or_else(|| "boot-unconfigured".to_string(), |l| l.boot_id());
        self.ledger_write(|ledger| {
            ledger.intent(&Self::intent_row(
                ctx,
                session,
                &spec.program,
                &spec.args,
                &spec.cwd,
                &boot_id,
            ))
        });
        if let Ok(mut contexts) = self.spawn_contexts.lock() {
            contexts.insert(session.clone(), ctx.clone());
        }
        let spawned = if spec.needs_pty {
            self.spawn_pty(session, spec)?
        } else {
            self.spawn_piped(session, spec)?
        };
        // The identity probe already landed inside the spawn path;
        // its fork-time start time IS the ledger's started_at (one
        // probe, not two — AJ-E503).
        let started_at = spawned.identity.as_ref().map(|i| i.start_time);
        // AJ-E502 Req.2: the PGID captured per path (piped: own group
        // leader; PTY: one getpgid) lands in the ledger patch before
        // `spawn` returns.
        self.ledger_write(|ledger| ledger.spawned(session, spawned.pid, spawned.pgid, started_at));
        if let Some(journal) = &self.journal {
            journal.record_spawned(
                session,
                spawned.pid,
                &spec.program,
                spawned.identity.as_ref(),
            )?;
        }
        if let Some(handle) = self
            .children
            .lock()
            .ok()
            .and_then(|children| children.get(session).cloned())
        {
            self.settle_identity(session, spawned.pid, handle);
        }
        let _ = self.events_tx.send(LifecycleEvent::Started {
            session: session.clone(),
            pid: spawned.pid,
        });
        Ok(spawned)
    }

    /// Spawns a desktop-class harness app (AJ-E413): tracked in the
    /// SAME spawn ledger as terminal instances (journal + lifecycle
    /// events + wait thread), with **no PTY and no output capture** —
    /// the window is the surface; stdout/stderr/stdin are null so the
    /// app can never hold our pipes.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::Spawn`] when the process cannot start.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    pub fn spawn_desktop(
        &self,
        session: &SessionId,
        launch: &juggler_adapter::DesktopLaunch,
        ctx: &crate::tree::SpawnContext,
    ) -> Result<SpawnedProcess, SupervisorError> {
        use std::process::{Command, Stdio};

        let boot_id = self
            .ledger
            .as_ref()
            .map_or_else(|| "boot-unconfigured".to_string(), |l| l.boot_id());
        self.ledger_write(|ledger| {
            ledger.intent(&Self::intent_row(
                ctx,
                session,
                &launch.program,
                &launch.args,
                &launch.cwd,
                &boot_id,
            ))
        });
        if let Ok(mut contexts) = self.spawn_contexts.lock() {
            contexts.insert(session.clone(), ctx.clone());
        }

        let mut command = Command::new(&launch.program);
        command
            .args(&launch.args)
            .envs(&launch.env)
            .current_dir(&launch.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn().map_err(|e| SupervisorError::Spawn {
            program: launch.program.clone(),
            message: e.to_string(),
        })?;
        let pid = child.id();
        let pgid = group::pgid_of(pid).ok();
        let identity = Self::spawn_identity(pid, pgid);
        let started_at = identity.as_ref().map(|i| i.start_time);
        self.ledger_write(|ledger| ledger.spawned(session, pid, pgid, started_at));
        let ring = Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY)));
        let child = Arc::new(Mutex::new(child));
        let finished = Arc::new(AtomicBool::new(false));
        let kill_child = Arc::clone(&child);
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = kill_child.lock().map(|mut c| c.kill());
        });
        let exit_at = Arc::new(AtomicI64::new(0));
        let handle = self.register(
            session.clone(),
            pid,
            ring,
            Arc::clone(&finished),
            force_kill,
            ChildExtras {
                pty: None,
                last_output_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
                pgid,
                identity: identity.clone(),
                exit_at_ms: Arc::clone(&exit_at),
            },
        );
        if let Some(journal) = &self.journal {
            journal.record_spawned(session, pid, &launch.program, identity.as_ref())?;
        }
        self.settle_identity(session, pid, handle);
        let _ = self.events_tx.send(LifecycleEvent::Started {
            session: session.clone(),
            pid,
        });

        let session_for_thread = session.clone();
        let journal = self.journal.clone();
        let ledger = self.ledger.clone();
        let events = self.events_tx.clone();
        let done = Arc::clone(&finished);
        std::thread::spawn(move || {
            let status = child.lock().ok().and_then(|mut c| c.wait().ok());
            if let Some(status) = status {
                emit_exit(
                    &session_for_thread,
                    pid,
                    exit_kind_std(status),
                    &done,
                    &exit_at,
                    journal.as_ref(),
                    ledger.as_ref(),
                    &events,
                );
            } else {
                done.store(true, Ordering::Release);
            }
        });

        Ok(SpawnedProcess {
            session: session.clone(),
            pid,
            pgid,
            identity,
        })
    }

    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn spawn_piped(
        &self,
        session: &SessionId,
        spec: &LaunchSpec,
    ) -> Result<SpawnedProcess, SupervisorError> {
        use std::process::{Command, Stdio};

        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .envs(&spec.env)
            .current_dir(&spec.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // AJ-E502: the piped root becomes its OWN group leader at
        // spawn — every in-group helper is later reachable as one
        // owned group (deterministic kill), not a ppid-walk snapshot.
        // Where groups do not exist, spawn proceeds groupless (the
        // honest `None`, never a pretended group).
        if let Err(group::GroupError::UnsupportedPlatform) = group::make_group_leader(&mut command)
        {
            eprintln!(
                "ai-juggler: process groups unsupported; tracking pid-only for {}",
                session.as_str()
            );
        }
        let child = command.spawn().map_err(|e| SupervisorError::Spawn {
            program: spec.program.clone(),
            message: e.to_string(),
        })?;
        let pid = child.id();
        let pgid = group::pgid_of(pid).ok();
        let identity = Self::spawn_identity(pid, pgid);
        let ring = Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY)));
        let child = Arc::new(Mutex::new(child));
        let finished = Arc::new(AtomicBool::new(false));
        let kill_child = Arc::clone(&child);
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = kill_child.lock().map(|mut c| c.kill());
        });
        // Register before readers start so output tracking exists from
        // the first chunk (AJ-E405).
        let exit_at = Arc::new(AtomicI64::new(0));
        let handle = self.register(
            session.clone(),
            pid,
            Arc::clone(&ring),
            Arc::clone(&finished),
            force_kill,
            ChildExtras {
                pty: None,
                last_output_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
                pgid,
                identity: identity.clone(),
                exit_at_ms: Arc::clone(&exit_at),
            },
        );
        let last_output = Arc::clone(&handle.last_output_ms);
        let (out_atomic, err_atomic) = (Arc::clone(&last_output), last_output);
        let (out_tx, err_tx) = (
            std::sync::mpsc::SyncSender::clone(&self.activity_tx),
            std::sync::mpsc::SyncSender::clone(&self.activity_tx),
        );
        let (out_session, err_session) = (session.clone(), session.clone());
        spawn_reader(
            child_lock(&child).stdout.take(),
            Arc::clone(&ring),
            out_atomic,
            out_session,
            out_tx,
        );
        spawn_reader(
            child_lock(&child).stderr.take(),
            ring,
            err_atomic,
            err_session,
            err_tx,
        );

        let session_for_thread = session.clone();
        let journal = self.journal.clone();
        let ledger = self.ledger.clone();
        let events = self.events_tx.clone();
        let done = Arc::clone(&finished);
        std::thread::spawn(move || {
            let status = child.lock().ok().and_then(|mut c| c.wait().ok());
            if let Some(status) = status {
                emit_exit(
                    &session_for_thread,
                    pid,
                    exit_kind_std(status),
                    &done,
                    &exit_at,
                    journal.as_ref(),
                    ledger.as_ref(),
                    &events,
                );
            } else {
                done.store(true, Ordering::Release);
            }
        });

        Ok(SpawnedProcess {
            session: session.clone(),
            pid,
            pgid,
            identity,
        })
    }

    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    fn spawn_pty(
        &self,
        session: &SessionId,
        spec: &LaunchSpec,
    ) -> Result<SpawnedProcess, SupervisorError> {
        use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

        let pty_system: Box<dyn PtySystem> = Box::new(NativePtySystem {});
        let pair = pty_system
            .openpty(PtySize::default())
            .map_err(|e| SupervisorError::Spawn {
                program: spec.program.clone(),
                message: e.to_string(),
            })?;
        let mut builder = CommandBuilder::new(&spec.program);
        builder.cwd(&spec.cwd);
        builder.args(&spec.args);
        // Env semantics (spec §4): the overlay merges over the supervisor's
        // base environment. portable-pty's builder does not reliably inherit
        // the parent env on all platforms, so inherit explicitly, then apply
        // the overlay last so it wins.
        for (key, value) in std::env::vars_os() {
            builder.env(key, value);
        }
        for (key, value) in &spec.env {
            builder.env(key, value);
        }
        let mut child = pair
            .slave
            .spawn_command(builder)
            .map_err(|e| SupervisorError::Spawn {
                program: spec.program.clone(),
                message: e.to_string(),
            })?;
        let Some(pid) = child.process_id() else {
            return Err(SupervisorError::Spawn {
                program: spec.program.clone(),
                message: "pty child had no process id".into(),
            });
        };
        let killer = std::sync::Mutex::new(child.clone_killer());
        let ring = Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY)));
        // Output tracking exists from chunk 1 without pre-registering the
        // handle: the atomic is independent of the children map.
        let last_output = Arc::new(std::sync::atomic::AtomicI64::new(0));
        if let Ok(reader) = pair.master.try_clone_reader() {
            spawn_reader(
                Some(reader),
                Arc::clone(&ring),
                Arc::clone(&last_output),
                session.clone(),
                std::sync::mpsc::SyncSender::clone(&self.activity_tx),
            );
        }
        drop(pair.slave);
        let master = pair.master;
        let writer = master.take_writer().map_err(|e| SupervisorError::Spawn {
            program: spec.program.clone(),
            message: format!("cannot take pty writer: {e}"),
        })?;
        let pty = Some(Arc::new(Mutex::new(PtyHandle { writer, master })));

        let finished = Arc::new(AtomicBool::new(false));
        let exit_at = Arc::new(AtomicI64::new(0));
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = killer.lock().map(|mut k| k.kill());
        });

        let session_for_thread = session.clone();
        let journal = self.journal.clone();
        let ledger = self.ledger.clone();
        let events = self.events_tx.clone();
        let done = Arc::clone(&finished);
        let exit_timer = Arc::clone(&exit_at);
        std::thread::spawn(move || {
            let status = child.wait().ok();
            if let Some(status) = status {
                // portable-pty reports a single exit code; a signal death surfaces
                // as a non-zero code here (skeleton-level approximation).
                let kind = ExitKind::Code(i32::try_from(status.exit_code()).unwrap_or(i32::MAX));
                emit_exit(
                    &session_for_thread,
                    pid,
                    kind,
                    &done,
                    &exit_timer,
                    journal.as_ref(),
                    ledger.as_ref(),
                    &events,
                );
            } else {
                done.store(true, Ordering::Release);
            }
        });

        // AJ-E502 Req.1: the PTY path RECORDS the child's PGID (one
        // getpgid) instead of assuming portable-pty's session
        // grouping — the identity comes from the kernel, not the
        // wrapper's convention.
        let pgid = group::pgid_of(pid).ok();
        let identity = Self::spawn_identity(pid, pgid);
        self.register(
            session.clone(),
            pid,
            ring,
            finished,
            force_kill,
            ChildExtras {
                pty,
                last_output_ms: last_output,
                pgid,
                identity: identity.clone(),
                exit_at_ms: exit_at,
            },
        );
        Ok(SpawnedProcess {
            session: session.clone(),
            pid,
            pgid,
            identity,
        })
    }

    #[allow(clippy::needless_pass_by_value)] // Arc handles are moved intentionally
    /// Registers a child handle; `parts.pty` and `parts.last_output_ms`
    /// come from the caller because the PTY path starts its reader before
    /// the handle exists (output tracking from chunk 1). Returns the
    /// stored handle (the settle thread refines its identity).
    fn register(
        &self,
        session: SessionId,
        pid: u32,
        ring: Arc<Mutex<OutputRing>>,
        finished: Arc<AtomicBool>,
        force_kill: Arc<dyn Fn() + Send + Sync>,
        parts: ChildExtras,
    ) -> Arc<ChildHandle> {
        let handle = Arc::new(ChildHandle {
            pid,
            pgid: parts.pgid,
            identity: Mutex::new(parts.identity),
            exit_at_ms: parts.exit_at_ms,
            ring,
            finished,
            last_output_ms: parts.last_output_ms,
            force_kill,
            pty: parts.pty,
        });
        if let Ok(mut children) = self.children.lock() {
            children.insert(session, handle.clone());
        }
        handle
    }

    /// Stops a session: the verified TERM → grace → KILL ladder over
    /// the session's process group (AJ-E507). The ROOT keeps its
    /// exact path — it is our direct child, reaped into the
    /// `finished` flag, where an unreaped zombie would answer
    /// `kill(pid, 0)` probes as alive — so it is TERM'd and, if the
    /// flag never sets inside `grace`, force-killed. Every OTHER
    /// group member goes through [`crate::tree::terminate_group`]:
    /// TERM, the grace window, then KILL for TERM-ignorers.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn stop(&self, session: &SessionId, grace: Duration) -> Result<(), SupervisorError> {
        let handle = self
            .children
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?
            .get(session)
            .cloned()
            .ok_or_else(|| SupervisorError::UnknownSession(session.clone()))?;

        if let Ok(mut stopped) = self.user_stopped.lock() {
            stopped.retain(|id| id != session);
            stopped.push(session.clone());
        }
        send_terminate(handle.pid);

        // The rest of the group: the verified ladder. The root is
        // excluded — a dead-but-unreaped root would make the ladder
        // escalate on a corpse. Refusals surface honestly on the next
        // reconciler tick as orphans (AJ-E504); session stop's report
        // remains the Exited lifecycle event.
        //
        // GROUP GUARD: the ladder applies ONLY to the child's OWN
        // group (`pgid == pid` — the deterministic-kill group the
        // piped path creates, AJ-E502). A child that never became a
        // leader (PTY/desktop paths, or a wrapper that `setsid`'d)
        // shares SOMEONE ELSE's group — signaling it would TERM
        // strangers, up to and including this process (the desktop
        // suite's three stop() calls caught exactly that).
        let helpers: Vec<u32> = handle
            .pgid
            .filter(|pgid| *pgid == handle.pid)
            .map(crate::tree::pgid_members)
            .unwrap_or_default()
            .into_iter()
            .filter(|pid| *pid != handle.pid)
            .collect();
        if !helpers.is_empty() {
            crate::tree::terminate_group(grace, move || helpers, || false);
        }

        let deadline = Instant::now() + grace;
        while Instant::now() < deadline && !handle.finished.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        if !handle.finished.load(Ordering::Acquire) {
            (handle.force_kill)();
        }
        Ok(())
    }

    /// Snapshot of the retained output tail for a session.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn output(&self, session: &SessionId) -> Result<Vec<u8>, SupervisorError> {
        let handle = self
            .children
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?
            .get(session)
            .cloned()
            .ok_or_else(|| SupervisorError::UnknownSession(session.clone()))?;
        handle
            .ring
            .lock()
            .map(|ring| ring.snapshot())
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))
    }

    /// Live descendant process ids of a session's process tree (ppid walk).
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn descendants(&self, session: &SessionId) -> Result<Vec<u32>, SupervisorError> {
        let handle = self
            .children
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?
            .get(session)
            .cloned()
            .ok_or_else(|| SupervisorError::UnknownSession(session.clone()))?;
        // AJ-E504 Req.1: walk the reconciler's cached ppid map — no
        // fresh full refresh per query. Before the first tick the
        // snapshot does not exist yet: one counted cold-path scan
        // (the regression gate reads the counter).
        if let Some(table) = self.snapshot_table() {
            let mut found = Vec::new();
            let mut queue: std::collections::VecDeque<u32> = table
                .parents
                .get(&handle.pid)
                .cloned()
                .into_iter()
                .flatten()
                .collect();
            while let Some(pid) = queue.pop_front() {
                found.push(pid);
                queue.extend(table.parents.get(&pid).cloned().into_iter().flatten());
            }
            return Ok(found);
        }
        Ok(descendants_of(handle.pid))
    }

    /// When output last arrived for the session (epoch ms); `None` when
    /// unknown/never. Feeds the Stuck rule (AJ-E405).
    #[must_use]
    pub fn last_output_ms(&self, session: &SessionId) -> Option<i64> {
        let children = self.children.lock().ok()?;
        let handle = children.get(session)?;
        let at = handle.last_output_ms.load(Ordering::Relaxed);
        (at > 0).then_some(at)
    }

    /// The last `max_bytes` of captured output (for prompt and
    /// rate-limit signature detection; never the whole ring).
    #[must_use]
    pub fn output_tail(&self, session: &SessionId, max_bytes: usize) -> Option<String> {
        let output = self.output(session).ok()?;
        let start = output.len().saturating_sub(max_bytes);
        Some(String::from_utf8_lossy(&output[start..]).into_owned())
    }

    /// Drains sessions with fresh output since the last call — the
    /// event-driven recovery signal (<300 ms, Req.5).
    pub fn drain_activity(&self) -> Vec<SessionId> {
        let Ok(receiver) = self.activity_rx.lock() else {
            return Vec::new();
        };
        let mut drained = Vec::new();
        while let Ok(session) = receiver.try_recv() {
            if drained.last() != Some(&session) {
                drained.push(session);
            }
        }
        drained
    }

    /// Batched live stats for every supervised session, read from ONE
    /// already-refreshed process table (AJ-E401: the read model must not
    /// trigger a table refresh per row). The caller owns the [`sysinfo`]
    /// table — via [`LiveStatsCollector`] — so CPU% deltas persist
    /// between ticks. Finished sessions report definitive zeros; a live
    /// root sums CPU/RSS over the root process and its descendants
    /// (children counted, the root itself not).
    ///
    /// CPU% is only meaningful when the table has been refreshed at least
    /// twice; [`LiveStatsCollector::refresh`] masks the first sample.
    #[must_use]
    pub fn live_stats(&self, system: &sysinfo::System) -> HashMap<SessionId, LiveProcessStats> {
        let parents = parents_map(system);
        self.live_roots()
            .into_iter()
            .map(|(session, root, finished)| {
                let stats = if finished {
                    LiveProcessStats::dead()
                } else {
                    tree_stats(system, &parents, root)
                };
                (session, stats)
            })
            .collect()
    }

    /// Supervised roots as (session, root pid, finished) — the shared
    /// enumeration for the collector and the read model.
    #[must_use]
    pub fn live_roots(&self) -> Vec<(SessionId, u32, bool)> {
        self.children
            .lock()
            .map(|children| {
                children
                    .iter()
                    .map(|(session, handle)| {
                        (
                            session.clone(),
                            handle.pid,
                            handle.finished.load(Ordering::Relaxed),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The root's process-group id as captured at spawn (AJ-E502) —
    /// the containment key helpers are compared against at reconcile
    /// time. `None` when the platform lacks groups (Windows seam,
    /// M7-5) or the session is not supervised.
    #[must_use]
    pub fn root_pgid(&self, session: &SessionId) -> Option<u32> {
        self.children
            .lock()
            .ok()
            .and_then(|children| children.get(session).and_then(|h| h.pgid))
    }

    /// The root's recorded identity (AJ-E503) — the verify-before-kill
    /// input for the reconciler (E504) and the kill ladder (E507/
    /// E508). `None` when the spawn probe never landed (verification
    /// then fails closed) or the session is not supervised.
    #[must_use]
    pub fn root_identity(&self, session: &SessionId) -> Option<crate::tree::ProcessIdentity> {
        self.children
            .lock()
            .ok()?
            .get(session)?
            .identity
            .lock()
            .ok()?
            .clone()
    }

    /// Publishes the reconciler's process-table snapshot (one per
    /// tick, from the ONE shared `sysinfo::System` — AJ-E504 Req.1).
    fn publish_table(&self, table: std::sync::Arc<ProcessTable>) {
        if let Ok(mut slot) = self.table.lock() {
            *slot = Some(table);
        }
    }

    /// Spawn contexts as `(session, ctx)` pairs (AJ-E505: the hog
    /// rules walk per-Provider populations).
    #[must_use]
    pub fn spawn_contexts_snapshot(&self) -> Vec<(SessionId, crate::tree::SpawnContext)> {
        self.spawn_contexts
            .lock()
            .map(|contexts| {
                contexts
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The last published process-table snapshot (`None` before the
    /// first tick — callers then take the counted cold-path scan).
    #[must_use]
    pub fn snapshot_table(&self) -> Option<std::sync::Arc<ProcessTable>> {
        self.table.lock().ok().and_then(|slot| slot.clone())
    }

    /// Sets the orphan grace for one Provider (seconds); the `""` key
    /// overrides the default for every Provider without its own entry
    /// (AJ-E504 Req.3 — per-Provider tunable, stored in settings by
    /// the app at startup).
    pub fn set_orphan_grace(&self, provider: &str, secs: u64) {
        if let Ok(mut graces) = self.orphan_grace_secs.lock() {
            graces.insert(provider.to_string(), secs.max(1));
        }
    }

    /// The effective orphan grace for `provider` (seconds): its own
    /// entry, else the `""` default override, else
    /// [`DEFAULT_ORPHAN_GRACE_SECS`] (30 — the false-alert budget;
    /// never lowered for demos).
    #[must_use]
    pub fn orphan_grace_for(&self, provider: &str) -> u64 {
        self.orphan_grace_secs
            .lock()
            .ok()
            .and_then(|graces| graces.get(provider).or_else(|| graces.get("")).copied())
            .unwrap_or(DEFAULT_ORPHAN_GRACE_SECS)
    }

    /// Feeds keyboard input to a PTY session's process.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised, [`SupervisorError::NotATty`] for piped sessions.
    pub fn write_input(&self, session: &SessionId, bytes: &[u8]) -> Result<(), SupervisorError> {
        let handle = self.child(session)?;
        let pty = handle.pty.as_ref().ok_or(SupervisorError::NotATty {
            session: session.clone(),
        })?;
        let mut pty = pty
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?;
        pty.writer
            .write_all(bytes)
            .and_then(|()| pty.writer.flush())
            .map_err(SupervisorError::Io)
    }

    /// Resizes a PTY session (cols/rows in character cells).
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised, [`SupervisorError::NotATty`] for piped sessions.
    pub fn resize_pty(
        &self,
        session: &SessionId,
        cols: u16,
        rows: u16,
    ) -> Result<(), SupervisorError> {
        let handle = self.child(session)?;
        let pty = handle.pty.as_ref().ok_or(SupervisorError::NotATty {
            session: session.clone(),
        })?;
        let pty = pty
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?;
        pty.master
            .resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SupervisorError::Io(std::io::Error::other(e.to_string())))
    }

    /// Subscribes to this session's live output stream (each chunk as
    /// written). Snapshot-then-tap is race-free: the ring forwards every
    /// byte to listeners, so a snapshot followed by a tap yields the
    /// stream exactly once from the snapshot point onward.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError::UnknownSession`] when the id is not
    /// supervised.
    pub fn tap(
        &self,
        session: &SessionId,
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, SupervisorError> {
        let handle = self.child(session)?;
        let mut ring = handle
            .ring
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?;
        let (tx, rx) = std::sync::mpsc::channel();
        ring.add_listener(tx);
        Ok(rx)
    }

    /// Sessions from previous app runs whose processes are still alive
    /// (journal adoption; spec §5.3 crash re-adoption).
    /// One startup recovery pass (AJ-E510, pass2 §11.2): over the
    /// ledger's live rows — (1) identity-verify each (failures become
    /// `Unverifiable`: listed, never killed); (2) rebuild ADOPTED
    /// handles for verified live roots (output unavailable, stop =
    /// the verified group ladder through [`Supervisor::stop`] —
    /// closing F-3); (3) helpers of roots that died while Juggler was
    /// down enter `Orphaned` (their `reaped_by_us=false` root is the
    /// `left behind while Juggler was closed` marker — attribution
    /// preserved); (4) dead roots patch `exited`. Returns the report
    /// for the one-time summary (Req.3) and the tier suspension.
    #[allow(clippy::similar_names)] // pid/pgid are the kernel's own names
    pub fn recover(
        &self,
        rows_by_session: &[(SessionId, Vec<juggler_core::storage::LedgerRow>)],
    ) -> RecoveryReport {
        use crate::tree::Verification;
        let mut report = RecoveryReport::default();
        for (session, rows) in rows_by_session {
            let Some(root) = rows.iter().find(|row| row.role == "root") else {
                continue;
            };
            let Some(root_pid) = root.pid else { continue };
            let root_alive = crate::tree::verify_start_time(root_pid, &root_identity(root))
                == Verification::Verified;

            for row in rows {
                let Some(pid) = row.pid else { continue };
                if !matches!(row.state.as_str(), "running" | "orphaned") {
                    continue;
                }
                let identity = row_identity(row);
                // E504/E507's aliasing rule: a ROOT's exe legitimately
                // moves across `exec` (`/bin/sh` records, the kernel
                // later reads `/bin/bash`) — roots verify by
                // boot + start time; helpers carry the kernel-true
                // triple from the reconciler's table.
                let verified = if row.role == "root" {
                    crate::tree::verify_start_time(pid, &identity) == Verification::Verified
                } else {
                    crate::tree::verify(pid, &identity) == Verification::Verified
                };
                if row.role == "root" {
                    if root_alive && verified {
                        // (2) re-adopt with a working stop path.
                        self.register_adopted(session, root);
                        report.adopted.push(AdoptionRecord {
                            session: session.clone(),
                            pid,
                            program: root.exe_path.clone(),
                        });
                    } else if crate::orphans::process_alive(pid).is_some() {
                        // (1) alive but unverifiable: listed, never killed.
                        self.ledger_write(|ledger| ledger.mark_unverifiable(pid, &row.boot_id));
                        report.unverifiable.push(pid);
                    } else {
                        // (4) died while we were down.
                        self.ledger_write(|ledger| {
                            ledger.mark_exited(pid, &row.boot_id, "unknown", false)
                        });
                    }
                } else if !verified {
                    if crate::orphans::process_alive(pid).is_some() {
                        self.ledger_write(|ledger| ledger.mark_unverifiable(pid, &row.boot_id));
                        report.unverifiable.push(pid);
                    }
                } else if !root_alive {
                    // (3) a live helper of a root that died while
                    // Juggler was closed — the durable down-period
                    // marker + original attribution.
                    self.ledger_write(|ledger| ledger.mark_orphaned_down(pid, &row.boot_id));
                    report.left_behind.push(pid);
                }
            }
        }
        report
    }

    /// Registers an ADOPTED handle (AJ-E510, F-3): no ring readers, no
    /// PTY (output from before the crash is honestly unavailable),
    /// `finished` live — `stop()` now works through the same verified
    /// group ladder as any supervised session.
    fn register_adopted(&self, session: &SessionId, root: &juggler_core::storage::LedgerRow) {
        let pid = root.pid.unwrap_or(0);
        let kill_pid = pid;
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            // No std Child to `.kill()` — the adopted root is not our
            // fork. The honest fallback: a direct KILL through the ONE
            // platform signal seam (unix signals / no-op off-unix).
            for member in crate::tree::pgid_members(kill_pid) {
                let _ = crate::tree::signal_pid(member, crate::tree::Signal::Kill);
            }
        });
        let handle = Arc::new(ChildHandle {
            pid,
            pgid: root.pgid,
            identity: Mutex::new(Some(row_identity(root))),
            exit_at_ms: Arc::new(AtomicI64::new(0)),
            ring: Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY))),
            finished: Arc::new(AtomicBool::new(false)),
            last_output_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            force_kill,
            pty: None,
        });
        if let Ok(mut children) = self.children.lock() {
            children.insert(session.clone(), Arc::clone(&handle));
        }
        if let Ok(mut adopted) = self.adopted.lock() {
            adopted.insert(session.clone(), handle);
        }
        // The stop path reads the spawn context for nothing, but the
        // reconciler's helper upserts DO — re-seed it from the root
        // row's attribution so helpers keep their provider/account.
        if let Ok(mut contexts) = self.spawn_contexts.lock() {
            contexts
                .entry(session.clone())
                .or_insert_with(|| crate::tree::SpawnContext {
                    provider: root.provider.clone(),
                    account_id: root.account_id.clone(),
                    project_id: root.project_id.clone(),
                    machine: juggler_core::model::MachineId::new(root.machine_id.clone()),
                    role: "root",
                });
        }
    }

    /// The re-adopted session set (AJ-E510): sessions whose verified
    /// live roots got handles back at startup — the `Running —
    /// re-adopted` marker for the Cleanup Centre, port map, and the
    /// read-model reconciliation (they stay `Running`, never
    /// `Stopped`).
    pub fn alive_adopted_sessions(&self) -> Vec<SessionId> {
        // AJ-E510: the marker set built by crash recovery — the
        // re-adopted sessions ARE supervised again (stoppable via the
        // E507 path); the pre-E510 heuristic (journal-live minus
        // supervised) could not express that and double-counted live
        // sessions (the AJ-E505 port-classification bug).
        self.adopted
            .lock()
            .map(|adopted| adopted.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The supervision journal, when one is open (descendant snapshots
    /// and kill audit writes, AJ-E410).
    #[must_use]
    pub fn journal_ref(&self) -> Option<&Journal> {
        self.journal.as_ref()
    }

    /// True when the session was stopped through [`Supervisor::stop`]
    /// (user intent — not a crash), regardless of how the exit surfaces.
    #[must_use]
    pub fn was_stopped(&self, session: &SessionId) -> bool {
        self.user_stopped
            .lock()
            .is_ok_and(|stopped| stopped.iter().any(|id| id == session))
    }

    /// True when this app's supervisor owns a handle for the session
    /// (live or finished — adopted-from-journal sessions are not).
    #[must_use]
    pub fn is_supervised(&self, session: &SessionId) -> bool {
        self.children
            .lock()
            .is_ok_and(|children| children.contains_key(session))
    }

    /// True when the session's process has exited.
    #[must_use]
    pub fn is_finished(&self, session: &SessionId) -> bool {
        self.children
            .lock()
            .ok()
            .and_then(|children| {
                children
                    .get(session)
                    .map(|h| h.finished.load(Ordering::Acquire))
            })
            .unwrap_or(true)
    }

    fn child(&self, session: &SessionId) -> Result<Arc<ChildHandle>, SupervisorError> {
        self.children
            .lock()
            .map_err(|_| SupervisorError::UnknownSession(session.clone()))?
            .get(session)
            .cloned()
            .ok_or_else(|| SupervisorError::UnknownSession(session.clone()))
    }

    /// Non-blocking event read.
    #[must_use]
    pub fn try_recv(&self) -> Option<LifecycleEvent> {
        self.events_rx.lock().ok().and_then(|rx| rx.try_recv().ok())
    }

    /// Blocks up to `timeout` for the next lifecycle event.
    #[must_use]
    pub fn recv_timeout(&self, timeout: Duration) -> Option<LifecycleEvent> {
        self.events_rx
            .lock()
            .ok()
            .and_then(|rx| rx.recv_timeout(timeout).ok())
    }
}

fn spawn_reader(
    reader: Option<impl std::io::Read + Send + 'static>,
    ring: Arc<Mutex<OutputRing>>,
    last_output_ms: Arc<std::sync::atomic::AtomicI64>,
    session: SessionId,
    activity: std::sync::mpsc::SyncSender<SessionId>,
) {
    let Some(mut reader) = reader else {
        return;
    };
    std::thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        let mut last_notify = 0_i64;
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let now = Timestamp::now().millis();
                    last_output_ms.store(now, Ordering::Relaxed);
                    if let Ok(mut ring) = ring.lock() {
                        ring.push(&chunk[..n]);
                    }
                    // Throttle activity signals to ~1/250 ms per session —
                    // recovery latency stays <300 ms without spam.
                    if now - last_notify >= 250 {
                        last_notify = now;
                        if activity.send(session.clone()).is_err() {
                            break; // receiver gone
                        }
                    }
                }
            }
        }
    });
}

/// Helper: lock a mutex-wrapped child without unwrap (clippy: no unwrap
/// in production paths).
fn child_lock(
    child: &Arc<Mutex<std::process::Child>>,
) -> std::sync::MutexGuard<'_, std::process::Child> {
    // A poisoned child mutex means the wait thread panicked; treat as
    // no-op by leaking a dummy is impossible — fall back to blocking
    // lock, mapping poison to an empty guard via unwrap_or_else.
    child
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[allow(clippy::too_many_arguments)] // one flat exit fan-out, grouping would churn 3 callers
fn emit_exit(
    session: &SessionId,
    pid: u32,
    kind: ExitKind,
    done: &AtomicBool,
    exit_at: &AtomicI64,
    journal: Option<&Journal>,
    ledger: Option<&crate::tree::SharedLedger>,
    events: &std::sync::mpsc::Sender<LifecycleEvent>,
) {
    done.store(true, Ordering::Release);
    // The eviction timer starts HERE (AJ-E504 Req.4): the retention
    // window is measured from the observed exit, not the stop call.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default();
    exit_at.store(now, Ordering::Release);
    if let Some(journal) = journal {
        let _ = journal.record_exited(session, pid, kind);
    }
    // The ledger exit patch (AJ-E501): roots are reaped by us; a
    // failure degrades health on the next supervisor touch (the flag
    // lives on the shared supervisor, mirrored here best-effort).
    if let Some(ledger) = ledger {
        let exit_kind = match kind {
            ExitKind::Code(code) => format!("code:{code}"),
            ExitKind::Signaled(signal) => {
                format!(
                    "signal:{}",
                    signal.map_or_else(|| "unknown".into(), |s| s.to_string())
                )
            }
        };
        if ledger
            .mark_exited(pid, &ledger.boot_id(), &exit_kind, true)
            .is_err()
        {
            eprintln!("ai-juggler: process ledger degraded at exit patch (pid {pid})");
        }
    }
    let _ = events.send(LifecycleEvent::Exited {
        session: session.clone(),
        pid,
        kind,
    });
}

fn exit_kind_std(status: std::process::ExitStatus) -> ExitKind {
    match status.code() {
        Some(code) => ExitKind::Code(code),
        None => signaled_kind(status),
    }
}

#[cfg(unix)]
fn signaled_kind(status: std::process::ExitStatus) -> ExitKind {
    use std::os::unix::process::ExitStatusExt;
    ExitKind::Signaled(status.signal())
}

#[cfg(not(unix))]
fn signaled_kind(_status: std::process::ExitStatus) -> ExitKind {
    ExitKind::Signaled(None)
}

#[cfg(unix)]
fn send_terminate(pid: u32) {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    let nix_pid = Pid::from_raw(i32::try_from(pid).unwrap_or_default());
    let _ = kill(nix_pid, Some(Signal::SIGTERM));
}

#[cfg(not(unix))]
fn send_terminate(_pid: u32) {
    // No SIGTERM on this platform; the stop ladder goes straight to kill.
}

fn descendants_of(root: u32) -> Vec<u32> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

    let mut system = crate::new_system();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut parents: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            parents
                .entry(parent.as_u32())
                .or_default()
                .push(pid.as_u32());
        }
    }
    let mut found = Vec::new();
    let mut queue: std::collections::VecDeque<u32> =
        parents.get(&root).cloned().into_iter().flatten().collect();
    while let Some(pid) = queue.pop_front() {
        found.push(pid);
        queue.extend(parents.get(&pid).cloned().into_iter().flatten());
    }
    found
}

/// Sums CPU/RSS over the root process and its descendants from a table
/// that has already been refreshed by the caller (no refresh here —
/// `Supervisor::live_stats` batches many sessions over one table).
fn tree_stats(
    system: &sysinfo::System,
    parents: &HashMap<u32, Vec<u32>>,
    root: u32,
) -> LiveProcessStats {
    let mut cpu = 0.0_f64;
    let mut rss = 0_u64;
    let mut child_count = 0_u32;
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<u32> = std::iter::once(root).collect();
    while let Some(pid) = queue.pop_front() {
        if !seen.insert(pid) {
            continue; // ppid cycles must not loop forever
        }
        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            cpu += f64::from(process.cpu_usage());
            rss += process.memory();
            if pid != root {
                child_count += 1;
            }
        }
        queue.extend(parents.get(&pid).cloned().into_iter().flatten());
    }
    LiveProcessStats {
        child_count,
        cpu_pct: Some(cpu),
        rss_bytes: rss,
    }
}

/// Path-qualified helper for callers constructing journal locations.
#[must_use]
pub fn default_journal_path(state_dir: &Path) -> PathBuf {
    state_dir.join("supervision.jsonl")
}
