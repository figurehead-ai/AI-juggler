//! The subagents feature's commands.

use std::sync::Arc;

use crate::error::AppResult;
use crate::state::AppState;

/// The subagents a session started, for the inspector.
///
/// # Errors
///
/// As [`super::list`].
#[tauri::command]
pub async fn subtasks_list(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<super::dto::SubtasksInfo> {
    crate::ipc::blocking(state.inner(), "the subagents", move |state| {
        super::list(state, &session_id)
    })
    .await
}
