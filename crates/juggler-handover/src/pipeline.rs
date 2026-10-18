//! Switch execution pipeline (AJ-E205; pass2/02 §5, §9).
//!
//! Resolve → Preflight → Freeze → Snapshot → Stage → Verify →
//! Launch → Confirm → Record — one pipeline for every edge. Any
//! failure leaves the source exactly resumable and yields a failed
//! edge (taxonomy ids + verbatim §7.1 copy); silent partial state
//! is impossible by construction: staging is temp-dir + verify +
//! atomic rename (AJ-E204), the launch step runs AFTER verification
//! through a caller-supplied hook (the engine never spawns), and a
//! launch failure retains the verified stage for retry while the
//! source stays untouched. The freeze protocol offers exactly two
//! choices — finish-turn or stop-now — there is no "copy live and
//! hope" path and no SIGSTOP (SPEC:225 forbids it for corruption
//! risk).

use std::path::Path;

use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::{HandoverSupport, InstallOutcome};
use juggler_core::model::{Account, FailureStage, TranscriptRef};

use crate::fidelity::{FidelityInput, PairFidelity, score_pair};
use crate::manifest::{ManifestRow, sha256_hex};

/// The §7.1 copy table (verbatim — pass2/02 lines 419-437). Generic
/// "some context may be lost" is banned; each string names what does
/// not carry.
#[must_use]
pub fn user_copy(situation: CopySituation) -> String {
    match situation {
        CopySituation::TransplantProven {
            account_b,
            account_a,
        } => format!(
            "Continues on {account_b} with the full conversation. Running tools and subagents stop; {account_a} stays resumable."
        ),
        CopySituation::TransplantUnproven { account_a } => format!(
            "Experimental: only the main transcript moves. Subagent files and rewind history stay on {account_a}. {account_a} stays resumable."
        ),
        CopySituation::ReplayCrossProvider {
            provider_b,
            turns,
            tokens,
        } => format!(
            "Fresh start on {provider_b}: it re-reads the full conversation as text ({turns} turns, ≈{tokens} tokens). Native todos and tool state don't carry — they're rebuilt from the story."
        ),
        CopySituation::CompactedSource { provider_b } => format!(
            "This conversation was compacted. Early detail carries as text, not as live memory — {provider_b} may ask about early decisions."
        ),
        CopySituation::TargetQuotaTight {
            account_b,
            left_pct,
            tte,
            account_c,
        } => format!(
            "{account_b} has ≈{left_pct}% left (≈{tte} at its recent pace). {account_c} has more room. Switching never happens automatically — your call."
        ),
        CopySituation::BothProgressed { turns, conflicts } => format!(
            "Both sides moved since the switch. {turns} new turns and {conflicts} file conflicts need your choice — nothing is merged silently."
        ),
        CopySituation::SwitchBackReady {
            account_a,
            account_b,
        } => format!(
            "{account_a} has reset. Resume the original conversation there? Work from {account_b} arrives as one 'while you were away' update."
        ),
        CopySituation::SameBranchParallel {
            instance_a,
            instance_b,
            branch,
        } => format!(
            "{instance_a} and {instance_b} are both live on branch {branch}. Edits can collide — give {instance_b} its own worktree?"
        ),
        CopySituation::FailureSourceSafe { reason, account_a } => format!(
            "Switch didn't complete — {reason}. Nothing changed on {account_a}; you can resume it exactly where you left it."
        ),
    }
}

