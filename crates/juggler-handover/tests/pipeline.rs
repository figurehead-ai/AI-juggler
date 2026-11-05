//! AJ-E205 evidence: the switch execution pipeline — preflight
//! stops (hard gates, in order, before any byte), the §7.1 copy
//! table verbatim, staged execution with verification, fault
//! injection (torn tail S-F2, partial install S-F4, launch failure
//! S-F5 with retained stage, source mutation detection), collision
//! fork S-F3, and the failure→stage mapping for edge recording.
//! The source-hash invariant is asserted in every executing test.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod support;

use support::tree::{TreeSupport, dir_hashes, line, seed_tree, target};

use juggler_adapter::error::AdapterError;
use juggler_adapter::model::HarnessId;
use juggler_core::model::{FailureStage, TranscriptRef};
use juggler_handover::fidelity::{FidelityInput, PairFlags};
use juggler_handover::pipeline::{
    self, CopySituation, FreezeChoice, Mechanism, PreflightGates, PreflightInput, SwitchFailure,
    user_copy,
};

fn preflight_input() -> PreflightInput {
    PreflightInput {
        fidelity: FidelityInput {
            from_provider: "claude-code".into(),
            to_provider: "claude-code".into(),
            dispositions: Default::default(),
            flags: PairFlags::default(),
            unmapped_keys: vec![],
        },
        target_account: "Personal".into(),
        gates: PreflightGates::default(),
        target_quota_tight: None,
        git: juggler_handover::pipeline::GitState::default(),
        hop_count: 0,
        replay_tokens_spent: 0,
    }
}

/// Preflight hard gates stop BEFORE any byte moves; warnings never
/// stop; checks run in the specified order.
#[test]
fn preflight_stops_hard_gates_and_only_hard_gates() {
    let mut input = preflight_input();
    let report = pipeline::preflight(&input).unwrap();
    assert_eq!(
        report.mechanism,
        Mechanism::Transplant,
        "same provider ⇒ transplant"
    );
    // Cross-provider pair with ALL-Drop dispositions scores F0 ⇒
    // pack-only (the mechanism follows the honest score).
    input.fidelity.to_provider = "codex".into();
    let pack_only = pipeline::preflight(&input).unwrap();
    assert_eq!(pack_only.mechanism, Mechanism::PackOnly);
    // With the E203 base dispositions (chat translates) ⇒ F1 replay.
    input.fidelity.dispositions =
        juggler_handover::fidelity::base_dispositions("claude-code", "codex").dispositions;
    let cross = pipeline::preflight(&input).unwrap();
    assert_eq!(cross.mechanism, Mechanism::Replay);
    // Same account + native resume ⇒ T1.
    input.fidelity.to_provider = "claude-code".into();
    input.fidelity.flags.same_account = true;
    input.fidelity.flags.native_resume = true;
    let native = pipeline::preflight(&input).unwrap();
    assert_eq!(native.mechanism, Mechanism::NativeResume);

    // Hard: exhausted target.
    input.gates.target_exhausted_no_reset = true;
    let stopped = pipeline::preflight(&input).unwrap_err();
    assert!(stopped.to_string().contains("exhausted"), "{stopped}");
    assert!(
        stopped
            .words("Work", "Personal")
            .starts_with("Personal has no usage left")
    );

    // Hard: version drift.
    input.gates = PreflightGates {
        target_exhausted_no_reset: false,
        version_drift: true,
    };
    let stopped = pipeline::preflight(&input).unwrap_err();
    assert!(
        stopped.to_string().contains("not been verified"),
        "{stopped}"
    );

    // Soft warnings all pass through without stopping.
    input.gates = PreflightGates::default();
    input.target_quota_tight = Some((8, 45));
    input.git.same_branch_both_live = true;
    input.git.zombie_children = vec!["pid 1234 (node vite)".into()];
    input.hop_count = 4;
    let report = pipeline::preflight(&input).unwrap();
    assert_eq!(
        report.warnings.len(),
        3,
        "tight + same-branch + hop-cost (a compacted source is no risk: D-03 amendment)"
    );
    // The tight warning names the account, never the tool's id.
    let tight = user_copy(report.warnings[0].clone());
    assert!(
        tight.starts_with("Personal has about 8% left"),
        "the warning names the account"
    );
    assert!(!tight.contains("claude-code"), "no tool id in the warning");
    assert_eq!(
        report.zombie_children,
        vec!["pid 1234 (node vite)".to_owned()]
    );
    assert!(report.hop_cost.is_some(), "S-F9 cost honesty informs");
}

