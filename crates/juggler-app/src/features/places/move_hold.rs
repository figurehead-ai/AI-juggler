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
/// this AI Juggler running now, or of another one (`HELD`); a move left
/// part way or whose AI Juggler is gone (`HELD_UNFINISHED`).
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

/// AJ-E2326 R1: how long a conversation stays held once its move was
/// refused or undone: the move opens it again where it was meanwhile, and
/// a start here would make a second writer on it.
pub const SETTLE_MS: i64 = 30_000;

/// Words: a move of the conversation runs.
pub(crate) const CONVERSATION_MOVING: &str = "this conversation is being moved to another account; it opens where it lands when the move is done";
/// Words: its move was just refused or undone, and it is opening again
/// where it was.
pub(crate) const CONVERSATION_SETTLING: &str =
    "this conversation's move just stopped and it is opening again where it was; continue it there";
/// Words: a move of it didn't finish.
pub(crate) const CONVERSATION_UNFINISHED: &str =
    "a move of this conversation didn't finish; undo the move (Moves) to open it again";

/// A conversation's mark in the shared move area: the move (with its
/// owner) and, once it was refused or undone, until when it settles.
#[derive(serde::Serialize, serde::Deserialize)]
struct ConversationMark {
    #[serde(flatten)]
    marker: hold::Marker,
    #[serde(default)]
    until_ms: Option<i64>,
}

fn conversation_mark(state: &AppState, conversation: &str) -> std::path::PathBuf {
    hold::area_at(&state.data_dir)
        .join("conversations")
        .join(hold::key(conversation))
}

fn write_mark(path: &std::path::Path, mark: &ConversationMark) {
    let Some(dir) = path.parent() else {
        return;
    };
    let next = path.with_extension("next");
    let written = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(&next, serde_json::to_vec(mark).unwrap_or_default()))
        .and_then(|()| std::fs::rename(&next, path));
    if let Err(why) = written {
        tracing::warn!(target: "juggler::move", "couldn't mark the conversation held: {why}");
    }
}

/// AJ-E2326 R1: a move's hold on its conversation, for every AI Juggler
/// on this Mac (a mark in the shared move area beside the profiles'
/// markers): no start path continues it here while the move runs, nor for
/// [`SETTLE_MS`] after it was refused or undone. A move that succeeded
/// lets it go; one left part way keeps it ([`conversation_held`]).
pub struct ConversationHold {
    state: Arc<AppState>,
    conversation: String,
    move_id: String,
}

impl ConversationHold {
    /// Holds `conversation` for move `move_id` (best effort: the
    /// profiles' hold is the move's own safety).
    #[must_use]
    pub fn take(state: &Arc<AppState>, conversation: &str, move_id: &str) -> Self {
        if let Some(owner) = hold::me() {
            mark_conversation(state, conversation, move_id, owner, None);
        }
        Self {
            state: Arc::clone(state),
            conversation: conversation.to_owned(),
            move_id: move_id.to_owned(),
        }
    }
}

impl Drop for ConversationHold {
    fn drop(&mut self) {
        use juggler_handover::moving::Phase;
        let path = conversation_mark(&self.state, &self.conversation);
        let phase = hold::open_move(&self.state.data_dir, &self.move_id).map(|t| t.manifest.phase);
        match phase {
            Ok(Phase::Undone) => settle_conversation(
                &self.state,
                &self.conversation,
                &self.move_id,
                juggler_core::model::Timestamp::now().millis() + SETTLE_MS,
            ),
            // Part way: held until it is undone (by the person, or the
            // recovery at the next start).
            Ok(Phase::BackedUp | Phase::Applying | Phase::Applied | Phase::Undoing) => {}
            _ => {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

/// Marks `conversation` settling until `until_ms` after move `move_id`
/// was refused or undone.
pub fn settle_conversation(state: &AppState, conversation: &str, move_id: &str, until_ms: i64) {
    let owner = hold::me().unwrap_or(crate::features::ownership::ProcId {
        pid: 0,
        start_micros: 0,
    });
    mark_conversation(state, conversation, move_id, owner, Some(until_ms));
}

/// Writes `conversation`'s mark for move `move_id`, by `owner`: held
/// while the move runs (`until_ms` `None`), or settling until `until_ms`.
fn mark_conversation(
    state: &AppState,
    conversation: &str,
    move_id: &str,
    owner: crate::features::ownership::ProcId,
    until_ms: Option<i64>,
) {
    let mark = ConversationMark {
        marker: hold::Marker {
            move_id: move_id.to_owned(),
            place: conversation.to_owned(),
            owner_pid: owner.pid,
            owner_start_micros: u64::try_from(owner.start_micros).unwrap_or(u64::MAX),
        },
        until_ms,
    };
    write_mark(&conversation_mark(state, conversation), &mark);
}

/// AJ-E2326 R1: why `conversation` can't be started here now, in words;
/// `None`: it can. A move of it runs (`CONVERSATION_MOVING`), was
/// refused or undone moments ago (`CONVERSATION_SETTLING`), or was left
/// part way (`CONVERSATION_UNFINISHED`). Every start path asks it (the
/// sessions' one start, `sessions::spawn::start_session`).
#[must_use]
pub fn conversation_held(state: &AppState, conversation: &str) -> Option<&'static str> {
    use juggler_handover::moving::Phase;
    let path = conversation_mark(state, conversation);
    let mark: ConversationMark = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
    let let_go = || {
        let _ = std::fs::remove_file(&path);
        None
    };
    if let Some(until) = mark.until_ms {
        return if juggler_core::model::Timestamp::now().millis() < until {
            Some(CONVERSATION_SETTLING)
        } else {
            let_go()
        };
    }
    let alive = mark.marker.owner_alive();
    match hold::open_move(&state.data_dir, &mark.marker.move_id).map(|t| t.manifest.phase) {
        Ok(Phase::Committed | Phase::Verified | Phase::Undone) => let_go(),
        Ok(Phase::Planned) | Err(_) if !alive => let_go(),
        Ok(Phase::Planned) | Err(_) => Some(CONVERSATION_MOVING),
        Ok(_) if alive => Some(CONVERSATION_MOVING),
        Ok(_) => Some(CONVERSATION_UNFINISHED),
    }
}
