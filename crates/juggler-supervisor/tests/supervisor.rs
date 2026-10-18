//! Supervisor skeleton integration tests (M0-5): spawn/capture/stop, kill →
//! journal, and crash re-adoption. Unix-only: tests shell out to /bin/sh.

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::SessionId;
use juggler_supervisor::{ExitKind, Journal, LifecycleEvent, Supervisor};

fn sh_spec(script: &str, cwd: &std::path::Path) -> LaunchSpec {
    LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn wait_for_exit(supervisor: &Supervisor, session: &SessionId) -> LifecycleEvent {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if let Some(event @ LifecycleEvent::Exited { .. }) =
            supervisor.recv_timeout(Duration::from_millis(100))
        {
            assert_eq!(event.session(), session);
            return event;
        }
    }
    panic!("no exit event within deadline");
}

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
fn spawn_capture_and_stop_cleanly() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("s-clean");

    let spawned = supervisor
        .spawn(
            &session,
            &sh_spec("echo hello-juggler; sleep 60", workdir.path()),
            &ledger_test_ctx(),
        )
        .unwrap();
    assert_eq!(spawned.session, session);

    // Output is captured into the ring buffer.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let out = supervisor.output(&session).unwrap();
        if String::from_utf8_lossy(&out).contains("hello-juggler") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "output not captured: {:?}",
            String::from_utf8_lossy(&out)
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // Stop ladder terminates within the grace window.
    supervisor.stop(&session, Duration::from_secs(5)).unwrap();
    let event = wait_for_exit(&supervisor, &session);
    let LifecycleEvent::Exited { kind, .. } = event else {
        unreachable!()
    };
    assert!(
        matches!(kind, ExitKind::Signaled(_) | ExitKind::Code(_)),
        "expected termination, got {kind:?}"
    );
}

#[test]
fn external_sigkill_is_journaled_as_exit() {
    let workdir = tempfile::tempdir().unwrap();
    let state = workdir.path().join("state");
    let supervisor = Supervisor::with_journal(&state).unwrap();
    let session = SessionId::new("s-killed");

    let spawned = supervisor
        .spawn(
            &session,
            &sh_spec("sleep 300", workdir.path()),
            &ledger_test_ctx(),
        )
        .unwrap();

    // Kill -9 from outside (simulating the OS OOM killer).
    let status = std::process::Command::new("kill")
        .arg("-9")
        .arg(spawned.pid.to_string())
        .status()
        .unwrap();
    assert!(status.success());

    let event = wait_for_exit(&supervisor, &session);
    let LifecycleEvent::Exited { kind, .. } = event else {
        unreachable!()
    };
    assert!(matches!(kind, ExitKind::Signaled(Some(9))), "got {kind:?}");

    // The journal recorded the exit, so a later adoption will not pick it up.
    let orphans = Journal::open(state.join("supervision.jsonl"))
        .unwrap()
        .adopt_orphans()
        .unwrap();
    assert!(!orphans.iter().any(|o| o.session == session));
}

#[test]
fn children_survive_drop_and_are_readopted_from_journal() {
    let workdir = tempfile::tempdir().unwrap();
    let state = workdir.path().join("state");
    let session = SessionId::new("s-orphan");

    let pid = {
        let supervisor = Supervisor::with_journal(&state).unwrap();
        let spawned = supervisor
            .spawn(
                &session,
                &sh_spec("sleep 300", workdir.path()),
                &ledger_test_ctx(),
            )
            .unwrap();
        spawned.pid
        // Supervisor dropped here; the child must survive (spec §5.3).
    };
    // Wait past the identity-settle window (AJ-E503: `sh -c` execs
    // `sleep`, so adoption verifies against the SETTLED journal
    // identity — the immediate pre-exec record alone would fail
    // closed, by design).
    std::thread::sleep(Duration::from_millis(600));

    // "Restart": a fresh supervisor consults the journal.
    let orphans = Journal::open(state.join("supervision.jsonl"))
        .unwrap()
        .adopt_orphans()
        .unwrap();
    let adopted = orphans
        .iter()
        .find(|o| o.session == session)
        .unwrap_or_else(|| panic!("expected {session} to be adopted"));
    assert_eq!(adopted.pid, pid);
    assert!(adopted.program.contains("sh"));

    // Cleanup: kill the orphan so the test leaves nothing behind.
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}

