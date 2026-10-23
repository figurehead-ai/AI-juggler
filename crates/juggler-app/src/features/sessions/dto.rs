//! The session read model's wire type (AJ-E401), moved here from
//! `commands.rs` (ADR-0019 §3).

use serde::Serialize;
use ts_rs::TS;

use crate::commands::{AttentionInfo, NeedsAttention, QuotaSnapshotInfo, TokenCountsInfo};

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
    /// I-08: how many subagents the session started are still running,
    /// read from its transcript (AJ-E1520); `null` until it has been read
    /// (not live, or no transcript yet) — never a guessed count.
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
}
