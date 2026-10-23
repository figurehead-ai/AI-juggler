//! AJ-E1005 (with AJ-E1007 to follow): an Instance's lifecycle beyond its
//! process status — `reviewed` (it leaves the triage's finished list).
//! Each step is an appended, timestamped event in the lifecycle journal;
//! nothing about the Instance's data changes.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Core: marks an ended Instance reviewed; returns when (epoch ms).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an Instance that is still live
/// (there is nothing to review yet), or the typed storage error for an
/// unknown one.
pub fn instance_mark_reviewed_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<i64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    let record = storage.sessions().get(&id)?;
    if !crate::commands::is_terminal(record.status) {
        return Err(AppError::Validation {
            subject: "review",
            message: "only a session that has ended can be marked reviewed".to_owned(),
        });
    }
    Ok(storage.sessions().record_lifecycle(&id, "reviewed")?)
}

/// AJ-E1005: marks an ended Instance reviewed (it leaves the triage's
/// finished list; the event is kept in its history).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`instance_mark_reviewed_impl`].
#[tauri::command]
pub fn instance_mark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    let at = instance_mark_reviewed_impl(state.inner(), &session_id)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(at)
}

/// Core (AJ-E1503): undoes Mark reviewed — an `unreviewed` event, so the
/// session is back on the triage's finished list and both steps stay in
/// its history. Returns when (epoch ms).
///
/// # Errors
///
/// Returns [`AppError::Validation`] when the session's latest step is not
/// `reviewed`, or the typed storage error for an unknown one.
pub fn instance_unmark_reviewed_impl(state: &Arc<AppState>, session_id: &str) -> AppResult<i64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let id = SessionId::new(session_id);
    storage.sessions().get(&id)?;
    let latest = storage.sessions().lifecycle_latest()?;
    if latest.get(session_id).map(|(event, _)| event.as_str()) != Some("reviewed") {
        return Err(AppError::Validation {
            subject: "review",
            message: "this session isn't marked reviewed".to_owned(),
        });
    }
    Ok(storage.sessions().record_lifecycle(&id, "unreviewed")?)
}

/// AJ-E1503: undoes Mark reviewed (the toast's Undo).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`instance_unmark_reviewed_impl`].
#[tauri::command]
pub fn instance_unmark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    let at = instance_unmark_reviewed_impl(state.inner(), &session_id)?;
    crate::commands::emit_sessions(&app, state.inner())?;
    Ok(at)
}
