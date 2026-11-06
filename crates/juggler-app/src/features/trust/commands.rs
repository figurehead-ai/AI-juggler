//! The Privacy page's command (AJ-E1518): it opens only what
//! [`super::link_url`] knows, in the user's browser.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Opens a tool's terms, a cited test's file, or the private security
/// report, in the user's browser.
///
/// # Errors
///
/// [`AppError::Validation`] for anything [`super::link_url`] doesn't know;
/// [`AppError::Io`] when the browser can't be asked.
#[tauri::command]
pub async fn trust_open(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    kind: String,
    id: String,
) -> AppResult<()> {
    let url = super::link_url(state.inner(), &kind, &id).ok_or(AppError::Validation {
        subject: "link",
        message: "That page isn't available.".to_owned(),
    })?;
    crate::webview_guard::open_web(&app, url, crate::webview_guard::Opened::ByApp)
}