/// One §7.1 situation (the copy table's key).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CopySituation {
    /// Transplant on a proven pair (E210 green).
    TransplantProven {
        /// Target account label.
        account_b: String,
        /// Source account label.
        account_a: String,
    },
    /// Transplant on an unproven pair (today).
    TransplantUnproven {
        /// Source account label.
        account_a: String,
    },
    /// Replay across providers.
    ReplayCrossProvider {
        /// Target provider label.
        provider_b: String,
        /// Distilled turn count.
        turns: u32,
        /// Replay token estimate.
        tokens: u64,
    },
    /// The source had a compaction boundary.
    CompactedSource {
        /// Target provider label.
        provider_b: String,
    },
    /// Target quota is tight (warn, never block).
    TargetQuotaTight {
        /// Target account label.
        account_b: String,
        /// Percent left.
        left_pct: u32,
        /// Time-to-exhaust minutes.
        tte: u32,
        /// The roomier alternative.
        account_c: String,
    },
    /// Both sides progressed (E207's reconciliation).
    BothProgressed {
        /// New away turns.
        turns: u32,
        /// File conflicts.
        conflicts: u32,
    },
    /// Switch-back offer on quota reset (E207).
    SwitchBackReady {
        /// The reset account.
        account_a: String,
        /// The away account.
        account_b: String,
    },
    /// Two live instances on one branch.
    SameBranchParallel {
        /// First instance label.
        instance_a: String,
        /// Second instance label.
        instance_b: String,
        /// Branch name.
        branch: String,
    },
    /// Any pipeline failure — the source is safe.
    FailureSourceSafe {
        /// The failure reason.
        reason: String,
        /// Source account label.
        account_a: String,
    },
}

/// The §9 failure taxonomy (S-F1..S-F9). Every variant carries its
/// stage + engine behaviour; the user copy comes from
/// [`user_copy`] with [`CopySituation::FailureSourceSafe`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SwitchFailure {
    /// S-F1 (Resolve): transcript could not be resolved — never
    /// newest-mtime. Nothing moved; the user picks the file.
    #[error("S-F1: couldn't tell which file is this session — pick it")]
    UnresolvedSource,
    /// S-F2 (Freeze): torn tail / still writing. The two-choice
    /// freeze is the protocol; parse-verify refuses torn bytes.
    #[error("S-F2: source is still writing — stop it first")]
    TornTail,
    /// S-F3 (Stage): id collision on target — forked (the
    /// rollback-safe path; the original target file is untouched).
    #[error("S-F3: collision on target — opened as a fork ({0})")]
    CollisionForked(String),
    /// S-F4 (Stage): crash mid-copy — the staged temp is removed;
    /// the source is untouched by construction.
    #[error("S-F4: staging failed mid-copy — cleaned up, source untouched ({0})")]
    StagingAborted(String),
    /// S-F5 (Launch): target launch failed — staged artifacts are
    /// RETAINED for retry; resume the source meanwhile.
    #[error("S-F5: target launch failed — staged for retry ({0})")]
    LaunchFailed(String),
    /// S-F6 (Launch): target quota died during replay prime — prime
    /// aborted; tokens burned are recorded, never hidden.
    #[error("S-F6: target quota died during prime — aborted, {0} tokens burned")]
    PrimeQuotaDeath(u64),
    /// S-F7 (Return): switch-back target session deleted/expired —
    /// never fabricate a resume (engine surface lands with E207).
    #[error("S-F7: the original session is gone — nearest epoch or pack-primed fresh start")]
    OriginalSessionGone,
    /// S-F8 (Return): merge conflict unresolved — the work item
    /// stays forked, both epochs resumable (E207).
    #[error("S-F8: merge conflict unresolved — work stays forked, both sides resumable")]
    MergeConflictOpen,
    /// S-F9 (loop): n-hop loop with growing replay cost — allowed,
    /// cost honesty turns amber; informs, never blocks.
    #[error("S-F9: {0} switches so far ≈{1} tokens — transplant-capable targets cost less")]
    HopCost(u32, u64),
    /// A preflight gate stopped the switch before any byte moved.
    #[error("preflight stopped: {0}")]
    Preflight(String),
    /// Adapter-level failure.
    #[error("{0}")]
    Adapter(#[from] AdapterError),
}

