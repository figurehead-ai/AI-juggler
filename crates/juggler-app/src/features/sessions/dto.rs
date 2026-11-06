//! The session read model's wire type (AJ-E401), moved here from
//! `commands.rs` (ADR-0019 §3).

use serde::Serialize;
use ts_rs::TS;

/// An Instance row for the UI (the conversation lives inside; AJ-E107).
/// One payload carries every indicator-row and inspector field I-01..I-11
/// (AJ-E401): absent data is `null`, never `0`, `""`, or a guessed name.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceInfo {
    /// Instance id.
    pub id: String,
    /// Launch settings id it runs under.
    pub launch_recipe_id: String,
    /// Provider id (wire field `harness` aliased one version); `unknown`
    /// when the launch settings row is gone — never blank.
    #[serde(alias = "harness")]
    pub provider: String,
    /// Owning account id; `null` when the launch settings row is gone.
    pub account_id: Option<String>,
    /// Owning Account label for display (wire field `label` aliased one
    /// version); `unknown` when unresolvable — never blank.
    #[serde(alias = "label")]
    pub account_label: String,
    /// I-03 display value: the RUNNING model once observed, else the
    /// launch overlay, else `default model` — never a guessed name.
    pub model: Option<String>,
    /// The launch-overlay model (intent, not telemetry; AJ-E412).
    pub launch_model: Option<String>,
    /// The transcript-OBSERVED running model; `null` until observed —
    /// never filled from the overlay.
    pub running_model: Option<String>,
    /// How `model` is known: `launch` (overlay only, not verified
    /// live), `in-session` (observed, differs from the overlay),
    /// `transcript` (observed), `unknown` (neither).
    pub model_source: String,
    /// I-04: lifecycle status string (storage vocabulary).
    pub status: String,
    /// I-05: quota window snapshot; `null` when no feed exists.
    pub quota: Option<QuotaSnapshotInfo>,
    /// I-06: token counters; `null` until usage has been observed.
    pub tokens: Option<TokenCountsInfo>,
    /// I-07: CPU percent summed over the process tree; `null` until the
    /// resource tick has two samples.
    pub cpu_pct: Option<f64>,
    /// I-07: resident bytes summed over the process tree; `null` until
    /// sampled (a finished Instance is not measurable — not zero).
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// I-08: live descendant count; `null` until sampled, `0` after exit
    /// (nothing can be running then).
    #[ts(type = "number | null")]
    pub child_count: Option<u32>,
    /// I-08: how many subagents the session started (and theirs) are
    /// still running, as its tool's adapter reads them — the same reader
    /// as its Subagents tab (AJ-E1520, `features::subagents::count`);
    /// `null` until they have been read (not live, its tool records none
    /// AI Juggler reads, or not read yet) — never a guessed count.
    #[ts(type = "number | null")]
    pub subtask_count: Option<u64>,
    /// I-09: git branch of the working directory, parsed from
    /// `.git/HEAD` (no subprocess); `null` for non-git or detached.
    pub git_branch: Option<String>,
    /// I-09: dirty file count; `null` — requires a `git status` walk that
    /// must never run per-row on the hot path (collector comes later).
    #[ts(type = "number | null")]
    pub git_dirty_count: Option<u64>,
    /// I-10: why this Instance needs its user; `null` when it does not.
    pub needs_attention: Option<NeedsAttention>,
    /// AJ-E405: the structured reason (rule + measured inputs + since)
    /// behind `needs_attention`; `null` when healthy or pre-E405 rows.
    pub attention: Option<AttentionInfo>,
    /// Start time, epoch milliseconds.
    #[ts(type = "number")]
    pub started_at_ms: i64,
    /// I-11: last observed activity, epoch milliseconds; `null` when
    /// never observed.
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// Fleet dimension (D-13): machine the Instance runs on.
    pub machine_id: String,
    /// Working directory (AJ-E407 Overview: copyable project + cwd).
    pub cwd: String,
    /// Instance kind: `terminal` | `desktop`.
    pub kind: String,
    /// True when crash recovery re-adopted this Instance's verified
    /// root (AJ-E510): `Running — picked up after restart`, terminal detached
    /// (output from before the crash is unavailable), stop via the
    /// AJ-E507 sheet.
    pub adopted: bool,
    /// Owning project id, when the cwd resolved to a tracked project.
    pub project_id: Option<String>,
    /// The native conversation inside this Instance, once discovered
    /// (AJ-E104: Instance row → conversation inside; None while Starting).
    pub conversation_id: Option<String>,
    /// AJ-E413 Req.1: the per-Provider isolation claim for desktop
    /// Instances, verbatim (`Single-account app — Juggler tracks,
    /// cannot isolate` for single-account apps); `null` for terminal
    /// Instances and when the provider's adapter is not registered.
    pub isolation_label: Option<String>,
    /// AJ-E1001: the latest identity assertion — `verified` |
    /// `first_seen` | `signed_out` | `unverified` (absent before any
    /// check, e.g. adopted after a crash).
    #[ts(optional)]
    pub identity_state: Option<String>,
    /// The identity it ran as (masked), when known.
    #[ts(optional)]
    pub identity_label: Option<String>,
    /// AJ-E1002: commits ahead of the upstream (cached `git status`,
    /// at most every 30 s; absent = unknown).
    #[ts(optional, type = "number")]
    pub git_ahead: Option<u64>,
    /// Commits behind the upstream.
    #[ts(optional, type = "number")]
    pub git_behind: Option<u64>,
    /// It runs in a linked git worktree.
    #[ts(optional)]
    pub git_worktree: Option<bool>,
    /// AJ-E1003: parked until its quota resets — the reset's
    /// confidence (`measured` | `estimated` | `unknown`); absent when not
    /// parked.
    #[ts(optional)]
    pub parked: Option<String>,
    /// The reset it waits for, epoch ms (countdown only when measured).
    #[ts(optional, type = "number")]
    pub parked_resets_at_ms: Option<i64>,
    /// AJ-E1005/AJ-E1007: its latest lifecycle step (`reviewed`), absent
    /// when it has none.
    #[ts(optional)]
    pub lifecycle: Option<String>,
    /// When that step was taken, epoch ms.
    #[ts(optional, type = "number")]
    pub lifecycle_at_ms: Option<i64>,
    /// AJ-E1502: the user's name for it, shown over its own title.
    #[ts(optional)]
    pub label: Option<String>,
    /// AJ-E1536: its conversation's title (the tool's summary, else the
    /// first thing asked), from the conversation index; `null` until
    /// that's read. Shown when the user hasn't named it.
    #[ts(optional)]
    pub title: Option<String>,
    /// AJ-E1502: pinned to the top of its group in the rail.
    #[ts(optional)]
    pub pinned: Option<bool>,
    /// AJ-E2302 (D-40): how it runs and its exact state, when its runtime
    /// says more than its process does (`features::runtime::described`).
    #[ts(optional)]
    pub runtime: Option<crate::features::runtime::dto::RuntimeInfo>,
}

