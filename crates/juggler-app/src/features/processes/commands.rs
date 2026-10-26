//! The Processes page's commands (AJ-E2401..E2406). Reads come from the
//! graph the 2 s tick built (no scan per call); the two that act are the
//! user's own: a mark (Keep, Remind me later) and the D-41b stop.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use juggler_core::storage::{MarkKind, ProcessMark};

use super::dto::{
    CleanupEventInfo, CleanupPolicyInfo, ProcessCountsInfo, ProcessGraphRowInfo, ProcessQueryInput,
    ProcessSnapshotInfo, ProcessStatusInfo, RulePreviewInfo,
};
use crate::commands::{KillExecuteInfo, KillVerifyInfo};
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

/// What Ended and Cleanup history look back over.
pub const KEPT_MS: i64 = 7 * 24 * 3_600_000;
/// The longest Remind me later, minutes (a day).
pub const SNOOZE_MAX_MINUTES: u32 = 24 * 60;

/// One page of the view the input asks for (`page`: the cursor of the
/// page before and how many rows).
///
/// # Errors
///
/// None today (an unreadable store reads as an empty Ended view).
pub fn page_impl(
    state: &Arc<AppState>,
    input: &ProcessQueryInput,
    page: &PageRequest,
) -> AppResult<Page<ProcessGraphRowInfo>> {
    let limit = u32::try_from(page.limit()).ok();
    if input.view == "ended" {
        let (items, next_cursor) = ended_page(state, page.cursor.as_deref(), limit);
        return Ok(Page {
            total: None,
            items,
            next_cursor,
        });
    }
    let graph = state.processes.graph();
    let rows = super::query::rows(&graph, input);
    let (items, next_cursor, total) = super::query::page(rows, page.cursor.as_deref(), limit);
    Ok(Page {
        items,
        next_cursor,
        total: Some(total),
    })
}

/// The view's state: the graph's version and counts, and whether the
/// ledger records.
#[must_use]
pub fn status_impl(state: &Arc<AppState>) -> ProcessStatusInfo {
    let graph = state.processes.graph();
    ProcessStatusInfo {
        version: state.processes.version(),
        counts: ProcessCountsInfo::of(&graph.counts),
        built_ms: graph.built_ms,
        ledger_degraded: state.supervisor.ledger_degraded(),
    }
}

/// Ended (7 days): the ledger's ended processes and the other apps'
/// sightings no longer running, newest first, one page by
/// `(last_seen, pid)`.
fn ended_page(
    state: &Arc<AppState>,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> (Vec<ProcessGraphRowInfo>, Option<String>) {
    let now = crate::commands::now_millis();
    let since = now - KEPT_MS;
    let limit = limit
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(super::query::DEFAULT_LIMIT)
        .clamp(1, super::query::MAX_LIMIT);
    let after = cursor.and_then(|cursor| {
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
    // Each process once: a ledger from before one process kept one row
    // could hold two ended rows for it (processes lab 2026-10-26, bug 1).
    let mut listed: HashSet<(u32, i64)> = HashSet::new();
    for row in ours {
        let Some(pid) = row.pid else { continue };
        if !listed.insert((pid, row.started_at.unwrap_or_default())) {
            continue;
        }
        rows.push((row.last_seen, pid, super::ended::ours_row(&row, now)));
    }
    for sighting in theirs {
        if running.contains(&(sighting.pid, sighting.start_time))
            || !listed.insert((sighting.pid, sighting.start_time))
        {
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
    page: PageRequest,
) -> AppResult<Page<ProcessGraphRowInfo>> {
    crate::ipc::blocking(state.inner(), "the processes", move |state| {
        page_impl(state, &input, &page)
    })
    .await
}

/// The view's state (its version, counts, and whether the ledger records).
///
/// # Errors
///
/// None today.
#[tauri::command]
pub async fn processes_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<ProcessStatusInfo> {
    crate::ipc::blocking(state.inner(), "the processes' state", |state| {
        Ok(status_impl(state))
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
    page: &PageRequest,
) -> AppResult<Page<CleanupEventInfo>> {
    let now = crate::commands::now_millis();
    let limit = page.limit();
    let after = page.cursor.as_deref().and_then(|cursor| {
        let (at, id) = cursor.split_once(':')?;
        Some((at.parse().ok()?, id.parse().ok()?))
    });
    let rows = state
        .audit
        .page(now - KEPT_MS, after, limit, session_id, project_id)?;
    let next = (rows.len() == limit)
        .then(|| rows.last().map(|row| format!("{}:{}", row.at_ms, row.id)))
        .flatten();
    let mut ours: Vec<String> = rows
        .iter()
        .filter(|record| record.actor != super::stop::ACTOR_OBSERVED)
        .map(|record| record.session_id.clone())
        .collect();
    ours.sort_unstable();
    ours.dedup();
    let meta = super::observe::ours_meta(state, &ours);
    let items = rows
        .iter()
        .map(|record| {
            let history = record.to_history();
            let session_title = match meta.get(&history.session_id) {
                Some(meta) => meta
                    .conversation
                    .as_deref()
                    .and_then(|id| state.processes.title_of(&history.provider, id))
                    .or_else(|| (!meta.title.is_empty()).then(|| meta.title.clone())),
                None => state
                    .processes
                    .title_of(&history.provider, &history.session_id),
            };
            CleanupEventInfo {
                session_title,
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
    Ok(Page {
        items,
        next_cursor: next,
        total: None,
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
    page: PageRequest,
) -> AppResult<Page<CleanupEventInfo>> {
    crate::ipc::blocking(state.inner(), "the cleanup history", move |state| {
        history_impl(state, session_id.as_deref(), project_id.as_deref(), &page)
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

/// Opens a session's folder in Finder: the folder the graph knows for that
/// session (never a path the window sends). False when there is none.
///
/// # Errors
///
/// None today.
pub fn reveal_impl(state: &Arc<AppState>, session_key: &str) -> AppResult<bool> {
    let graph = state.processes.graph();
    let folder = graph
        .apps
        .iter()
        .flat_map(|app| app.sessions.iter())
        .find(|session| session.key == session_key || session.session_id == session_key)
        .and_then(|session| session.folder.clone())
        .filter(|folder| folder.is_dir());
    let Some(folder) = folder else {
        return Ok(false);
    };
    Ok(std::process::Command::new("/usr/bin/open")
        .arg(folder)
        .status()
        .is_ok_and(|status| status.success()))
}

/// Reveal folder: a session's folder in Finder.
///
/// # Errors
///
/// As [`reveal_impl`].
#[tauri::command]
pub async fn processes_reveal(
    state: tauri::State<'_, Arc<AppState>>,
    session_key: String,
) -> AppResult<bool> {
    crate::ipc::blocking(state.inner(), "the session's folder", move |state| {
        reveal_impl(state, &session_key)
    })
    .await
}

/// Show in Activity Monitor: opens macOS's own Activity Monitor (another
/// app's process is looked at there, in full).
///
/// # Errors
///
/// None today.
#[tauri::command]
pub async fn processes_activity_monitor(state: tauri::State<'_, Arc<AppState>>) -> AppResult<bool> {
    crate::ipc::blocking(state.inner(), "Activity Monitor", |_| {
        Ok(std::process::Command::new("/usr/bin/open")
            .args(["-b", "com.apple.ActivityMonitor"])
            .status()
            .is_ok_and(|status| status.success()))
    })
    .await
}
