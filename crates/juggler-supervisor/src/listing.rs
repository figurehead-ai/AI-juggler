//! AJ-E2401: one listing's read of every process, as the observed trees
//! are measured from it (the observed tools' trees, and the Processes
//! view's own roots). Split from `live_stats` (its size budget); the
//! measuring is unchanged.
//!
//! AJ-E805 (0.4.5, idle CPU; D-110): where the system lists every process
//! in one call (`juggler_proc_list`), the listing reads only the processes
//! of the watched trees (ours, the observed tools'), not all of them:
//! `sysinfo` reads each process's arguments on every refresh whatever it
//! is asked, and the listing every 4 s was a fifth of the app's idle CPU
//! in the lab. The zombies come from the same call (no `/bin/ps`).
//!
//! AJ-E805 round 4 (D-110's fifth block): the observed trees are measured
//! from the kernel's task record (`juggler_proc_list::task`: memory and
//! CPU time), not through `sysinfo`, which read each of their processes'
//! arguments again at every listing (the Claude app's ~300 processes on a
//! busy Mac). A process new to the collector's table is read whole there
//! once (its program's name for the row, its command line for its role);
//! after that, a listing reads its task record alone.

use std::collections::{HashMap, HashSet};

use crate::cpu_since::CpuSince;
use crate::live_stats::{
    CollectorWork, ObservedRoot, ObservedTree, ProcessRow, TreeSample, observed_sums, parents_map,
    push_ring_sample,
};
use crate::rows::{Rows, TableRows, tree_rows};
use crate::zombies::{Zombie, Zombies};

/// What one listing found: each process's children and start time (no
/// zombie: they're [`Zombies`]).
#[derive(Debug, Default)]
pub(crate) struct Listed {
    /// Each process's children.
    pub(crate) children: HashMap<u32, Vec<u32>>,
    /// Each process's start time, epoch seconds.
    starts: HashMap<u32, u64>,
    /// How many processes the listing read.
    pub(crate) read: u64,
    /// Of them, read whole (program and arguments).
    pub(crate) whole: u64,
}

impl Listed {
    fn of_list(all: &[juggler_proc_list::Listed]) -> Self {
        let mut listed = Self::default();
        for process in all.iter().filter(|process| !process.zombie) {
            listed.starts.insert(process.pid, process.start_secs);
            if process.ppid != 0 {
                listed
                    .children
                    .entry(process.ppid)
                    .or_default()
                    .push(process.pid);
            }
        }
        listed
    }

    fn of_table(table: &sysinfo::System) -> Self {
        Self {
            read: table.processes().len() as u64,
            whole: table.processes().len() as u64,
            children: parents_map(table),
            starts: table
                .processes()
                .iter()
                .map(|(pid, process)| (pid.as_u32(), process.start_time()))
                .collect(),
        }
    }

    /// `roots` and every process under them, as listed (a root gone since
    /// is left out, with nothing under it).
    fn under(&self, roots: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut queue: Vec<u32> = roots.to_vec();
        while let Some(pid) = queue.pop() {
            if !self.starts.contains_key(&pid) || !seen.insert(pid) {
                continue;
            }
            out.push(pid);
            if let Some(kids) = self.children.get(&pid) {
                queue.extend(kids);
            }
        }
        out
    }

    /// Every process under `roots` that `held` doesn't hold yet, or holds
    /// for an earlier process with that pid (another start time).
    pub(crate) fn new_under(&self, roots: &[u32], held: &sysinfo::System) -> Vec<u32> {
        self.under(roots)
            .into_iter()
            .filter(|pid| {
                held.process(sysinfo::Pid::from_u32(*pid))
                    .is_none_or(|process| self.starts.get(pid) != Some(&process.start_time()))
            })
            .collect()
    }
}

/// The zombies a listing names.
fn zombies_of(all: &[juggler_proc_list::Listed]) -> Vec<Zombie> {
    all.iter()
        .filter(|process| process.zombie)
        .map(|process| Zombie {
            pid: process.pid,
            ppid: process.ppid,
            start_time: i64::try_from(process.start_secs).unwrap_or_default(),
            name: process.name.clone(),
        })
        .collect()
}

/// The zombies again, while a tree is `watched`: from the system's listing,
/// else one `ps` ([`Zombies::refresh`]).
pub(crate) fn read_zombies(zombies: &mut Zombies, watched: bool, now_secs: i64) {
    if !watched {
        zombies.clear();
        return;
    }
    match juggler_proc_list::list() {
        Some(all) => zombies.replace(zombies_of(&all)),
        None => zombies.refresh(now_secs),
    }
}

