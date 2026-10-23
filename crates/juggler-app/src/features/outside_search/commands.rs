//! The opt-in outside search's commands (AJ-E1538): two reads, and the
//! switch, which builds the index or deletes it.

use std::sync::Arc;

use super::{OutsideSearchEstimateInfo, OutsideSearchInfo};
use crate::error::AppResult;
use crate::state::AppState;

/// Whether it is on, and how far the index has got.
///
/// # Errors
///
/// As [`super::status_impl`].
#[tauri::command]
pub async fn outside_search_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<OutsideSearchInfo> {
    crate::ipc::blocking(state.inner(), "the search index", super::status_impl).await
}

/// The index's likely size before it is turned on.
///
/// # Errors
///
/// As [`super::estimate_impl`].
#[tauri::command]
pub async fn outside_search_estimate(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<OutsideSearchEstimateInfo> {
    crate::ipc::blocking(state.inner(), "the search index", super::estimate_impl).await
}

/// Turns it on (building starts in the background) or off (the index is
/// deleted).
///
/// # Errors
///
/// As [`super::set_impl`].
#[tauri::command]
pub async fn outside_search_set(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<OutsideSearchInfo> {
    crate::ipc::blocking(state.inner(), "the search index", move |state| {
        super::set_impl(state, on)
    })
    .await
}