/// I-05: quota window snapshot (SPEC §6). `null` when no feed exists for
/// the Provider — never a fabricated percentage (AJ-E401 honesty contract).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QuotaSnapshotInfo {
    /// Percent of the window's allowance already used (0–100).
    pub used_percentage: f64,
    /// When the window resets, epoch milliseconds, if known.
    #[ts(type = "number | null")]
    pub resets_at_ms: Option<i64>,
    /// Trust level of the number: `measured` | `computed` | `estimated`.
    pub confidence: String,
}

/// I-06: token counters (SPEC §6). `null` until usage has actually been
/// observed — a never-sampled Instance reports no numbers, not zeros.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TokenCountsInfo {
    /// Prompt tokens consumed.
    #[ts(type = "number")]
    pub input: u64,
    /// Completion tokens consumed (thinking included, billed as output).
    #[ts(type = "number")]
    pub output: u64,
    /// Tokens served from the prompt cache.
    #[ts(type = "number")]
    pub cache_read: u64,
    /// Tokens written to the prompt cache.
    #[ts(type = "number")]
    pub cache_write: u64,
    /// Reasoning tokens (subset semantics per harness; kept separate for
    /// analytics).
    #[ts(type = "number")]
    pub reasoning: u64,
}

/// AJ-E405: the structured attention reason carried on the row — the
/// UI renders it verbatim (rule + measured inputs + since); it never
/// re-derives the reason from the status alone.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AttentionInfo {
    /// Rule id: `stuck_no_output` | `waiting_prompt` | `quota_exit` | `limit_hit` | `tool_asks`.
    pub rule: String,
    /// Human-readable measured inputs (`no output 6m`, `cpu 0.3%`).
    pub inputs: Vec<String>,
    /// When the condition began, epoch milliseconds.
    #[ts(type = "number")]
    pub since_ms: i64,
}

