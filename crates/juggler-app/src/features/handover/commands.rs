//! More handover commands (AJ-E2017: moved from commands.rs).

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::handover::source::session_handover_impl;
use crate::features::launch::dto::LaunchResult;
use crate::features::sessions::read_model::emit_sessions;
use crate::state::AppState;

/// The juggle (spec §7 T1/T2 v0): move a live session's transcript to
/// another account and resume it there — same project, same context, one
/// click. User-initiated only (automation policy).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn session_handover(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<LaunchResult> {
    crate::ipc::blocking(state.inner(), "session handover", move |state| {
        let result = session_handover_impl(state, &session_id, &target_profile_id)?;
        emit_sessions(&app, state)?;
        Ok(result)
    })
    .await
}
