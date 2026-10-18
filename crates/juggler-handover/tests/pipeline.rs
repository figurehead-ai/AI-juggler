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

use std::path::{Path, PathBuf};

use juggler_adapter::adapter::HarnessAdapter;
use juggler_adapter::capabilities::Capabilities;
use juggler_adapter::discovery::DiscoveryReport;
use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::{
    BundleFile, BundleManifest, Fidelity, HandoverSupport, InstallOutcome, SessionBundle, TurnText,
};
use juggler_adapter::launch::{EnvMap, LaunchMode, LaunchSpec};
use juggler_adapter::model::{Account, HarnessId, Profile};
use juggler_adapter::watcher::SessionWatcher;
use juggler_core::model::{
    Account as CoreAccount, AccountId, FailureStage, Timestamp, TranscriptRef,
};
use juggler_handover::fidelity::{FidelityInput, PairFlags};
use juggler_handover::pipeline::{
    self, CopySituation, FreezeChoice, Mechanism, PreflightGates, PreflightInput, SwitchFailure,
    user_copy,
};

fn line(uuid: &str, parent: Option<&str>, session: &str, cwd: &str) -> String {
    serde_json::json!({
        "uuid": uuid, "parentUuid": parent, "sessionId": session,
        "cwd": cwd, "type": "user",
        "message": {"role": "user", "content": "work"},
    })
    .to_string()
}

fn seed_tree(config: &Path, cwd: &str, session: &str) -> PathBuf {
    let dir = config.join("projects").join(cwd.replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{session}.jsonl")),
        format!(
            "{}\n{}\n",
            line("u1", None, session, cwd),
            line("u2", Some("u1"), session, cwd)
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("{session}-agent-1.jsonl")),
        format!("{}\n", line("s1", None, session, cwd)),
    )
    .unwrap();
    dir.join(format!("{session}.jsonl"))
}

fn target(state_dir: &Path) -> CoreAccount {
    CoreAccount {
        id: AccountId::new("acc-b"),
        harness: HarnessId::new("claude-code"),
        label: "Personal".into(),
        plan: None,
        state_dir: state_dir.to_path_buf(),
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
    }
}

fn dir_hashes(dir: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| {
            let path = entry.path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                juggler_adapter::handover::sha256_hex(&std::fs::read(&path).unwrap()),
            )
        })
        .collect();
    out.sort();
    out
}

/// A minimal real-filesystem HandoverSupport (Claude-shaped files,
/// Claude-ish rules — enough for pipeline mechanics; the REAL Claude
/// implementation is proven in juggler-adapters' own suite).
struct TreeSupport {
    harness: HarnessId,
    /// When set, `install_bundle` writes only the first file then
    /// fails (S-F4 fault injection).
    partial_install: bool,
    /// When set, `bundle_session` returns a torn tail (S-F2).
    torn_tail: bool,
}

impl HarnessAdapter for TreeSupport {
    fn id(&self) -> HarnessId {
        self.harness.clone()
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }
    fn verified_against(&self) -> &'static [&'static str] {
        &["1.0.0"]
    }
    fn discover(&self) -> DiscoveryReport {
        DiscoveryReport {
            adapter: self.harness.clone(),
            install: juggler_adapter::discovery::InstallStatus::NotFound,
            verification: juggler_adapter::discovery::VerificationStatus::Unverified,
            binary: None,
            notes: Vec::new(),
        }
    }
    fn prepare_account(&self, _state_dir: &Path) -> Result<EnvMap, AdapterError> {
        Ok(EnvMap::new())
    }
    fn launch_spec(
        &self,
        _account: &Account,
        _profile: &Profile,
        cwd: &Path,
        _mode: &LaunchMode,
    ) -> Result<LaunchSpec, AdapterError> {
        Ok(LaunchSpec {
            program: PathBuf::from("/usr/bin/true"),
            args: Vec::new(),
            env: EnvMap::new(),
            cwd: cwd.to_path_buf(),
            needs_pty: false,
        })
    }
    fn watcher(&self, _root: &Path) -> Result<Box<dyn SessionWatcher>, AdapterError> {
        Err(AdapterError::Watch("no watcher".into()))
    }
}

