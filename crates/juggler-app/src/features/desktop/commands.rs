//! The desktop commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::desktop::dto::{DesktopLaunchInfo, FocusOutcomeInfo};
use crate::features::desktop::{desktop_focus_with, desktop_launch_arc};
use crate::features::sessions::read_model::emit_sessions;
use crate::focus;
use crate::state::AppState;

/// Launches the Provider's desktop app under an Account and tracks it
/// as a desktop-kind Instance in the same ledger (AJ-E413). Capability-
/// gated: providers without `DESKTOP_APP` are refused by name.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message when the provider ships no
/// desktop app or the process cannot start.
#[tauri::command]
pub async fn desktop_launch(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<DesktopLaunchInfo> {
    crate::ipc::blocking(state.inner(), "desktop launch", move |state| {
        let result = desktop_launch_arc(state, &account_id)?;
        emit_sessions(&app, state)?;
        Ok(result)
    })
    .await
}

/// Runs the AJ-E414 focus tiers for one desktop Instance: activate the
/// app by its process, then (window focus on in Settings) match the
/// recorded window identity, clear `AXMinimized`, and `AXRaise` (see
/// `focus.rs`). Off the main thread, answering within 1.5 s (AJ-E2102).
/// The outcome says exactly which tier ran — app activation alone is
/// never reported as window focus success.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message when the Instance is unknown
/// or not desktop-kind (focus owns windows, terminals own panels).
#[tauri::command]
pub async fn desktop_focus(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<FocusOutcomeInfo> {
    crate::ipc::blocking(state.inner(), "bringing the window forward", move |state| {
        let state = Arc::clone(state);
        let exact = crate::os_access::exact_windows_allowed(&state);
        crate::features::conversations::goto::within(
            crate::features::conversations::goto::LIMIT,
            move || {
                desktop_focus_with(
                    &state,
                    &session_id,
                    std::sync::Arc::new(focus::OsascriptRunner),
                    exact,
                )
            },
        )
        .unwrap_or_else(|| {
            Ok(FocusOutcomeInfo {
                outcome: "Still bringing it forward…".to_owned(),
                permission_missing: false,
                window_title: None,
            })
        })
    })
    .await
}
