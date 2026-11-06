//! The app's info and its settings commands.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::app::dto::AppInfo;
use crate::state::AppState;

#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn app_info() -> AppInfo {
    AppInfo::current()
}

/// Reads one app-level setting (schema v10 kv); `None` = never set.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub async fn app_setting_get(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
) -> AppResult<Option<String>> {
    crate::ipc::blocking(state.inner(), "app setting get", move |state| {
        let storage = lock_storage_arc(state)?;
        storage.app_settings().get(&key).map_err(AppError::from)
    })
    .await
}

/// Writes one app-level setting (empty value = reset to default).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub async fn app_setting_set(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
    value: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "app setting set", move |state| {
        crate::settings_policy::app_setting_set_impl(state, &key, &value)
    })
    .await
}

/// Opens the system Accessibility privacy pane (the deep link behind
/// the AJ-E414 permission explainer; the link is fixed — never a
/// caller-supplied URL).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the pane cannot open.
pub async fn open_accessibility_settings(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            None::<&str>,
        )
        .map_err(|e| AppError::Io {
            context: "open Accessibility settings",
            message: e.to_string(),
        })
}
