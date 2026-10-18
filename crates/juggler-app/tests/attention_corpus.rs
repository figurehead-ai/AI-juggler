//! AJ-E405 fixture corpus: the heuristic engine against REAL fake-harness
//! processes (hang / wait-permission / quota-exit / crash / healthy).
//!
//! Classification target (EPICS M2-5): ≥ 90% precision AND recall. The
//! per-run table prints for the evidence doc (`docs/qa/e4-attention-fixtures.md`).
//! Debounce uses a synthetic clock anchored at spawn time (the engine is
//! pure over injected `now_ms`), so the corpus runs in seconds while
//! exercising the real debounce gaps.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::path::PathBuf;
use std::time::Duration;

use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::SessionId;
use juggler_supervisor::{
    AttentionTransition, ExitKind, HeuristicEngine, HeuristicInputs, HeuristicThresholds,
    LifecycleEvent, Supervisor,
};

const RUNS_PER_CLASS: usize = 3;

fn fake_harness_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_fake-harness") {
        return PathBuf::from(path);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let debug = target.join("debug/fake-harness");
    let release = target.join("release/fake-harness");
    if debug.exists() {
        debug
    } else if release.exists() {
        release
    } else {
        unreachable!("fake-harness binary not found under {}", target.display());
    }
}

fn spec(args: &[&str], cwd: &std::path::Path) -> LaunchSpec {
    LaunchSpec {
        program: fake_harness_bin(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Prediction {
    Stuck,
    WaitingUser,
    QuotaBlocked,
    None,
}

fn classify_run(
    supervisor: &Supervisor,
    engine: &HeuristicEngine,
    session: &SessionId,
    args: &[&str],
    cwd: &std::path::Path,
    expect_exit: bool,
) -> Prediction {
    supervisor.spawn(session, &spec(args, cwd), &ctx()).unwrap();
    // Wait for Started + output to land.
    // Generous deadlines: the workspace suite runs test binaries in
    // parallel, so CI runners can be briefly starved.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(LifecycleEvent::Started { .. }) =
            supervisor.recv_timeout(Duration::from_millis(50))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "no Started event");
    }
    std::thread::sleep(Duration::from_millis(1200));

    if expect_exit {
        // Quota/crash fixtures end by themselves; classify the exit.
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(LifecycleEvent::Exited { kind, .. }) =
                supervisor.recv_timeout(Duration::from_millis(100))
            {
                let code = match kind {
                    ExitKind::Code(code) => code,
                    ExitKind::Signaled(_) => -1,
                };
                let tail = supervisor.output_tail(session, 4096).unwrap_or_default();
                let now = juggler_core::model::Timestamp::now().millis();
                return engine
                    .classify_exit(code, &tail, now, None)
                    .map(|_| Prediction::QuotaBlocked)
                    .unwrap_or(Prediction::None);
            }
            assert!(std::time::Instant::now() < deadline, "no Exited event");
        }
    }

    // Live fixtures: two debounced evaluations with a synthetic clock
    // anchored at spawn (+35 s, +71 s) — the engine never wall-reads.
    let last_output = supervisor.last_output_ms(session);
    let anchor = last_output.unwrap_or_else(|| juggler_core::model::Timestamp::now().millis());
    let mut prediction = Prediction::None;
    for step in [35_000_i64, 71_000] {
        let inputs = HeuristicInputs {
            last_output_ms: last_output,
            cpu_pct: None, // unsampled CPU never blocks the rules (honest)
            transcript_mtime_ms: last_output, // transcript writes track output here
            prompt_alive: juggler_supervisor::prompt_alive(
                &supervisor.output_tail(session, 2048).unwrap_or_default(),
            ),
            now_ms: anchor + step,
        };
        if let Some(AttentionTransition { status, .. }) = engine.evaluate(session, &inputs) {
            prediction = match status {
                juggler_core::model::SessionStatus::Stuck => Prediction::Stuck,
                juggler_core::model::SessionStatus::WaitingUser => Prediction::WaitingUser,
                _ => Prediction::QuotaBlocked,
            };
        }
    }
    supervisor.stop(session, Duration::from_secs(5)).unwrap();
    let _ = supervisor.recv_timeout(Duration::from_millis(300));
    prediction
}

