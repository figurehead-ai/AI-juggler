//! More packs commands (AJ-E2017: moved from commands.rs).

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::packs::export::pack_export_impl;
use crate::state::AppState;

/// AJ-E209 Req.5: exports an epoch's pack (the graph's Export action)
/// — conversation turns rendered through the AJ-E208 redaction
/// policy (the GLOBAL choice — a standalone export has no
/// destination Provider), written under `<app-data>/exports/`.
/// Returns where it was saved, and what was hidden, in one line.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the epoch cannot be resolved,
/// the policy is unset (exports block like switches), or the write
/// fails.
#[tauri::command]
pub async fn pack_export(
    state: tauri::State<'_, Arc<AppState>>,
    epoch_id: String,
) -> AppResult<crate::features::handover::view::SavedHandoverInfo> {
    crate::ipc::blocking(state.inner(), "pack export", move |state| {
        pack_export_impl(state, &epoch_id)
    })
    .await
}