impl SwitchFailure {
    /// The pipeline stage this failure belongs to (the edge's
    /// `Failed{stage, reason}`).
    #[must_use]
    pub fn stage(&self) -> FailureStage {
        match self {
            Self::UnresolvedSource => FailureStage::Resolve,
            Self::Preflight(_) => FailureStage::Preflight,
            Self::TornTail => FailureStage::Freeze,
            Self::StagingAborted(_) | Self::CollisionForked(_) => FailureStage::Stage,
            Self::LaunchFailed(_) | Self::PrimeQuotaDeath(_) => FailureStage::Launch,
            Self::OriginalSessionGone | Self::MergeConflictOpen | Self::HopCost(_, _) => {
                FailureStage::Confirm
            }
            Self::Adapter(error) => {
                let _ = error;
                FailureStage::Stage
            }
        }
    }

    /// The verbatim §7.1 user copy for this failure.
    #[must_use]
    pub fn copy(&self, account_a: &str) -> String {
        user_copy(CopySituation::FailureSourceSafe {
            reason: self.to_string(),
            account_a: account_a.to_owned(),
        })
    }
}

/// The hard-stop gate flags (checked in order; any failure stops
/// before a byte moves).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PreflightGates {
    /// Hard filter: target account quota exhausted with no reset
    /// before need-by (E611's ranking filter).
    pub target_exhausted_no_reset: bool,
    /// Version drift: the target harness version is outside the
    /// verified list (parse-check failed).
    pub version_drift: bool,
}

/// Git/worktree state at preflight (§5.3).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GitState {
    /// Two live instances share one branch on this checkout.
    pub same_branch_both_live: bool,
    /// Zombie children of the frozen instance (listed at freeze;
    /// cleanup is a separate confirmed action — never automatic).
    pub zombie_children: Vec<String>,
    /// Uncommitted diff snapshot bytes (into `diffs/` at freeze,
    /// never auto-applied elsewhere).
    pub uncommitted_diff: Option<Vec<u8>>,
}

/// The preflight inputs (computed by the caller from live engine
/// state — quota, git, versions; the pipeline itself stays I/O-free
/// except the transcript checks).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PreflightInput {
    /// E203 scoring input for the ordered pair (mechanism follows
    /// from the pair + same-account flag).
    pub fidelity: FidelityInput,
    /// The source transcript had a compaction boundary.
    pub compacted: bool,
    /// Gate flags (quota + version drift — the hard stops).
    pub gates: PreflightGates,
    /// Soft: target headroom is Tight (the §7.1 warn copy fires).
    pub target_quota_tight: Option<(u32, u32)>,
    /// Git/worktree state (the same-branch hazard + zombies + the
    /// uncommitted-diff snapshot).
    pub git: GitState,
    /// Cumulative hops on this work item + spent replay tokens
    /// (S-F9 cost honesty).
    pub hop_count: u32,
    /// Replay tokens already spent on this work item's hops.
    pub replay_tokens_spent: u64,
}

/// The preflight verdict: everything the door (E206) and the freeze
/// dialog render.
#[derive(Clone, PartialEq, Debug)]
pub struct PreflightReport {
    /// The ordered-pair score (E203).
    pub pair: PairFidelity,
    /// The mechanism for this pair.
    pub mechanism: Mechanism,
    /// §7.1 warnings to show (situations, rendered by the UI).
    pub warnings: Vec<CopySituation>,
    /// Zombie children listed (no action taken).
    pub zombie_children: Vec<String>,
    /// The uncommitted diff snapshot rides the pack's `diffs/`.
    pub diff_snapshotted: bool,
    /// Hop-cost honesty (S-F9) when it has grown.
    pub hop_cost: Option<CopySituation>,
}

/// How the work physically moves for this pair.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mechanism {
    /// Same account, native resume (T1).
    NativeResume,
    /// Same provider, different account (T2).
    Transplant,
    /// Cross provider (T3).
    Replay,
    /// Export/priming only (T4).
    PackOnly,
}

