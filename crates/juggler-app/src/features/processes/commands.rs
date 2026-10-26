//! The Processes page's commands (AJ-E2401..E2406). Reads come from the
//! graph the 2 s tick built (no scan per call); the two that act are the
//! user's own: a mark (Keep, Remind me later) and the D-41b stop.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::storage::{MarkKind, ProcessMark};

use super::dto::{
    CleanupEventInfo, CleanupHistoryPageInfo, CleanupPolicyInfo, ProcessCountsInfo,
    ProcessGraphRowInfo, ProcessPageInfo, ProcessQueryInput, ProcessSnapshotInfo, RulePreviewInfo,
};
use crate::commands::{KillExecuteInfo, KillVerifyInfo};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// What Ended and Cleanup history look back over.
pub const KEPT_MS: i64 = 7 * 24 * 3_600_000;
/// The longest Remind me later, minutes (a day).
pub const SNOOZE_MAX_MINUTES: u32 = 24 * 60;

/// One page of the view the input asks for.
///
/// # Errors
///
/// None today (an unreadable store reads as an empty Ended view).
pub fn page_impl(state: &Arc<AppState>, input: &ProcessQueryInput) -> AppResult<ProcessPageInfo> {
    let graph = state.processes.graph();
    let counts = ProcessCountsInfo::of(&graph.counts);
    let version = state.processes.version();
    if input.view == "ended" {
        let (items, next_cursor) = ended_page(state, input);
        return Ok(ProcessPageInfo {
            total: items.len() as u64,
            items,
            next_cursor,
            version,
            counts,
            built_ms: graph.built_ms,
        });
    }
    let rows = super::query::rows(&graph, input);
    let (items, next_cursor, total) =
        super::query::page(rows, input.cursor.as_deref(), input.limit);
    Ok(ProcessPageInfo {
        items,
        next_cursor,
        total,
        version,
        counts,
        built_ms: graph.built_ms,
    })
}

/// Ended (7 days): the ledger's ended processes and the other apps'
/// sightings no longer running, newest first, one page by
/// `(last_seen, pid)`.
fn ended_page(
    state: &Arc<AppState>,
    input: &ProcessQueryInput,
) -> (Vec<ProcessGraphRowInfo>, Option<String>) {
    let now = crate::commands::now_millis();
    let since = now - KEPT_MS;
    let limit = input
        .limit
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(super::query::DEFAULT_LIMIT)
        .clamp(1, super::query::MAX_LIMIT);
    let after = input.cursor.as_deref().and_then(|cursor| {
        let (seen, pid) = cursor.split_once(':')?;
        Some((seen.parse::<i64>().ok()?, pid.parse::<u32>().ok()?))
    });
    let machine = crate::commands::local_machine_id(state);
    let ours = state
        .ledger
        .ended_rows(&machine, since, after, limit)
        .unwrap_or_default();
    let graph = state.processes.graph();
    let running: HashSet<(u32, i64)> = graph
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|session| session.processes.iter())
        .map(|node| (node.pid, node.start_time))
        .collect();
    let theirs = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.sightings().page(since, after, limit).ok())
        .unwrap_or_default();
    let mut rows: Vec<(i64, u32, ProcessGraphRowInfo)> = Vec::new();
    for row in ours {
        let Some(pid) = row.pid else { continue };
        rows.push((row.last_seen, pid, super::ended::ours_row(&row, now)));
    }
    for sighting in theirs {
        if running.contains(&(sighting.pid, sighting.start_time)) {
            continue;
        }
        rows.push((
            sighting.last_seen,
            sighting.pid,
            super::ended::theirs_row(&sighting, now),
        ));
    }
    rows.sort_by_key(|row| std::cmp::Reverse((row.0, row.1)));
    rows.truncate(limit);
    let next = (rows.len() == limit)
        .then(|| rows.last().map(|(seen, pid, _)| format!("{seen}:{pid}")))
        .flatten();
    (rows.into_iter().map(|(_, _, row)| row).collect(), next)
}

