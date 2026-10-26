//! The usage history's commands (0.1.5): reads only, each off the main
//! thread and bounded.

use std::sync::Arc;

use super::dto::UsageSessionInfo;
use super::{UsageHistoryInfo, UsageLimitsInfo, UsageQueryInput, UsageSessionDetailInfo};
use crate::error::AppResult;
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

/// A range's usage, summarised for the charts.
///
/// # Errors
///
/// As [`super::history_impl`].
#[tauri::command]
pub async fn usage_history_summary(
    state: tauri::State<'_, Arc<AppState>>,
    input: UsageQueryInput,
) -> AppResult<UsageHistoryInfo> {
    crate::ipc::blocking(state.inner(), "the usage history", move |state| {
        super::history_impl(state, &input)
    })
    .await
}

/// A page of a range's sessions, the biggest first.
///
/// # Errors
///
/// As [`super::sessions_page_impl`].
#[tauri::command]
pub async fn usage_history_sessions(
    state: tauri::State<'_, Arc<AppState>>,
    input: UsageQueryInput,
    page: PageRequest,
) -> AppResult<Page<UsageSessionInfo>> {
    crate::ipc::blocking(state.inner(), "the usage history", move |state| {
        super::sessions_page_impl(state, &input, &page)
    })
    .await
}

/// The first usage, epoch ms (one profile's when named): where "All"
/// starts.
///
/// # Errors
///
/// As [`super::first_impl`].
#[tauri::command]
pub async fn usage_history_first(
    state: tauri::State<'_, Arc<AppState>>,
    profile: Option<String>,
) -> AppResult<Option<i64>> {
    crate::ipc::blocking(state.inner(), "the usage history", move |state| {
        super::first_impl(state, profile.as_deref())
    })
    .await
}

/// A profile's 5-hour and weekly windows over a range.
///
/// # Errors
///
/// As [`super::limits_impl`].
#[tauri::command]
pub async fn usage_history_limits(
    state: tauri::State<'_, Arc<AppState>>,
    profile: String,
    start_ms: i64,
    end_ms: i64,
) -> AppResult<UsageLimitsInfo> {
    crate::ipc::blocking(state.inner(), "the limit history", move |state| {
        super::limits_impl(state, &profile, start_ms, end_ms)
    })
    .await
}

/// One session's usage turn by turn, and its share of its profile's
/// windows.
///
/// # Errors
///
/// As [`super::session_impl`].
#[tauri::command]
pub async fn usage_history_session(
    state: tauri::State<'_, Arc<AppState>>,
    profile: String,
    conversation: String,
) -> AppResult<UsageSessionDetailInfo> {
    crate::ipc::blocking(state.inner(), "the session's usage", move |state| {
        super::session_impl(state, &profile, &conversation)
    })
    .await
}
