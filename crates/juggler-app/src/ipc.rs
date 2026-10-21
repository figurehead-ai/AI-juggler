//! The IPC edge's shared mechanics (ENGINEERING-GUIDE T1): a command that
//! touches SQLite, files, the network or processes is `async` and runs
//! that work off the main thread through [`blocking`].

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Runs `body` on the blocking pool with the app state; a task that
/// could not finish (it panicked or was cancelled) is an `Io` error named
/// by `context`, never a silent default.
///
/// # Errors
///
/// Whatever `body` returns, or [`AppError::Io`] when the task fails.
pub async fn blocking<T: Send + 'static>(
    state: &Arc<AppState>,
    context: &'static str,
    body: impl FnOnce(&Arc<AppState>) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let state = Arc::clone(state);
    tauri::async_runtime::spawn_blocking(move || body(&state))
        .await
        .map_err(|error| AppError::Io {
            context,
            message: error.to_string(),
        })?
}
