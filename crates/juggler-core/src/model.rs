//! Domain model (spec §3).
//!
//! These types are the vocabulary of the whole system; every layer above
//! storage speaks them. They are deliberately I/O-free and independent of any
//! specific harness.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

macro_rules! id_newtype {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Arc<str>);

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Ok(Self::new(value))
            }
        }

        impl $name {
            /// Creates the identifier from any string-like value.
            #[must_use]
            pub fn new(value: impl Into<Arc<str>>) -> Self {
                Self(value.into())
            }

            /// Views the identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.as_str())
            }
        }
    };
}

id_newtype! {
    /// Identifies a harness kind, e.g. `claude-code`, `codex`, `zcode`.
    HarnessId
}

/// ADR-0008 terminology: `Provider` is the user-facing noun for a tool
/// family. Additive alias — same type, same wire values, no migration.
pub type ProviderId = HarnessId;

impl HarnessId {
    /// Alias constructor matching the ADR-0008 vocabulary.
    #[must_use]
    pub fn provider(value: impl Into<std::sync::Arc<str>>) -> Self {
        Self::new(value)
    }
}
id_newtype! {
    /// Identifies a managed account (harness login) owned by the user.
    AccountId
}
id_newtype! {
    /// Identifies a Launch settings recipe (per-Account overlay used at
    /// launch). ADR-0008/D-10: "Profile" is demoted; alias retained.
    ProfileId
}
/// ADR-0008 vocabulary: canonical name for the launch-settings id.
pub type LaunchRecipeId = ProfileId;
id_newtype! {
    /// Identifies a supervised session, as tracked by AI Juggler.
    SessionId
}
id_newtype! {
    /// Identifies a machine in the fleet (`docs/SPEC.md` §11); the local
    /// machine uses a stable default id.
    MachineId
}
id_newtype! {
    /// Identifies a tracked local project (directory) — the juggling
    /// anchor (ADR-0007).
    ProjectId
}
id_newtype! {
    /// The *native* conversation identifier inside an Instance
    /// The native conversation id of a Provider session. See amendment A-1.
    ConversationId
}

/// Unix epoch milliseconds — the crate's canonical timestamp representation.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Current wall-clock time; falls back to the epoch if the system clock
    /// is before 1970, and saturates if it is absurdly far in the future.
    #[must_use]
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or_default();
        Self(millis)
    }

    /// Creates a timestamp from explicit epoch milliseconds.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Returns epoch milliseconds.
    #[must_use]
    pub const fn millis(self) -> i64 {
        self.0
    }
}

/// A vendor plan tier label, e.g. `max-20x`, `plus`, `glm-pro`.
///
/// Deliberately an opaque string: tiers are vendor-specific and churn often
/// (see the research notes); the usage engine interprets them per harness.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlanTier(String);

