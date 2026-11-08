//! ADR-0031 (D-93): the Move to account commands (the switch screen's
//! How choice); the move itself is `features::handover::moving`.

use std::sync::Arc;

use juggler_adapter::moving::NotesChoice;

use crate::error::AppResult;
use crate::features::handover::moving::{
    MoveDoneInfo, MoveOfferInfo, MoveRecordInfo, execute, moves, offer, undo,
};
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// Tauri: the Move offer for the switch screen.
///
/// # Errors
///
/// As [`offer`].
#[tauri::command]
pub async fn switch_move_offer(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<MoveOfferInfo> {
    crate::ipc::blocking(state.inner(), "check the move", move |state| {
        offer(state, &session_id, &target_profile_id)
    })
    .await
}

/// Tauri: moves the conversation (the user's click on Move).
///
/// # Errors
///
/// As [`execute`].
#[tauri::command]
pub async fn switch_move_execute(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    take_notes: Option<bool>,
) -> AppResult<MoveDoneInfo> {
    let notes = if take_notes == Some(true) {
        NotesChoice::TakeSource
    } else {
        NotesChoice::KeepTarget
    };
    crate::ipc::blocking(state.inner(), "move", move |state| {
        let done = execute(state, &session_id, &target_profile_id, notes)?;
        let _ = crate::features::sessions::read_model::emit_sessions(&app, state);
        Ok(done)
    })
    .await
}

/// Tauri: undoes a move (the user's click on Undo).
///
/// # Errors
///
/// As [`undo`].
#[tauri::command]
pub async fn switch_move_undo(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    move_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "undo the move", move |state| {
        let said = undo(state, &move_id)?;
        let _ = crate::features::sessions::read_model::emit_sessions(&app, state);
        Ok(said)
    })
    .await
}

/// Tauri: the moves AI Juggler knows of (Undo, and one left part way),
/// newest first, a page at a time.
///
/// # Errors
///
/// A cursor this app didn't make.
#[tauri::command]
pub async fn switch_moves(
    state: tauri::State<'_, Arc<AppState>>,
    page: Option<PageRequest>,
) -> AppResult<Page<MoveRecordInfo>> {
    let page = page.unwrap_or_default();
    crate::ipc::blocking(state.inner(), "list moves", move |state| {
        crate::paging::page(
            moves(state),
            |row| SortKey::new(vec![-row.at_ms], row.move_id.clone()),
            &page,
        )
    })
    .await
}
