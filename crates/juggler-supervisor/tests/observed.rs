//! AJ-E1402: AI tools AI Juggler did not start are measured read-only
//! over their whole tree; supervised pids (and AI Juggler itself) never
//! count twice.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use juggler_supervisor::supervisor::observed_sums;
use juggler_supervisor::{LiveStatsCollector, ObservedRoot, ProcessRow, Supervisor};

fn row(pid: u32, parent: u32, rss: u64, cpu: f64) -> ProcessRow {
    ProcessRow {
        pid,
        parent_pid: Some(parent),
        name: format!("p{pid}"),
        cpu_pct: Some(cpu),
        rss_bytes: rss,
        ..ProcessRow::default()
    }
}

#[test]
fn excluded_subtrees_are_pruned_from_the_sums() {
    // Terminal `claude` (10) → node (11) → AI Juggler (12) → its session (13).
    let rows = vec![
        row(10, 1, 400, 5.0),
        row(11, 10, 100, 1.0),
        row(12, 11, 300, 2.0),
        row(13, 12, 900, 50.0),
        row(14, 11, 50, 0.5),
    ];
    let all = observed_sums(&rows, &HashSet::new());
    assert_eq!(all.rss_bytes, 1_750);
    assert_eq!(all.child_count, 4);
    let excluded: HashSet<u32> = [12].into_iter().collect();
    let sums = observed_sums(&rows, &excluded);
    assert_eq!(
        sums.rss_bytes, 550,
        "AI Juggler and its session are not the tool's"
    );
    assert_eq!(sums.child_count, 2);
    assert!((sums.cpu_pct.unwrap() - 6.5).abs() < 1e-9);
}

#[cfg(unix)]
#[test]
fn a_tree_we_did_not_start_is_measured_without_touching_it() {
    // A small tree outside the supervisor: sh → two sleeps.
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", "sleep 30 & sleep 30 & wait"])
        .spawn()
        .unwrap();
    let root = child.id();
    let supervisor = Supervisor::new();
    let mut collector = LiveStatsCollector::new();
    collector.set_observed_roots(vec![ObservedRoot {
        pid: root,
        start_time: None,
    }]);
    let mut found = None;
    for tick in 0..20 {
        collector.tick(&supervisor, i64::from(tick) * 2_000);
        let snapshot = collector.observed_snapshot();
        if let Some(tree) = snapshot.get(&root)
            && tree.stats.child_count >= 2
        {
            found = Some(tree.clone());
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let tree = found.expect("the observed tree with its two children");
    assert!(tree.stats.rss_bytes > 0);
    assert!(!tree.ring.is_empty());
    // Nothing of it reached the supervisor's sessions.
    assert!(collector.stats_snapshot().is_empty());
    // Read-only: the tree is still running after being measured.
    assert!(child.try_wait().unwrap().is_none(), "never signalled");
    child.kill().unwrap();
    child.wait().unwrap();
    // A root that exited drops out at the next tick.
    for tick in 20..23 {
        collector.tick(&supervisor, i64::from(tick) * 2_000);
    }
    assert!(!collector.observed_snapshot().contains_key(&root));
}

/// Review of #68: a helper of an observed tree is refreshed on EVERY tick,
/// not only on full scans. `sysinfo` divides a process's CPU delta since
/// its own last refresh by the time since the last refresh call, so a
/// helper skipped on the hot ticks read several times too high at each
/// full scan and froze in between. One busy helper (a single-threaded
/// loop, at most one core) must never read far above 100 %.
#[cfg(unix)]
#[test]
fn a_busy_helper_reads_one_core_not_five() {
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", "sh -c 'while :; do :; done' & wait"])
        .spawn()
        .unwrap();
    let root = child.id();
    let supervisor = Supervisor::new();
    let mut collector = LiveStatsCollector::new();
    collector.set_observed_roots(vec![ObservedRoot {
        pid: root,
        start_time: None,
    }]);
    let mut readings = Vec::new();
    // Ticks 1–3 are full scans (warm-up), then every 5th: 13 ticks cover
    // hot ticks and two full scans after the helper is known.
    for tick in 1..=13_i64 {
        collector.tick(&supervisor, tick * 300);
        std::thread::sleep(std::time::Duration::from_millis(300));
        if tick > 3
            && let Some(tree) = collector.observed_snapshot().get(&root)
            && let Some(cpu) = tree.stats.cpu_pct
        {
            readings.push(cpu);
        }
    }
    child.kill().unwrap();
    let _ = child.wait();
    let _ = std::process::Command::new("pkill")
        .args(["-f", "while :; do :; done"])
        .status();
    assert!(readings.len() >= 8, "measured: {readings:?}");
    let max = readings.iter().copied().fold(0.0_f64, f64::max);
    assert!(
        max < 160.0,
        "a single busy helper read {max:.0} % of a core: {readings:?}"
    );
    assert!(
        readings.iter().filter(|cpu| **cpu > 50.0).count() >= readings.len() / 2,
        "and it reads as busy, not frozen at a stale value: {readings:?}"
    );
}

/// Review of #68: a root is pinned to the process the scan saw. If the pid
/// now names a different process (another start time), it is never
/// measured as the tool and is dropped.
#[cfg(unix)]
#[test]
fn a_reused_pid_is_never_measured_as_the_tool() {
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let pid = child.id();
    let supervisor = Supervisor::new();
    let mut collector = LiveStatsCollector::new();
    // The scan saw a process with this pid that started at another time.
    collector.set_observed_roots(vec![ObservedRoot {
        pid,
        start_time: Some(1),
    }]);
    for tick in 1..=3_i64 {
        collector.tick(&supervisor, tick * 2_000);
    }
    assert!(
        !collector.observed_snapshot().contains_key(&pid),
        "a different process under a reused pid is not the tool"
    );
    child.kill().unwrap();
    let _ = child.wait();
}
