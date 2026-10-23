//! The monthly caps' commands (AJ-E1514): one read, three writes to the
//! caps' own settings. None of them touches a session.

use std::sync::Arc;

use super::{BudgetInfo, BudgetInput};
use crate::error::AppResult;
use crate::state::AppState;

/// Every cap with this month's estimate; the month starts at
/// `month_start_ms`, the screen's local month.
///
/// # Errors
///
/// As [`super::list_impl`].
#[tauri::command]
pub async fn budgets_list(
    state: tauri::State<'_, Arc<AppState>>,
    month_start_ms: i64,
) -> AppResult<Vec<BudgetInfo>> {
    crate::ipc::blocking(state.inner(), "the monthly caps", move |state| {
        super::list_impl(state, month_start_ms)
    })
    .await
}

/// Saves a new cap or changes one; returns its id.
///
/// # Errors
///
/// As [`super::save_impl`].
#[tauri::command]
pub async fn budget_save(
    state: tauri::State<'_, Arc<AppState>>,
    input: BudgetInput,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "the cap", move |state| {
        super::save_impl(state, input)
    })
    .await
}

/// Removes a cap.
///
/// # Errors
///
/// As [`super::remove_impl`].
#[tauri::command]
pub async fn budget_remove(state: tauri::State<'_, Arc<AppState>>, id: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the cap", move |state| {
        super::remove_impl(state, &id)
    })
    .await
}

/// The user has seen a cap's warning at `level` this month.
///
/// # Errors
///
/// As [`super::seen_impl`].
#[tauri::command]
pub async fn budget_seen(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    month_start_ms: i64,
    level: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the cap", move |state| {
        super::seen_impl(state, &id, month_start_ms, &level)
    })
    .await
}
