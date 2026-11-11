//! More projects commands (AJ-E2017: moved from commands.rs).

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::projects::ProjectInfo;
use crate::features::projects::backfill::projects_backfill_impl;
use crate::features::projects::list::projects_list_impl;
use crate::state::AppState;

/// Backfills the projects rail with the user's REAL history: every folder
/// any account's conversations ran in (AJ-E1529: both tools, read by
/// their adapters into the conversation index), folded into its project
/// (worktrees present, removed or pruned into their repository), with the
/// accounts that worked there, how many conversations, and when last.
/// Idempotent; safe to run at every startup.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn projects_backfill(state: tauri::State<'_, Arc<AppState>>) -> AppResult<u32> {
    crate::ipc::blocking(state.inner(), "projects backfill", move |state| {
        projects_backfill_impl(state)
    })
    .await
}

/// Lists tracked projects with their live instance, account history, and
/// totals — the juggling view (ADR-0007).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn projects_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<ProjectInfo>> {
    crate::ipc::blocking(state.inner(), "list the projects", projects_list_impl).await
}