/// The rows of the Processes page.
///
/// # Errors
///
/// As [`page_impl`].
#[tauri::command]
pub async fn processes_page(
    state: tauri::State<'_, Arc<AppState>>,
    input: ProcessQueryInput,
) -> AppResult<ProcessPageInfo> {
    crate::ipc::blocking(state.inner(), "the processes", move |state| {
        page_impl(state, &input)
    })
    .await
}

/// The counts now (the top bar's and This Mac's "to clean up").
///
/// # Errors
///
/// None today.
#[tauri::command]
pub async fn processes_counts(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<ProcessCountsInfo> {
    crate::ipc::blocking(state.inner(), "the process counts", |state| {
        Ok(ProcessCountsInfo::of(&state.processes.graph().counts))
    })
    .await
}

/// Sets or clears the user's mark on one process: `keep`, `snooze` (for
/// `minutes`, at most a day) or none.
///
/// # Errors
///
/// [`AppError::Validation`] for another mark; the typed storage error.
pub fn mark_impl(
    state: &Arc<AppState>,
    pid: u32,
    start_time: i64,
    mark: Option<&str>,
    minutes: Option<u32>,
) -> AppResult<()> {
    let now = crate::commands::now_millis();
    let boot = state.ledger.boot_id().to_owned();
    let mark = match mark {
        None => None,
        Some("keep") => Some((MarkKind::Keep, None)),
        Some("snooze") => {
            let minutes = minutes.unwrap_or(60).clamp(1, SNOOZE_MAX_MINUTES);
            Some((MarkKind::Snooze, Some(now + i64::from(minutes) * 60_000)))
        }
        Some(_) => {
            return Err(AppError::Validation {
                subject: "mark",
                message: "a mark is keep or snooze".into(),
            });
        }
    };
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        match mark {
            Some((kind, until_ms)) => storage.sightings().set_mark(&ProcessMark {
                pid,
                start_time,
                boot_id: boot,
                kind,
                until_ms,
                created_at: now,
            })?,
            None => storage.sightings().clear_mark(pid, start_time, &boot)?,
        }
    }
    state.processes.set_mark(pid, start_time, mark);
    Ok(())
}

/// Keep (not a leftover), Remind me later, or neither, for one process.
///
/// # Errors
///
/// As [`mark_impl`].
#[tauri::command]
pub async fn processes_mark(
    state: tauri::State<'_, Arc<AppState>>,
    pid: u32,
    start_time: i64,
    mark: Option<String>,
    minutes: Option<u32>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the process's mark", move |state| {
        mark_impl(state, pid, start_time, mark.as_deref(), minutes)
    })
    .await
}

/// D-41b: the verify sheet for another app's leftover (zero signals).
///
/// # Errors
///
/// None today (a refusal is in the sheet's data).
#[tauri::command]
pub async fn processes_observed_verify(
    state: tauri::State<'_, Arc<AppState>>,
    pid: u32,
    start_time: Option<i64>,
) -> AppResult<KillVerifyInfo> {
    crate::ipc::blocking(state.inner(), "the process's check", move |state| {
        Ok(super::stop::verify(state, pid, start_time))
    })
    .await
}

/// D-41b: the user's stop of another app's leftover, through the check
/// again, the audit and the ladder. Never called by any rule.
///
/// # Errors
///
/// As [`super::stop::execute`].
#[tauri::command]
pub async fn processes_observed_stop(
    state: tauri::State<'_, Arc<AppState>>,
    pid: u32,
    start_time: Option<i64>,
    attempt: String,
) -> AppResult<KillExecuteInfo> {
    crate::ipc::blocking(state.inner(), "the process's stop", move |state| {
        let flag = state.register_kill_cancel(&attempt);
        let result = super::stop::execute(state, state.audit.as_ref(), pid, start_time, &flag);
        state.unregister_kill_cancel(&attempt);
        result
    })
    .await
}

