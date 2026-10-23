//! The terminal tools' commands (AJ-E1506).

use std::sync::Arc;

use juggler_core::model::SessionId;
use tauri_plugin_opener::OpenerExt;

use super::links::{self, LinkTarget};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Opens a link terminal `id` printed: a web address in the browser, a
/// file in the editor chosen for projects (else shown in Finder), a
/// folder in Finder. A relative path starts from the session's folder.
///
/// # Errors
///
/// As [`links::resolve`]; [`AppError::Io`] when the Mac can't open it.
#[tauri::command]
pub async fn terminal_open_link(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    target: String,
) -> AppResult<()> {
    let (found, editor) =
        crate::ipc::blocking(state.inner(), "opening a terminal link", move |state| {
            let storage = crate::commands::lock_storage_arc(state)?;
            // A login terminal has no session, and so no folder.
            let cwd = storage
                .sessions()
                .get(&SessionId::new(id))
                .ok()
                .map(|record| record.cwd);
            let editor = storage
                .app_settings()
                .get(crate::manage::EDITOR_KEY)?
                .filter(|name| !name.is_empty());
            Ok((
                links::resolve(&target, cwd.as_deref(), dirs::home_dir().as_deref())?,
                editor,
            ))
        })
        .await?;
    let opener = app.opener();
    let result = match (&found, editor) {
        (LinkTarget::Web(url), _) => opener.open_url(url, None::<&str>),
        (LinkTarget::File(path), Some(editor)) => {
            opener.open_path(path.to_string_lossy(), Some(editor))
        }
        (LinkTarget::File(path) | LinkTarget::Folder(path), _) => opener.reveal_item_in_dir(path),
    };
    result.map_err(|error| AppError::Io {
        context: "open the terminal's link",
        message: error.to_string(),
    })
}

/// Opens terminal `id` in its own window (or brings that window forward),
/// titled with the session's name.
///
/// # Errors
///
/// As [`super::window::pop_out`]; the supervisor's error for a terminal
/// it doesn't know.
#[tauri::command]
pub async fn terminal_pop_out(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    title: String,
) -> AppResult<()> {
    // Only a terminal AI Juggler is running gets a window.
    state.supervisor.output(&SessionId::new(id.as_str()))?;
    super::window::pop_out(&app, &id, &title)
}

/// Closes terminal `id`'s own window; it shows in the main window again.
#[tauri::command]
pub async fn terminal_pop_in(app: tauri::AppHandle, id: String) {
    super::window::pop_in(&app, &id);
}

/// The terminals in their own windows now.
#[tauri::command]
pub async fn terminal_windows(app: tauri::AppHandle) -> Vec<String> {
    super::window::windows(&app)
}
