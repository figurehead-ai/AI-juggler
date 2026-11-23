//! ADR-0031, AJ-E2320: a move's hold on the profiles it keeps closed, in
//! this AI Juggler's memory and (by its marker) for every AI Juggler on
//! this Mac; split from `places/mod.rs` by feature.

use std::sync::Arc;

use super::hold;
use crate::state::AppState;

/// ADR-0031: holds `ids` closed for move `move_id` while it lives:
/// nothing in AI Juggler starts them or writes their records meanwhile.
/// AJ-E2320: in this AI Juggler and, by a marker in the shared move area
/// ([`hold`]), in every other on this Mac; the marker outlives a move
/// left part way (or a crash) until the move is committed or undone.
pub struct MoveHold {
    state: Arc<AppState>,
    ids: Vec<String>,
    move_id: String,
}

impl MoveHold {
    /// Holds `ids` for move `move_id`.
    ///
    /// # Errors
    ///
    /// Another move (of this AI Juggler or another, running or crashed)
    /// holds one of them, in words.
    pub fn take(state: &Arc<AppState>, ids: &[String], move_id: &str) -> Result<Self, String> {
        let mut now = state
            .places
            .moving
            .lock()
            .map_err(|_| hold::BUSY.to_owned())?;
        if ids.iter().any(|id| now.contains(id)) {
            return Err(hold::BUSY.to_owned());
        }
        hold::take(&hold::area_at(&state.data_dir), ids, move_id)?;
        now.extend(ids.iter().cloned());
        Ok(Self {
            state: Arc::clone(state),
            ids: ids.to_vec(),
            move_id: move_id.to_owned(),
        })
    }
}

impl Drop for MoveHold {
    fn drop(&mut self) {
        if let Ok(mut now) = self.state.places.moving.lock() {
            for id in &self.ids {
                now.remove(id);
            }
        }
        if hold::settled(&self.state.data_dir, &self.move_id) {
            hold::release(
                &hold::area_at(&self.state.data_dir),
                &self.ids,
                &self.move_id,
            );
        }
    }
}

/// Why `id` is held for a move, in words; `None`: it isn't. A move of
/// this AI Juggler running now, or of another one ([`HELD`]); a move left
/// part way or whose AI Juggler is gone ([`HELD_UNFINISHED`]).
#[must_use]
pub fn why_held(state: &Arc<AppState>, id: &str) -> Option<&'static str> {
    if state.places.moving.lock().is_ok_and(|now| now.contains(id)) {
        return Some(HELD);
    }
    why_held_on_disk(state, id)
}

/// [`why_held`] by the shared area's marker alone (called under the
/// in-memory hold's lock).
pub(crate) fn why_held_on_disk(state: &Arc<AppState>, id: &str) -> Option<&'static str> {
    match hold::held(&hold::area_at(&state.data_dir), id) {
        hold::Held::Free => None,
        hold::Held::Moving(Some(marker)) if marker.mine() => Some(HELD_UNFINISHED),
        hold::Held::Moving(_) => Some(HELD),
        hold::Held::Crashed(_) => Some(HELD_UNFINISHED),
    }
}

/// Whether a move holds `id` closed now (of any AI Juggler, running or
/// crashed).
#[must_use]
pub fn held_for_move(state: &Arc<AppState>, id: &str) -> bool {
    why_held(state, id).is_some()
}

/// AJ-E2325: whether a move of this AI Juggler runs now (its in-memory
/// hold; a crashed move's marker on disk doesn't count).
#[must_use]
pub fn moving_here(state: &Arc<AppState>) -> bool {
    state
        .places
        .moving
        .lock()
        .map_or(true, |now| !now.is_empty())
}

/// Whether any move (of any AI Juggler) holds a profile now.
#[must_use]
pub fn any_move(state: &Arc<AppState>) -> bool {
    state.places.moving.lock().is_ok_and(|now| !now.is_empty())
        || hold::any_held(&hold::area_at(&state.data_dir))
}

/// The words for a place a move holds.
pub(crate) const HELD: &str = "a switch is moving conversations into or out of this profile; it starts again by itself when the move is done";

/// The words for a place a move that didn't finish holds.
pub(crate) const HELD_UNFINISHED: &str = "a switch's move into or out of this profile didn't finish, so AI Juggler keeps it closed; undo the move to open it again";
