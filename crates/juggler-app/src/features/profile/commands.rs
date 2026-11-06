//! The profile commands. Each runs off the main thread.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::profile::dto::{LaunchRecipeInfo, ProfileInfo};
use crate::features::profile::{create_profile, profile_infos};
use crate::state::AppState;

/// Lists Launch settings (canonical name `launch_recipes_list`;
/// legacy alias registered in the handler).
#[tauri::command(rename = "launch_recipes_list")]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn profiles_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<LaunchRecipeInfo>> {
    // Off the main thread (AJ-E1535): the storage lock may be held by a
    // tick, and New session opens without waiting on it.
    crate::ipc::blocking(state.inner(), "list launch settings", |state| {
        profile_infos(&*lock_storage_arc(state)?)
    })
    .await
}

/// Creates a profile under an account.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn profile_create(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    name: String,
    model: Option<String>,
    default_cwd: Option<String>,
) -> AppResult<ProfileInfo> {
    crate::ipc::blocking(state.inner(), "profile create", move |state| {
        if let Some(cwd) = &default_cwd {
            crate::picked::work_folder(state, cwd)?;
        }
        create_profile(state, &account_id, &name, model, default_cwd)
    })
    .await
}

/// Deletes launch settings (D-60): refused while one of their sessions is
/// open; the sessions made with them stay in History
/// ([`crate::manage::launch_settings_remove_impl`]).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn profile_delete(
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "profile delete", move |state| {
        crate::manage::launch_settings_remove_impl(state, &profile_id)
    })
    .await
}

/// Legacy alias of `launch_recipes_list` (AJ-E107 Req.2).
#[tauri::command]
///
/// # Errors
///
/// Returns the parent command's typed error on failure.
pub async fn profiles_list_alias(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> crate::error::AppResult<Vec<super::dto::LaunchRecipeInfo>> {
    profiles_list(state).await
}
