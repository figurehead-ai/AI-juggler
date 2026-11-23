//! The sign-ins' commands (AJ-E1553, AJ-E1545 R8).

use std::sync::Arc;

use super::SignInInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Every sign-in AI Juggler runs, and those kept from before a restart
/// (stale ones cleared: their App reads signed in by now, R3).
///
/// # Errors
///
/// None of its own (a blocking-pool failure).
#[tauri::command]
pub async fn sign_ins_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<SignInInfo>> {
    crate::ipc::blocking(state.inner(), "list the sign-ins", |state| {
        Ok(state.sign_ins.list(state))
    })
    .await
}

/// Opens the page sign-in `id`'s tool showed (its adapter read it).
///
/// # Errors
///
/// As [`super::open_page`].
#[tauri::command]
pub async fn sign_ins_open_page(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "open the sign-in page", move |state| {
        super::open_page(&app, state, &id)
    })
    .await
}

/// Takes sign-in `id` off the list, ending it if it still runs.
///
/// # Errors
///
/// As [`super::dismiss`].
#[tauri::command]
pub async fn sign_ins_dismiss(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "dismiss a sign-in", move |state| {
        super::dismiss(&app, state, &id)
    })
    .await
}
