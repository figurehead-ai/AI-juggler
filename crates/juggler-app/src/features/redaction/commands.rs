//! The redaction commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::redaction::dto::RedactionPolicyInfo;
use crate::features::redaction::{
    redaction_policy_get_impl, redaction_policy_set_global_impl, redaction_policy_set_override_impl,
};
use crate::state::AppState;

/// AJ-E208: the redaction policy for one Provider (settings surface
/// + the switch confirm line).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when storage reads fail.
#[tauri::command]
pub async fn redaction_policy_get(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
) -> AppResult<RedactionPolicyInfo> {
    crate::ipc::blocking(state.inner(), "redaction policy get", move |state| {
        redaction_policy_get_impl(state, &provider)
    })
    .await
}

/// AJ-E208: sets (or clears) the GLOBAL policy — Settings →
/// Privacy. `None` clears (back to unset — the first pack blocks).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when storage writes fail.
#[tauri::command]
pub async fn redaction_policy_set_global(
    state: tauri::State<'_, Arc<AppState>>,
    policy: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "redaction policy set global", move |state| {
        redaction_policy_set_global_impl(state, policy.as_deref())
    })
    .await
}

/// AJ-E208: sets (or clears) one Provider's override — Provider
/// settings.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on an unknown wire name or a
/// storage write failure.
#[tauri::command]
pub async fn redaction_policy_set_override(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    policy: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(
        state.inner(),
        "redaction policy set override",
        move |state| redaction_policy_set_override_impl(state, &provider, policy.as_deref()),
    )
    .await
}
