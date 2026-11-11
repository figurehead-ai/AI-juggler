//! AJ-E2903: the quality read an Account's page asks for.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::quality::transcript::instance_quality_impl;
use crate::features::sessions::dto::QualitySignalsInfo;
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

/// Computes the D-07 quality signals for one Instance (AJ-E409): reads
/// the native transcript when present (per-turn usage + timestamps +
/// error markers); every field is a labelled signal or its honest
/// empty — never a composite score.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
#[tauri::command]
pub async fn instance_quality(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<QualitySignalsInfo> {
    crate::ipc::blocking(state.inner(), "instance quality", move |state| {
        instance_quality_impl(state, &session_id)
    })
    .await
}
