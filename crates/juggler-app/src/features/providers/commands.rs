//! The providers commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::providers::dto::{ProviderCapabilityInfo, ProviderInstallInfo};
use crate::features::providers::{providers_capabilities_impl, providers_discover_impl};
use crate::state::AppState;

/// ADR-0018: each registered Provider's static capabilities (no
/// process is spawned — unlike [`providers_discover`]).
///
/// # Errors
///
/// Never in practice; typed for the command surface.
#[tauri::command]
pub async fn providers_capabilities(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ProviderCapabilityInfo>> {
    crate::ipc::blocking(state.inner(), "providers capabilities", move |state| {
        Ok(providers_capabilities_impl(state))
    })
    .await
}

/// AJ-E301: probes every registered Provider in parallel (thread per
/// adapter — `--version` subprocesses never touch the UI thread; the
/// 2s-per-provider hard cap is the subprocess timeout inside each
/// adapter's `discover()`).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on registry failures (never for a
/// missing binary — that is a ROW, not an error).
#[tauri::command]
pub async fn providers_discover(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ProviderInstallInfo>> {
    crate::ipc::blocking(state.inner(), "providers discover", move |state| {
        providers_discover_impl(state)
    })
    .await
}
