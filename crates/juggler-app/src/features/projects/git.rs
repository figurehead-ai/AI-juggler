//! AJ-E1533: a project's git state for its page's header — its branch,
//! how far it is ahead of or behind its upstream, and what isn't
//! committed — read from the project's own root (the same reader, and
//! the same 2 s floor, as a session's). The UI names the project by its
//! id; the folder is the one AI Juggler recorded (C1).

use std::sync::Arc;

use juggler_core::model::ProjectId;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;
use crate::worktree::{GitStatusInfo, status_info};

/// The project's git state now, `None` for a folder that isn't a git
/// checkout (or is gone).
///
/// # Errors
///
/// [`crate::error::AppError::NotFound`] for an unknown project; storage
/// errors.
pub fn git_status_impl(
    state: &Arc<AppState>,
    project_id: &str,
) -> AppResult<Option<GitStatusInfo>> {
    let root = lock_storage_arc(state)?
        .projects()
        .get(&ProjectId::new(project_id))?
        .root;
    Ok(status_info(state, &root))
}

/// A project's git state, for its page's header.
///
/// # Errors
///
/// As [`git_status_impl`].
#[tauri::command]
pub async fn projects_git_status(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<Option<GitStatusInfo>> {
    crate::ipc::blocking(state.inner(), "the project's git state", move |state| {
        git_status_impl(state, &project_id)
    })
    .await
}