#[test]
fn process_tree_descendants_are_tracked() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("s-tree");

    // sh spawns a child; the supervisor must see the whole tree.
    let spawned = supervisor
        .spawn(
            &session,
            &sh_spec("sleep 300 & sleep 300", workdir.path()),
            &ledger_test_ctx(),
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(500));

    let descendants = supervisor.descendants(&session).unwrap();
    assert!(
        !descendants.is_empty(),
        "expected at least one descendant process"
    );

    supervisor.stop(&session, Duration::from_secs(5)).unwrap();
    wait_for_exit(&supervisor, &session);

    // AJ-E507 Req.4: `stop` delegates to the TERM→grace→KILL ladder
    // over the verified group — the helpers must die WITH the root
    // (before E507 they outlived it and fell to the orphan
    // reconciler; that was F-9). Probe every tracked pid until none
    // answers `kill -0`.
    let mut members = descendants;
    members.push(spawned.pid);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut all_gone = false;
    while std::time::Instant::now() < deadline {
        if members.iter().all(|pid| !process_alive_via_kill(*pid)) {
            all_gone = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let survivors = members
        .iter()
        .filter(|pid| process_alive_via_kill(**pid))
        .collect::<Vec<_>>();
    assert!(
        all_gone,
        "group members survived stop(): still alive {survivors:?}"
    );
}

/// `kill -0` liveness probe (test-side; the ladder has its own).
/// The ladder's aliveness contract: a process whose IMAGE exited is
/// gone (a zombie only awaits the parent's `wait`; on init-less CI
/// containers the reparented orphan can stay zombie indefinitely —
/// that is the OS's reaping job, not the ladder's).
fn image_exited(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            // The state letter follows the comm field in parentheses.
            if let Some(close) = stat.rfind(')') {
                let state = stat[close + 1..].trim().chars().next();
                return state == Some('Z');
            }
        }
    }
    !process_alive_via_kill(pid)
}

/// Waits up to `grace` for the image to be gone AND (best effort on
/// reaping hosts) the pid fully collected.
fn process_alive_via_kill(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn pty_spawn_captures_output() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("s-pty");

    let spec = LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), "echo from-pty; exit 0".into()],
        env: BTreeMap::new(),
        cwd: workdir.path().to_path_buf(),
        needs_pty: true,
    };
    supervisor
        .spawn(&session, &spec, &ledger_test_ctx())
        .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let out = supervisor.output(&session).unwrap();
        if String::from_utf8_lossy(&out).contains("from-pty") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "pty output not captured: {:?}",
            String::from_utf8_lossy(&out)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    wait_for_exit(&supervisor, &session);
}

#[test]
fn unknown_sessions_error_cleanly() {
    let supervisor = Supervisor::new();
    let ghost = SessionId::new("ghost");
    assert!(supervisor.output(&ghost).is_err());
    assert!(supervisor.stop(&ghost, Duration::from_secs(1)).is_err());
    assert!(supervisor.descendants(&ghost).is_err());
}

#[test]
fn pty_sessions_roundtrip_input_output_and_resize() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("s-pty-cat");

    // cat under a PTY: everything we write is echoed back through the tap.
    let spec = LaunchSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), "cat".into()],
        env: EnvMap::new(),
        cwd: workdir.path().to_path_buf(),
        needs_pty: true,
    };
    supervisor
        .spawn(&session, &spec, &ledger_test_ctx())
        .unwrap();
    let tap = supervisor.tap(&session).unwrap();

    supervisor
        .write_input(&session, b"juggler-pty-roundtrip\n")
        .unwrap();
    let mut collected = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Ok(chunk) = tap.recv_timeout(Duration::from_millis(200)) {
            collected.extend_from_slice(&chunk);
            if collected.windows(21).any(|w| w == b"juggler-pty-roundtrip") {
                break;
            }
        }
    }
    assert!(
        collected.windows(21).any(|w| w == b"juggler-pty-roundtrip"),
        "typed input must stream back through the tap: {:?}",
        String::from_utf8_lossy(&collected)
    );

    // Resize is accepted (the kernel owns the winsize; no error means the
    // master is live).
    supervisor.resize_pty(&session, 120, 40).unwrap();

    // The ring snapshot contains the same stream (replay-after-reopen).
    supervisor.write_input(&session, b"tail-marker\n").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let snapshot = supervisor.output(&session).unwrap();
        if snapshot.windows(11).any(|w| w == b"tail-marker") {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        supervisor
            .output(&session)
            .unwrap()
            .windows(11)
            .any(|w| w == b"tail-marker")
    );

    supervisor.stop(&session, Duration::from_secs(2)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !supervisor.is_finished(&session) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(supervisor.is_finished(&session));

    // Piped sessions have no PTY: the typed error says so.
    let piped = SessionId::new("s-piped");
    supervisor
        .spawn(&piped, &sh_spec("true", workdir.path()), &ledger_test_ctx())
        .unwrap();
    let err = supervisor.write_input(&piped, b"x").unwrap_err();
    assert!(err.to_string().contains("no pty"), "{err}");
}

