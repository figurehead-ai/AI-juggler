//! The ranking commands. Each runs off the main thread.

use crate::error::AppResult;
use crate::features::ranking::dto::RankedRowInfo;
use crate::features::ranking::ranking_rows_inner;

/// AJ-E611: the explainable ranking command (thin wrapper over
/// [`ranking_rows_inner`] — suggest-only, the user clicks).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage failures.
#[tauri::command]
pub async fn ranking_rows(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    project: Option<String>,
) -> AppResult<Vec<RankedRowInfo>> {
    // Off the main thread (AJ-E1535): New session opens without waiting.
    crate::ipc::blocking(state.inner(), "rank the accounts", move |state| {
        ranking_rows_inner(state, project)
    })
    .await
}
