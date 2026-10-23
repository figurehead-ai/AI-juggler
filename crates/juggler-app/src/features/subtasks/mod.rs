//! AJ-E1520: the subagents each session started (I-08) and what became of
//! them, read from the session's transcript by the tool's parser
//! (`juggler_adapters::subtasks`); nothing here knows which tool wrote it.
//!
//! A live session's transcript is read on the 2 s tick, only what was
//! appended since the last read; the status row shows how many subagents
//! are still running, and the inspector lists them all.

pub mod commands;
pub mod dto;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use juggler_adapters::{SubtaskLog, SubtaskTail};
use juggler_core::model::SessionId;

use crate::commands::{is_terminal, lock_storage_arc};
use crate::error::AppResult;
use crate::state::AppState;

/// Each live session's transcript, as read so far.
#[derive(Debug, Default)]
pub struct Subtasks {
    tails: Mutex<HashMap<SessionId, (PathBuf, SubtaskTail)>>,
}

impl Subtasks {
    /// How many of a session's subagents are still running; `None` when
    /// its transcript hasn't been read (not live, or no transcript yet).
    #[must_use]
    pub fn running(&self, session: &SessionId) -> Option<u64> {
        let tails = self.tails.lock().ok()?;
        let (_, tail) = tails.get(session)?;
        Some(tail.log().running() as u64)
    }

    fn log(&self, session: &SessionId) -> Option<SubtaskLog> {
        let tails = self.tails.lock().ok()?;
        tails.get(session).map(|(_, tail)| tail.log().clone())
    }
}

/// The status row's subagent count (I-08): how many of each session's
/// subagents are still running, where its transcript has been read.
#[must_use]
pub fn counted(
    state: &Arc<AppState>,
    mut rows: Vec<crate::features::sessions::dto::InstanceInfo>,
) -> Vec<crate::features::sessions::dto::InstanceInfo> {
    for row in &mut rows {
        row.subtask_count = state.subtasks.running(&SessionId::new(row.id.as_str()));
    }
    rows
}

/// Reads what each live session's transcript gained; forgets sessions
/// that ended. True when a running count changed (the caller emits).
pub fn tick(state: &Arc<AppState>) -> bool {
    let live: Vec<(SessionId, PathBuf)> = {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        storage
            .sessions()
            .list(None)
            .unwrap_or_default()
            .into_iter()
            .filter(|record| !is_terminal(record.status))
            .filter_map(|record| Some((record.id, record.transcript_ref?.path)))
            .collect()
    };
    // The transcript is read outside the lock: a slow disk never holds up
    // the status row.
    let mut current: HashMap<SessionId, (PathBuf, SubtaskTail)> = match state.subtasks.tails.lock()
    {
        Ok(mut tails) => std::mem::take(&mut *tails),
        Err(_) => return false,
    };
    let mut changed = false;
    let mut next = HashMap::with_capacity(live.len());
    for (session, path) in live {
        let (old_path, mut tail) = current
            .remove(&session)
            .unwrap_or_else(|| (path.clone(), SubtaskTail::default()));
        if old_path != path {
            tail = SubtaskTail::default();
        }
        let before = tail.log().running();
        // No transcript (yet, or any more): not read, so not counted —
        // never a guessed 0.
        if tail.read(&path).is_err() {
            changed |= before > 0;
            continue;
        }
        changed |= tail.log().running() != before;
        next.insert(session, (path, tail));
    }
    // Sessions that ended or went away.
    changed |= current.values().any(|(_, tail)| tail.log().running() > 0);
    if let Ok(mut tails) = state.subtasks.tails.lock() {
        *tails = next;
    }
    changed
}

/// Every subagent session `session_id` started, for the inspector: from
/// the live read when there is one, else from its transcript on disk.
///
/// # Errors
///
/// The session is unknown.
pub fn list(state: &Arc<AppState>, session_id: &str) -> AppResult<dto::SubtasksInfo> {
    let session = SessionId::new(session_id);
    let record = lock_storage_arc(state)?.sessions().get(&session)?;
    let live = !is_terminal(record.status);
    let log = state.subtasks.log(&session).or_else(|| {
        let path = record.transcript_ref.as_ref()?.path.clone();
        let bytes = juggler_adapters::read_session_file(&path).ok()?;
        Some(juggler_adapters::subtasks_of(&bytes))
    });
    Ok(dto::SubtasksInfo::new(log.as_ref(), live))
}
