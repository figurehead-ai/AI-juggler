//! The agent bridge's commands (D-30): the question's answer and the
//! switches. Only the app's own window calls them; an agent can't.

use std::sync::Arc;

use super::dto::AgentBridgeInfo;
use crate::error::AppResult;
use crate::state::AppState;

/// Whether sessions get the agent tools, and where they're off.
///
/// # Errors
///
/// As [`super::settings::info`].
#[tauri::command]
pub async fn agent_bridge_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<AgentBridgeInfo> {
    crate::ipc::blocking(state.inner(), "the agent tools", super::settings::info).await
}

/// The answer to the one-time question, or the switch for every session.
///
/// # Errors
///
/// As [`super::settings::answer_impl`].
#[tauri::command]
pub async fn agent_bridge_answer(
    state: tauri::State<'_, Arc<AppState>>,
    on: bool,
) -> AppResult<AgentBridgeInfo> {
    crate::ipc::blocking(state.inner(), "the agent tools", move |state| {
        super::settings::answer_impl(state, on)
    })
    .await
}

/// Switches the tools off (or back on) for one project or profile.
///
/// # Errors
///
/// As [`super::settings::set_impl`].
#[tauri::command]
pub async fn agent_bridge_set(
    state: tauri::State<'_, Arc<AppState>>,
    scope: String,
    id: String,
    on: bool,
) -> AppResult<AgentBridgeInfo> {
    crate::ipc::blocking(state.inner(), "the agent tools", move |state| {
        super::settings::set_impl(state, &scope, &id, on)
    })
    .await
}
