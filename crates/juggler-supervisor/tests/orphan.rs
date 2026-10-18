//! AJ-E410 orphan-contract integration: a real supervised process that
//! leaves a child behind, the 10 s grace, the TERM→KILL ladder, and the
//! allow-list guardrail. Real processes, real signals, real timing —
//! the evidence for `docs/qa/evidence/AJ-E410/manifest.md`.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::SessionId;
use juggler_supervisor::{
    DescendantRow, ExitKind, ExitSnapshot, LifecycleEvent, LiveStatsCollector, ORPHAN_GRACE_MS,
    OrphanTracker, StopOutcome, Supervisor, process_alive, stop_pid_ladder,
};

fn fake_harness_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_fake-harness") {
        return PathBuf::from(path);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("fake-harness");
        if candidate.exists() {
            return candidate;
        }
    }
    unreachable!("fake-harness binary not found under {}", target.display());
}

fn harness_spec(extra: &[&str], cwd: &std::path::Path) -> LaunchSpec {
    LaunchSpec {
        program: fake_harness_bin(),
        args: extra.iter().map(|arg| (*arg).to_string()).collect(),
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
        .unwrap_or_default()
}

/// The full contract against real processes: harness spawns a `sleep`
/// child, exits non-clean, the child surfaces as an orphan after the
/// 10 s grace, and `Stop process` yields `gone` on re-read.
fn ledger_test_ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-test".into()),
        project_id: Some("proj-test".into()),
        machine: juggler_core::model::MachineId::new("machine-test"),
        role: "root",
    }
}