impl PlanTier {
    /// Creates a plan tier label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// Views the label as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlanTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A harness login the user owns, materialized as an isolated state directory
/// managed by AI Juggler (`docs/SPEC.md` §3).
#[derive(Clone, PartialEq, Debug)]
pub struct Account {
    /// Stable identifier.
    pub id: AccountId,
    /// Which harness this account logs into.
    pub harness: HarnessId,
    /// Human label, e.g. `Work Max`.
    pub label: String,
    /// Vendor plan tier, if known.
    pub plan: Option<PlanTier>,
    /// The isolated harness state directory AI Juggler manages for this
    /// account (`CLAUDE_CONFIG_DIR` / `CODEX_HOME` / `HOME` root).
    pub state_dir: PathBuf,
    /// D-08 Tier-1 "clean safe" opt-in: when true, leftovers verified as
    /// journal-recorded descendants are stopped automatically 60 s after
    /// the Instance ends (every kill audit-logged). Off by default —
    /// the default tier is notify-only.
    pub cleanup_opt_in: bool,
    /// Optional user-entered renewal date, local calendar `YYYY-MM-DD`
    /// (D-12: never inferred; `None` renders `Expiry: unknown`).
    pub renewal_date: Option<String>,
    /// AJ-E415 per-Account live-preview switch (Req.3): `ScreenCaptureKit`
    /// mirror of a desktop Instance's window. Off by default — enabling
    /// it is the user's explicit Screen-Recording consent moment.
    pub preview_opt_in: bool,
    /// AJ-E307 Req.1: import provenance — the source's CANONICAL path
    /// (symlinks resolved at import time). `None` = provenance-unknown
    /// (pre-E307 accounts; the migration backfills nothing, never
    /// guesses).
    pub import_source_path: Option<String>,
    /// The discovery origin: `cc-switch` | `t3` | `alias` |
    /// `home-pattern` | `manual`. `None` = provenance-unknown.
    pub import_origin: Option<String>,
    /// Import completion time, epoch ms. `None` = never imported.
    pub imported_at: Option<i64>,
}

/// Settings applied on top of an account's defaults when launching a profile.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SettingsOverlay {
    /// Preferred model identifier, harness-interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Extra environment variables injected into the launched process.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// Launch settings: a per-Account overlay applied when an Instance
/// starts (`docs/SPEC.md` §3; ADR-0008 D-10 — "Profile" demoted).
#[derive(Clone, PartialEq, Debug)]
pub struct LaunchRecipe {
    /// Stable identifier.
    pub id: LaunchRecipeId,
    /// Human label for this settings set (e.g. `Opus pairing`).
    pub name: String,
    /// The Account this recipe starts under.
    pub account: AccountId,
    /// Settings layered over the Account defaults.
    pub settings_overlay: SettingsOverlay,
    /// Working directory used when the user does not pick one.
    pub default_cwd: Option<PathBuf>,
}

/// ADR-0008: old name kept one minor version as a pure alias.
pub type Profile = LaunchRecipe;

/// Lifecycle state of a supervised session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionStatus {
    /// Process is starting up.
    Starting,
    /// Healthy and doing work.
    Running,
    /// Alive but blocked on a user decision (e.g. permission prompt).
    WaitingUser,
    /// Suspected hung (heuristic trip; `docs/SPEC.md` §5.1).
    Stuck,
    /// Exited after hitting an account quota wall.
    QuotaBlocked,
    /// Completed normally.
    Finished,
    /// Stopped by the user (our stop ladder) — not a failure.
    Stopped,
    /// Exited abnormally.
    Failed,
    /// Work continued elsewhere via handover; resumable.
    HandedOver,
}

impl SessionStatus {
    /// All variants in canonical order.
    #[must_use]
    pub const fn all() -> [SessionStatus; 9] {
        [
            SessionStatus::Starting,
            SessionStatus::Running,
            SessionStatus::WaitingUser,
            SessionStatus::Stuck,
            SessionStatus::QuotaBlocked,
            SessionStatus::Finished,
            SessionStatus::Stopped,
            SessionStatus::Failed,
            SessionStatus::HandedOver,
        ]
    }

    /// Stable machine-readable name (storage format).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Starting => "starting",
            SessionStatus::Running => "running",
            SessionStatus::WaitingUser => "waiting_user",
            SessionStatus::Stuck => "stuck",
            SessionStatus::QuotaBlocked => "quota_blocked",
            SessionStatus::Finished => "finished",
            SessionStatus::Stopped => "stopped",
            SessionStatus::Failed => "failed",
            SessionStatus::HandedOver => "handed_over",
        }
    }

    /// Parses a stored status name; returns `None` for unknown values
    /// (unknown statuses must degrade, never crash — QUALITY.md §1).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "starting" => Some(SessionStatus::Starting),
            "running" => Some(SessionStatus::Running),
            "waiting_user" => Some(SessionStatus::WaitingUser),
            "stuck" => Some(SessionStatus::Stuck),
            "quota_blocked" => Some(SessionStatus::QuotaBlocked),
            "finished" => Some(SessionStatus::Finished),
            "stopped" => Some(SessionStatus::Stopped),
            "failed" => Some(SessionStatus::Failed),
            "handed_over" => Some(SessionStatus::HandedOver),
            _ => None,
        }
    }
}

/// Token counters for a session snapshot (`docs/SPEC.md` §6).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TokenCounts {
    /// Prompt tokens consumed.
    pub input: u64,
    /// Completion tokens consumed (thinking included, billed as output).
    pub output: u64,
    /// Tokens served from prompt cache.
    pub cache_read: u64,
    /// Tokens written to the prompt cache.
    pub cache_write: u64,
    /// Reasoning tokens (subset semantics per harness; kept separate for
    /// analytics).
    pub reasoning: u64,
}

