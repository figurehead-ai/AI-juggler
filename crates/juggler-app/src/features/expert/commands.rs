//! The expert commands. Each runs off the main thread.

use std::sync::Arc;

use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::features::expert::dto::ExpertSetupInfo;
use crate::features::expert::{expert_reveal_target, expert_setup_info_impl};
use crate::state::AppState;

/// AJ-E309 Req.5: the Expert-setup facts.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown Provider, Account,
/// or Launch settings (or Launch settings of another Account).
#[tauri::command]
pub async fn expert_setup_info(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    account_id: Option<String>,
    recipe_id: Option<String>,
) -> AppResult<ExpertSetupInfo> {
    crate::ipc::blocking(state.inner(), "expert setup info", move |state| {
        expert_setup_info_impl(
            state,
            &provider,
            account_id.as_deref(),
            recipe_id.as_deref(),
        )
    })
    .await
}

/// AJ-E309 Req.5: [Reveal in Finder] for a managed folder. The path
/// is resolved from AI Juggler's own state (an Account's folder, or
/// the Provider's accounts folder) — never caller-supplied — and the
/// nearest existing folder inside the data dir is revealed when the
/// target does not exist yet. Read-only: nothing is created.
///
/// # Errors
///
/// Returns the typed [`AppError`] for unknown ids or when Finder
/// cannot be opened.
#[tauri::command]
pub async fn expert_reveal(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    account_id: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "expert reveal", move |state| {
        let target = expert_reveal_target(state, &provider, account_id.as_deref())?;
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|error| AppError::Io {
                context: "reveal in Finder",
                message: error.to_string(),
            })
    })
    .await
}
