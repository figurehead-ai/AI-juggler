//! The recovery commands. Each runs off the main thread.

use std::sync::Arc;

use super::{dto::RecoverySummaryInfo, recovery_summary_get_arc};
use crate::{
    commands::lock_storage_arc,
    error::{AppError, AppResult},
    state::AppState,
};

/// Reads the unconsumed recovery summary, if one is pending (shown
/// once, then dismissed forever until a new recovery finds something).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
#[tauri::command]
pub async fn recovery_summary_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Option<RecoverySummaryInfo>> {
    crate::ipc::blocking(state.inner(), "recovery summary get", move |state| {
        recovery_summary_get_arc(state)
    })
    .await
}

/// Dismisses the summary (the only way it goes away; no permanent
/// dismiss shortcut exists by design).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
#[tauri::command]
pub async fn recovery_summary_dismiss(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "recovery summary dismiss", move |state| {
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set("recovery_summary_consumed", "1")
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}
