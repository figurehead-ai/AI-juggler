//! AJ-E3013 (D-116): the connector's commands. Only the app's own window
//! calls them; an agent can't (the bridge's catalog names none of this).

use std::sync::Arc;

use super::dto::{
    ConnectorPreviewInfo, ConnectorRemovalInfo, ConnectorStatusInfo, DesktopMemberInfo,
};
use crate::error::AppResult;
use crate::state::AppState;

/// What Add would write into the desktop App `app_id`, each file before
/// and after (the consent card's diff).
///
/// # Errors
///
/// As [`super::preview_impl`].
#[tauri::command]
pub async fn connector_preview(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
) -> AppResult<ConnectorPreviewInfo> {
    crate::ipc::blocking(state.inner(), "AI Juggler's connector", move |state| {
        super::preview_impl(state, &app_id)
    })
    .await
}

/// Adds AI Juggler's connector to the desktop App `app_id` (the user's
/// click on the card).
///
/// # Errors
///
/// As [`super::add_impl`].
#[tauri::command]
pub async fn connector_add(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
) -> AppResult<ConnectorStatusInfo> {
    crate::ipc::blocking(state.inner(), "AI Juggler's connector", move |state| {
        super::add_impl(state, &app_id)
    })
    .await
}

/// Takes AI Juggler's connector out of the desktop App `app_id`.
///
/// # Errors
///
/// As [`super::remove_impl`].
#[tauri::command]
pub async fn connector_remove(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
) -> AppResult<ConnectorRemovalInfo> {
    crate::ipc::blocking(state.inner(), "AI Juggler's connector", move |state| {
        super::remove_impl(state, &app_id)
    })
    .await
}

/// Where AI Juggler's connector stands in the desktop App `app_id`.
///
/// # Errors
///
/// As [`super::status_impl`].
#[tauri::command]
pub async fn connector_status(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
) -> AppResult<ConnectorStatusInfo> {
    crate::ipc::blocking(state.inner(), "AI Juggler's connector", move |state| {
        super::status_impl(state, &app_id)
    })
    .await
}

/// A pool's desktop members: connector, connected, and how a new pool
/// message reaches each (D-116, D-119).
///
/// # Errors
///
/// As [`super::desktop_members_impl`].
#[tauri::command]
pub async fn connector_desktop_members(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<Vec<DesktopMemberInfo>> {
    crate::ipc::blocking(state.inner(), "the pool's desktop members", move |state| {
        super::desktop_members_impl(state, &pool_id)
    })
    .await
}
