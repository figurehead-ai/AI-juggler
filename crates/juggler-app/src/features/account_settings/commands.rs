//! The account settings copy's commands (AJ-E1508). Only `apply` writes,
//! and only on the user's click.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::switch_setup::{AppliedInfo, SetupCheckInfo};
use crate::state::AppState;

/// What a new account (`target`, or one about to be made) could start with
/// from another one.
///
/// # Errors
///
/// As [`super::check_impl`].
#[tauri::command]
pub async fn account_settings_check(
    state: tauri::State<'_, Arc<AppState>>,
    target: Option<String>,
    source: String,
) -> AppResult<SetupCheckInfo> {
    crate::ipc::blocking(state.inner(), "compare the settings", move |state| {
        super::check_impl(state, target.as_deref(), &source)
    })
    .await
}

/// Copies the picked settings into the new account.
///
/// # Errors
///
/// As [`super::apply_impl`].
#[tauri::command]
pub async fn account_settings_apply(
    state: tauri::State<'_, Arc<AppState>>,
    target: String,
    source: String,
    offers: Vec<String>,
) -> AppResult<Vec<AppliedInfo>> {
    crate::ipc::blocking(state.inner(), "copy the settings", move |state| {
        super::apply_impl(state, &target, &source, &offers)
    })
    .await
}