/// A vendor quota window state at a point in time (5-hour or weekly pool).
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct QuotaWindow {
    /// Percent of the window's allowance already used (0–100).
    pub used_percentage: f64,
    /// When the window resets, if known.
    pub resets_at: Option<Timestamp>,
}

/// How trustworthy a usage number is (`docs/SPEC.md` §6 honesty rule).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Confidence {
    /// Observed directly from a vendor-provided feed.
    Measured,
    /// Deterministically computed from local data and published formulas.
    Computed,
    /// Inferred from token activity; approximate by nature. The default:
    /// never overstate trust.
    #[default]
    Estimated,
}

/// Usage state attached to a session record.
#[derive(Clone, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct UsageSnapshot {
    /// Token counters.
    pub tokens: TokenCounts,
    /// Known quota window state, if any feed exists for the harness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<QuotaWindow>,
    /// Trust level of `window`, if present.
    pub confidence: Confidence,
}

/// Why an Instance needs its user (AJ-E405, SPEC §5.1 heuristics): the
/// rule that tripped, the measured inputs that fed it, and when the
/// condition began. The UI renders this verbatim — it never re-derives
/// the reason from the status alone.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct AttentionReason {
    /// Rule id: `stuck_no_output` | `waiting_prompt` | `quota_exit`.
    pub rule: String,
    /// Human-readable measured inputs (`no output 6.2m`, `cpu 0.3%`).
    pub inputs: Vec<String>,
    /// When the condition began, epoch milliseconds.
    #[serde(default)]
    pub since_ms: i64,
}

/// Points at a harness-native transcript on disk (owned by the harness, read
/// only by AI Juggler). The owning harness is implied by the profile's
/// account.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct TranscriptRef {
    /// Absolute path to the native transcript/rollout file.
    pub path: PathBuf,
    /// The harness's own session identifier, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_id: Option<String>,
}

/// What kind of harness occurrence an instance is (ADR-0007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstanceKind {
    /// A CLI harness running in an embedded terminal under an isolated
    /// environment.
    #[default]
    Terminal,
    /// A desktop-class harness running as a windowed application
    /// (supervised process, no terminal surface).
    Desktop,
}

impl InstanceKind {
    /// Wire name (`terminal` / `desktop`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Desktop => "desktop",
        }
    }

    /// Parses a stored wire name; unknown values yield `None` (degrade,
    /// never panic).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "terminal" => Some(Self::Terminal),
            "desktop" => Some(Self::Desktop),
            _ => None,
        }
    }
}

/// Imported history enrichment for a project (parsed from transcripts that
/// predate AI Juggler — e.g. t3.codes profiles).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectHistory {
    /// Account labels seen working in this project before AI Juggler.
    pub accounts: Vec<String>,
    /// Native sessions discovered in imported transcripts.
    pub native_sessions: u64,
}

/// A tracked local project — the juggling anchor (ADR-0007): the
/// directory instances work in, keyed by canonical root so account
/// switches and handovers have a stable pivot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    /// Stable identifier assigned by AI Juggler.
    pub id: ProjectId,
    /// Canonical absolute root directory.
    pub root: PathBuf,
    /// Display name (directory name unless the user renames).
    pub name: String,
    /// When the project was first seen.
    pub created_at: Timestamp,
    /// Last time an instance ran in this project.
    pub last_activity: Option<Timestamp>,
    /// Pre-AI-Juggler history parsed from imported transcripts (v4).
    pub history: ProjectHistory,
}

/// ADR-0008 alias: wire strings unchanged; naming only.
pub type InstanceStatus = SessionStatus;

id_newtype! {
    /// Identifies a Work item — the user's continuing task on one
    /// project (AJ-E201; pass2/02 §2.1). Survives every switch.
    WorkItemId
}
id_newtype! {
    /// Identifies one instance-epoch node in the Switch Graph: one
    /// Instance running under one Account from launch/resume until it
    /// stops, freezes, or hands off (AJ-E201).
    EpochId
}
id_newtype! {
    /// Identifies one executed switch — an edge in the Switch Graph
    /// (AJ-E201).
    SwitchEdgeId
}
id_newtype! {
    /// Identifies a portable context pack (AJ-E202; referenced by
    /// edges so lineage and fidelity evidence always exist).
    PackId
}

