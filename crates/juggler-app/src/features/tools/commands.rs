//! The tools catalog's commands.

use std::sync::Arc;

use super::ToolSpecInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Every registered tool as data: the UI reads names, marks, sign-in
/// words, install pages and first run's groups from here, never from a
/// table of its own.
///
/// # Errors
///
/// [`crate::error::AppError::Io`] when the read could not finish.
#[tauri::command]
pub async fn tools_catalog(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<ToolSpecInfo>> {
    crate::ipc::blocking(state.inner(), "tools catalog", |state| {
        Ok(super::catalog_impl(state))
    })
    .await
}
