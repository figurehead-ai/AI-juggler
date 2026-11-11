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
    /// `terminal` | `desktop`: where it ran, so its App is named as the
    /// rest of the window names it ("Claude app · Work Max", not the
    /// tool's CLI on that folder).
    pub kind: String,
    /// Its project folder's name, when tracked.
    pub project: Option<String>,
    /// `crashed` | `error` | `needs_you` | `running` | `finished` |
    /// `stopped` | `switched` | `ended` (D-96 C: an ended one by how it
    /// ended).
    pub group: String,
    /// Its status now.
    pub status: String,
    /// Plain words for the group.
    pub words: String,
    /// Its status changes inside the window, oldest first.
    pub transitions: Vec<TransitionInfo>,
    /// AJ-E3307: when it entered its group, epoch ms — began to wait
    /// (needs you: [`waiting_since`], the rule the window's Needs you,
    /// rail and bell read) or ended (crashed, finished, ended: its last
    /// activity, the rail's "Ended today" rule); none while it runs.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
    /// D-96 C: how it ended, in detail, when its signal had one (`exit
    /// code 1`, `signal 9`).
    pub detail: Option<String>,
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
    /// D-97 (5): true when no last look was kept (and none was asked
    /// for), so the window is the 12 h default: Home says so in words
    /// ("In the last 12 hours"), never "since you last looked".
    pub default_window: bool,
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

/// The rank of a session waiting on the user, and of one still running.
const NEEDS_YOU: u8 = 2;
const RUNNING: u8 = 3;

/// A row's group, its words and its rank (crashed → error → needs you →
/// still running → finished → stopped by you → ended). D-96 C (AJ-E3307):
/// an ended one by how it ended (`InstanceInfo::ending`: its process's
/// exit or the user's Stop), and only "Ended" when nothing said.
fn group_of(status: &str, ended_how: Option<&str>) -> (&'static str, &'static str, u8) {
    match status {
        "stuck" => ("needs_you", "Needs you — no output for a while", NEEDS_YOU),
        "waiting_user" => ("needs_you", "Needs you — waiting for an answer", NEEDS_YOU),
        "quota_blocked" => ("needs_you", "Needs you — quota reached", NEEDS_YOU),
        "running" | "starting" => ("running", "Still running", RUNNING),
        _ => match ended_how {
            Some("crashed") => ("crashed", "Crashed", 0),
            Some("error") => ("error", "Error", 1),
            Some("finished") => ("finished", "Finished", 4),
            Some("stopped") => ("stopped", "Stopped by you", 5),
            // The rc27 P1 walk: its work went on in the switch's session.
            Some("switched") => ("switched", "Stopped by a switch", 5),
            _ => ("ended", "Ended", 6),
        },
    }
}

/// A session's group by its status, except a turn AI Juggler's stop cut
/// off: that waits for Resume, not an answer (the runtime lab 3, where it
/// read "waiting for an answer").
fn group_of_session(
    status: &str,
    rule: Option<&str>,
    ended_how: Option<&str>,
) -> (&'static str, &'static str, u8) {
    if rule == Some(crate::features::runtime::pickup::CUT_OFF_RULE) {
        return (
            "needs_you",
            "Needs you — its turn was cut off when AI Juggler stopped",
            NEEDS_YOU,
        );
    }
    group_of(status, ended_how)
}

/// AJ-E3307: when a session began to wait — its attention's start (a
/// tool's ask: when it was asked), else its first open ask, else its last
/// activity. The one rule (`ui/src/lib/waiting.ts` reads the same fields):
/// never the journal's last change, nor a last activity a report touched
/// again while it waited.
#[must_use]
pub fn waiting_since(row: &crate::features::sessions::dto::InstanceInfo) -> i64 {
    row.attention
        .as_ref()
        .map(|attention| attention.since_ms)
        .or_else(|| {
            row.runtime
                .as_ref()
                .and_then(|runtime| runtime.asks.first())
                .map(|ask| ask.asked_at_ms)
        })
        .or(row.last_activity_ms)
        .unwrap_or(row.started_at_ms)
}

/// When a row entered its group (rank as [`group_of`]): began to wait,
/// ended (its last activity: the moment its exit was recorded, and the
/// rail's "Ended today" reads the same), or `None` while it runs.
fn entered_ms(row: &crate::features::sessions::dto::InstanceInfo, rank: u8) -> Option<i64> {
    match rank {
        NEEDS_YOU => Some(waiting_since(row)),
        RUNNING => None,
        _ => Some(row.last_activity_ms.unwrap_or(row.started_at_ms)),
    }
}

/// A row's group by [`group_of_session`].
fn group_of_row(
    row: &crate::features::sessions::dto::InstanceInfo,
) -> (&'static str, &'static str, u8) {
    group_of_session(
        &row.status,
        row.attention
            .as_ref()
            .map(|attention| attention.rule.as_str()),
        row.ending.as_ref().map(|ending| ending.how.as_str()),
    )
}