#[test]
fn twenty_streaming_sessions_fan_out_without_deadlock() {
    // M1-4 perf floor: 20 concurrent PTY sessions each streaming output
    // through a tap; every stream must deliver within the deadline and
    // the supervisor must stay responsive (60 fps UI budget depends on
    // this path never wedging).
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let mut sessions = Vec::new();
    for i in 0..20 {
        let session = SessionId::new(format!("s-stream-{i}"));
        let spec = LaunchSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".into(),
                "i=0; while [ $i -lt 50 ]; do echo line-$i; i=$((i+1)); done".to_string(),
            ],
            env: EnvMap::new(),
            cwd: workdir.path().to_path_buf(),
            needs_pty: true,
        };
        supervisor
            .spawn(&session, &spec, &ledger_test_ctx())
            .unwrap();
        let tap = supervisor.tap(&session).unwrap();
        sessions.push((session, tap));
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    for (session, rx) in sessions {
        let mut tail = Vec::new();
        let mut got_last = false;
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(chunk) => {
                    tail.extend_from_slice(&chunk);
                    if tail.windows(7).any(|w| w == b"line-49") {
                        got_last = true;
                        break;
                    }
                }
                Err(_) if supervisor.is_finished(&session) => break,
                Err(_) => continue,
            }
        }
        assert!(
            got_last || tail.windows(7).any(|w| w == b"line-49"),
            "stream for {session} never delivered its final line"
        );
    }
}

/// AJ-E401: the batched live-stats collector reads every supervised
/// session from one process-table refresh; the first sample cannot know
/// CPU deltas, so `cpu_pct` is honestly `None` (never a fabricated 0.0),
/// and a finished session reports definitive zeros.
#[test]
fn live_stats_collector_batches_and_masks_first_cpu_sample() {
    use juggler_supervisor::LiveStatsCollector;

    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let session = SessionId::new("s-live-stats");

    // Spawn `sleep` DIRECTLY (no shell): /bin/sh -c forks on some
    // platforms (Linux dash) but not others (macOS), so a shell child
    // count would be environment-dependent. `sleep` never has children —
    // child_count 0 is a deterministic fact.
    supervisor
        .spawn(
            &session,
            &LaunchSpec {
                program: PathBuf::from("/bin/sleep"),
                args: vec!["30".into()],
                env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
                cwd: workdir.path().to_path_buf(),
                needs_pty: false,
            },
            &ledger_test_ctx(),
        )
        .unwrap();

    let mut collector = LiveStatsCollector::new();
    let first = collector.refresh(&supervisor);
    let stat = first
        .get(&session)
        .expect("supervised session present in batched stats");
    assert_eq!(stat.cpu_pct, None, "first sample has no CPU delta yet");
    assert!(
        stat.rss_bytes > 0,
        "a live /bin/sleep must report resident memory"
    );
    assert_eq!(stat.child_count, 0, "sleep never spawns children");

    // Second sample: sysinfo now has a delta window, CPU is a number.
    std::thread::sleep(Duration::from_millis(300));
    let second = collector.refresh(&supervisor);
    assert!(second[&session].cpu_pct.is_some());

    // After exit the handle stays with `finished` set: definitive zeros,
    // never an unmeasurable mystery.
    supervisor.stop(&session, Duration::from_secs(5)).unwrap();
    let _ = wait_for_exit(&supervisor, &session);
    let third = collector.refresh(&supervisor);
    let dead = third
        .get(&session)
        .expect("finished session still reported (dead zeros)");
    assert_eq!(dead.child_count, 0);
    assert_eq!(dead.rss_bytes, 0);
    assert_eq!(dead.cpu_pct, Some(0.0));
}

