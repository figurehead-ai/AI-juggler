//! The accounts' commands (AJ-E2109): the list; naming one and setting
//! its plan, renewal and fee (D-62).

use std::sync::Arc;

use super::SubscriptionInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Every profile, grouped into the account (subscription) it's signed in
/// to, worked out now from each profile's sign-in metadata.
///
/// # Errors
///
/// As [`super::list_with_apps_impl`].
#[tauri::command]
pub async fn subscriptions_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<SubscriptionInfo>> {
    crate::ipc::blocking(state.inner(), "the accounts", super::list_with_apps_impl).await
}

/// D-62 (AJ-E3205): names an Account (`null` or blank: its default).
///
/// # Errors
///
/// As [`super::rename_impl`].
#[tauri::command]
pub async fn subscriptions_rename(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    name: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "naming the account", move |state| {
        super::rename_impl(state, &id, name.as_deref())
    })
    .await
}

/// D-62 (AJ-E3202 req. 4): sets an Account's `plan`, `renewal` or `fee`
/// (`null`: clears it); where its Apps disagreed, this is the choice.
///
/// # Errors
///
/// As [`super::terms::set_impl`].
#[tauri::command]
pub async fn subscriptions_set_term(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    term: String,
    value: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "setting the account's terms", move |state| {
        super::terms::set_impl(state, &id, &term, value.as_deref())
    })
    .await
}