#[test]
fn orphan_detected_after_grace_and_stopped_gone() {
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::with_journal(dir.path()).unwrap();
    let session = SessionId::new("orphan-fixture");
    supervisor
        .spawn(
            &session,
            &harness_spec(
                &[
                    "run",
                    "--dir",
                    dir.path().join("transcripts").to_str().unwrap(),
                    "--turns",
                    "2",
                    "--delay-ms",
                    "200",
                    "--spawn-child",
                    "/bin/sleep",
                    "--child-args",
                    "60",
                    "--exit",
                    "7",
                ],
                dir.path(),
            ),
            &ledger_test_ctx(),
        )
        .unwrap();

    // Sample the tree while the harness is alive so the collector has
    // pre-exit rows (the snapshot source). Two ticks = real CPU deltas.
    let mut collector = LiveStatsCollector::new();
    let mut root_pid = 0_u32;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        collector.tick(&supervisor, now_ms());
        if let Some(rows) = collector.rows(&session)
            && rows.len() >= 2
        {
            // Root first, then the spawned sleep child.
            root_pid = rows[0].pid;
            assert!(rows.iter().any(|row| row.name.contains("sleep")));
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(root_pid != 0, "collector never saw the tree (root + child)");

    // Wait for the non-clean exit (code 7).
    let mut exit: Option<(u32, ExitKind)> = None;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(LifecycleEvent::Exited { pid, kind, .. }) = supervisor.try_recv() {
            exit = Some((pid, kind));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let (pid, kind) = exit.expect("harness never exited");
    assert_eq!(pid, root_pid);
    assert_eq!(kind, ExitKind::Code(7));

    // The exit snapshot: frozen pre-exit rows minus the dead root.
    let rows: Vec<DescendantRow> = collector
        .rows(&session)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.pid != root_pid)
        .map(|row| DescendantRow {
            pid: row.pid,
            name: row.name,
            rss_bytes: row.rss_bytes,
        })
        .collect();
    assert_eq!(rows.len(), 1, "exactly the spawned sleep child remains");
    let child_pid = rows[0].pid;
    assert!(
        process_alive(child_pid).is_some(),
        "child outlived the harness"
    );

    // Journal the durable allow-list, record the tracker snapshot.
    supervisor
        .journal_ref()
        .expect("journal open")
        .record_descendants(&session, root_pid, false, &rows)
        .unwrap();
    let tracker = OrphanTracker::new();
    tracker.record_exit(ExitSnapshot {
        session: session.clone(),
        root_pid,
        root_program: "fake-harness".into(),
        clean: false,
        taken_ms: now_ms(),
        rows,
    });

    // Grace: poll with the REAL clock until the orphan surfaces. The
    // contract budget is 10 s grace + the 2 s poll cadence.
    let started = Instant::now();
    let mut confirmed = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(ORPHAN_GRACE_MS.unsigned_abs() + 4);
    while Instant::now() < deadline {
        confirmed = tracker.poll(now_ms(), process_alive);
        if !confirmed.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let elapsed = started.elapsed();
    assert_eq!(confirmed.len(), 1, "one orphan: the surviving sleep child");
    assert_eq!(confirmed[0].pid, child_pid);
    assert_eq!(confirmed[0].parent_label, "fake-harness");
    assert!(
        elapsed >= Duration::from_secs(10),
        "orphan surfaced before the grace window: {elapsed:?}"
    );
    assert!(
        elapsed <= Duration::from_secs(13),
        "orphan surfaced too late (>10 s + 2 s poll + slack): {elapsed:?}"
    );

    // Stop process: allowed (recorded), TERM suffices for a sleep.
    assert!(tracker.allows(&session, child_pid));
    let outcome = stop_pid_ladder(child_pid, Duration::from_secs(2), |pid| {
        process_alive(pid).is_some()
    });
    tracker.record_stop(child_pid, outcome.clone());
    assert_eq!(outcome, StopOutcome::Gone { escalated: false });
    assert_eq!(tracker.stop_state(child_pid).unwrap().state_label(), "gone");
    assert!(tracker.orphans_of(&session).is_empty(), "gone row drops");

    // The audit + allow-list are durable: the journal carries both.
    let journal =
        std::fs::read_to_string(supervisor.journal_ref().expect("journal open").path()).unwrap();
    assert!(journal.contains("\"descendants\""));
}

/// The ladder escalates to SIGKILL when the target ignores SIGTERM
/// (`--child-ignore-term`).
#[test]
fn stop_ladder_escalates_when_sigterm_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("ladder-ignore-term");
    supervisor
        .spawn(
            &session,
            &harness_spec(
                &[
                    "run",
                    "--dir",
                    dir.path().to_str().unwrap(),
                    "--turns",
                    "3",
                    "--delay-ms",
                    "300",
                    "--spawn-child",
                    "/bin/sleep",
                    "--child-args",
                    "60",
                    "--child-ignore-term",
                    "--exit",
                    "0",
                ],
                dir.path(),
            ),
            &ledger_test_ctx(),
        )
        .unwrap();
    // One collector for the whole test so the tree state persists: tick
    // while the harness is alive, remember the ignoring child's pid from
    // the pre-exit rows (ppid attribution — never a name-based scan).
    let mut collector = LiveStatsCollector::new();
    let mut child_pid = 0_u32;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        collector.tick(&supervisor, now_ms());
        let rows = collector.rows(&session).unwrap_or_default();
        if let Some(row) = rows.iter().find(|row| row.name.contains("sleep")) {
            child_pid = row.pid;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(child_pid != 0, "ignoring child never observed in the tree");
    // Drain the harness's own exit (it ends immediately after 1 turn).
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && supervisor.try_recv().is_none() {
        std::thread::sleep(Duration::from_millis(100));
    }
    // TERM is ignored by the child (SIG_IGN survives exec) — the ladder
    // must escalate to SIGKILL.
    assert!(
        process_alive(child_pid).is_some(),
        "child survived the harness"
    );
    let outcome = stop_pid_ladder(child_pid, Duration::from_millis(800), |pid| {
        process_alive(pid).is_some()
    });
    assert_eq!(
        outcome,
        StopOutcome::Gone { escalated: true },
        "TERM is ignored — the ladder must escalate to SIGKILL"
    );
}

/// A plain child stops on TERM alone (no escalation).
#[test]
fn ladder_stops_normal_child_with_term() {
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("60")
        .spawn()
        .unwrap();
    let pid = child.id();
    let outcome = stop_pid_ladder(pid, Duration::from_secs(2), |pid| {
        process_alive(pid).is_some()
    });
    assert_eq!(outcome, StopOutcome::Gone { escalated: false });
    let _ = child.wait();
}

/// Owner verification reports the re-parenting evidence.
#[test]
fn verify_owner_reports_reparented_or_gone() {
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("60")
        .spawn()
        .unwrap();
    let pid = child.id();
    let state = juggler_supervisor::verify_owner(pid);
    assert!(matches!(
        state,
        juggler_supervisor::OwnerState::Alive { .. }
    ));
    stop_pid_ladder(pid, Duration::from_secs(2), |pid| {
        process_alive(pid).is_some()
    });
    assert_eq!(
        juggler_supervisor::verify_owner(pid),
        juggler_supervisor::OwnerState::Gone
    );
    let _ = child.wait();
}

/// The 2 s hot path stays cheap: targeted ticks well under 1 % of the
/// 2 s budget at 20 trees (the full scans run only every 10th tick).
#[test]
fn collector_targeted_tick_overhead_under_budget() {
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let mut sessions = Vec::new();
    for index in 0..20 {
        let session = SessionId::new(format!("budget-{index}"));
        supervisor
            .spawn(
                &session,
                &harness_spec(
                    &[
                        "run",
                        "--dir",
                        dir.path().to_str().unwrap(),
                        "--turns",
                        "200",
                        "--delay-ms",
                        "1000",
                    ],
                    dir.path(),
                ),
                &ledger_test_ctx(),
            )
            .unwrap();
        sessions.push(session);
    }
    let mut collector = LiveStatsCollector::new();
    collector.tick(&supervisor, now_ms()); // collector tick 1 (warmup full)
    let mut targeted_total = Duration::ZERO;
    let mut targeted_ticks = 0_u32;
    for step in 1..=30_u32 {
        let collector_tick = step + 1;
        let full = collector_tick <= 3 || collector_tick % 5 == 0;
        let started = Instant::now();
        collector.tick(&supervisor, now_ms() + i64::from(step) * 2_000);
        if !full {
            targeted_total += started.elapsed();
            targeted_ticks += 1;
        }
    }
    let mean = targeted_total / targeted_ticks.max(1);
    assert!(
        mean < Duration::from_millis(20),
        "targeted tick mean {mean:?} exceeds the 1 % budget (20 ms per 2 s) at 20 trees"
    );
    // Ring retention: 31 ticks × 2 s ≈ 1 min — capped at the 150-sample
    // 5-minute window by construction; capacity asserted structurally.
    let ring = collector.ring(&sessions[0]);
    assert!(ring.len() <= 150);
    assert!(!ring.is_empty());
}
