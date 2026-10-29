//! AJ-E2401: one listing's read of every process, as the observed trees
//! are measured from it (the observed tools' trees, and the Processes
//! view's own roots). Split from `live_stats` (its size budget); the
//! measuring is unchanged.

use std::collections::HashMap;

use crate::cpu_since::CpuSince;
use crate::live_stats::{
    CollectorWork, ObservedRoot, ObservedTree, TreeSample, observed_sums, push_ring_sample,
    tree_rows,
};

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