/// AJ-E409: the Quality/Performance signal set (D-07) — individually
/// labelled signals with tiers; NO composite score exists (grep gate).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct QualitySignalsInfo {
    /// tokens/min: five 1-min bars (oldest first) or `null` = no data.
    #[ts(type = "Array<number> | null")]
    pub tokens_bars: Option<Vec<u64>>,
    /// Trailing-5-min average tokens/min (a real 0 when idle).
    #[ts(type = "number")]
    pub avg_tokens_per_min: f64,
    /// `2 errors / 41 turns` (exact rendered string) or the
    /// provider-specific not-measurable reason.
    pub error_label: String,
    /// True when the provider has no structured error feed.
    pub errors_not_measurable: bool,
    /// Context tokens in use, when the provider reports them.
    #[ts(type = "number | null")]
    pub context_used: Option<u64>,
    /// Provider-reported context window size, when known.
    #[ts(type = "number | null")]
    pub context_window: Option<u64>,
    /// Cumulative session tokens (labelled `cumulative, not context`).
    pub cumulative: TokenCountsInfo,
    /// Session cost USD from the versioned pricing table (`Est ·
    /// value-equiv.`); `null` when the model has no pricing row.
    #[ts(type = "number | null")]
    pub session_cost_usd: Option<f64>,
    /// The model the cost was priced against (null = unpriced).
    pub priced_model: Option<String>,
    /// Quota forecast RANGE label (`At this pace, its limit lasts about
    /// 13–27 min more`) or `Not using its limit right now`.
    pub quota_forecast: String,
    /// The AJ-E405 stuck rule + inputs, when tripped (permanent `?`).
    pub stuck: Option<AttentionInfo>,
    /// AJ-E2903: times it went probably stuck (AI Juggler's recorded states).
    pub stuck_episodes: u32,
    /// AJ-E2903: its context at the end, as kept when it ended.
    #[ts(type = "[number, number] | null")]
    pub end_context: Option<(u64, u64)>,
}

/// I-10: why an Instance needs its user (`null` when it does not).
/// Derived from the lifecycle status here; AJ-E405 replaces the
/// derivation with structured heuristic reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum NeedsAttention {
    /// A permission/wait prompt is alive.
    WaitingUser,
    /// No-output heuristics tripped (rendered `Stuck?` — heuristic).
    Stuck,
    /// Rate-limit signature on exit.
    QuotaBlocked,
    /// Non-zero, non-user exit.
    Failed,
}

/// I-07/I-08 delta payload of the 5 s resource tick (`session://resources`,
/// AJ-E401 Req.4). One emission per tick carries every live Instance —
/// coalesced by construction.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceResourcesInfo {
    /// Instance id.
    pub id: String,
    /// CPU utilization summed over the process tree, percent; `null`
    /// until two table samples exist.
    pub cpu_pct: Option<f64>,
    /// Resident memory summed over the process tree, bytes; `null`
    /// until sampled.
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// Live descendant count; `null` until sampled.
    #[ts(type = "number | null")]
    pub child_count: Option<u32>,
}

/// I-05 delta payload of the 30 s quota tick (`session://quota`,
/// AJ-E401 Req.4). `quota` is `null` while no feed exists for the
/// Provider (the two-clock engine, AJ-E606+, populates it later).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstanceQuotaInfo {
    /// Instance id.
    pub id: String,
    /// Quota window snapshot, or `null` when unknown.
    pub quota: Option<QuotaSnapshotInfo>,
}