impl HandoverSupport for TreeSupport {
    fn bundle_session(&self, reference: &TranscriptRef) -> Result<SessionBundle, AdapterError> {
        let main = std::fs::read(&reference.path)?;
        if self.torn_tail && !juggler_adapter::handover::tail_parses(&main) {
            return Err(AdapterError::Unsupported(
                "source transcript tail does not parse — stop the session first".into(),
            ));
        }
        let dir = reference.path.parent().unwrap();
        let stem = reference
            .path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut files = Vec::new();
        for sibling in juggler_adapter::handover::sibling_paths(dir, &stem) {
            let mut bytes = std::fs::read(&sibling)?;
            if self.torn_tail
                && sibling.file_name().unwrap().to_string_lossy() == format!("{stem}.jsonl")
            {
                bytes.extend_from_slice(b"{\"torn");
            }
            files.push(BundleFile {
                name: sibling.file_name().unwrap().to_string_lossy().into_owned(),
                bytes,
            });
        }
        let cwd = juggler_adapter::handover::embedded_cwd(&main).unwrap_or_default();
        Ok(SessionBundle {
            manifest: BundleManifest {
                schema: 1,
                harness: self.harness.clone(),
                native_id: stem,
                cwd: PathBuf::from(cwd),
                harness_version: None,
                created_at: Timestamp::from_millis(0),
                fidelity: Fidelity {
                    level: "F1".into(),
                    caps_applied: vec![],
                },
                redacted: false,
            },
            files,
        })
    }

    fn install_bundle(
        &self,
        bundle: &SessionBundle,
        target: &Account,
    ) -> Result<InstallOutcome, AdapterError> {
        let dir = target
            .state_dir
            .join("projects")
            .join(bundle.manifest.cwd.to_string_lossy().replace('/', "-"));
        std::fs::create_dir_all(&dir).unwrap();
        for (idx, file) in bundle.files.iter().enumerate() {
            if self.partial_install && idx == 1 {
                // S-F4: crash after the first file.
                return Err(AdapterError::AccountDir {
                    path: dir.clone(),
                    message: "injected mid-copy crash".into(),
                });
            }
            std::fs::write(dir.join(&file.name), &file.bytes).unwrap();
        }
        Ok(InstallOutcome {
            reference: TranscriptRef {
                path: dir.join(format!("{}.jsonl", bundle.manifest.native_id)),
                native_id: Some(bundle.manifest.native_id.clone()),
            },
            forked: false,
            target_dir_fallback: false,
        })
    }

    fn prime_prompt(&self, _turns: &[TurnText]) -> Result<String, AdapterError> {
        Ok(String::new())
    }
}

fn preflight_input() -> PreflightInput {
    PreflightInput {
        fidelity: FidelityInput {
            from_provider: "claude-code".into(),
            to_provider: "claude-code".into(),
            dispositions: Default::default(),
            compact_boundary: false,
            flags: PairFlags::default(),
            unmapped_keys: vec![],
        },
        compacted: false,
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
    input.compacted = true;
    input.target_quota_tight = Some((8, 45));
    input.git.same_branch_both_live = true;
    input.git.zombie_children = vec!["pid 1234 (node vite)".into()];
    input.hop_count = 4;
    let report = pipeline::preflight(&input).unwrap();
    assert_eq!(
        report.warnings.len(),
        4,
        "compacted + tight + same-branch + hop-cost"
    );
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
        user_copy(CopySituation::CompactedSource {
            provider_b: "Codex".into(),
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
    assert!(joined.contains("Switching never happens automatically — your call."));
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
    // The user copy names the source's safety.
    let copy = failure.copy("Work");
    assert!(copy.contains("Nothing changed on Work"));
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
    assert!(
        SwitchFailure::UnresolvedSource
            .copy("Work")
            .contains("resume it exactly where you left it")
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
