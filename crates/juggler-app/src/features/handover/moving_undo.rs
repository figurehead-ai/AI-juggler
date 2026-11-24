//! Undo of a recorded move (or a repair, 0.5.1 F91) by its id: both
//! profiles quit, the engine's Undo, both started again, the conversation
//! opened where it was. Moved out of `moving.rs` for its size (0.5.1).

use std::sync::Arc;

use juggler_handover::moving::{Lock, Stopped, Transaction};

use super::{
    KnownPlace, MoveSystem, MoveUndoneInfo, Real, door, engine, hold, lock, pause, place_name,
    prime_links, quiet, refused, reopen, repair, start_again, still_down, stop_places,
    writing_of_move,
};
use crate::error::AppResult;
use crate::state::AppState;

/// Undoes move `move_id`: both profiles quit, the engine's Undo (backups
/// checked first, the work done since kept aside), both started again, the
/// conversation opened where it was. Returns where the work done since
/// was kept.
///
/// # Errors
///
/// A backup doesn't match (nothing changed), a profile wouldn't quit, or
/// the restore failed (the journal says how far).
pub fn undo(state: &Arc<AppState>, move_id: &str) -> AppResult<MoveUndoneInfo> {
    // 0.5.1 F93: its reopen's link sheet before anything quits.
    prime_links(state);
    undo_with(state, move_id, &Real::quiet())
}

pub(crate) fn undo_with(
    state: &Arc<AppState>,
    move_id: &str,
    system: &dyn MoveSystem,
) -> AppResult<MoveUndoneInfo> {
    undo_as(state, move_id, system, true)
}

/// The recorded move `move_id` (its id checked first: never a path).
pub(super) fn open_by_id(state: &Arc<AppState>, move_id: &str) -> AppResult<Transaction> {
    if move_id.is_empty()
        || !move_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(refused("that isn't a move's id"));
    }
    hold::open_move(&state.data_dir, move_id).map_err(engine)
}

/// What Undo and Finish take before they touch a recorded move `txn`: the
/// lock of its files, its hold in every AI Juggler, and its tool's adapter.
pub(super) fn take_move(
    state: &Arc<AppState>,
    txn: &Transaction,
) -> AppResult<(
    Lock,
    crate::features::places::MoveHold,
    juggler_adapter::adapter::SharedAdapter,
)> {
    let lock = lock(state, &txn.manifest.plan.roots)?;
    let hold =
        crate::features::places::MoveHold::take(state, &txn.manifest.places, &txn.manifest.id)
            .map_err(refused)?;
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(
            txn.manifest.tool.as_str(),
        ))
        .ok_or_else(|| refused("this tool's conversations travel as a copy"))?;
    Ok((lock, hold, adapter))
}

/// [`undo_with`]; `reopen`: the conversation opened where it was after
/// (the user's Undo), not by startup recovery (AJ-E2321).
#[allow(clippy::too_many_lines, reason = "Undo, step by step, as the move's")]
pub(crate) fn undo_as(
    state: &Arc<AppState>,
    move_id: &str,
    system: &dyn MoveSystem,
    reopen: bool,
) -> AppResult<MoveUndoneInfo> {
    let mut txn = open_by_id(state, move_id)?;
    // A later move (in any state but undone) that touches the same files
    // owns them now: undoing this one would put old copies over it.
    if door::moved_again(&hold::all_moves(&state.data_dir), &txn) {
        return Err(refused(
            "these conversations were moved again since; undo the later move first, or move them back",
        ));
    }
    // Every profile it stopped must still be one AI Juggler knows, to stop
    // it again: one it can't name might be running and writing.
    let mut places: Vec<(String, KnownPlace)> = Vec::new();
    for id in &txn.manifest.places {
        let known = crate::features::places::with_book(state, |book| book.places.get(id).cloned())
            .ok()
            .flatten()
            .ok_or_else(|| {
                refused("an app profile of this move isn't one AI Juggler knows any more; add it again (Settings › Desktop apps), then Undo")
            })?;
        places.push((id.clone(), known));
    }
    let (_lock, _hold, adapter) = take_move(state, &txn)?;
    let _held = crate::features::places::ConversationHold::take(
        state,
        &txn.manifest.request.conversation,
        move_id,
    );
    let writing = writing_of_move(&adapter, &txn)?;
    let both = (&places[..], &txn.manifest.request);
    let why_not = quiet(
        state,
        both,
        &txn.manifest.accounts,
        system,
        "Undo",
        &writing,
    );
    if !why_not.is_empty() {
        return Err(refused(juggler_handover::moving::words(&why_not)));
    }
    // R4: a refused Undo says "nothing was undone", not the Move's words.
    // AJ-E2319 R5: what kept running is told, as the move tells it.
    let stopping = stop_places(state, both, system, ("undone", Some(&txn)), &writing)?;
    let (stopped_places, kept_running) = (stopping.stopped, stopping.notes);
    if let Err(started) = still_down(state, &stopped_places, system) {
        start_again(state, &stopped_places, &txn.manifest.request, system);
        return Err(refused(format!("{started}, so nothing was undone")));
    }
    let result = txn.undo(
        &Stopped::checked(),
        juggler_core::model::Timestamp::now().millis(),
    );
    let kept = match result {
        Ok(kept) => {
            // Nothing is left to restore from them (0.4.9).
            let _ = txn.drop_backups();
            start_again(state, &stopped_places, &txn.manifest.request, system);
            kept
        }
        Err(error) => {
            // Left closed: Undo again finishes it on a quiet folder.
            pause(state, &stopped_places);
            return Err(refused(format!(
                "{error}. Both profiles stay closed; Undo the move again to finish"
            )));
        }
    };
    // Keep alive, paused by a move left part way, watches again.
    let _ = crate::features::places::with_book(state, |book| {
        for place in &stopped_places {
            if let Some(known) = book.places.get_mut(&place.id)
                && known
                    .paused
                    .as_deref()
                    .is_some_and(|why| why.starts_with("a move stopped"))
            {
                known.paused = None;
            }
        }
    });
    // 0.5.1 F91: a repair's Undo gives the worktree back; it opens
    // nothing (no conversation moved).
    if let Some(line) = txn.notes(repair::REPAIR).pop() {
        let worktree = line["worktree"].as_str().unwrap_or("its worktree");
        let owner = places.first().map_or_else(
            || "the App it belonged to".to_owned(),
            |(_, p)| place_name(state, p),
        );
        return Ok(MoveUndoneInfo {
            words: format!(
                "Repair undone: the worktree {worktree} belongs to {owner} again. Its files weren't touched."
            ),
            kept: kept.display().to_string(),
            notes: kept_running,
        });
    }
    // Opened where it was, and seen open there (AJ-E2326 R2).
    let mut words = "Undone: the conversation is back where it was. Anything done since the move is kept aside; Copy details says where.".to_owned();
    if let (true, Some((id, place))) = (reopen, places.first()) {
        let seen = reopen::reopen_where_it_was(
            state,
            (id, place),
            &txn.manifest.request,
            writing.mover,
            system,
        );
        let _ = txn.timeline("reopened", 0, serde_json::json!({ "seen": seen }));
        if seen.is_none() {
            let name = place_name(state, place);
            words = format!(
                "Undone: the conversation is back where it was, but AI Juggler couldn't see it open in {name}. Open it from {name}. Anything done since the move is kept aside; Copy details says where."
            );
        }
    }
    // R4: no path in the words (Copy details has it).
    Ok(MoveUndoneInfo {
        words,
        kept: kept.display().to_string(),
        notes: kept_running,
    })
}