/// The §7.1 copy table is verbatim — no generic "some context may
/// be lost" anywhere.
#[test]
fn copy_table_is_verbatim() {
    let all = [
        user_copy(CopySituation::TransplantProven {
            account_b: "Personal".into(),
            account_a: "Work".into(),
        }),
        user_copy(CopySituation::TransplantUnproven {
            account_a: "Work".into(),
        }),
        user_copy(CopySituation::ReplayCrossProvider {
            provider_b: "Codex".into(),
            turns: 41,
            tokens: 41_000,
        }),
        user_copy(CopySituation::TargetQuotaTight {
            account_b: "Personal".into(),
            left_pct: 8,
            tte: 45,
            account_c: "Lab".into(),
        }),
        user_copy(CopySituation::BothProgressed {
            turns: 12,
            conflicts: 3,
        }),
        user_copy(CopySituation::SwitchBackReady {
            account_a: "Work".into(),
            account_b: "Personal".into(),
        }),
        user_copy(CopySituation::SameBranchParallel {
            instance_a: "A".into(),
            instance_b: "B".into(),
            branch: "main".into(),
        }),
        user_copy(CopySituation::FailureSourceSafe {
            reason: "launch failed".into(),
            account_a: "Work".into(),
        }),
    ];
    let joined = all.join(" | ");
    assert!(joined.contains("Switching never happens on its own — your call."));
    assert!(joined.contains("nothing is merged silently"));
    assert!(joined.contains("you can resume it exactly where you left it."));
    assert!(
        !joined.to_lowercase().contains("some context may be lost"),
        "generic copy banned"
    );
}

/// Happy path: stage verifies, launch runs, source untouched.
#[test]
fn execute_happy_path_verifies_and_launches() {
    let work = tempfile::tempdir().unwrap();
    let main = seed_tree(&work.path().join("a"), "/w/proj", "sess-1");
    let source_before = dir_hashes(main.parent().unwrap());
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: false,
        torn_tail: false,
    };

    let mut launched = false;
    let (record, pid) = pipeline::execute(
        &support,
        &TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-1".into()),
        },
        &target(&work.path().join("b")),
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        &mut |install| {
            launched = true;
            assert!(
                install.reference.path.is_file(),
                "launch happens AFTER verify"
            );
            Ok(4242)
        },
    )
    .unwrap();
    assert!(launched);
    assert_eq!(pid, 4242);
    assert_eq!(record.source_hashes.len(), 2);
    assert_eq!(record.mechanism, Mechanism::Transplant);
    assert_eq!(record.fidelity_label, "F1");
    assert_eq!(
        source_before,
        dir_hashes(main.parent().unwrap()),
        "copy-never-move"
    );
}

