//! AJ-E1102/E1103: the usage-stats commands; each runs off the main
//! thread (the interface events only queue a count).

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::{
    METRICS_DOC_URL, UsageStatsInfo, UsageStatsOverview, UsageStatsSetResult,
    usage_stats_delete_local_impl, usage_stats_info_impl, usage_stats_overview_impl,
    usage_stats_report_impl, usage_stats_set_impl, usage_stats_set_sync_impl,
    usage_stats_ui_event_impl, usage_stats_ui_gauge_impl,
};

/// AJ-E1109: turns the personal stats sync on or off.
///
/// # Errors
///
/// Per [`usage_stats_set_sync_impl`].
#[tauri::command]
pub async fn usage_stats_set_sync(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<UsageStatsOverview> {
    crate::ipc::blocking(state.inner(), "usage stats set sync", move |state| {
        usage_stats_set_sync_impl(state, on)
    })
    .await
}

/// AJ-E1105: opens the public list of every metric (a fixed address;
/// the interface passes none).
///
/// # Errors
///
/// Returns [`AppError::Io`] when the browser can't be opened.
#[tauri::command]
pub async fn usage_stats_open_docs(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_url(METRICS_DOC_URL, None::<&str>)
        .map_err(|e| AppError::Io {
            context: "usage stats",
            message: e.to_string(),
        })
}

/// AJ-E1105: what Settings ▸ Privacy shows.
///
/// # Errors
///
/// Per [`usage_stats_overview_impl`].
#[tauri::command]
pub async fn usage_stats_overview(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<UsageStatsOverview> {
    crate::ipc::blocking(state.inner(), "usage stats overview", move |state| {
        usage_stats_overview_impl(state)
    })
    .await
}

/// AJ-E1105: one report's exact JSON.
///
/// # Errors
///
/// Per [`usage_stats_report_impl`].
#[tauri::command]
pub async fn usage_stats_report(
    state: tauri::State<'_, Arc<AppState>>,
    status: String,
    week: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "usage stats report", move |state| {
        usage_stats_report_impl(state, &status, &week)
    })
    .await
}

/// AJ-E1105: "Delete local stats".
///
/// # Errors
///
/// Per [`usage_stats_delete_local_impl`].
#[tauri::command]
pub async fn usage_stats_delete_local(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<UsageStatsSetResult> {
    crate::ipc::blocking(state.inner(), "usage stats delete local", move |state| {
        usage_stats_delete_local_impl(state)
    })
    .await
}

/// AJ-E1103: counts one interface event (ignored unless stats are on and
/// it is an interface metric with schema values).
#[tauri::command]
pub async fn usage_stats_ui_event(
    metric: String,
    dims: std::collections::BTreeMap<String, String>,
    value: Option<f64>,
) {
    usage_stats_ui_event_impl(&metric, &dims, value);
}

/// AJ-E1103: records an interface display preference as a gauge.
#[tauri::command]
pub async fn usage_stats_ui_gauge(metric: String, value: String) {
    usage_stats_ui_gauge_impl(&metric, &value);
}

/// AJ-E1102: the usage-stats choice and what applies.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn usage_stats_info(state: tauri::State<'_, Arc<AppState>>) -> AppResult<UsageStatsInfo> {
    crate::ipc::blocking(state.inner(), "usage stats info", move |state| {
        Ok(usage_stats_info_impl(state))
    })
    .await
}

/// AJ-E1102: sets the usage-stats mode (`off` deletes local stats).
///
/// # Errors
///
/// Per [`usage_stats_set_impl`].
#[tauri::command]
pub async fn usage_stats_set(
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> AppResult<UsageStatsSetResult> {
    crate::ipc::blocking(state.inner(), "usage stats set", move |state| {
        usage_stats_set_impl(state, &mode)
    })
    .await
}
