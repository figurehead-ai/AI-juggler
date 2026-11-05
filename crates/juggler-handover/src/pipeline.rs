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
use crate::manifest::ManifestRow;

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
            "Beta, not proven yet: the whole session is carried over, with its subagents and file history. {account_a} stays resumable."
        ),
        CopySituation::ReplayCrossProvider {
            provider_b,
            turns,
            tokens,
        } => format!(
            "Fresh start on {provider_b}: it reads the conversation, what each tool did, the task list and the project's changes ({turns} turns, ≈{tokens} tokens). Tool state and subagents don't carry."
        ),
        CopySituation::TargetQuotaTight {
            account_b,
            left_pct,
            tte,
            account_c,
        } => format!(
            "{account_b} has about {left_pct}% left, about {tte} min at its recent pace; {account_c} may have more room. Switching never happens on its own — your call."
        ),
        CopySituation::BothProgressed { turns, conflicts } => format!(
            "Both sides moved since the switch. {turns} new turns and {conflicts} file conflicts need your choice — nothing is merged silently."
        ),
        CopySituation::SwitchBackReady {
            account_a,
            account_b,
        } => crate::switch_back::suggestion_copy(&account_a, &account_b),
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
        /// The Account that reset, by its name (D-62: not its folder's).
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

/// Why a preflight gate stopped a switch before any byte moved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum PreflightStop {
    /// The target account has no usage left and no reset before the
    /// work would need it (E611's hard filter).
    TargetExhausted,
    /// The target runs a tool version this switch hasn't been checked
    /// against.
    VersionDrift,
}

impl std::fmt::Display for PreflightStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TargetExhausted => {
                "target account is exhausted with no reset before you'd need it"
            }
            Self::VersionDrift => {
                "target runs a harness version this switch has not been verified against"
            }
        })
    }
}

/// The §9 failure taxonomy (S-F1..S-F9). Every variant carries its
/// stage + engine behaviour. `Display` is the program's own text, for
/// the error log; what a person reads is [`SwitchFailure::words`]
/// (plain, the accounts named, never the detail in brackets).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SwitchFailure {
    /// S-F1 (Resolve): transcript could not be resolved — never
    /// newest-mtime. Nothing moved; the user picks the file.
    #[error("couldn't tell which file holds this session's conversation — pick it")]
    UnresolvedSource,
    /// S-F2 (Freeze): torn tail / still writing. The two-choice
    /// freeze is the protocol; parse-verify refuses torn bytes.
    #[error("the session is still writing its conversation — stop it first")]
    TornTail,
    /// S-F3 (Stage): id collision on target — forked (the
    /// rollback-safe path; the original target file is untouched).
    #[error(
        "the other account already has this conversation, so it opened as a separate copy ({0})"
    )]
    CollisionForked(String),
    /// S-F4 (Stage): crash mid-copy — the staged temp is removed;
    /// the source is untouched by construction.
    #[error(
        "copying stopped partway — what was copied is cleaned up, and the original is untouched ({0})"
    )]
    StagingAborted(String),
    /// S-F5 (Launch): target launch failed — staged artifacts are
    /// RETAINED for retry; resume the source meanwhile.
    #[error(
        "the session on the other account didn't start — the switch is ready to try again ({0})"
    )]
    LaunchFailed(String),
    /// S-F6 (Launch): target quota died during replay prime — prime
    /// aborted; tokens burned are recorded, never hidden.
    #[error(
        "the other account ran out of usage while picking up the work — stopped, {0} tokens used"
    )]
    PrimeQuotaDeath(u64),
    /// S-F7 (Return): switch-back target session deleted/expired —
    /// never fabricate a resume (engine surface lands with E207).
    #[error("the original conversation is gone")]
    OriginalSessionGone,
    /// S-F8 (Return): merge conflict unresolved — the work item
    /// stays forked, both epochs resumable (E207).
    #[error(
        "files changed on both sides and need your choice — the work stays split, and both sides can still be resumed"
    )]
    MergeConflictOpen,
    /// S-F9 (loop): n-hop loop with growing replay cost — allowed,
    /// cost honesty turns amber; informs, never blocks.
    #[error("{0} switches so far, about {1} tokens — a switch within the same tool costs less")]
    HopCost(u32, u64),
    /// A preflight gate stopped the switch before any byte moved.
    #[error("preflight stopped: {0}")]
    Preflight(PreflightStop),
    /// Adapter-level failure.
    #[error("{0}")]
    Adapter(#[from] AdapterError),
}

