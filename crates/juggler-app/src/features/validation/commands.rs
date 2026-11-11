//! The validation commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::validation::dto::ValidationCellInfo;
use crate::features::validation::{validation_dogfood_record_impl, validation_matrix_impl};
use crate::state::AppState;

/// AJ-E210: the validation matrix (static researched rows merged with
/// any recorded live cells). Read-only.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage failures.
#[tauri::command]
pub async fn validation_matrix(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ValidationCellInfo>> {
    crate::ipc::blocking(state.inner(), "validation matrix", move |state| {
        validation_matrix_impl(state)
    })
    .await
}

/// AJ-E210: records a live dogfood cell for one ordered pair — a
/// HUMAN ran the pair on real binaries and reports the outcome (the
/// app never launches vendor binaries for validation). Recording a
/// PASS on a real-CI-green pair flips its badge to "· Proven"; a
/// FAIL lowers the advertised ceiling one level until fixed.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the pair is not in the
/// researched matrix (never invented) or the store fails.
#[allow(
    clippy::too_many_arguments,
    reason = "the Tauri command's flat wire args"
)]
#[tauri::command]
pub async fn validation_dogfood_record(
    state: tauri::State<'_, Arc<AppState>>,
    from: String,
    to: String,
    verdict: String,
    at: String,
    project_shape: String,
    source_sha256: String,
    target_sha256: String,
    note: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the tried-by-hand record", move |state| {
        validation_dogfood_record_impl(
            state,
            &from,
            &to,
            &verdict,
            &at,
            &project_shape,
            &source_sha256,
            &target_sha256,
            &note,
        )
    })
    .await
}
