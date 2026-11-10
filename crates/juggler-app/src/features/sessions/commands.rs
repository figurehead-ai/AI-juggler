//! The sessions feature's commands.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::sessions::dto::InstanceInfo;
use crate::features::sessions::read_model::emit_sessions;
use crate::features::sessions::read_model::live_stats_of;
use crate::features::sessions::read_model::working_records;
use crate::state::AppState;
use juggler_core::model::SessionId;
use std::time::Duration;

/// The copyable resume line for a session (Copy resume command). Takes the
/// session's id; the account, folder and conversation come from AI
/// Juggler's own records (SECURITY C1).
///
/// # Errors
///
/// As [`super::resume_command_impl`].
#[tauri::command]
pub async fn sessions_resume_command(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "the resume command", move |state| {
        super::resume_command_impl(state, &session_id)
    })
    .await
}

/// AJ-E1509: a launch setting's own choice about restarting its crashed
/// sessions — `null` to follow Settings › Sessions again.
///
/// # Errors
///
/// As [`super::restart::set_launch_choice`].
#[tauri::command]
pub async fn sessions_restart_choice_set(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
    restart: Option<bool>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the restart choice", move |state| {
        super::restart::set_launch_choice(state, &recipe_id, restart)
    })
    .await
}

/// History, a project's or an account's Sessions: every session (AI
/// Juggler's and the ones started outside it), a page at a time, open
/// first, then newest (0.1.5 review: they read every session at once).
///
/// # Errors
///
/// As [`super::rows::sessions_page_impl`].
#[tauri::command]
pub async fn sessions_page(
    state: tauri::State<'_, Arc<AppState>>,
    query: super::rows::SessionsQuery,
    page: crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<super::rows::SessionRowInfo>> {
    crate::ipc::blocking(state.inner(), "the sessions", move |state| {
        super::rows::sessions_page_impl(state, &query, &page)
    })
    .await
}

/// How many sessions a list holds: open, ended and archived (a list's
/// heading, History's Archived tab).
///
/// # Errors
///
/// As [`super::rows::sessions_counts_impl`].
#[tauri::command]
pub async fn sessions_counts(
    state: tauri::State<'_, Arc<AppState>>,
    query: super::rows::SessionsQuery,
) -> AppResult<super::rows::SessionsCountsInfo> {
    crate::ipc::blocking(state.inner(), "the session counts", move |state| {
        super::rows::sessions_counts_impl(state, &query)
    })
    .await
}

/// These sessions, as the app's list has them: a page opened for a
/// session past the working set (an old one, from History). At most
/// [`crate::paging::MAX_LIMIT`] ids.
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn sessions_get(
    state: tauri::State<'_, Arc<AppState>>,
    ids: Vec<String>,
) -> AppResult<Vec<super::dto::InstanceInfo>> {
    crate::ipc::blocking(state.inner(), "the sessions asked for", move |state| {
        let ids: Vec<juggler_core::model::SessionId> = ids
            .iter()
            .take(crate::paging::MAX_LIMIT)
            .map(|id| juggler_core::model::SessionId::new(id.as_str()))
            .collect();
        let records = crate::commands::lock_storage_arc(state)?
            .sessions()
            .get_many(&ids)?;
        crate::features::sessions::list::instances_of(state, records)
    })
    .await
}

/// A session's History tab (wf09, AJ-E1533): its status changes,
/// lifecycle steps and switches, oldest first, a page at a time.
///
/// # Errors
///
/// As [`super::history::history_page_impl`].
#[tauri::command]
pub async fn sessions_history(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    page: crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<super::history::SessionEventInfo>> {
    crate::ipc::blocking(state.inner(), "the session's history", move |state| {
        super::history::history_page_impl(state, &session_id, &page)
    })
    .await
}

/// E31 (AJ-E3103): what a list of sessions' filters offer: the projects,
/// Apps, Accounts and Macs its rows are on, each with how many rows it
/// shows. Read when a list's filters are first shown, never before.
///
/// # Errors
///
/// As [`super::facets::sessions_facets_impl`].
#[tauri::command]
pub async fn sessions_facets(
    state: tauri::State<'_, Arc<AppState>>,
    query: super::rows::SessionsQuery,
) -> AppResult<super::facets::SessionsFacetsInfo> {
    crate::ipc::blocking(state.inner(), "the sessions' filters", move |state| {
        super::facets::sessions_facets_impl(state, &query)
    })
    .await
}

/// Lists Instances (board feed), newest first (canonical name
/// `instances_list`; legacy alias registered in the handler).
#[tauri::command(rename = "instances_list")]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn sessions_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<InstanceInfo>> {
    crate::ipc::blocking(state.inner(), "list the sessions", |state| {
        // Collector lock first and alone: the resource tick shares it, and
        // no code path ever holds the storage and collector locks together.
        let live = live_stats_of(state);
        let records = working_records(&*lock_storage_arc(state)?)?;
        crate::features::sessions::list::rows_with(state, records, &live)
    })
    .await
}

/// Stops a session via the supervisor ladder (grace, then kill).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn stop_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "stop session", move |state| {
        stop_impl(state, &session_id)?;
        emit_sessions(&app, state)?;
        Ok(())
    })
    .await
}

/// What a stopped session with no process says.
pub const NO_PROCESS_WORDS: &str = "Stopped: its process had already ended";

/// Stops `session`: its process through the supervisor's ladder. One
/// with no process left (never supervised in this run, or an adopted one
/// that ended since) has nothing to signal: it ends at once, with words
/// (rc27 live pass 2: a ghost driven session's Stop said "Nothing was
/// stopped" and it stayed Working).
///
/// # Errors
///
/// The supervisor's refusals for a process that still runs.
pub fn stop_impl(state: &Arc<AppState>, session: &str) -> AppResult<()> {
    let id = SessionId::new(session);
    if state.supervisor.is_finished(&id) {
        // The machine identity locks storage itself: resolved first.
        let machine = crate::commands::local_machine_id(state);
        let live = lock_storage_arc(state)?
            .sessions()
            .get(&id)
            .is_ok_and(|record| {
                record.machine == machine && !super::read_model::is_terminal(record.status)
            });
        if !live && !state.supervisor.is_supervised(&id) {
            // Not a session of this Mac's, or not one at all.
            return Err(juggler_supervisor::SupervisorError::UnknownSession(id).into());
        }
        if state.supervisor.is_supervised(&id) {
            // Its exit is recorded (or is being): stopping is done.
            let _ = state.supervisor.stop(&id, Duration::from_secs(5));
        }
        if live {
            state.runtime.forget(session);
            crate::features::runtime::pickup::end(state, &id, NO_PROCESS_WORDS);
        }
        return Ok(());
    }
    state.supervisor.stop(&id, Duration::from_secs(5))?;
    Ok(())
}

/// Legacy alias of `instances_list` (AJ-E107 Req.2).
#[tauri::command]
///
/// # Errors
///
/// Returns the parent command's typed error on failure.
pub async fn sessions_list_alias(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> crate::error::AppResult<Vec<super::dto::InstanceInfo>> {
    sessions_list(state).await
}
