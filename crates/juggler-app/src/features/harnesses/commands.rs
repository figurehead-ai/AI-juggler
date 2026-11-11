//! The harnesses commands. Each runs off the main thread.

use crate::features::harnesses::dto::ProviderInfo;
use crate::features::providers::provider_display;

/// Lists the Providers this build can manage (canonical name; the
/// legacy `harnesses_list` alias is registered in the handler).
#[tauri::command(rename = "providers_list")]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn harnesses_list() -> Vec<ProviderInfo> {
    let registry = crate::state::default_registry();
    registry
        .ids()
        .into_iter()
        .map(|id| ProviderInfo {
            name: provider_display(&registry, id.as_str()).0,
            id: id.to_string(),
        })
        .collect()
}

/// Legacy alias of `providers_list` (AJ-E107 Req.2: one-version IPC
/// aliases keep deserialising while UI call sites migrate).
#[tauri::command]
pub async fn harnesses_list_alias() -> Vec<super::dto::ProviderInfo> {
    harnesses_list().await
}