/// The full scan: every process's program, command line, CPU and memory
/// into `table`. Where the system lists cheaply and not `whole`, only the
/// processes it doesn't hold yet (or holds for an earlier process: another
/// start time or program name) and `ours` are read whole, the ones gone are
/// dropped, and every process's resident memory is read on its own (This
/// Mac's other apps), which it returns with how many it read whole.
/// Elsewhere, or `whole`, every process is read, as before (`None`). `sysinfo` reads each process's
/// arguments on every refresh: the full scan read all of them every 10 s
/// (AJ-E805).
pub(crate) fn full_scan(
    table: &mut sysinfo::System,
    cpu: &mut CpuSince,
    ours: &[u32],
    whole: bool,
) -> (u64, Option<HashMap<u32, u64>>) {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let kind = ProcessRefreshKind::nothing()
        .without_tasks()
        .with_cpu()
        .with_memory()
        .with_exe(UpdateKind::Always)
        .with_cmd(UpdateKind::Always);
    let at = std::time::Instant::now();
    let Some(all) = juggler_proc_list::list().filter(|_| !whole) else {
        table.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        cpu.note_all(table, at);
        return (table.processes().len() as u64, None);
    };
    let listed: HashMap<u32, &juggler_proc_list::Listed> = all
        .iter()
        .filter(|process| !process.zombie)
        .map(|process| (process.pid, process))
        .collect();
    let pids: Vec<u32> = listed
        .values()
        .filter(|listed| {
            table
                .process(sysinfo::Pid::from_u32(listed.pid))
                .is_none_or(|held| !same_process(held, listed))
        })
        .map(|listed| listed.pid)
        .chain(
            table
                .processes()
                .keys()
                .map(|pid| pid.as_u32())
                .filter(|pid| !listed.contains_key(pid)),
        )
        .chain(ours.iter().copied())
        .collect();
    let read = refresh_these(table, cpu, pids, kind, at);
    let resident = listed
        .keys()
        .filter_map(|pid| Some((*pid, juggler_proc_list::task(*pid)?.resident)))
        .collect();
    (read, Some(resident))
}

/// Reads `pids` into `table` (`kind`; each once; a gone one dropped), notes
/// their CPU time (`at`: just before) and forgets the gone ones' in `cpu`;
/// how many it read.
fn refresh_these(
    table: &mut sysinfo::System,
    cpu: &mut CpuSince,
    mut pids: Vec<u32>,
    kind: sysinfo::ProcessRefreshKind,
    at: std::time::Instant,
) -> u64 {
    pids.sort_unstable();
    pids.dedup();
    let targets: Vec<sysinfo::Pid> = pids.into_iter().map(sysinfo::Pid::from_u32).collect();
    table.refresh_processes_specifics(sysinfo::ProcessesToUpdate::Some(&targets), true, kind);
    cpu.note(table, &targets, at);
    cpu.forget_gone(table);
    targets.len() as u64
}

/// Whether `held` (read earlier) is the process the listing names: the
/// same start time (`sysinfo` reads none for another user's process), and
/// its program's name the kernel's short name (a process that replaced
/// itself with another program keeps its pid and start time).
fn same_process(held: &sysinfo::Process, listed: &juggler_proc_list::Listed) -> bool {
    let named = |name: &std::ffi::OsStr| name.to_string_lossy().starts_with(&listed.name);
    (held.start_time() == listed.start_secs || held.start_time() == 0)
        && (named(held.name())
            || held
                .exe()
                .and_then(std::path::Path::file_name)
                .is_some_and(named))
}

/// One process of an observed tree, as a listing measured it: what the
/// system's listing names, its program's name, and its task record.
#[derive(Debug, Clone)]
struct Measured {
    ppid: u32,
    start_secs: u64,
    name: String,
    resident: u64,
    cpu_ns: u64,
}

/// What the listings keep between reads.
#[derive(Debug, Default)]
pub(crate) struct ListingState {
    /// Where the system lists every process in one call: the last
    /// listing's measures of the observed trees, by pid. `None` elsewhere.
    measured: Option<HashMap<u32, Measured>>,
    /// Elsewhere: every process, read through `sysinfo` at each listing.
    table: sysinfo::System,
    /// What the reads saw of each process's CPU time.
    cpu: CpuSince,
}

impl ListingState {
    /// The last listing's rows, as the observed trees are built from them.
    pub(crate) fn rows(&self) -> Box<dyn Rows + '_> {
        match &self.measured {
            Some(measured) => Box::new(MeasuredRows {
                measured,
                cpu: &self.cpu,
                now_secs: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs()),
            }),
            None => Box::new(TableRows {
                system: &self.table,
                cpu: &self.cpu,
            }),
        }
    }
}

/// A listing's measures, as rows: no executable or command line (those are
/// the collector's table's, [`crate::LiveStatsCollector::process_briefs`]).
struct MeasuredRows<'a> {
    measured: &'a HashMap<u32, Measured>,
    cpu: &'a CpuSince,
    now_secs: u64,
}

impl Rows for MeasuredRows<'_> {
    fn holds(&self, pid: u32) -> bool {
        self.measured.contains_key(&pid)
    }

    fn row(&self, pid: u32) -> Option<ProcessRow> {
        let measured = self.measured.get(&pid)?;
        Some(ProcessRow {
            pid,
            parent_pid: (measured.ppid != 0).then_some(measured.ppid),
            name: measured.name.clone(),
            cpu_pct: self.cpu.pct_of(pid, measured.start_secs),
            rss_bytes: measured.resident,
            age_ms: self
                .now_secs
                .saturating_sub(measured.start_secs)
                .saturating_mul(1000),
            child_count: 0,
            depth: 0,
            start_time: i64::try_from(measured.start_secs).unwrap_or_default(),
            exe: None,
            cmd: None,
            zombie: false,
            cpu_time_ms: measured.cpu_ns / 1_000_000,
        })
    }
}

