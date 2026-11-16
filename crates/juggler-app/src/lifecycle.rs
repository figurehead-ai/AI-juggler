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
    if !crate::features::sessions::read_model::is_terminal(record.status) {
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
pub async fn instance_mark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    crate::ipc::blocking(state.inner(), "instance mark reviewed", move |state| {
        let at = instance_mark_reviewed_impl(state, &session_id)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(at)
    })
    .await
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
pub async fn instance_unmark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<i64> {
    crate::ipc::blocking(state.inner(), "instance unmark reviewed", move |state| {
        let at = instance_unmark_reviewed_impl(state, &session_id)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(at)
    })
    .await
}

/// Core (AJ-E2910): Home's "Mark all N reviewed" — every ended session in
/// `session_ids`, in one transaction; a live one refuses the lot.
///
/// # Errors
///
/// [`AppError::Validation`] for a live session or more than 500; the
/// typed storage error otherwise.
pub fn instances_mark_reviewed_impl(
    state: &Arc<AppState>,
    session_ids: &[String],
) -> AppResult<i64> {
    if session_ids.len() > 500 {
        return Err(AppError::Validation {
            subject: "review",
            message: "mark at most 500 at once".to_owned(),
        });
    }
    let storage = crate::commands::lock_storage_arc(state)?;
    let ids: Vec<SessionId> = session_ids
        .iter()
        .map(|id| SessionId::new(id.as_str()))
        .collect();
    for id in &ids {
        if !crate::features::sessions::read_model::is_terminal(storage.sessions().get(id)?.status) {
            return Err(AppError::Validation {
                subject: "review",
                message: "only sessions that have ended can be marked reviewed".to_owned(),
            });
        }
    }
    Ok(storage.sessions().record_lifecycle_many(&ids, "reviewed")?)
}

/// Core (AJ-E2910): undoes "Mark all reviewed" (the toast's Undo).
///
/// # Errors
///
/// The typed storage error.
pub fn instances_unmark_reviewed_impl(
    state: &Arc<AppState>,
    session_ids: &[String],
) -> AppResult<i64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let ids: Vec<SessionId> = session_ids
        .iter()
        .map(|id| SessionId::new(id.as_str()))
        .collect();
    Ok(storage
        .sessions()
        .record_lifecycle_many(&ids, "unreviewed")?)
}

/// What Home's "Mark all N reviewed" did: how many it marked, and when
/// (its batch, for the toast's Undo).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReviewedAllInfo {
    /// How many were marked.
    #[ts(type = "number")]
    pub count: u64,
    /// When, epoch ms: the batch's stamp.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// Core (the 0.4.6 RC walk K2, F-K2-7a): Home's "Mark all N reviewed"
/// marks every session its list holds (`query`'s, read as `to_review`:
/// the same rows its count and its pages are), not only those the app
/// holds, in one transaction.
///
/// # Errors
///
/// The typed storage error.
pub fn sessions_mark_reviewed_impl(
    state: &Arc<AppState>,
    query: &crate::features::sessions::rows::SessionsQuery,
) -> AppResult<ReviewedAllInfo> {
    let ids: Vec<SessionId> = crate::features::sessions::rows::to_review_ids(state, query)?
        .into_iter()
        .map(SessionId::new)
        .collect();
    let at_ms = if ids.is_empty() {
        juggler_core::model::Timestamp::now().millis()
    } else {
        crate::commands::lock_storage_arc(state)?
            .sessions()
            .record_lifecycle_many(&ids, "reviewed")?
    };
    Ok(ReviewedAllInfo {
        count: ids.len() as u64,
        at_ms,
    })
}

/// Core (F-K2-7a): undoes one "Mark all N reviewed" (batch `at_ms`): puts
/// back on the list each session whose latest state is still that batch's
/// "reviewed" (one archived or reviewed again since stays as it is).
/// Returns how many.
///
/// # Errors
///
/// The typed storage error.
pub fn sessions_unmark_reviewed_impl(state: &Arc<AppState>, at_ms: i64) -> AppResult<u64> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let ids: Vec<SessionId> = storage
        .sessions()
        .lifecycle_latest()?
        .into_iter()
        .filter(|(_, (event, at))| event == "reviewed" && *at == at_ms)
        .map(|(id, _)| SessionId::new(id))
        .collect();
    if !ids.is_empty() {
        storage
            .sessions()
            .record_lifecycle_many(&ids, "unreviewed")?;
    }
    Ok(ids.len() as u64)
}

/// F-K2-7a: Home's Mark all N reviewed, every one its list holds.
///
/// # Errors
///
/// As [`sessions_mark_reviewed_impl`].
#[tauri::command]
pub async fn sessions_mark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    query: crate::features::sessions::rows::SessionsQuery,
) -> AppResult<ReviewedAllInfo> {
    crate::ipc::blocking(state.inner(), "mark all reviewed", move |state| {
        let done = sessions_mark_reviewed_impl(state, &query)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(done)
    })
    .await
}

/// F-K2-7a: undoes one Mark all N reviewed.
///
/// # Errors
///
/// As [`sessions_unmark_reviewed_impl`].
#[tauri::command]
pub async fn sessions_unmark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    at_ms: i64,
) -> AppResult<u64> {
    crate::ipc::blocking(state.inner(), "unmark all reviewed", move |state| {
        let count = sessions_unmark_reviewed_impl(state, at_ms)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(count)
    })
    .await
}

/// AJ-E2910: Home's Mark all reviewed.
///
/// # Errors
///
/// As [`instances_mark_reviewed_impl`].
#[tauri::command]
pub async fn instances_mark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_ids: Vec<String>,
) -> AppResult<i64> {
    crate::ipc::blocking(state.inner(), "mark all reviewed", move |state| {
        let at = instances_mark_reviewed_impl(state, &session_ids)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(at)
    })
    .await
}

/// AJ-E2910: undoes Home's Mark all reviewed.
///
/// # Errors
///
/// As [`instances_unmark_reviewed_impl`].
#[tauri::command]
pub async fn instances_unmark_reviewed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_ids: Vec<String>,
) -> AppResult<i64> {
    crate::ipc::blocking(state.inner(), "unmark all reviewed", move |state| {
        let at = instances_unmark_reviewed_impl(state, &session_ids)?;
        crate::features::sessions::read_model::emit_sessions(&app, state)?;
        Ok(at)
    })
    .await
}
