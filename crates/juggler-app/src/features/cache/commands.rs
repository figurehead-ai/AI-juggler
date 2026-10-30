//! The local cache's commands (ADR-0023): where it stands, the switch,
//! the limit, and Clear cache.

use std::sync::Arc;

use super::dto::CacheInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Where the cache stands: on or off, its size, its limit, each cache.
///
/// # Errors
///
/// As [`super::status_impl`].
#[tauri::command]
pub async fn cache_status(state: tauri::State<'_, Arc<AppState>>) -> AppResult<CacheInfo> {
    crate::ipc::blocking(state.inner(), "the local cache", super::status_impl).await
}

/// Turns the cache on or off (off removes its files).
///
/// # Errors
///
/// As [`super::set_on_impl`].
#[tauri::command]
pub async fn cache_set_on(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<CacheInfo> {
    crate::ipc::blocking(state.inner(), "the local cache", move |state| {
        super::set_on_impl(state, on)
    })
    .await
}

/// Sets the limit: `null` for automatic, else megabytes.
///
/// # Errors
///
/// As [`super::set_limit_impl`].
#[tauri::command]
pub async fn cache_set_limit(
    state: tauri::State<'_, Arc<AppState>>,
    limit_mb: Option<u64>,
) -> AppResult<CacheInfo> {
    crate::ipc::blocking(state.inner(), "the local cache", move |state| {
        super::set_limit_impl(state, limit_mb)
    })
    .await
}

/// Clears every cache now.
///
/// # Errors
///
/// As [`super::clear_impl`].
#[tauri::command]
pub async fn cache_clear(state: tauri::State<'_, Arc<AppState>>) -> AppResult<CacheInfo> {
    crate::ipc::blocking(state.inner(), "the local cache", super::clear_impl).await
}