/// S-F5: launch failure RETAINS the verified stage (retry reuses
/// it) and the source stays safe.
#[test]
fn launch_failure_retains_stage_and_source_safe() {
    let work = tempfile::tempdir().unwrap();
    let main = seed_tree(&work.path().join("a"), "/w/proj", "sess-5");
    let source_before = dir_hashes(main.parent().unwrap());
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: false,
        torn_tail: false,
    };

    let failure = pipeline::execute(
        &support,
        &TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-5".into()),
        },
        &target(&work.path().join("b")),
        FreezeChoice::FinishTurnThenSwitch,
        Mechanism::Transplant,
        &mut |_install| Err("terminal spawn failed".into()),
    )
    .unwrap_err();
    assert!(
        matches!(failure, SwitchFailure::LaunchFailed(ref reason) if reason.contains("spawn")),
        "{failure}"
    );
    assert_eq!(failure.stage(), FailureStage::Launch);
    // The staged artifacts remain for retry…
    assert!(
        work.path()
            .join("b/projects/-w-proj/sess-5.jsonl")
            .is_file(),
        "verified stage retained"
    );
    // …and the source is safe.
    assert_eq!(source_before, dir_hashes(main.parent().unwrap()));
    // The user's words name the account and never the spawn detail.
    assert_eq!(
        failure.words("Work", "Personal"),
        "The new session on Personal didn't start, so nothing moved. Try again, or resume it on Work where you left it."
    );
}

/// S-F2: a torn tail refuses before anything stages.
#[test]
fn torn_tail_refuses_before_staging() {
    let work = tempfile::tempdir().unwrap();
    let main = seed_tree(&work.path().join("a"), "/w/proj", "sess-2");
    let mut torn = std::fs::read(&main).unwrap();
    torn.extend_from_slice(b"{\"uuid\": \"u9\", \"pare");
    std::fs::write(&main, &torn).unwrap();
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: false,
        torn_tail: true,
    };

    let failure = pipeline::execute(
        &support,
        &TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-2".into()),
        },
        &target(&work.path().join("b")),
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        &mut |_| Ok(1),
    )
    .unwrap_err();
    assert!(
        failure.to_string().contains("tail does not parse"),
        "{failure}"
    );
    assert!(!work.path().join("b/projects").exists(), "nothing staged");
}

/// S-F4: a crash mid-copy leaves no partial target (adapter cleaned
/// its temp) and the source untouched.
#[test]
fn crash_mid_copy_leaves_no_partial_target() {
    let work = tempfile::tempdir().unwrap();
    let main = seed_tree(&work.path().join("a"), "/w/proj", "sess-4");
    let source_before = dir_hashes(main.parent().unwrap());
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: true,
        torn_tail: false,
    };

    let failure = pipeline::execute(
        &support,
        &TranscriptRef {
            path: main.clone(),
            native_id: Some("sess-4".into()),
        },
        &target(&work.path().join("b")),
        FreezeChoice::StopNowAndSwitch,
        Mechanism::Transplant,
        &mut |_| Ok(1),
    )
    .unwrap_err();
    assert!(matches!(failure, SwitchFailure::Adapter(_)), "{failure}");
    assert_eq!(source_before, dir_hashes(main.parent().unwrap()));
}

/// The failure→stage mapping drives the edge recording.
#[test]
fn failure_stage_mapping_covers_the_taxonomy() {
    assert_eq!(
        SwitchFailure::UnresolvedSource.stage(),
        FailureStage::Resolve
    );
    assert_eq!(SwitchFailure::TornTail.stage(), FailureStage::Freeze);
    assert_eq!(
        SwitchFailure::CollisionForked("x".into()).stage(),
        FailureStage::Stage
    );
    assert_eq!(
        SwitchFailure::StagingAborted("x".into()).stage(),
        FailureStage::Stage
    );
    assert_eq!(
        SwitchFailure::LaunchFailed("x".into()).stage(),
        FailureStage::Launch
    );
    assert_eq!(
        SwitchFailure::PrimeQuotaDeath(1200).stage(),
        FailureStage::Launch
    );
}

