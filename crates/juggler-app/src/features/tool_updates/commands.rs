//! The tool update check's commands (AJ-E1611).

use std::sync::Arc;

use super::ToolUpdateInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// What `tool`'s page says about its versions; `None` for a tool no
/// adapter registers. Runs the tool's own `--version` (never an update).
///
/// # Errors
///
/// [`crate::error::AppError::Io`] when the read could not finish.
#[tauri::command]
pub async fn tool_updates_status(
    state: tauri::State<'_, Arc<AppState>>,
    tool: String,
) -> AppResult<Option<ToolUpdateInfo>> {
    crate::ipc::blocking(state.inner(), "tool update status", move |state| {
        Ok(super::status_impl(state, &tool))
    })
    .await
}

/// Turns the once-a-day check for new tool versions on or off.
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn tool_updates_set_check(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "tool update setting", move |state| {
        super::set_on(state, on)
    })
    .await
}