/// The user's continuing task on one project (pass2/02 §2.1). The
/// juggling anchor is the `ProjectRecord`; Instances come and go
/// under the Work item, which survives every switch.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkItem {
    /// Stable identifier assigned by AI Juggler.
    pub id: WorkItemId,
    /// The one project this work belongs to (Invariant S-G2: the
    /// graph is per-project; cross-project moves are T4 exports,
    /// never edges).
    pub project: ProjectId,
    /// Human-readable task title.
    pub title: String,
    /// When the work item was created.
    pub created_at: Timestamp,
}

/// One instance-epoch node in the Switch Graph: an Instance running
/// under one Account from launch/resume until it stops, is frozen, or
/// hands off. Resuming the same native session after switch-back
/// creates a NEW epoch whose `resumed_from_epoch` points at the
/// original and whose `native_id` equals the original's — that
/// pointer is what makes switch-back a resume, not a copy.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Epoch {
    /// Stable identifier assigned by AI Juggler.
    pub id: EpochId,
    /// The work item this epoch belongs to.
    pub work: WorkItemId,
    /// The AI-Juggler Instance this epoch ran in, once launched
    /// (`None` while staged — never fabricated).
    pub session_ref: Option<SessionId>,
    /// The vendor-native conversation id (Claude session uuid, Codex
    /// rollout id, `zcode` session id), when discovered.
    pub native_id: Option<String>,
    /// The account this epoch ran under.
    pub account: AccountId,
    /// The provider this epoch ran under.
    pub provider: HarnessId,
    /// The original epoch this one resumes (`None` for first
    /// launch; set on native-resume after switch-back, Req.5).
    pub resumed_from_epoch: Option<EpochId>,
}

impl Epoch {
    /// Whether this node can be resumed natively (Req.6: every epoch
    /// with a native session exposes Resume — `HandedOver` is
    /// resting, not terminal).
    #[must_use]
    pub fn can_resume(&self) -> bool {
        self.native_id.is_some()
    }
}

/// How one switch physically moved the work (pass2/02 §3; T-codes
/// stay engine-internal per deep/09 D2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum SwitchMechanism {
    /// T1: same-account native resume (`claude --resume` etc.).
    NativeResume,
    /// T2: same-provider, different-account transcript transplant.
    Transplant,
    /// T3: cross-provider replay from the pack.
    Replay,
    /// T4: pack export/priming only.
    Pack,
}

impl SwitchMechanism {
    /// Wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeResume => "native-resume",
            Self::Transplant => "transplant",
            Self::Replay => "replay",
            Self::Pack => "pack",
        }
    }

    /// Parses a stored wire name; unknown values yield `None`
    /// (degrade, never panic).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "native-resume" => Some(Self::NativeResume),
            "transplant" => Some(Self::Transplant),
            "replay" => Some(Self::Replay),
            "pack" => Some(Self::Pack),
            _ => None,
        }
    }
}

/// Which pipeline stage a switch failed in (pass2/02 §5.1; the
/// failure taxonomy S-F1..S-F9 maps onto these stages in AJ-E205).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum FailureStage {
    /// Target/mechanism resolution failed (S-F1).
    Resolve,
    /// Preflight gate failed (score, quota, drift, git state).
    Preflight,
    /// Freeze failed or was aborted.
    Freeze,
    /// Snapshot/bundle of the source failed.
    Snapshot,
    /// Staging (temp dir write/verify) failed.
    Stage,
    /// Manifest verification failed.
    Verify,
    /// Target launch failed (staged artifacts retained for retry).
    Launch,
    /// Post-launch confirmation failed.
    Confirm,
}

impl FailureStage {
    /// Wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Resolve => "resolve",
            Self::Preflight => "preflight",
            Self::Freeze => "freeze",
            Self::Snapshot => "snapshot",
            Self::Stage => "stage",
            Self::Verify => "verify",
            Self::Launch => "launch",
            Self::Confirm => "confirm",
        }
    }

    /// Parses a stored wire name; unknown values yield `None`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "resolve" => Some(Self::Resolve),
            "preflight" => Some(Self::Preflight),
            "freeze" => Some(Self::Freeze),
            "snapshot" => Some(Self::Snapshot),
            "stage" => Some(Self::Stage),
            "verify" => Some(Self::Verify),
            "launch" => Some(Self::Launch),
            "confirm" => Some(Self::Confirm),
            _ => None,
        }
    }
}