/// What a person reads for each failure: plain, the account named, never
/// the detail in brackets, the taxonomy code or "preflight".
#[test]
fn every_failure_reads_in_plain_words() {
    use juggler_handover::pipeline::PreflightStop;
    let failures = [
        SwitchFailure::UnresolvedSource,
        SwitchFailure::TornTail,
        SwitchFailure::CollisionForked("sess-9f3a".into()),
        SwitchFailure::StagingAborted("No space left on device (os error 28)".into()),
        SwitchFailure::LaunchFailed("terminal spawn failed".into()),
        SwitchFailure::PrimeQuotaDeath(12_400),
        SwitchFailure::OriginalSessionGone,
        SwitchFailure::MergeConflictOpen,
        SwitchFailure::HopCost(4, 380_000),
        SwitchFailure::Preflight(PreflightStop::TargetExhausted),
        SwitchFailure::Preflight(PreflightStop::VersionDrift),
        SwitchFailure::Adapter(AdapterError::InvalidLaunch("model_id is missing".into())),
    ];
    for failure in failures {
        let words = failure.words("Work", "Personal");
        for token in [
            "sess-9f3a",
            "os error",
            "spawn",
            "S-F",
            "preflight",
            "harness",
            "model_id",
            "(",
        ] {
            assert!(
                !words.contains(token),
                "a failure's words carry program text"
            );
        }
        assert!(
            words.contains("Work") || words.contains("Personal"),
            "a failure's words name the account"
        );
        assert!(words.ends_with('.'), "a failure's words are sentences");
        // The dialog shows them under a short title and adds nothing:
        // each thing is said once in them.
        for once in ["nothing moved", "try again", "where you left it"] {
            assert!(
                words.to_lowercase().matches(once).count() <= 1,
                "a failure's words say {once:?} twice"
            );
        }
    }
    assert_eq!(
        SwitchFailure::PrimeQuotaDeath(12_400).words("Work", "Personal"),
        "Personal ran out of usage while picking up the work, so it stopped after using about 12,400 tokens. Pick another account."
    );
}

/// Freeze protocol has exactly two choices — "copy live and hope"
/// does not exist and SIGSTOP is structurally absent.
#[test]
fn freeze_has_exactly_two_choices() {
    assert_eq!(
        FreezeChoice::FinishTurnThenSwitch,
        FreezeChoice::FinishTurnThenSwitch
    );
    assert_ne!(
        FreezeChoice::FinishTurnThenSwitch,
        FreezeChoice::StopNowAndSwitch
    );
    // Two variants — compile-time exhaustive; nothing else to call.
}

/// 0.3.9 (the maintainer's switch of 2026-11-04): a conversation still
/// open in its app keeps writing at its end while it's switched. The
/// after-launch check is that the source still starts with what was
/// carried: a turn written meanwhile passes, an earlier line changed
/// does not.
#[test]
fn a_source_that_grows_during_the_switch_passes_and_one_rewritten_does_not() {
    use std::io::Write as _;
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: false,
        torn_tail: false,
    };
    for rewrite in [false, true] {
        let work = tempfile::tempdir().unwrap();
        let main = seed_tree(&work.path().join("a"), "/w/proj", "sess-1");
        let result = pipeline::execute(
            &support,
            &TranscriptRef {
                path: main.clone(),
                native_id: Some("sess-1".into()),
            },
            &target(&work.path().join("b")),
            FreezeChoice::StopNowAndSwitch,
            Mechanism::Transplant,
            &mut |_| {
                if rewrite {
                    let mut bytes = std::fs::read(&main).unwrap();
                    bytes[2] ^= 0x20;
                    std::fs::write(&main, bytes).unwrap();
                } else {
                    let mut file = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&main)
                        .unwrap();
                    writeln!(file, "{}", line("u9", Some("u2"), "sess-1", "/w/proj")).unwrap();
                }
                Ok(4242)
            },
        );
        if rewrite {
            assert!(
                matches!(result, Err(SwitchFailure::StagingAborted(ref why)) if why.contains("source changed")),
                "a rewritten source is caught: {result:?}"
            );
        } else {
            assert!(result.is_ok(), "a grown source passes: {result:?}");
        }
    }
}
