//! AJ-E1004: the morning summary — what happened while the user was away.
//!
//! One surface, severity-ordered (crashed → needs you → still running →
//! finished → ended), built from the Instance status journal (every
//! transition timestamped in storage — read, never reconstructed) plus the
//! E5 cleanup inventory's leftovers. Cost is shown only where it is
//! measured; otherwise the row says Unavailable (D-06/D-07). No composite
//! "health" score.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use juggler_core::model::{SessionId, Timestamp};

use crate::error::AppResult;
use crate::state::AppState;

/// The app setting holding when the user last looked at the summary.
const SEEN_KEY: &str = "summary_seen_at_ms";
/// Without a last look, the summary covers this far back.
const DEFAULT_WINDOW_MS: i64 = 12 * 3_600_000;

/// One status change in a row's history.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TransitionInfo {
    /// The previous status (`null` for the first).
    pub from: Option<String>,
    /// The new status.
    pub to: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// One Instance in the summary.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SummaryRowInfo {
    /// The Instance.
    pub session_id: String,
    /// Its Account.
    pub account_label: String,
    /// Its Provider.
    pub provider: String,
    /// Its project folder's name, when tracked.
    pub project: Option<String>,
    /// `crashed` | `needs_you` | `running` | `finished` | `ended`.
    pub group: String,
    /// Its status now.
    pub status: String,
    /// Plain words for the group.
    pub words: String,
    /// Its status changes inside the window, oldest first.
    pub transitions: Vec<TransitionInfo>,
    /// Tokens used, when measured.
    #[ts(type = "number | null")]
    pub tokens: Option<u64>,
    /// Processes it left behind (E5 inventory: still running after it
    /// closed).
    pub leftovers: u32,
}

/// The summary.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OvernightSummaryInfo {
    /// The window's start, epoch ms.
    #[ts(type = "number")]
    pub since_ms: i64,
    /// When this was built, epoch ms.
    #[ts(type = "number")]
    pub generated_ms: i64,
    /// Severity-ordered rows.
    pub rows: Vec<SummaryRowInfo>,
    /// Count per group (the headline).
    pub counts: BTreeMap<String, u32>,
    /// Leftover processes in all.
    pub leftovers: u32,
}

fn group_of(status: &str) -> (&'static str, &'static str, u8) {
    match status {
        "failed" => ("crashed", "Crashed", 0),
        "stuck" => ("needs_you", "Needs you — no output for a while", 1),
        "waiting_user" => ("needs_you", "Needs you — waiting for an answer", 1),
        "quota_blocked" => ("needs_you", "Needs you — quota reached", 1),
        "running" | "starting" => ("running", "Still running", 2),
        "finished" => ("finished", "Finished", 3),
        _ => ("ended", "Ended", 4),
    }
}

/// Core: the summary since `since_ms` (default: the user's last look,
/// else 12 h).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage
/// failures.
pub fn overnight_summary_impl(
    state: &Arc<AppState>,
    since_ms: Option<i64>,
) -> AppResult<OvernightSummaryInfo> {
    let now = Timestamp::now().millis();
    let rows = crate::commands::instances_shared_snapshot(state)?;
    let leftovers_by: BTreeMap<String, u32> =
        crate::commands::cleanup_inventory_inner(state, "all-managed", false)
            .groups
            .iter()
            .map(|group| {
                let orphaned = group
                    .rows
                    .iter()
                    .filter(|row| row.state == "orphaned")
                    .count();
                (
                    group.session.clone(),
                    u32::try_from(orphaned).unwrap_or(u32::MAX),
                )
            })
            .collect();
    let storage = crate::commands::lock_storage_arc(state)?;
    let since = since_ms
        .or_else(|| {
            storage
                .app_settings()
                .get(SEEN_KEY)
                .ok()
                .flatten()
                .and_then(|raw| raw.parse().ok())
        })
        .unwrap_or(now - DEFAULT_WINDOW_MS);
    let changed: BTreeSet<String> = storage
        .sessions()
        .changed_since(since)?
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    let projects: BTreeMap<String, String> = storage
        .projects()
        .list()
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.to_string(), project.name))
        .collect();
    let mut out = Vec::new();
    for row in rows {
        let (group, words, rank) = group_of(&row.status);
        if !changed.contains(&row.id) && rank != 2 {
            continue; // untouched in the window and not running
        }
        let transitions = storage
            .sessions()
            .transitions(&SessionId::new(row.id.clone()))?
            .into_iter()
            .filter(|(_, _, at)| *at >= since)
            .map(|(from, to, at_ms)| TransitionInfo { from, to, at_ms })
            .collect();
        out.push((
            rank,
            SummaryRowInfo {
                session_id: row.id.clone(),
                account_label: row.account_label.clone(),
                provider: row.provider.clone(),
                project: row
                    .project_id
                    .as_ref()
                    .and_then(|id| projects.get(id).cloned()),
                group: group.to_owned(),
                status: row.status.clone(),
                words: words.to_owned(),
                transitions,
                tokens: row.tokens.as_ref().map(|t| t.input + t.output),
                leftovers: leftovers_by.get(&row.id).copied().unwrap_or(0),
            },
        ));
    }
    out.sort_by_key(|(rank, row)| {
        (
            *rank,
            std::cmp::Reverse(row.transitions.last().map_or(0, |t| t.at_ms)),
        )
    });
    let rows: Vec<SummaryRowInfo> = out.into_iter().map(|(_, row)| row).collect();
    let mut counts = BTreeMap::new();
    for row in &rows {
        *counts.entry(row.group.clone()).or_insert(0) += 1;
    }
    let leftovers = rows.iter().map(|row| row.leftovers).sum();
    Ok(OvernightSummaryInfo {
        since_ms: since,
        generated_ms: now,
        rows,
        counts,
        leftovers,
    })
}

/// Core: the user looked — the next summary starts from now.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage
/// failures.
pub fn overnight_summary_seen_impl(state: &Arc<AppState>) -> AppResult<()> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SEEN_KEY, &Timestamp::now().millis().to_string())?;
    Ok(())
}

/// AJ-E1004: the summary since the user's last look.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per
/// [`overnight_summary_impl`].
#[tauri::command]
pub async fn overnight_summary(
    state: tauri::State<'_, Arc<AppState>>,
    since_ms: Option<i64>,
) -> AppResult<OvernightSummaryInfo> {
    crate::ipc::blocking(state.inner(), "overnight summary", move |state| {
        overnight_summary_impl(state, since_ms)
    })
    .await
}

/// AJ-E1004: marks the summary seen.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) per
/// [`overnight_summary_seen_impl`].
#[tauri::command]
pub async fn overnight_summary_seen(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "overnight summary seen", move |state| {
        overnight_summary_seen_impl(state)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::group_of;

    #[test]
    fn severity_orders_crashes_first_and_names_every_state() {
        let ranks: Vec<u8> = [
            "failed",
            "stuck",
            "waiting_user",
            "quota_blocked",
            "running",
            "finished",
            "stopped",
        ]
        .iter()
        .map(|status| group_of(status).2)
        .collect();
        assert_eq!(ranks, [0, 1, 1, 1, 2, 3, 4]);
        assert_eq!(group_of("quota_blocked").1, "Needs you — quota reached");
    }
}
