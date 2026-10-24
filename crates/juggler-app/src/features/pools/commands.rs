//! The pools' commands (AJ-E1702, AJ-E1703): two reads, and the user's
//! changes to pools, members, roles and grants. Only the app's own window
//! calls them; none of them touches a session.

use std::sync::Arc;

use super::{PoolCards, PoolsInfo};
use crate::error::AppResult;
use crate::state::AppState;

/// Every pool and every session's place.
///
/// # Errors
///
/// As [`super::pools_list_impl`].
#[tauri::command]
pub async fn pools_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pools", super::pools_list_impl).await
}

/// One pool's context cards.
///
/// # Errors
///
/// As [`super::cards::cards_impl`].
#[tauri::command]
pub async fn pool_cards(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<PoolCards> {
    crate::ipc::blocking(state.inner(), "the pool's cards", move |state| {
        super::cards::cards_impl(state, &pool_id)
    })
    .await
}

/// Makes a pool (optionally for one project).
///
/// # Errors
///
/// As [`super::pool_create_impl`].
#[tauri::command]
pub async fn pool_create(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    project_id: Option<String>,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_create_impl(state, &name, project_id.as_deref())
    })
    .await
}

/// Renames a pool.
///
/// # Errors
///
/// As [`super::pool_rename_impl`].
#[tauri::command]
pub async fn pool_rename(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    name: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_rename_impl(state, &pool_id, &name)
    })
    .await
}

/// Deletes a pool; its sessions go back to "All sessions".
///
/// # Errors
///
/// As [`super::pool_delete_impl`].
#[tauri::command]
pub async fn pool_delete(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_delete_impl(state, &pool_id)
    })
    .await
}

/// Moves a session into a pool.
///
/// # Errors
///
/// As [`super::pool_move_impl`].
#[tauri::command]
pub async fn pool_move(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pool_id: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_move_impl(state, &session_id, &pool_id)
    })
    .await
}

/// Sets a session's role in its pool.
///
/// # Errors
///
/// As [`super::pool_set_role_impl`].
#[tauri::command]
pub async fn pool_set_role(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    role: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_set_role_impl(state, &session_id, &role)
    })
    .await
}

/// Sets what a role may do in a pool.
///
/// # Errors
///
/// As [`super::pool_set_grant_impl`].
#[tauri::command]
pub async fn pool_set_grant(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    capability: String,
    role: String,
    allowed: bool,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_set_grant_impl(state, &pool_id, &capability, &role, allowed)
    })
    .await
}