impl SwitchFailure {
    /// The §9 taxonomy code (`S-F1`..`S-F9`), for logs and the evidence:
    /// the user's words never carry it (AJ-E1543, plain words).
    #[must_use]
    pub fn code(&self) -> Option<&'static str> {
        Some(match self {
            Self::UnresolvedSource => "S-F1",
            Self::TornTail => "S-F2",
            Self::CollisionForked(_) => "S-F3",
            Self::StagingAborted(_) => "S-F4",
            Self::LaunchFailed(_) => "S-F5",
            Self::PrimeQuotaDeath(_) => "S-F6",
            Self::OriginalSessionGone => "S-F7",
            Self::MergeConflictOpen => "S-F8",
            Self::HopCost(_, _) => "S-F9",
            Self::Preflight(_) | Self::Adapter(_) => return None,
        })
    }

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

    /// What a person reads: what happened, then what to do next, each
    /// said once (the switch dialog adds only a short title above it),
    /// with the accounts named — `source` is the account the work comes
    /// from, `target` the one it was going to. The detail in brackets (an
    /// OS string, a spawn error, an id) never shows; it stays in
    /// `Display`, which goes to the error log.
    #[must_use]
    pub fn words(&self, source: &str, target: &str) -> String {
        match self {
            Self::UnresolvedSource => format!(
                "AI Juggler couldn't tell which file holds this conversation on {source}, so nothing moved. Resume it on {source} where you left it."
            ),
            Self::TornTail => format!(
                "The original on {source} is still writing its reply, so nothing moved. Let it finish, or stop it, and try again."
            ),
            Self::CollisionForked(_) => {
                format!("{target} already has this conversation, so it opened as a separate copy.")
            }
            Self::StagingAborted(_) => format!(
                "Copying to {target} stopped partway, so nothing moved. What was copied has been cleaned up: try again."
            ),
            Self::LaunchFailed(_) => {
                format!(
                    "The new session on {target} didn't start, so nothing moved. Try again, or resume it on {source} where you left it."
                )
            }
            Self::PrimeQuotaDeath(tokens) => format!(
                "{target} ran out of usage while picking up the work, so it stopped after using about {} tokens. Pick another account.",
                thousands(*tokens)
            ),
            Self::OriginalSessionGone => format!(
                "The original conversation on {source} is gone, so there's nothing to switch back to."
            ),
            Self::MergeConflictOpen => format!(
                "Files changed on both {source} and {target}, and they need your choice. The work stays split, and both can still be resumed."
            ),
            Self::HopCost(hops, tokens) => format!(
                "Moving to {target} makes {hops} switches for this work, about {} tokens in all. Switching within the same tool costs less.",
                thousands(*tokens)
            ),
            Self::Preflight(PreflightStop::TargetExhausted) => format!(
                "{target} has no usage left and doesn't reset before you'd need it, so nothing moved. Pick another account."
            ),
            Self::Preflight(PreflightStop::VersionDrift) => format!(
                "{target}'s version of its tool hasn't been checked with this kind of switch yet, so nothing moved. Pick another account."
            ),
            Self::Adapter(error) => adapter_words(error, source, target),
        }
    }

    /// Whether trying the same switch again can succeed (0.3.9: a refusal
    /// that would refuse again offered Retry, and every Retry failed).
    #[must_use]
    pub fn retryable(&self) -> bool {
        match self {
            // The same switch would stop the same way: another account,
            // or the user's own choice, is the way on.
            Self::OriginalSessionGone
            | Self::CollisionForked(_)
            | Self::UnresolvedSource
            | Self::PrimeQuotaDeath(_)
            | Self::MergeConflictOpen
            | Self::HopCost(..)
            | Self::Preflight(_) => false,
            // Trying again helps once the reply finishes, the disk has
            // room or the folder can be written; never for a conversation
            // too large, gone, or already there.
            Self::Adapter(AdapterError::Unsupported(message)) => message.contains("does not parse"),
            Self::Adapter(AdapterError::AccountDir { message, .. }) => {
                !message.contains("already has this file")
            }
            Self::Adapter(AdapterError::Io(error)) => !error.to_string().contains("too large"),
            _ => true,
        }
    }
}