/// One page of Cleanup history, optionally one session's or project's.
///
/// # Errors
///
/// The typed storage error.
pub fn history_impl(
    state: &Arc<AppState>,
    session_id: Option<&str>,
    project_id: Option<&str>,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> AppResult<CleanupHistoryPageInfo> {
    let now = crate::commands::now_millis();
    let limit = limit
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(super::query::DEFAULT_LIMIT)
        .clamp(1, super::query::MAX_LIMIT);
    let after = cursor.and_then(|cursor| {
        let (at, id) = cursor.split_once(':')?;
        Some((at.parse().ok()?, id.parse().ok()?))
    });
    let rows = state
        .audit
        .page(now - KEPT_MS, after, limit, session_id, project_id)?;
    let next = (rows.len() == limit)
        .then(|| rows.last().map(|row| format!("{}:{}", row.at_ms, row.id)))
        .flatten();
    let items = rows
        .iter()
        .map(|record| {
            let history = record.to_history();
            CleanupEventInfo {
                id: history.id,
                at_ms: history.at_ms,
                actor: history.actor,
                automatic: history.automatic,
                pid: history.pid,
                name: super::roles::program_name(&record.exe_path).to_owned(),
                session_id: history.session_id,
                tool: history.provider,
                account_id: history.account_id,
                project_id: record.project_id.clone(),
                outcome: history.outcome,
                ports_before: history.ports_before,
                ports_after: history.ports_after,
            }
        })
        .collect();
    Ok(CleanupHistoryPageInfo {
        items,
        next_cursor: next,
    })
}

/// Cleanup history (7 days), one page, by session or project.
///
/// # Errors
///
/// As [`history_impl`].
#[tauri::command]
pub async fn processes_history(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: Option<String>,
    project_id: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> AppResult<CleanupHistoryPageInfo> {
    crate::ipc::blocking(state.inner(), "the cleanup history", move |state| {
        history_impl(
            state,
            session_id.as_deref(),
            project_id.as_deref(),
            cursor.as_deref(),
            limit,
        )
    })
    .await
}

/// What an account's automatic rule would tidy now (nothing signalled).
///
/// # Errors
///
/// As [`super::rules::dry_run`].
#[tauri::command]
pub async fn processes_dry_run(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    tier: u8,
) -> AppResult<RulePreviewInfo> {
    crate::ipc::blocking(state.inner(), "the rule's dry run", move |state| {
        super::rules::dry_run(state, &account_id, tier)
    })
    .await
}

/// Each account's automatic cleanup, with pause and the crash hold.
///
/// # Errors
///
/// As [`super::rules::policy`].
#[tauri::command]
pub async fn processes_policy(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CleanupPolicyInfo> {
    crate::ipc::blocking(state.inner(), "the cleanup rules", |state| {
        super::rules::policy(state)
    })
    .await
}

/// A snapshot of what runs now to save: names, roles, states, numbers and
/// keyed hashes of command lines only (never a line, a folder or a title).
///
/// # Errors
///
/// None today.
pub fn snapshot_impl(state: &Arc<AppState>) -> AppResult<ProcessSnapshotInfo> {
    let graph = state.processes.graph();
    let salt = state.processes.salt();
    let pids: Vec<u32> = graph
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .flat_map(|session| session.processes.iter().map(|node| node.pid))
        .collect();
    let lines: HashMap<u32, (i64, String)> = state
        .resources
        .lock()
        .map(|collector| {
            pids.iter()
                .filter_map(|pid| collector.brief(*pid).map(|b| (*pid, (b.start_time, b.cmd))))
                .collect()
        })
        .unwrap_or_default();
    let json = super::ended::snapshot_json(&graph, &|pid, start| {
        lines
            .get(&pid)
            .filter(|(at, _)| *at == start)
            .map(|(_, line)| super::sightings::cmd_hash(&salt, line))
    });
    Ok(ProcessSnapshotInfo {
        file_name: format!("processes-{}.json", graph.built_ms),
        json,
    })
}

/// The snapshot to save.
///
/// # Errors
///
/// As [`snapshot_impl`].
#[tauri::command]
pub async fn processes_snapshot(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<ProcessSnapshotInfo> {
    crate::ipc::blocking(state.inner(), "the process snapshot", snapshot_impl).await
}
