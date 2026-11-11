//! The launch commands. Each runs off the main thread.

use std::sync::Arc;

use crate::commands::find_profile_with_account_arc;
use crate::error::AppResult;
use crate::features::launch::dto::LaunchResult;
use crate::features::launch::launch_profile_as;
use crate::features::launch::recipe_env::launch_recipe_env_set_impl;
use crate::features::port::dto::PortWarningInfo;
use crate::features::port::launch_port_warning_inner;
use crate::features::sessions::read_model::emit_sessions;
use crate::state::AppState;

/// AJ-E309 Req.5: one Launch settings' environment overrides.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) for an unknown id.
#[tauri::command]
pub async fn launch_recipe_env_get(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    crate::ipc::blocking(state.inner(), "launch recipe env get", move |state| {
        let (recipe, _) = find_profile_with_account_arc(state, &recipe_id)?;
        Ok(recipe.settings_overlay.env)
    })
    .await
}

/// AJ-E309 Req.5: replaces one Launch settings' environment
/// overrides (the env overlay editor's Save). Validated first —
/// nothing is written on a refusal.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) for an unknown id, a refused
/// overlay, or a storage failure.
#[tauri::command]
pub async fn launch_recipe_env_set(
    state: tauri::State<'_, Arc<AppState>>,
    recipe_id: String,
    env: std::collections::BTreeMap<String, String>,
) -> AppResult<std::collections::BTreeMap<String, String>> {
    crate::ipc::blocking(state.inner(), "launch recipe env set", move |state| {
        launch_recipe_env_set_impl(state, &recipe_id, env)
    })
    .await
}

#[tauri::command]
/// Starts a session; `runtime` (`terminal` | `driven`) is this session's
/// own choice (AJ-E2304: "Start in terminal"), else the profile's.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn launch_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    cwd: Option<String>,
    runtime: Option<String>,
) -> AppResult<LaunchResult> {
    let runtime = runtime
        .as_deref()
        .and_then(juggler_adapter::Runtime::from_word);
    crate::ipc::blocking(state.inner(), "launch session", move |state| {
        if let Some(cwd) = &cwd {
            crate::picked::work_folder(state, cwd)?;
        }
        let result = launch_profile_as(state, &profile_id, cwd, runtime)?;
        emit_sessions(&app, state)?;
        Ok(result)
    })
    .await
}

/// The pre-launch warning (Req.4): a project's expected dev port
/// (learned from its instances' observed listeners) held by a managed
/// orphan. Launch itself is NEVER blocked — the UI offers `Launch
/// anyway` beside `Free this port`.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
#[tauri::command]
pub async fn launch_port_warning(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: Option<String>,
) -> AppResult<Option<PortWarningInfo>> {
    crate::ipc::blocking(state.inner(), "launch port warning", move |state| {
        Ok(launch_port_warning_inner(state, project_id))
    })
    .await
}