/// Edge lifecycle (pass2/02 §9: Staged → Verified → Launched →
/// Confirmed, or Failed{stage,reason}). Exhaustive by construction —
/// E209 renders it without inventing states.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum EdgeState {
    /// Artifacts staged + verified, awaiting launch.
    Staged,
    /// Manifest verified against staged bytes.
    Verified,
    /// Target launched; `to_epoch` is set at this transition.
    Launched,
    /// Target confirmed alive and continuing the work.
    Confirmed,
    /// The switch failed at `stage`; the source is exactly resumable
    /// and staged artifacts are retained per AJ-E205.
    Failed {
        /// Which pipeline stage failed.
        stage: FailureStage,
        /// Plain-language reason (user copy comes from §7.1).
        reason: String,
    },
}

impl EdgeState {
    /// Wire name (`staged|verified|launched|confirmed|failed`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Verified => "verified",
            Self::Launched => "launched",
            Self::Confirmed => "confirmed",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Who executed a switch (every edge is user-clicked; the engine
/// only prepares — ADR-0003/SPEC:26).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum EdgeActor {
    /// An explicit user action.
    User,
    /// Engine bookkeeping (e.g. a compensating edge for undo, which
    /// is itself the result of a user click).
    Engine,
}

impl EdgeActor {
    /// Wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Engine => "engine",
        }
    }

    /// Parses a stored wire name; unknown values yield `None`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "user" => Some(Self::User),
            "engine" => Some(Self::Engine),
            _ => None,
        }
    }
}

/// One executed switch — an edge in the Switch Graph (pass2/02
/// §2.1). `to_epoch` stays `None` until launch succeeds; edges are
/// append-only (Invariant S-G1): undo/failure/merge insert
/// compensating edges, never rewrite history. Merges carry two
/// parents.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct SwitchEdge {
    /// Stable identifier assigned by AI Juggler.
    pub id: SwitchEdgeId,
    /// The work item this edge belongs to.
    pub work: WorkItemId,
    /// The epoch the work moved from.
    pub from_epoch: EpochId,
    /// The epoch the work moved to (`None` until launch succeeds).
    pub to_epoch: Option<EpochId>,
    /// Account moved from.
    pub from_account: AccountId,
    /// Account moved to.
    pub to_account: AccountId,
    /// Provider moved from.
    pub from_provider: HarnessId,
    /// Provider moved to.
    pub to_provider: HarnessId,
    /// How the work physically moved.
    pub mechanism: SwitchMechanism,
    /// The pack this edge staged (manifest-only for native-resume —
    /// lineage and fidelity evidence always exist).
    pub pack_id: Option<PackId>,
    /// The fidelity label shown to the user at execution (F0–F4,
    /// AJ-E203; `None` pre-computation).
    pub fidelity_shown: Option<String>,
    /// sha256 over the edge's artifact manifest (AJ-E202).
    pub manifest_sha256: Option<String>,
    /// Lifecycle state.
    pub state: EdgeState,
    /// Who executed it.
    pub actor: EdgeActor,
    /// First parent edge (lineage; the incoming edge most recently
    /// executed against `from_epoch`).
    pub parent_edge_a: Option<SwitchEdgeId>,
    /// Second parent edge — set ONLY on merge edges (two-away-parent
    /// reconciliation, AJ-E207).
    pub parent_edge_b: Option<SwitchEdgeId>,
    /// When the edge was created.
    pub created_at: Timestamp,
}

/// One artifact manifest row attached to an edge (AJ-E202): every
/// file the switch touched (or explicitly dropped), hashed and
/// dispositioned — the carry-list's evidence base.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestEntry {
    /// Path relative to the pack root.
    pub rel_path: String,
    /// sha256 hex of the file's bytes.
    pub sha256: String,
    /// Size in bytes.
    pub bytes: u64,
    /// Artifact kind (`conversation.md`, `rules`, `diffs`, …).
    pub kind: String,
    /// Disposition: `copy|translate|regenerate|drop`.
    pub disposition: String,
}