/// A tool-level failure in words: what happened and what to do, never
/// "something went wrong" when the reason is known (0.3.9).
fn adapter_words(error: &AdapterError, source: &str, target: &str) -> String {
    let too_large = format!(
        "This conversation is too large to carry to {target} in one piece, so nothing moved. Continue it with a summary on {target} instead."
    );
    match error {
        AdapterError::Io(io) if io.kind() == std::io::ErrorKind::StorageFull => {
            disk_full(target, &io.to_string())
        }
        AdapterError::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied => {
            not_allowed(target)
        }
        AdapterError::Io(io) if io.to_string().contains("too large") => too_large,
        // An installer names the folder it was writing; the system's
        // reason is in its message.
        AdapterError::AccountDir { message, .. }
            if message.contains("(os error 28)") || message.contains("not enough free space") =>
        {
            disk_full(target, message)
        }
        AdapterError::AccountDir { message, .. }
            if message.contains("(os error 13)") || message.contains("(os error 1)") =>
        {
            not_allowed(target)
        }
        AdapterError::AccountDir { message, .. } if message.contains("already has this file") => {
            format!(
                "{target} already has this conversation's files, so nothing was replaced and nothing moved. Open it on {target} instead."
            )
        }
        AdapterError::Unsupported(message) if message.contains("does not parse") => format!(
            "The original on {source} is still writing its reply, so nothing moved. Let it finish, or stop it, and try again."
        ),
        AdapterError::Unsupported(message) if message.contains("too large") => too_large,
        AdapterError::Unsupported(message) if message.contains("not found") => format!(
            "This conversation's file on {source} isn't there any more, so nothing moved. Resume it on {source} where you left it."
        ),
        _ => format!(
            "The switch to {target} stopped before anything moved. Try again, or resume it on {source} meanwhile; copy the details below for a bug report."
        ),
    }
}

/// Out of room: how much more it needs, when the check said.
fn disk_full(target: &str, message: &str) -> String {
    // The check says how much more, in decimal MB (as disks count it).
    let more = message
        .split("needs ")
        .nth(1)
        .and_then(|rest| rest.split(" MB").next())
        .and_then(|mb| mb.parse::<u64>().ok())
        .map(|mb| {
            if mb >= 1000 {
                format!(" It needs about {}.{} GB more.", mb / 1000, mb % 1000 / 100)
            } else {
                format!(" It needs about {mb} MB more.")
            }
        })
        .unwrap_or_default();
    format!(
        "There isn't enough free disk space to copy this conversation to {target}, so nothing moved; AI Juggler always leaves {} free.{more} Free some space, then try again.",
        juggler_adapter::handover::DISK_RESERVE_WORDS
    )
}

fn not_allowed(target: &str) -> String {
    format!(
        "AI Juggler wasn't allowed to write to {target}'s folder, so nothing moved. Check its folder's permissions, then try again."
    )
}

/// `12345` as `12,345`: every count the switch says (its failures here,
/// its dialog's words in the app).
#[must_use]
pub fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
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
    /// The target account's name (the words a warning names it by).
    pub target_account: String,
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

impl Mechanism {
    /// Its wire word (`native-resume`, `transplant`, `replay`, `pack`).
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::NativeResume => "native-resume",
            Self::Transplant => "transplant",
            Self::Replay => "replay",
            Self::PackOnly => "pack",
        }
    }
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
        return Err(SwitchFailure::Preflight(PreflightStop::TargetExhausted));
    }

    // 4. Version-drift parse check — abort (offer replay is E206's
    // door copy; preflight stops).
    if input.gates.version_drift {
        return Err(SwitchFailure::Preflight(PreflightStop::VersionDrift));
    }

    let mut warnings = Vec::new();
    // 3b. Tight quota → warn, never block.
    if let Some((left_pct, tte)) = input.target_quota_tight {
        warnings.push(CopySituation::TargetQuotaTight {
            account_b: input.target_account.clone(),
            left_pct,
            tte,
            account_c: "another account".to_owned(),
        });
    }
    // 5. Git/worktree: same-branch both-live warn + worktree offer.
    if input.git.same_branch_both_live {
        warnings.push(CopySituation::SameBranchParallel {
            instance_a: "This session".to_owned(),
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

/// Where a running switch is (0.3.9: the dialog showed a still list for
/// as long as a large copy took, and people couldn't tell it was working).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchStep {
    /// Copying the conversation's files.
    Copying,
    /// Checking the copy against what was carried.
    Checking,
    /// Starting it on the target.
    Starting,
    /// Checking the original is as it was, and recording the switch.
    Confirming,
}

impl SwitchStep {
    /// The wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Copying => "copying",
            Self::Checking => "checking",
            Self::Starting => "starting",
            Self::Confirming => "confirming",
        }
    }
}