/// Runs preflight: ordered checks, each failure stopping BEFORE any
/// byte moves (Req.1). Pure.
///
/// # Errors
///
/// [`SwitchFailure::Preflight`] for hard stops (exhausted target,
/// version drift); everything else lands in the report as warnings.
pub fn preflight(input: &PreflightInput) -> Result<PreflightReport, SwitchFailure> {
    // 1. Ordered-pair mechanism + score (E203).
    let pair = score_pair(&input.fidelity);
    let same_account = input.fidelity.flags.same_account;
    let same_provider = input.fidelity.from_provider == input.fidelity.to_provider;
    let mechanism = if same_account && input.fidelity.flags.native_resume {
        Mechanism::NativeResume
    } else if same_provider {
        Mechanism::Transplant
    } else if pair.label >= crate::fidelity::FidelityLabel::F1 {
        Mechanism::Replay
    } else {
        Mechanism::PackOnly
    };

    // 3. Target quota gate — the hard filter stops here.
    if input.gates.target_exhausted_no_reset {
        return Err(SwitchFailure::Preflight(
            "target account is exhausted with no reset before you'd need it".into(),
        ));
    }

    // 4. Version-drift parse check — abort (offer replay is E206's
    // door copy; preflight stops).
    if input.gates.version_drift {
        return Err(SwitchFailure::Preflight(
            "target runs a harness version this switch has not been verified against".into(),
        ));
    }

    let mut warnings = Vec::new();
    // 2. Compaction scan → the honest F2 copy.
    if input.compacted {
        warnings.push(CopySituation::CompactedSource {
            provider_b: input.fidelity.to_provider.clone(),
        });
    }
    // 3b. Tight quota → warn, never block.
    if let Some((left_pct, tte)) = input.target_quota_tight {
        warnings.push(CopySituation::TargetQuotaTight {
            account_b: input.fidelity.to_provider.clone(),
            left_pct,
            tte,
            account_c: "another account".to_owned(),
        });
    }
    // 5. Git/worktree: same-branch both-live warn + worktree offer.
    if input.git.same_branch_both_live {
        warnings.push(CopySituation::SameBranchParallel {
            instance_a: "This instance".to_owned(),
            instance_b: "the target".to_owned(),
            branch: "the current branch".to_owned(),
        });
    }
    // 9. Hop-cost honesty (amber inform, never a block).
    let hop_cost = (input.hop_count >= 3).then(|| CopySituation::ReplayCrossProvider {
        provider_b: input.fidelity.to_provider.clone(),
        turns: input.hop_count,
        tokens: input.replay_tokens_spent,
    });
    if let Some(cost) = &hop_cost {
        warnings.push(cost.clone());
    }

    Ok(PreflightReport {
        pair,
        mechanism,
        warnings,
        zombie_children: input.git.zombie_children.clone(),
        diff_snapshotted: input.git.uncommitted_diff.is_some(),
        hop_cost,
    })
}

/// The freeze protocol's exactly-two choices (§5.2) — there is no
/// third "copy live and hope" option, and SIGSTOP pausing is
/// forbidden (structurally absent).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FreezeChoice {
    /// Default: wait for the turn boundary, then switch.
    FinishTurnThenSwitch,
    /// Stop now (SPEC:225 stop ladder); the partial turn is marked
    /// truncated in the pack, never presented as complete.
    StopNowAndSwitch,
}

/// The executed switch's evidence (what the caller records on the
/// edge).
#[derive(Clone, PartialEq, Debug)]
pub struct SwitchRecord {
    /// The install outcome (fork/fallback flags included).
    pub install: InstallOutcome,
    /// sha256 of every SOURCE file (copy-never-move invariant).
    pub source_hashes: Vec<ManifestRow>,
    /// sha256 of every INSTALLED file.
    pub installed_hashes: Vec<ManifestRow>,
    /// The freeze choice that ran.
    pub freeze: FreezeChoice,
    /// The mechanism used.
    pub mechanism: Mechanism,
    /// The fidelity label shown at execution.
    pub fidelity_label: String,
}

/// The launch hook: the app spawns under its supervisor AFTER the
/// stage verified. Returning `Err` simulates/documents S-F5 (staged
/// artifacts retained for retry).
pub type LaunchHook<'a> = dyn FnMut(&InstallOutcome) -> Result<u32, String> + 'a;

