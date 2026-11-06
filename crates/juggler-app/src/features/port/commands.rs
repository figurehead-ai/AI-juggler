//! The port commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::port::dto::PortInventoryInfo;
use crate::features::port::port_inventory_inner;
use crate::state::AppState;

/// The port inventory (Req.3): every listener classified as exactly
/// one of managed-orphan / managed-running / re-adopted /
/// not-managed; unmanaged rows are display-only.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
#[tauri::command]
pub async fn port_inventory(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<PortInventoryInfo> {
    crate::ipc::blocking(state.inner(), "port inventory", move |state| {
        Ok(port_inventory_inner(state))
    })
    .await
}
