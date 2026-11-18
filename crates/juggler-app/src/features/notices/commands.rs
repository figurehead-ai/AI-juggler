//! The notices' commands (ADR-0035 §2, AJ-E2023): the notices to show, and
//! dismissing one.

use std::sync::Arc;

use super::dto::RemoteNoticeInfo;
use crate::error::AppResult;
use crate::events::{AppEvent, EmitEvent as _};
use crate::state::AppState;

/// The notices in effect now, the dismissed left out, louder first.
///
/// # Errors
///
/// As [`super::active_now`].
#[tauri::command]
pub async fn notices_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<RemoteNoticeInfo>> {
    crate::ipc::blocking(state.inner(), "the notices", |state| {
        super::active_now(state)
    })
    .await
}

/// Dismisses a notice (remembered on this Mac); the notices left.
///
/// # Errors
///
/// As [`super::dismiss`].
#[tauri::command]
pub async fn notices_dismiss(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<Vec<RemoteNoticeInfo>> {
    let left = crate::ipc::blocking(state.inner(), "the notices", move |state| {
        super::dismiss(state, &id)
    })
    .await?;
    app.emit_event(AppEvent::RemoteConfigChanged, ());
    Ok(left)
}
