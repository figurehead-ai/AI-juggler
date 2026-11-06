//! The cost commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::cost::dto::{CostRollupsInfo, CostRowInfo};
use crate::features::cost::{cost_csv_export_impl, cost_rollups_impl, cost_rows_page_impl};
use crate::state::AppState;

/// AJ-E609 Req.3: the per-session cost matrix (session × model × day).
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn cost_rollups(
    state: tauri::State<'_, Arc<AppState>>,
    account: Option<String>,
) -> AppResult<CostRollupsInfo> {
    crate::ipc::blocking(state.inner(), "cost rollups", move |state| {
        Ok(cost_rollups_impl(state, account.as_deref()))
    })
    .await
}

/// AJ-E609 Req.6: streams the CSV to a temp file (chunked month
/// windows — never a full-table read) and returns its path. The rows
/// are the SAME rollup shape the UI displays.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on storage or I/O failure.
#[tauri::command]
pub async fn cost_csv_export(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: Option<String>,
    from_ms: Option<i64>,
    to_ms: Option<i64>,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "cost csv export", move |state| {
        let span = from_ms.zip(to_ms).filter(|(from, to)| from < to);
        cost_csv_export_impl(state, account.as_deref(), span)
    })
    .await
}

/// The cost table (one row per session and UTC day, its models folded
/// together), a page at a time: newest day first, then by session. Each
/// row's models are [`cost_rollups`]' rows (the same rollup, so the two
/// reconcile); the totals, the subscription view and the pricing flags
/// stay there. The whole table is never read: a page reads its session
/// days, then only the events of the days they're in, and names only
/// its own sessions.
///
/// # Errors
///
/// [`AppError::Validation`](crate::error::AppError::Validation) for a cursor this app didn't make; storage
/// errors.
#[tauri::command]
pub async fn cost_rows_page(
    state: tauri::State<'_, Arc<AppState>>,
    account: Option<String>,
    page: crate::paging::PageRequest,
) -> AppResult<crate::paging::Page<CostRowInfo>> {
    crate::ipc::blocking(state.inner(), "the cost table", move |state| {
        cost_rows_page_impl(state, account.as_deref(), &page)
    })
    .await
}