// --- AJ-E507: the ladder primitive's outcome vocabulary -------------

use juggler_supervisor::tree::{ProcessOutcome, terminate_group};

fn kill9(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}

/// Spawns `script` DETACHED (double-fork style: an outer `/bin/sh`
/// starts it, logs its pid, exits) and returns the pid. Targets must
/// NOT be direct children of the test process — an unreaped zombie
/// answers `kill -0` as alive and defeats the ladder's probes; the
/// real fixture shape (helper reparented to launchd) probes honestly.
fn detached_target(script: &str) -> (u32, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut outer = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("( {script} ) & echo $! > target.pid; exit 0"))
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    let _ = outer.wait();
    let pid = std::fs::read_to_string(dir.path().join("target.pid"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    (pid, dir)
}

#[test]
fn ladder_reports_already_gone_for_dead_pids() {
    // A pid that exited before the ladder ran: AlreadyGone, and
    // AlreadyGone is SUCCESS (Req.6) — never an error.
    let (pid, _dir) = detached_target("exec sleep 0.2");
    std::thread::sleep(Duration::from_millis(700));
    let outcome = terminate_group(Duration::from_secs(2), move || vec![pid], || false);
    assert_eq!(outcome.members, vec![(pid, ProcessOutcome::AlreadyGone)]);
    assert!(outcome.all_succeeded());
    assert!(!outcome.cancelled);
}

#[test]
fn ladder_exits_cooperative_helper_on_term() {
    let (pid, _dir) = detached_target("exec sleep 30");
    let outcome = terminate_group(Duration::from_secs(3), move || vec![pid], || false);
    assert_eq!(outcome.members, vec![(pid, ProcessOutcome::ExitedOnTerm)]);
    assert!(outcome.all_succeeded());
    assert!(image_exited(pid), "the helper's image exited on TERM");
}

/// A TERM-ignoring target that is READY before its pid returns
/// (python installs SIG_IGN, then touches `ignorer.ready`): the
/// ladder's TERM must never race the guard's installation — a shell
/// `trap` has the same startup window, and dash's subshell trap
/// handling differs from bash anyway.
fn detached_ignorer() -> (u32, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("ignorer.ready");
    let ready_path = ready.to_string_lossy().to_string();
    let mut outer = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "( exec python3 -c 'import signal,os,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); open(\"{ready_path}\", \"w\").close(); time.sleep(300)' ) & echo $! > target.pid; exit 0"
        ))
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    let _ = outer.wait();
    let pid = std::fs::read_to_string(dir.path().join("target.pid"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !ready.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        ready.exists(),
        "ignorer never became ready — fixture broken"
    );
    (pid, dir)
}

#[test]
fn ladder_escalates_to_kill_for_term_ignorer() {
    // The helper ignores TERM and keeps running: the ladder must KILL
    // it after grace and report `needed KILL` — the "Ignored SIGTERM"
    // shape (Req.3).
    let (pid, _dir) = detached_ignorer();
    let outcome = terminate_group(Duration::from_millis(300), move || vec![pid], || false);
    assert_eq!(outcome.members, vec![(pid, ProcessOutcome::NeededKill)]);
    assert!(outcome.all_succeeded());
    assert!(!process_alive_via_kill(pid));
}

#[test]
fn ladder_cancel_mid_grace_leaves_survivor_running() {
    // Cancel during grace: NO escalation, the survivor reports
    // `TERM'd — outcome unknown`, and the process is still alive.
    // (The target ignores TERM — cancelling does not unsend the
    // signal, so an unguarded target would legitimately die from it.)
    let (pid, _dir) = detached_ignorer();
    let outcome = terminate_group(Duration::from_secs(30), move || vec![pid], || true);
    assert_eq!(outcome.members, vec![(pid, ProcessOutcome::TermSent)]);
    assert!(outcome.cancelled);
    assert!(
        process_alive_via_kill(pid),
        "cancel must not escalate to KILL"
    );
    kill9(pid);
}
