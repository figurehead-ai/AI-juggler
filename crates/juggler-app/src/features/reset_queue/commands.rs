//! The reset queue commands. Each runs off the main thread.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::reset_queue::dto::ResetQueueInfo;
use crate::state::AppState;

/// AJ-E611 Req.6: queues a resume for an exhausted session — a
/// PASSIVE timer + notification; the resume fires only from the
/// notification/row click (`reset_queue_resume`).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub async fn reset_queue_queue(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    session: String,
    account: String,
    resume_at_ms: i64,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "reset queue queue", move |state| {
        let storage = lock_storage_arc(state)?;
        let id = format!("rq-{session}");
        storage
            .usage()
            .queue_reset(&id, &session, &account, resume_at_ms)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}

/// AJ-E611: the queue rows (countdown/ready/stale explained).
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn reset_queue_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ResetQueueInfo>> {
    crate::ipc::blocking_list(state.inner(), "reset queue list", |_, storage, now| {
        storage
            .usage()
            .open_reminders() // 0.3.2: a due one stays until opened or closed
            .unwrap_or_default()
            .into_iter()
            .map(|(id, session, account, resume_at)| ResetQueueInfo {
                id,
                session,
                account,
                resume_at_ms: resume_at,
                state: if resume_at <= now { "ready" } else { "queued" }.to_owned(),
            })
            .collect()
    })
    .await
}

/// AJ-E611: dismiss a queued resume (explicit user action).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub async fn reset_queue_dismiss(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "reset queue dismiss", move |state| {
        let storage = lock_storage_arc(state)?;
        storage
            .usage()
            .set_reset_state(&id, "dismissed")
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
}
