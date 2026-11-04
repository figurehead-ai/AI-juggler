//! The Floor's commands (AJ-E3505).

// Tauri command signatures take State by value.
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use juggler_floor::snapshot::FloorSnapshot;

use crate::error::AppResult;
use crate::state::AppState;

/// The company now, as the Floor draws it: for the main window's Floor
/// list. The first call starts keeping it current (`floor://changed`).
///
/// # Errors
///
/// Storage errors; a poisoned lock.
#[tauri::command]
pub async fn floor_snapshot(state: tauri::State<'_, Arc<AppState>>) -> AppResult<FloorSnapshot> {
    crate::ipc::blocking(state.inner(), "the Floor", |state| {
        super::current(state).map(|snapshot| (*snapshot).clone())
    })
    .await
}

/// Opens the Floor's window, or brings it forward.
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] where the Floor can't run yet
/// (why, in words), or when its window can't be made.
#[tauri::command]
pub async fn floor_open(app: tauri::AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || super::window::open(&app))
        .await
        .map_err(|e| crate::error::AppError::Refused(format!("The Floor couldn't open: {e}.")))?
}
