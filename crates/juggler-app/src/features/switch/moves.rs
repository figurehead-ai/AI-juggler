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

/// Tauri: the Move offer for the switch screen, into the App the user
/// picked (`target_place_id`, AJ-E2317; none: a terminal's folder).
///
/// # Errors
///
/// As [`offer`].
#[tauri::command]
pub async fn switch_move_offer(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    target_place_id: Option<String>,
) -> AppResult<MoveOfferInfo> {
    crate::ipc::blocking(state.inner(), "check the move", move |state| {
        offer(
            state,
            &session_id,
            &target_profile_id,
            target_place_id.as_deref(),
        )
    })
    .await
}

/// Tauri: moves the conversation (the user's click on Move) into the App
/// the user picked (`target_place_id`, AJ-E2317: by its id end to end).
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
    target_place_id: Option<String>,
    take_notes: Option<bool>,
) -> AppResult<MoveDoneInfo> {
    let notes = if take_notes == Some(true) {
        NotesChoice::TakeSource
    } else {
        NotesChoice::KeepTarget
    };
    crate::ipc::blocking(state.inner(), "move", move |state| {
        // 0.4.9: the switch screen shows how far it is.
        let steps = app.clone();
        let tell: crate::features::handover::moving::Tell = Box::new(move |progress| {
            use crate::events::{AppEvent, EmitEvent as _};
            steps.emit_event(AppEvent::MoveProgress, progress.clone());
        });
        let target = (
            session_id.as_str(),
            target_profile_id.as_str(),
            target_place_id.as_deref(),
        );
        let done = execute(state, target, notes, tell)?;
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
) -> AppResult<crate::features::handover::moving::MoveUndoneInfo> {
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

/// Tauri: AJ-E2328, the ownership check for Settings › Desktop apps: each
/// app's version and whether moves were checked on it, the conversations
/// whose worktree another profile owns (each with what Repair quits), and
/// the worktrees another profile's cleanup has seen. Reads only.
///
/// # Errors
///
/// None of its own (the blocking pool's).
#[tauri::command]
pub async fn switch_ownership(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<crate::features::handover::moving::repair::OwnershipInfo> {
    crate::ipc::blocking(state.inner(), "check worktree ownership", move |state| {
        Ok(crate::features::handover::moving::repair::findings(
            state,
            &crate::features::handover::moving::Real::quiet(),
        ))
    })
    .await
}

/// Tauri: AJ-E2328, Repair (the user's click, after the sheet naming the
/// profiles it quits): the worktree's ownership moved to the profile that
/// holds the conversation, by the move's own path.
///
/// # Errors
///
/// As [`crate::features::handover::moving::repair::repair`].
#[tauri::command]
pub async fn switch_ownership_repair(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
) -> AppResult<crate::features::handover::moving::repair::OwnershipRepairedInfo> {
    crate::ipc::blocking(state.inner(), "repair worktree ownership", move |state| {
        let done = crate::features::handover::moving::repair::repair(state, &key)?;
        let _ = crate::features::sessions::read_model::emit_sessions(&app, state);
        Ok(done)
    })
    .await
}