fn ctx() -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "fake-harness".into(),
        account_id: Some("acc-fake".into()),
        project_id: Some("proj-fake".into()),
        machine: juggler_core::model::MachineId::new("machine-test"),
        role: "root",
    }
}

#[test]
fn fixture_corpus_precision_and_recall_at_least_90_percent() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let engine = HeuristicEngine::new(HeuristicThresholds {
        stuck_no_output_secs: 3,
        stuck_cpu_max_pct: 5.0,
        stuck_transcript_stale_secs: 3,
        waiting_prompt_secs: 2,
    });

    // (truth, args, expect_exit)
    let classes: [(&str, &[&str], bool); 5] = [
        (
            "hang",
            &["run", "--dir", ".", "--turns", "2", "--stall-secs", "120"],
            false,
        ),
        (
            "wait",
            &[
                "run",
                "--dir",
                ".",
                "--turns",
                "1",
                "--wait-prompt",
                "deploy to prod",
            ],
            false,
        ),
        (
            "quota",
            &["run", "--dir", ".", "--turns", "1", "--quota-exit"],
            true,
        ),
        (
            "crash",
            &["run", "--dir", ".", "--turns", "1", "--exit", "3"],
            true,
        ),
        // A clean exit-0 run is the healthy class: engine must stay silent.
        (
            "healthy",
            &["run", "--dir", ".", "--turns", "2", "--exit", "0"],
            true,
        ),
    ];
    let truth_of = |class: &str| match class {
        "hang" => Prediction::Stuck,
        "wait" => Prediction::WaitingUser,
        "quota" => Prediction::QuotaBlocked,
        _ => Prediction::None,
    };

    let mut results: Vec<(&str, Prediction, Prediction)> = Vec::new();
    for (class, args, expect_exit) in classes {
        for run in 0..RUNS_PER_CLASS {
            let session = SessionId::new(format!("corpus-{class}-{run}"));
            let dir = workdir.path().join(format!("{class}-{run}"));
            std::fs::create_dir_all(&dir).unwrap();
            let predicted = classify_run(&supervisor, &engine, &session, args, &dir, expect_exit);
            engine.forget(&session);
            results.push((class, truth_of(class), predicted));
        }
    }

    // Confusion accounting per truth/predicted pair.
    let mut report = String::from("class\ttruth\tpredicted\n");
    for (class, truth, predicted) in &results {
        report.push_str(&format!("{class}\t{truth:?}\t{predicted:?}\n"));
    }
    let labels = [
        Prediction::Stuck,
        Prediction::WaitingUser,
        Prediction::QuotaBlocked,
    ];
    let mut failures = Vec::new();
    for label in labels {
        let truth_count = results
            .iter()
            .filter(|(_, truth, _)| *truth == label)
            .count();
        let predicted_count = results
            .iter()
            .filter(|(_, _, predicted)| *predicted == label)
            .count();
        let correct = results
            .iter()
            .filter(|(_, truth, predicted)| *truth == label && *predicted == label)
            .count();
        let recall = correct as f64 / truth_count as f64;
        let precision = if predicted_count == 0 {
            0.0
        } else {
            correct as f64 / predicted_count as f64
        };
        println!(
            "{label:?}: precision {precision:.2} ({correct}/{predicted_count}), recall {recall:.2} ({correct}/{truth_count})"
        );
        report.push_str(&format!(
            "{label:?}: precision {precision:.2}, recall {recall:.2}\n"
        ));
        if precision < 0.9 || recall < 0.9 {
            failures.push(format!(
                "{label:?} precision {precision:.2} recall {recall:.2}"
            ));
        }
    }
    assert!(failures.is_empty(), "below the 90% gate: {failures:?}");

    // Live-healthy guard (real clock, no synthetic time): a session with
    // continuously flowing output never trips any rule across the real
    // 31 s debounce gap.
    {
        let session = SessionId::new("corpus-live-healthy");
        let dir = workdir.path().join("live-healthy");
        std::fs::create_dir_all(&dir).unwrap();
        supervisor
            .spawn(
                &session,
                &spec(
                    &["run", "--dir", ".", "--turns", "20000", "--delay-ms", "300"],
                    &dir,
                ),
                &ctx(),
            )
            .unwrap();
        let _ = supervisor.recv_timeout(Duration::from_millis(300));
        for _ in 0..2 {
            std::thread::sleep(Duration::from_secs(31));
            let inputs = HeuristicInputs {
                last_output_ms: supervisor.last_output_ms(&session),
                cpu_pct: None,
                transcript_mtime_ms: supervisor.last_output_ms(&session),
                prompt_alive: juggler_supervisor::prompt_alive(
                    &supervisor.output_tail(&session, 2048).unwrap_or_default(),
                ),
                now_ms: juggler_core::model::Timestamp::now().millis(),
            };
            assert!(
                engine.evaluate(&session, &inputs).is_none(),
                "flowing output must never be flagged"
            );
        }
        supervisor.stop(&session, Duration::from_secs(5)).unwrap();
        let _ = supervisor.recv_timeout(Duration::from_millis(300));
    }
}