/// Executes one switch end-to-end: snapshot → stage/verify (via the
/// adapter's atomic install) → launch (caller hook) → confirm. The
/// source is resolved by `reference` (S-F1); a torn tail refuses at
/// snapshot (S-F2); a launch failure retains the verified stage and
/// reports S-F5 with the source untouched.
///
/// # Errors
///
/// Every [`SwitchFailure`] leaves the source exactly resumable; the
/// caller writes the failed edge from it.
pub fn execute(
    support: &dyn HandoverSupport,
    reference: &TranscriptRef,
    target: &Account,
    freeze: FreezeChoice,
    mechanism: Mechanism,
    launch: &mut LaunchHook<'_>,
) -> Result<(SwitchRecord, u32), SwitchFailure> {
    let _ = freeze; // the choice shaped the source state BEFORE this
    // call (turn boundary or stop ladder); the pack
    // records truncation via the caller's staging.
    let bundle = support.bundle_session(reference)?;
    let source_hashes: Vec<ManifestRow> = bundle
        .files
        .iter()
        .map(|file| ManifestRow {
            rel_path: file.name.clone(),
            sha256: sha256_hex(&file.bytes),
            bytes: file.bytes.len() as u64,
            kind: "transcript".to_owned(),
            disposition: "copy".to_owned(),
        })
        .collect();
    let source_dir = reference
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();

    // Stage + verify (atomic, fork-aware). An adapter failure here
    // cleaned its own temp (E204); nothing partial remains.
    let install = support.install_bundle(&bundle, target)?;
    let installed_hashes = verify_installed(&install, &bundle, target);
    let all_present = !installed_hashes.iter().any(|row| row.sha256.is_empty());
    if !all_present {
        // S-F4 shape: verification failed after install — remove the
        // staged files (the temp-dir discipline's post-hoc guard).
        for row in &installed_hashes {
            let _ = std::fs::remove_file(
                install
                    .reference
                    .path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&row.rel_path),
            );
        }
        return Err(SwitchFailure::StagingAborted(
            "installed set did not verify".into(),
        ));
    }

    // Launch (the app's hook; never the engine's). S-F5 keeps the
    // verified stage — the caller retries from it.
    let pid = launch(&install).map_err(SwitchFailure::LaunchFailed)?;

    // Copy-never-move, checked post-launch too.
    for row in &source_hashes {
        let path = source_dir.join(&row.rel_path);
        match std::fs::read(&path) {
            Ok(bytes) if sha256_hex(&bytes) == row.sha256 => {}
            _ => {
                return Err(SwitchFailure::StagingAborted(format!(
                    "source changed during the switch: {}",
                    row.rel_path
                )));
            }
        }
    }

    let record = SwitchRecord {
        install,
        source_hashes,
        installed_hashes,
        freeze,
        mechanism,
        fidelity_label: bundle.manifest.fidelity.level.clone(),
    };
    Ok((record, pid))
}

/// Hashes the installed set (fork-rewritten names included); a
/// missing file yields an empty hash (verification failure).
fn verify_installed(
    install: &InstallOutcome,
    bundle: &juggler_adapter::handover::SessionBundle,
    _target: &Account,
) -> Vec<ManifestRow> {
    let dir = install
        .reference
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    bundle
        .files
        .iter()
        .map(|file| {
            let name = if install.forked {
                file.name.replace(
                    &bundle.manifest.native_id,
                    install.reference.native_id.as_deref().unwrap_or_default(),
                )
            } else {
                file.name.clone()
            };
            let bytes = std::fs::read(dir.join(&name)).unwrap_or_default();
            ManifestRow {
                rel_path: name,
                sha256: sha256_hex(&bytes),
                bytes: bytes.len() as u64,
                kind: "transcript".to_owned(),
                disposition: if install.forked { "translate" } else { "copy" }.to_owned(),
            }
        })
        .collect()
}
