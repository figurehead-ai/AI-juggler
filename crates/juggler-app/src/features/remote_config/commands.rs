//! The remote configuration's commands (ADR-0035 §9, AJ-E2023): a
//! section's values in effect (`useRemote(section)`), and "About this
//! configuration". Both answer from memory.

use super::dto::{RemoteAboutInfo, RemoteSection, RemoteValuesInfo};
use crate::error::AppResult;

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