/// Who hears a switch's steps: the app, which tells the window. One
/// listener for the process; set once at start-up.
static PROGRESS: std::sync::OnceLock<Box<dyn Fn(SwitchStep) + Send + Sync>> =
    std::sync::OnceLock::new();

/// Sets who hears every switch's steps (the app, once).
pub fn on_progress(listener: Box<dyn Fn(SwitchStep) + Send + Sync>) {
    let _ = PROGRESS.set(listener);
}

fn report(step: SwitchStep) {
    if let Some(listener) = PROGRESS.get() {
        listener(step);
    }
}

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
    execute_cut(support, reference, target, freeze, mechanism, None, launch)
}

/// [`execute`], with the start of the conversation left out as `cut`
/// says (D-76; `None` carries everything, byte for byte as `execute`).
///
/// # Errors
///
/// As [`execute`]; a cut the tool can't make fails at the Stage.
pub fn execute_cut(
    support: &dyn HandoverSupport,
    reference: &TranscriptRef,
    target: &Account,
    freeze: FreezeChoice,
    mechanism: Mechanism,
    cut: Option<juggler_adapter::handover::Cut>,
    launch: &mut LaunchHook<'_>,
) -> Result<(SwitchRecord, u32), SwitchFailure> {
    // AJ-E2912: the steps, for debug mode and Report a bug (target
    // `aij::flow`): words and numbers only, never what's in the
    // conversation.
    let started = std::time::Instant::now();
    tracing::info!(
        target: "aij::flow",
        flow = "switch",
        step = "start",
        mechanism = mechanism.as_wire(),
        to_tool = target.harness.as_str(),
        cut = ?cut,
    );
    let result = execute_steps(support, reference, target, freeze, mechanism, cut, launch);
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match &result {
        Ok((record, _)) => tracing::info!(
            target: "aij::flow",
            flow = "switch",
            step = "done",
            ms,
            fidelity = record.fidelity_label.as_str(),
        ),
        Err(failure) => tracing::info!(
            target: "aij::flow",
            flow = "switch",
            step = "failed",
            ms,
            stage = ?failure.stage(),
            code = failure.code().unwrap_or("-"),
            said = %failure,
        ),
    }
    result
}

