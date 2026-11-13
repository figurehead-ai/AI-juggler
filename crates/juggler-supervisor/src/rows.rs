//! A process tree's rows (AJ-E410, AJ-E1402), read from wherever its
//! processes were measured: the collector's `sysinfo` table (AI Juggler's
//! own sessions), or the listing's own reads of the kernel's task records
//! (the observed tools' trees, AJ-E805: no `sysinfo` refresh, so no
//! process's arguments read again at every listing). Moved out of
//! `live_stats` (its size budget); the walk is unchanged.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::cpu_since::CpuSince;
use crate::live_stats::ProcessRow;
use crate::stats::LiveProcessStats;
use crate::zombies::{Zombie, Zombies};

/// Where a tree's rows are read from.
pub(crate) trait Rows {
    /// Whether `pid` is held (a live process, or one `sysinfo` still keeps).
    fn holds(&self, pid: u32) -> bool;

    /// `pid`'s row as held, its depth and child count left for the walk.
    fn row(&self, pid: u32) -> Option<ProcessRow>;
}

/// The collector's `sysinfo` table, with what its refreshes saw of each
/// process's CPU time.
pub(crate) struct TableRows<'a> {
    pub(crate) system: &'a sysinfo::System,
    pub(crate) cpu: &'a CpuSince,
}

impl Rows for TableRows<'_> {
    fn holds(&self, pid: u32) -> bool {
        self.system.process(sysinfo::Pid::from_u32(pid)).is_some()
    }

    fn row(&self, pid: u32) -> Option<ProcessRow> {
        let process = self.system.process(sysinfo::Pid::from_u32(pid))?;
        Some(ProcessRow {
            pid,
            parent_pid: process.parent().map(sysinfo::Pid::as_u32),
            name: process.name().to_string_lossy().into_owned(),
            cpu_pct: self.cpu.pct(process),
            rss_bytes: process.memory(),
            age_ms: process.run_time().saturating_mul(1000),
            child_count: 0,
            depth: 0,
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
            // A Mac's `sysinfo` keeps a process it knew that has since
            // finished as if it ran: `ps` says it's a zombie.
            zombie: process.status() == sysinfo::ProcessStatus::Zombie,
            cpu_time_ms: process.accumulated_cpu_time(),
        })
    }
}

/// Per-process rows (root first, breadth-first over descendants) plus
/// the tree aggregate, read from `rows` (refreshed by the caller).
pub(crate) fn tree_rows(
    rows: &dyn Rows,
    parents: &HashMap<u32, Vec<u32>>,
    root: u32,
    zombies: &Zombies,
) -> (Vec<ProcessRow>, LiveProcessStats) {
    let mut out = Vec::new();
    let mut cpu = 0.0_f64;
    let mut rss = 0_u64;
    let mut child_count = 0_u32;
    let mut seen: HashSet<u32> = HashSet::new();
    let mut queue: VecDeque<(u32, u8)> = VecDeque::from([(root, 0)]);
    while let Some((pid, depth)) = queue.pop_front() {
        if !seen.insert(pid) {
            continue; // ppid cycles must not loop forever
        }
        let children = parents.get(&pid).cloned().unwrap_or_default();
        let Some(mut row) = rows.row(pid) else {
            queue.extend(children.into_iter().map(|child| (child, depth + 1)));
            continue;
        };
        row.depth = depth;
        row.child_count = u32::try_from(children.len()).unwrap_or(u32::MAX);
        row.zombie = row.zombie || zombies.get(pid, row.parent_pid).is_some();
        cpu += row.cpu_pct.unwrap_or(0.0);
        rss += row.rss_bytes;
        if pid != root {
            child_count += 1;
        }
        out.push(row);
        // The zombies it hasn't collected, which `sysinfo` doesn't list.
        for zombie in zombies.children_of(pid) {
            if !rows.holds(zombie.pid) && seen.insert(zombie.pid) {
                out.push(zombie_row(zombie, depth + 1));
            }
        }
        queue.extend(children.into_iter().map(|child| (child, depth + 1)));
    }
    (
        out,
        LiveProcessStats {
            child_count,
            cpu_pct: Some(cpu),
            rss_bytes: rss,
        },
    )
}

/// A zombie's row: no memory, no CPU, nothing under it.
fn zombie_row(zombie: &Zombie, depth: u8) -> ProcessRow {
    ProcessRow {
        pid: zombie.pid,
        parent_pid: Some(zombie.ppid),
        name: zombie.name.clone(),
        cpu_pct: None,
        rss_bytes: 0,
        age_ms: 0,
        child_count: 0,
        depth,
        start_time: zombie.start_time,
        exe: None,
        cmd: None,
        zombie: true,
        cpu_time_ms: 0,
    }
}
