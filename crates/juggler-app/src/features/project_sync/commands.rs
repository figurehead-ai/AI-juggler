//! The project sync's commands (AJ-E1507). Only the `apply` ones write,
//! and only on the user's click.

use std::sync::Arc;

use super::{RulesDiffInfo, RulesSyncInfo, ServersSyncInfo};
use crate::error::AppResult;
use crate::features::switch_setup::AppliedInfo;
use crate::state::AppState;

/// The project's rules files and how each stands beside the master.
///
/// # Errors
///
/// As [`super::rules_impl`].
#[tauri::command]
pub async fn project_sync_rules(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    master: Option<String>,
) -> AppResult<RulesSyncInfo> {
    crate::ipc::blocking(state.inner(), "compare the rules", move |state| {
        super::rules_impl(state, &project_id, master.as_deref())
    })
    .await
}

/// What applying the master to one rules file would change.
///
/// # Errors
///
/// As [`super::diff_impl`].
#[tauri::command]
pub async fn project_sync_diff(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    master: String,
    file: String,
    how: String,
) -> AppResult<RulesDiffInfo> {
    crate::ipc::blocking(state.inner(), "show the changes", move |state| {
        super::diff_impl(state, &project_id, &master, &file, &how)
    })
    .await
}

/// Applies the master to one rules file, against exactly what was shown.
///
/// # Errors
///
/// As [`super::apply_impl`].
#[tauri::command]
pub async fn project_sync_apply(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    master: String,
    file: String,
    how: String,
    file_sha: Option<String>,
    master_sha: String,
) -> AppResult<AppliedInfo> {
    crate::ipc::blocking(state.inner(), "apply the rules", move |state| {
        super::apply_impl(
            state,
            &project_id,
            &master,
            &file,
            &how,
            file_sha.as_deref(),
            &master_sha,
        )
    })
    .await
}

/// Each account's MCP servers for the project, and what one could get
/// from another.
///
/// # Errors
///
/// As [`super::servers_impl`].
#[tauri::command]
pub async fn project_sync_servers(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<ServersSyncInfo> {
    crate::ipc::blocking(state.inner(), "compare the servers", move |state| {
        super::servers_impl(state, &project_id)
    })
    .await
}

/// Copies one MCP server from one account to another.
///
/// # Errors
///
/// As [`super::servers_apply_impl`].
#[tauri::command]
pub async fn project_sync_servers_apply(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
    from: String,
    to: String,
    offer: String,
) -> AppResult<AppliedInfo> {
    crate::ipc::blocking(state.inner(), "copy the server", move |state| {
        super::servers_apply_impl(state, &project_id, &from, &to, &offer)
    })
    .await
}
