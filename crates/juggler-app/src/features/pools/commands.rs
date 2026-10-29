//! The pools' commands (AJ-E1702, AJ-E1703): the reads, and the user's
//! changes to pools, members, roles, access levels and rules, and where
//! the user put each member on a pool's canvas (0.3). Only the app's own window
//! calls them; none of them touches a session.

use std::sync::Arc;

use super::{PoolCanvasInfo, PoolMemberInfo, PoolMemberRow, PoolPreviewInfo, PoolsInfo};
use crate::error::AppResult;
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

/// Every pool, and the working set's places.
///
/// # Errors
///
/// As [`super::pools_list_impl`].
#[tauri::command]
pub async fn pools_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pools", super::pools_list_impl).await
}

/// One page of a pool's members: each one's session and card.
///
/// # Errors
///
/// As [`super::members::members_page`].
#[tauri::command]
pub async fn pools_members(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    page: PageRequest,
) -> AppResult<Page<PoolMemberRow>> {
    crate::ipc::blocking(state.inner(), "the pool's members", move |state| {
        super::members::members_page(state, &pool_id, &page)
    })
    .await
}

/// One session's pool and role (any session, one past the working set
/// too).
///
/// # Errors
///
/// As [`super::members::place_impl`].
#[tauri::command]
pub async fn pools_place(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<PoolMemberInfo> {
    crate::ipc::blocking(state.inner(), "the session's pool", move |state| {
        super::members::place_impl(state, &session_id)
    })
    .await
}

/// Makes a pool (optionally for one project).
///
/// # Errors
///
/// As [`super::pool_create_impl`].
#[tauri::command]
pub async fn pools_create(
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
pub async fn pools_rename(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    name: String,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_rename_impl(state, &pool_id, &name)
    })
    .await
}

/// Deletes a pool; its sessions go back to their default pools.
///
/// # Errors
///
/// As [`super::pool_delete_impl`].
#[tauri::command]
pub async fn pools_delete(
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
pub async fn pools_move(
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
pub async fn pools_set_role(
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
pub async fn pools_set_access(
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

/// D-29: sets a pool's hop limit and its delivery stop-all.
///
/// # Errors
///
/// As [`super::access::set_delivery_impl`].
#[tauri::command]
pub async fn pools_set_delivery(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    hop_limit: u32,
    stopped: bool,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::access::set_delivery_impl(state, &pool_id, hop_limit, stopped)
    })
    .await
}

/// D-32: lets a pool span the user's paired Macs, or not.
///
/// # Errors
///
/// As [`super::pool_set_spans_macs_impl`].
#[tauri::command]
pub async fn pools_set_spans_macs(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    spans: bool,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the pool", move |state| {
        super::pool_set_spans_macs_impl(state, &pool_id, spans)
    })
    .await
}

/// Takes back an "always allow" rule; it stops applying at once.
///
/// # Errors
///
/// As [`super::access::revoke_impl`].
#[tauri::command]
pub async fn pools_revoke_rule(
    state: tauri::State<'_, Arc<AppState>>,
    rule_id: i64,
) -> AppResult<PoolsInfo> {
    crate::ipc::blocking(state.inner(), "the rule", move |state| {
        super::access::revoke_impl(state, rule_id)
    })
    .await
}

/// A pool's canvas for the members it shows: where the user put them,
/// who talks to whom, what waits on the user and the newest activity.
///
/// # Errors
///
/// As [`super::canvas::canvas_impl`].
#[tauri::command]
pub async fn pools_canvas(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    session_ids: Vec<String>,
) -> AppResult<PoolCanvasInfo> {
    crate::ipc::blocking(state.inner(), "the pool's canvas", move |state| {
        super::canvas::canvas_impl(state, &pool_id, session_ids)
    })
    .await
}

/// Puts a member's node where the user dropped it on its pool's canvas.
///
/// # Errors
///
/// As [`super::canvas::place_impl`].
#[tauri::command]
pub async fn pools_canvas_place(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
    session_id: String,
    x: f64,
    y: f64,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the pool's canvas", move |state| {
        super::canvas::place_impl(state, &pool_id, &session_id, x, y)
    })
    .await
}

/// Forgets where the user put a pool's members ("Tidy up").
///
/// # Errors
///
/// As [`super::canvas::tidy_impl`].
#[tauri::command]
pub async fn pools_canvas_tidy(
    state: tauri::State<'_, Arc<AppState>>,
    pool_id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "the pool's canvas", move |state| {
        super::canvas::tidy_impl(state, &pool_id)
    })
    .await
}

/// The Pools screen's cards: each named pool's open members' positions
/// and its tasks in progress.
///
/// # Errors
///
/// As [`super::canvas::previews_impl`].
#[tauri::command]
pub async fn pools_canvas_previews(
    state: tauri::State<'_, Arc<AppState>>,
    pool_ids: Vec<String>,
) -> AppResult<Vec<PoolPreviewInfo>> {
    crate::ipc::blocking(state.inner(), "the pools", move |state| {
        super::canvas::previews_impl(state, pool_ids)
    })
    .await
}
