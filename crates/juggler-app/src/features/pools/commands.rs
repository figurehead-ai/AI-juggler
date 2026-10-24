//! The pools' commands (AJ-E1702, AJ-E1703): two reads, and the user's
//! changes to pools, members, roles, access levels and rules. Only the app's own window
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

/// Sets a pool's access level (D-28); `actions` is the Custom set.
///
/// # Errors
///
/// As [`super::access::set_level_impl`].
#[tauri::command]
pub async fn pool_set_access(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    level: String,
    actions: Vec<String>,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::access::set_level_impl(state, &pool_id, &level, &actions)
    })
    .await
}

/// Takes back an "always allow" rule; it stops applying at once.
///
/// # Errors
///
/// As [`super::access::revoke_impl`].
#[tauri::command]
pub async fn agent_rule_revoke(
    state: tauri::State<'_, Arc<AppState>>,
    rule_id: i64,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the rule", move |state| {
        super::access::revoke_impl(state, rule_id)
    })
    .await
}

/// The user's answer to an agent's ask: `once`, `always` or `deny`.
///
/// # Errors
///
/// As [`super::access::answer_impl`].
#[tauri::command]
pub async fn agent_ask_answer(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    action: String,
    answer: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the answer", move |state| {
        super::access::answer_impl(state, &pool_id, &action, &answer)
    })
    .await
}