/// Reads the listing and says what it found. Where the system lists every
/// process in one call: each process's parent and start time (`ours`'s
/// trees' new processes are found from them), the zombies, and the
/// processes under `observed` measured from their task records; a process
/// new to the collector's `table` is read whole there first, once.
/// Elsewhere every process is read through `sysinfo`, as before.
pub(crate) fn read(
    state: &mut ListingState,
    (table, cpu): (&mut sysinfo::System, &mut CpuSince),
    ours: &[u32],
    observed: &[u32],
    zombies: &mut Zombies,
) -> Listed {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let at = std::time::Instant::now();
    let Some(all) = juggler_proc_list::list() else {
        state.measured = None;
        state.table.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .without_tasks()
                .with_cpu()
                .with_memory(),
        );
        state.cpu.note_all(&state.table, at);
        return Listed::of_table(&state.table);
    };
    if !state.table.processes().is_empty() {
        state.table = sysinfo::System::new();
    }
    if !(ours.is_empty() && observed.is_empty()) {
        zombies.replace(zombies_of(&all));
    }
    let mut listed = Listed::of_list(&all);
    let under: HashSet<u32> = listed.under(observed).into_iter().collect();
    let found: Vec<&juggler_proc_list::Listed> = all
        .iter()
        .filter(|process| !process.zombie && under.contains(&process.pid))
        .collect();
    let new: Vec<u32> = found
        .iter()
        .filter(|process| {
            table
                .process(sysinfo::Pid::from_u32(process.pid))
                .is_none_or(|held| !same_process(held, process))
        })
        .map(|process| process.pid)
        .collect();
    if !new.is_empty() {
        let whole = ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cpu()
            .with_memory()
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always);
        listed.whole = refresh_these(table, cpu, new, whole, at);
    }
    let mut measured = HashMap::with_capacity(found.len());
    for process in found {
        let Some(task) = juggler_proc_list::task(process.pid) else {
            continue; // gone since, or not this user's to read
        };
        let name = table
            .process(sysinfo::Pid::from_u32(process.pid))
            .filter(|held| held.start_time() == process.start_secs)
            .map_or_else(
                || process.name.clone(),
                |held| held.name().to_string_lossy().into_owned(),
            );
        state
            .cpu
            .note_reading(process.pid, process.start_secs, task.cpu_ns, at);
        measured.insert(
            process.pid,
            Measured {
                ppid: process.ppid,
                start_secs: process.start_secs,
                name,
                resident: task.resident,
                cpu_ns: task.cpu_ns,
            },
        );
    }
    state.cpu.keep_only(|pid| measured.contains_key(&pid));
    listed.read = measured.len() as u64;
    state.measured = Some(measured);
    listed
}

/// One listing's read, as the observed trees are measured from it.
pub(crate) struct Listing<'a> {
    pub(crate) rows: &'a dyn Rows,
    pub(crate) parents: &'a HashMap<u32, Vec<u32>>,
    pub(crate) supervised: &'a std::collections::HashSet<u32>,
    pub(crate) sampled: bool,
    pub(crate) now_ms: i64,
    pub(crate) zombies: &'a crate::zombies::Zombies,
}

impl Listing<'_> {
    /// Measures each root's tree into `trees`; returns the roots whose pid
    /// now names another process (dropped until the next scan).
    pub(crate) fn measure(
        &self,
        roots: &[ObservedRoot],
        trees: &mut HashMap<u32, ObservedTree>,
        work: &mut CollectorWork,
    ) -> Vec<u32> {
        let mut reused = Vec::new();
        for pinned in roots {
            let root = pinned.pid;
            let process = self.rows.row(root);
            let alive = process.as_ref().is_some_and(|process| !process.zombie);
            // The pid now names another process: never measure it as the
            // tool (and stop watching it until the next scan).
            let same = process.as_ref().is_none_or(|process| {
                pinned
                    .start_time
                    .is_none_or(|started| process.start_time == started)
            });
            if !same {
                reused.push(root);
            }
            if self.supervised.contains(&root) || !alive || !same {
                trees.remove(&root);
                continue;
            }
            let (rows, _) = tree_rows(self.rows, self.parents, root, self.zombies);
            work.targeted_observed += rows.len() as u64;
            // A supervised subtree never also counts as observed.
            let mut stats = observed_sums(&rows, self.supervised);
            if !self.sampled {
                stats.cpu_pct = None; // no delta yet — never a fake idle
            }
            let tree = trees.entry(root).or_default();
            tree.stats = stats;
            tree.pids = rows.iter().map(|row| row.pid).collect();
            tree.rows = rows;
            push_ring_sample(
                &mut tree.ring,
                TreeSample {
                    at_ms: self.now_ms,
                    cpu_pct: stats.cpu_pct,
                    rss_bytes: stats.rss_bytes,
                },
            );
        }
        reused
    }
}
