//! AJ-E2326 R2: a refused or undone move leaves the conversation where it
//! was, open. Before, a refused move started the source profile again but
//! didn't reopen the conversation, so it read as not live there, and the
//! very panel the person retried from offered "Continue in AI Juggler" (a
//! terminal session on the source account) in place of the move.
//!
//! The source profile is started again (when the move had quit it) and
//! the conversation is opened in it by the link the move sends the target
//! (the tool's own resume link, to the profile's own process); then the
//! conversation's own evidence that it opened there is waited for
//! (AJ-E2322's check: its session in the profile's tree, or the app adding
//! to its history), capped, woken by file events.

use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::moving::{InPlaceMove, MoveRequest};

use super::quit::Writing;
use super::{KnownPlace, MoveSystem, open_in, timeline};
use crate::state::AppState;

/// Opens `request`'s conversation again in the source profile `(id,
/// place)` and waits for it to show open there. Returns what showed it
/// open; `None`: not seen within the cap (or no link to open it by).
pub(super) fn reopen_where_it_was(
    state: &Arc<AppState>,
    (id, place): (&str, &KnownPlace),
    request: &MoveRequest,
    mover: &dyn InPlaceMove,
    system: &dyn MoveSystem,
) -> Option<String> {
    let link = mover.resume_link(&request.conversation)?;
    let pid = match open_in(state, (id, place), &link, request, (system, &mut || {})) {
        Ok(pid) => pid,
        Err(why) => {
            tracing::warn!(target: "juggler::move", "the conversation wasn't opened again where it was: {why}");
            return None;
        }
    };
    let source = Writing {
        mover,
        dirs: vec![request.source.tool_dir.clone()],
        conversations: vec![request.conversation.clone()],
    };
    let folders: Vec<PathBuf> = std::iter::once(request.source.tool_dir.clone())
        .chain(request.source.host_dir.clone())
        .collect();
    let seen = system.wait_open(&folders, timeline::OPEN_CAP, &mut || {
        timeline::seen_open(state, (pid, place), &source, None, system)
    });
    if seen.is_none() {
        tracing::info!(target: "juggler::move", "the conversation was sent to the source again but not seen open there");
    }
    seen
}
