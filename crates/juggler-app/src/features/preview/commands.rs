//! The preview commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::preview::{preview_start_impl, stop_preview_session};
use crate::state::AppState;

/// Starts the read-only live preview for one desktop Instance
/// (AJ-E415): strictly one at a time (the previous stream stops
/// first); requires the Account's explicit preview opt-in; frames and
/// phase changes arrive as `preview://frame` / `preview://state`
/// events. Read-only by construction — pixels flow app → panel only.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message for a non-desktop Instance,
/// a missing opt-in, or an unknown id.
pub async fn preview_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "preview start", move |state| {
        preview_start_impl(&app, state, session_id)
    })
    .await
}

/// Stops the live preview (panel close / selection switch). Bounded
/// teardown: the stop flag ends the loop within one poll interval.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on state poisoning.
pub async fn preview_stop(state: tauri::State<'_, Arc<AppState>>) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "preview stop", move |state| {
        stop_preview_session(state);
        Ok(())
    })
    .await
}