/// Elapsed milliseconds since `at`.
fn ms_since(at: std::time::Instant) -> u64 {
    u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// The bundle (cut when asked), each file's manifest row, and, for a cut
/// file, the frozen whole source's size and hash the copy-never-move
/// check compares the source with.
type Bundled = (
    juggler_adapter::handover::SessionBundle,
    Vec<ManifestRow>,
    Vec<Option<(u64, String)>>,
);

fn bundled(
    support: &dyn HandoverSupport,
    reference: &TranscriptRef,
    cut: Option<juggler_adapter::handover::Cut>,
) -> Result<Bundled, SwitchFailure> {
    let bundle = match cut {
        Some(cut) => support.bundle_session_cut(reference, cut)?,
        None => support.bundle_session(reference)?,
    };
    // What the copy-never-move check reads: each file as carried, or, for
    // a cut one, the frozen whole source it was cut from.
    let cut_checks: Vec<Option<(u64, String)>> = bundle
        .files
        .iter()
        .map(|file| {
            file.cut_from
                .as_ref()
                .map(|whole| {
                    juggler_adapter::handover::sha256_file(whole, None).map(|(h, n)| (n, h))
                })
                .transpose()
        })
        .collect::<std::io::Result<_>>()
        .map_err(juggler_adapter::error::AdapterError::Io)?;
    let source_hashes: Vec<ManifestRow> = bundle
        .files
        .iter()
        .map(|file| {
            Ok(ManifestRow {
                rel_path: file.name.clone(),
                sha256: file.sha256()?,
                bytes: file.len()?,
                kind: "transcript".to_owned(),
                // D-76: its start left out (the manifest says so).
                disposition: if file.cut_from.is_some() {
                    "cut"
                } else {
                    "copy"
                }
                .to_owned(),
            })
        })
        .collect::<std::io::Result<_>>()
        .map_err(juggler_adapter::error::AdapterError::Io)?;
    Ok((bundle, source_hashes, cut_checks))
}

fn execute_steps(
    support: &dyn HandoverSupport,
    reference: &TranscriptRef,
    target: &Account,
    freeze: FreezeChoice,
    mechanism: Mechanism,
    cut: Option<juggler_adapter::handover::Cut>,
    launch: &mut LaunchHook<'_>,
) -> Result<(SwitchRecord, u32), SwitchFailure> {
    let _ = freeze; // the choice shaped the source state BEFORE this
    // call (turn boundary or stop ladder); the pack
    // records truncation via the caller's staging.
    report(SwitchStep::Copying);
    let at = std::time::Instant::now();
    let (bundle, source_hashes, cut_checks) = bundled(support, reference, cut)?;
    tracing::info!(
        target: "aij::flow",
        flow = "switch",
        step = "bundled",
        files = source_hashes.len(),
        bytes = source_hashes.iter().map(|r| r.bytes).sum::<u64>(),
        ms = ms_since(at),
    );
    // Stage + verify (atomic, fork-aware). An adapter failure here
    // cleaned its own temp (E204); nothing partial remains.
    let at = std::time::Instant::now();
    let install = support.install_bundle(&bundle, target)?;
    tracing::info!(
        target: "aij::flow",
        flow = "switch",
        step = "installed",
        forked = install.forked,
        fallback = install.target_dir_fallback,
        ms = ms_since(at),
    );
    report(SwitchStep::Checking);
    let installed_hashes = verify_installed(&install, &bundle, target);
    let all_present = !installed_hashes.iter().any(|row| row.sha256.is_empty());
    if !all_present {
        // S-F4 shape: verification failed after install — remove the
        // staged files (the temp-dir discipline's post-hoc guard).
        if install.placed.is_empty() {
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
        } else {
            for (_, path) in &install.placed {
                let _ = std::fs::remove_file(path);
            }
        }
        return Err(SwitchFailure::StagingAborted(
            "installed set did not verify".into(),
        ));
    }

    // Launch (the app's hook; never the engine's). S-F5 keeps the
    // verified stage — the caller retries from it.
    report(SwitchStep::Starting);
    let at = std::time::Instant::now();
    let pid = launch(&install).map_err(SwitchFailure::LaunchFailed)?;
    tracing::info!(target: "aij::flow", flow = "switch", step = "launched", ms = ms_since(at));
    report(SwitchStep::Confirming);

    // Copy-never-move, checked post-launch too: each file where the
    // adapter read it (a session's file history isn't beside the
    // transcript: found by the live lab of 2026-10-28, every such switch
    // failed here as "the source changed").
    // 0.3.9: a conversation still open in its app keeps writing at its
    // end, so the check is that the source still starts with exactly what
    // was carried (a transcript only grows): nothing earlier was changed
    // or moved.
    for (row, cut) in source_hashes.iter().zip(&cut_checks) {
        let (bytes, sha256) = cut
            .as_ref()
            .map_or((row.bytes, &row.sha256), |(n, h)| (*n, h));
        if !support.source_unchanged(reference, &row.rel_path, bytes, sha256) {
            return Err(SwitchFailure::StagingAborted(format!(
                "source changed during the switch: {}",
                row.rel_path
            )));
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
    target: &Account,
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
            // ADR-0028: where the install says it placed it, when it says.
            if let Some((_, placed)) = install.placed.iter().find(|(name, _)| *name == file.name) {
                let (sha256, bytes) =
                    juggler_adapter::handover::sha256_file(placed, None).unwrap_or_default();
                return ManifestRow {
                    rel_path: placed
                        .file_name()
                        .map_or_else(|| file.name.clone(), |n| n.to_string_lossy().into_owned()),
                    sha256,
                    bytes,
                    kind: "transcript".to_owned(),
                    disposition: if install.forked { "translate" } else { "copy" }.to_owned(),
                };
            }
            let name = if install.forked {
                file.name.replace(
                    &bundle.manifest.native_id,
                    install.reference.native_id.as_deref().unwrap_or_default(),
                )
            } else {
                file.name.clone()
            };
            // AJ-E1540: a file-history file lives in the account's own
            // `file-history/`, not beside the transcript.
            let path = juggler_adapter::handover::account_area_path(&target.state_dir, &name)
                .unwrap_or_else(|| dir.join(&name));
            // Streamed: an installed transcript can be very large.
            let (sha256, bytes) =
                juggler_adapter::handover::sha256_file(&path, None).unwrap_or_default();
            ManifestRow {
                rel_path: name,
                sha256,
                bytes,
                kind: "transcript".to_owned(),
                disposition: if install.forked { "translate" } else { "copy" }.to_owned(),
            }
        })
        .collect()
}
