//! The sessions feature's commands.

use std::sync::Arc;

use crate::error::AppResult;
use crate::state::AppState;

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
