//! The isolation commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::isolation::dto::IsolationRowInfo;
use crate::features::isolation::isolation_check_impl;
use crate::state::AppState;

/// AJ-E308 Req.7: the post-import/create isolation checklist — six
/// named checks, metadata-only, run automatically; any fail blocks
/// \[Launch\] with the failing line named.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) (unknown account / storage).
#[tauri::command]
pub async fn isolation_check(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<Vec<IsolationRowInfo>> {
    crate::ipc::blocking(state.inner(), "isolation check", move |state| {
        isolation_check_impl(state, &account_id)
    })
    .await
}