/// A supervised Instance as tracked by AI Juggler (`docs/SPEC.md` §3,
/// ADR-0007/0008). The conversation lives *inside*: see `conversation`.
#[derive(Clone, PartialEq, Debug)]
pub struct InstanceRecord {
    /// Stable identifier assigned by AI Juggler (the *Instance* id;
    /// supervisor/journal key — ADR-0008 A-1).
    pub id: SessionId,
    /// The native conversation inside this Instance, once discovered
    /// (None while Starting — never a fabricated id).
    pub conversation: Option<ConversationId>,
    /// Profile the session runs under.
    pub profile: ProfileId,
    /// Machine the session runs on (fleet dimension; local default
    /// otherwise).
    pub machine: MachineId,
    /// The kind of harness occurrence (terminal CLI vs desktop window).
    pub kind: InstanceKind,
    /// The tracked project this session's cwd belongs to (juggling
    /// anchor, ADR-0007).
    pub project: Option<ProjectId>,
    /// Working directory of the session.
    pub cwd: PathBuf,
    /// Current lifecycle status.
    pub status: SessionStatus,
    /// When the session started.
    pub started_at: Timestamp,
    /// Last observed activity, if any.
    pub last_activity: Option<Timestamp>,
    /// Latest usage snapshot.
    pub usage: UsageSnapshot,
    /// Where the native transcript lives, once discovered.
    pub transcript_ref: Option<TranscriptRef>,
    /// Why this Instance needs its user (AJ-E405 heuristics); `None`
    /// while healthy. (Stored as its own JSON blob — see storage v6.)
    pub attention: Option<AttentionReason>,
    /// The model this Instance is actually running, once OBSERVED from
    /// the transcript feed (AJ-E412); `None` = not observed yet —
    /// never filled from the launch overlay. (Storage v9 JSON blob.)
    pub running: Option<RunningModel>,
    /// The desktop window identity for this Instance (AJ-E414 Req.2),
    /// recorded at launch (owning PID + bundle id) and enriched at
    /// first sight / each focus attempt (window title; `CGWindow` id
    /// where a source exists). `None` for terminal Instances — never
    /// guessed. (Storage v10 JSON blob.)
    pub window: Option<WindowIdentity>,
}

/// The exact window a desktop Instance owns (AJ-E414): focus matches
/// THIS window, not just the app. Re-discovered on every focus attempt;
/// stale identities are re-enumerated, never assumed.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct WindowIdentity {
    /// The OS process id that owns the window.
    pub pid: u32,
    /// Bundle identifier (macOS), when known.
    pub bundle_id: Option<String>,
    /// Window title at discovery time — the AX match key. `None` until
    /// first sight.
    pub title: Option<String>,
    /// `CGWindow` id, where a source exists for it (none today — the
    /// `AppleScript` enumeration exposes titles only, and CoreGraphics
    /// FFI is barred by the workspace `unsafe_code = "forbid"` law).
    /// Never fabricated.
    pub cgwindow_id: Option<u64>,
}

/// ADR-0008: old name kept one minor version as a pure alias.
pub type SessionRecord = InstanceRecord;

/// What an Instance is actually running right now (AJ-E412) — an
/// OBSERVED value from the transcript feed, never the launch intent
/// (D-06 honest fields: `SettingsOverlay.model` stays intent).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunningModel {
    /// Verbatim vendor model identifier (never normalised).
    pub model: String,
    /// Where it was observed.
    pub source: ModelSource,
}

/// The observation channel for a running model (wire vocabulary:
/// `launch | in-session | transcript | unknown`; the `launch` and
/// `unknown` classifications are computed at read time from absence —
/// only real observations are stored).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ModelSource {
    /// Observed in the transcript and equal to the launch overlay.
    Transcript,
    /// Observed in the transcript and different from the launch
    /// overlay (an in-session `/model` change).
    InSession,
}

impl ModelSource {
    /// Wire name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::InSession => "in-session",
        }
    }
}

impl InstanceRecord {
    /// Launch-settings accessor (field rename lands with AJ-E105).
    #[must_use]
    pub fn launch_settings(&self) -> &ProfileId {
        &self.profile
    }
}
