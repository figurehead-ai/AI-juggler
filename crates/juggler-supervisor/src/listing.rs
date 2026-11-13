//! AJ-E2401: one listing's read of every process, as the observed trees
//! are measured from it (the observed tools' trees, and the Processes
//! view's own roots). Split from `live_stats` (its size budget); the
//! measuring is unchanged.
//!
//! AJ-E805 (0.4.5, idle CPU; D-108): where the system lists every process
//! in one call (`juggler_proc_list`), the listing reads only the processes
//! of the watched trees (ours, the observed tools'), not all of them:
//! `sysinfo` reads each process's arguments on every refresh whatever it
//! is asked, and the listing every 4 s was a fifth of the app's idle CPU
//! in the lab. The zombies come from the same call (no `/bin/ps`).

use std::collections::HashMap;

use crate::cpu_since::CpuSince;
use crate::live_stats::{
    CollectorWork, ObservedRoot, ObservedTree, TreeSample, observed_sums, parents_map,
    push_ring_sample, tree_rows,
};
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

/// Reads the listing `table` (CPU and memory) and says what it found.
/// Where the system lists every process in one call, only the processes
/// under `roots` (the watched trees' roots) are read, the ones gone since
/// (or reused) dropped, and the zombies taken from the same listing;
/// elsewhere every process is read, as before.
pub(crate) fn read(
    table: &mut sysinfo::System,
    cpu: &mut CpuSince,
    roots: &[u32],
    zombies: &mut Zombies,
) -> Listed {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
    let kind = ProcessRefreshKind::nothing()
        .without_tasks()
        .with_cpu()
        .with_memory();
    let at = std::time::Instant::now();
    let Some(all) = juggler_proc_list::list() else {
        table.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        cpu.note_all(table, at);
        return Listed::of_table(table);
    };
    if !roots.is_empty() {
        zombies.replace(zombies_of(&all));
    }
    let mut listed = Listed::of_list(&all);
    let mut pids = listed.under(roots);
    pids.extend(
        table
            .processes()
            .iter()
            .filter(|(pid, process)| {
                listed.starts.get(&pid.as_u32()) != Some(&process.start_time())
            })
            .map(|(pid, _)| pid.as_u32()),
    );
    pids.sort_unstable();
    pids.dedup();
    let targets: Vec<sysinfo::Pid> = pids.into_iter().map(sysinfo::Pid::from_u32).collect();
    table.refresh_processes_specifics(ProcessesToUpdate::Some(&targets), true, kind);
    cpu.note(table, &targets, at);
    cpu.forget_gone(table);
    listed.read = targets.len() as u64;
    listed
}

/// One listing's read, as the observed trees are measured from it.
pub(crate) struct Listing<'a> {
    pub(crate) system: &'a sysinfo::System,
    pub(crate) cpu: &'a CpuSince,
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
            let process = self.system.process(sysinfo::Pid::from_u32(root));
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
            if self.supervised.contains(&root) || !alive || !same {
                trees.remove(&root);
                continue;
            }
            let (rows, _) = tree_rows(self.system, self.cpu, self.parents, root, self.zombies);
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
