//! The accounts' commands (AJ-E2109): read-only.

use std::sync::Arc;

use super::SubscriptionInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Every profile, grouped into the account (subscription) it's signed in
/// to, worked out now from each profile's sign-in metadata.
///
/// # Errors
///
/// As [`super::list_impl`].
#[tauri::command]
pub async fn subscriptions_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<SubscriptionInfo>> {
    crate::ipc::blocking(state.inner(), "the accounts", super::list_impl).await
}
