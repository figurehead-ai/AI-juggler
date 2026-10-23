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
