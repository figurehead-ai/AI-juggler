//! The remote configuration's commands (ADR-0035 §9, AJ-E2023): a
//! section's values in effect (`useRemote(section)`), and "About this
//! configuration". Both answer from memory.

use std::sync::Arc;

use super::dto::{RemoteAboutInfo, RemoteSection, RemoteValuesInfo};
use crate::error::AppResult;
use crate::state::AppState;

/// `section`'s values in effect, typed per section.
///
/// # Errors
///
/// None today: it answers from memory.
#[tauri::command]
pub async fn remote_config_values(section: RemoteSection) -> AppResult<RemoteValuesInfo> {
    Ok(super::current().values(section))
}

/// "About this configuration": where each value came from, the file in
/// use (its serial, channel and dates), when it was last checked, and
/// whether it is verified.
///
/// # Errors
///
/// None today: it answers from memory.
#[tauri::command]
pub async fn remote_config_about() -> AppResult<RemoteAboutInfo> {
    Ok(super::current().about())
}

/// Settings › Privacy "Check for configuration updates" (AJ-E1548 #15,
/// AJ-E2023 Req 3): on by default (D-129); off, nothing is fetched.
///
/// # Errors
///
/// Storage errors.
#[tauri::command]
pub async fn remote_config_set_check(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<()> {
    crate::ipc::blocking(
        state.inner(),
        "configuration updates setting",
        move |state| super::service::set_checking(state, on),
    )
    .await
}