/// Whether a row is in the window from `since`, `at_ms` when it entered
/// its group: one untouched in the window and not running is out;
/// AJ-E3307 (D2): an ended one counts when it ended inside the window, as
/// "Ended today" counts it — not when its row was last written (60
/// "finished" since 06:40 PM beside 11 ended today: rows written at once,
/// ended over a day and a half).
fn in_window(rank: u8, changed: bool, at_ms: Option<i64>, since: i64) -> bool {
    (changed || rank == RUNNING) && (rank == NEEDS_YOU || at_ms.is_none_or(|at| at >= since))
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
    let rows = crate::features::sessions::list::instances_shared_snapshot(state)?;
    let leftovers_by: BTreeMap<String, u32> =
        crate::features::cleanup::inventory::cleanup_inventory_inner(state, "all-managed", false)
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
    let asked_or_seen = since_ms.or_else(|| {
        storage
            .app_settings()
            .get(SEEN_KEY)
            .ok()
            .flatten()
            .and_then(|raw| raw.parse().ok())
    });
    let default_window = asked_or_seen.is_none();
    let since = asked_or_seen.unwrap_or(now - DEFAULT_WINDOW_MS);
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
        let (group, words, rank) = group_of_row(&row);
        let at_ms = entered_ms(&row, rank);
        if !in_window(rank, changed.contains(&row.id), at_ms, since) {
            continue;
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
                kind: row.kind.clone(),
                project: row
                    .project_id
                    .as_ref()
                    .and_then(|id| projects.get(id).cloned()),
                group: group.to_owned(),
                status: row.status.clone(),
                words: words.to_owned(),
                transitions,
                at_ms,
                detail: row.ending.as_ref().and_then(|ending| ending.detail.clone()),
                tokens: row.tokens.as_ref().map(|t| t.input + t.output),
                leftovers: leftovers_by.get(&row.id).copied().unwrap_or(0),
            },
        ));
    }
    // Needs you: the one waiting longest first (deep/11 §4.1, "Waiting
    // longest"; §6.1: oldest first). The rest: the latest first.
    out.sort_by_key(|(rank, row)| {
        let at = row
            .at_ms
            .unwrap_or_else(|| row.transitions.last().map_or(0, |t| t.at_ms));
        (*rank, if *rank == NEEDS_YOU { at } else { -at })
    });
    let rows: Vec<SummaryRowInfo> = out.into_iter().map(|(_, row)| row).collect();
    let mut counts = BTreeMap::new();
    for row in &rows {
        *counts.entry(row.group.clone()).or_insert(0) += 1;
    }
    let leftovers = rows.iter().map(|row| row.leftovers).sum();
    Ok(OvernightSummaryInfo {
        since_ms: since,
        default_window,
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
    use super::{group_of, group_of_session};

    #[test]
    fn severity_orders_crashes_first_and_names_every_state() {
        let ranks: Vec<u8> = [
            ("failed", Some("crashed")),
            ("failed", Some("error")),
            ("stuck", None),
            ("waiting_user", None),
            ("quota_blocked", None),
            ("running", None),
            ("finished", Some("finished")),
            ("stopped", Some("stopped")),
            ("stopped", None),
        ]
        .iter()
        .map(|(status, how)| group_of(status, *how).2)
        .collect();
        assert_eq!(ranks, [0, 1, 2, 2, 2, 3, 4, 5, 6]);
        assert_eq!(
            group_of("quota_blocked", None).1,
            "Needs you — quota reached"
        );
    }

    #[test]
    fn an_ended_one_says_how_it_ended_and_only_ended_when_nothing_said() {
        assert_eq!(group_of("stopped", Some("crashed")).1, "Crashed");
        assert_eq!(group_of("failed", Some("error")).1, "Error");
        assert_eq!(group_of("stopped", Some("stopped")).1, "Stopped by you");
        assert_eq!(
            group_of("handed_over", Some("switched")).1,
            "Stopped by a switch"
        );
        assert_eq!(group_of("finished", Some("finished")).1, "Finished");
        assert_eq!(group_of("handed_over", None).1, "Ended");
        assert_eq!(group_of("stopped", None).1, "Ended");
    }

    #[test]
    fn a_cut_off_turn_says_so_not_waiting_for_an_answer() {
        let cut = group_of_session("waiting_user", Some("turn_cut_off"), None);
        assert_eq!(
            cut,
            (
                "needs_you",
                "Needs you — its turn was cut off when AI Juggler stopped",
                2
            )
        );
        assert_eq!(
            group_of_session("waiting_user", Some("waiting_prompt"), None).1,
            "Needs you — waiting for an answer"
        );
        assert_eq!(
            group_of_session("waiting_user", None, None).1,
            "Needs you — waiting for an answer"
        );
    }
}
