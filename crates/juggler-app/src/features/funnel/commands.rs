//! The funnel commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::features::funnel::{funnel_event_impl, funnel_export_impl};
use crate::state::AppState;

/// AJ-E309 Req.6: appends one funnel event to the LOCAL store. Runs
/// on a blocking worker — never on the UI thread — and the UI never
/// awaits it on a user path.
///
/// # Errors
///
/// Returns the typed [`AppError`] on a schema violation (coded
/// `validation`) or a storage/I/O failure.
#[tauri::command]
pub async fn funnel_event(
    state: tauri::State<'_, Arc<AppState>>,
    kind: String,
    fields: std::collections::BTreeMap<String, String>,
) -> AppResult<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        funnel_event_impl(
            &state,
            &kind,
            fields,
            juggler_core::model::Timestamp::now().millis(),
        )
    })
    .await
    .map_err(|error| AppError::Io {
        context: "funnel event",
        message: error.to_string(),
    })?
}

/// AJ-E309 Req.7: the LOCAL funnel export — counts per kind, flow
/// steps reached, policy choices, time to first Instance, and the
/// per-field inference-override rates (> 30% flagged). Markdown text
/// returned to the webview, which saves it on the user's click; no
/// network call exists.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub async fn funnel_export(state: tauri::State<'_, Arc<AppState>>) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "funnel export", move |state| {
        funnel_export_impl(state)
    })
    .await
}
