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
