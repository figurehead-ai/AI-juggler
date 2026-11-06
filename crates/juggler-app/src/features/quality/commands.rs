//! AJ-E2903: the quality read an Account's page asks for.

use std::sync::Arc;

use crate::error::AppResult;
use crate::state::AppState;

/// How the sessions on an Account's folders went in the last 7 days.
///
/// # Errors
///
/// As [`super::account_quality_impl`].
#[tauri::command]
pub async fn quality_rollup(
    state: tauri::State<'_, Arc<AppState>>,
    folders: Vec<String>,
) -> AppResult<super::AccountQualityInfo> {
    crate::ipc::blocking(state.inner(), "how its sessions went", move |state| {
        super::account_quality_impl(
            state,
            &folders,
            juggler_core::model::Timestamp::now().millis(),
        )
    })
    .await
}