/// AJ-E405 live dogfood (REAL wall clock, thresholds shrunk so the run
/// fits a minute). Req.4's debounce law (2 evaluations ≥ 30 s apart)
/// supersedes the "≤ 15 s after threshold" phrasing: the emission lands
/// at the confirmation evaluation itself (≤ 1 tick, 15 s, in the app),
/// so the honest end-to-end budget is window + 30 s debounce + 15 s
/// tick. Asserted: total ≤ window + 46 s, with the breakdown printed.
/// Run explicitly: `cargo test -p juggler-app --test attention_corpus -- --ignored`.
#[test]
#[ignore = "wall-clock dogfood (~40 s); run explicitly for AJ-E405 evidence"]
fn live_dogfood_hang_fixture_flags_stuck_within_budget() {
    let workdir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::new();
    let engine = HeuristicEngine::new(HeuristicThresholds {
        stuck_no_output_secs: 3,
        stuck_cpu_max_pct: 5.0,
        stuck_transcript_stale_secs: 3,
        waiting_prompt_secs: 2,
    });
    let session = SessionId::new("dogfood-hang");
    supervisor
        .spawn(
            &session,
            &spec(
                &["run", "--dir", ".", "--turns", "2", "--stall-secs", "120"],
                workdir.path(),
            ),
            &ctx(),
        )
        .unwrap();
    let _ = supervisor.recv_timeout(Duration::from_millis(300));
    std::thread::sleep(Duration::from_millis(1200));

    let started = std::time::Instant::now();
    let threshold_at = {
        // First evaluation after the 3 s window passes.
        let mut threshold_at = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let inputs = HeuristicInputs {
                last_output_ms: supervisor.last_output_ms(&session),
                cpu_pct: None,
                transcript_mtime_ms: supervisor.last_output_ms(&session),
                prompt_alive: false,
                now_ms: juggler_core::model::Timestamp::now().millis(),
            };
            if engine.evaluate(&session, &inputs).is_some() {
                break threshold_at.expect("candidate must precede confirmation");
            }
            let last = supervisor.last_output_ms(&session).unwrap_or_default();
            let now = juggler_core::model::Timestamp::now().millis();
            if threshold_at.is_none() && (now - last) / 1000 >= 3 {
                threshold_at = Some(std::time::Instant::now());
            }
            assert!(
                started.elapsed() < Duration::from_secs(70),
                "no transition within 70 s"
            );
        }
    };
    let latency = started.elapsed();
    let after_threshold = threshold_at.elapsed();
    println!(
        "dogfood: confirmed Stuck {latency:?} after spawn ({after_threshold:?} after threshold: 3 s window + 30 s debounce law + emission)"
    );
    assert!(
        after_threshold <= Duration::from_secs(46),
        "transition {after_threshold:?} after threshold exceeds window+debounce+tick (46 s)"
    );
    supervisor.stop(&session, Duration::from_secs(5)).unwrap();
}
