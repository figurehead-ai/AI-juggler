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

/// Each live session's subagents. The tick reads transcripts into its own
/// map; readers see the last published view, so a read in progress never
/// hides a count.
#[derive(Debug, Default)]
pub struct Subtasks {
    /// The tick's own: each live session's transcript, as read so far.
    work: Mutex<HashMap<SessionId, (PathBuf, SubtaskTail)>>,
    /// What the tick last published, per live session.
    view: Mutex<HashMap<SessionId, SubtaskLog>>,
}

impl Subtasks {
    /// How many of a session's subagents are still running; `None` when
    /// its transcript hasn't been read (not live, or no transcript yet).
    #[must_use]
    pub fn running(&self, session: &SessionId) -> Option<u64> {
        let view = self.view.lock().ok()?;
        Some(view.get(session)?.running() as u64)
    }

    fn log(&self, session: &SessionId) -> Option<SubtaskLog> {
        self.view.lock().ok()?.get(session).cloned()
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

/// Reads what each live session's transcript gained and publishes it;
/// forgets sessions that ended. True when a running count changed (the
/// caller emits).
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
    // Only the tick takes `work` (one tick at a time); readers use `view`.
    let Ok(mut work) = state.subtasks.work.lock() else {
        return false;
    };
    let mut previous = std::mem::take(&mut *work);
    let mut changed = false;
    let mut updates: Vec<(SessionId, SubtaskLog)> = Vec::new();
    for (session, path) in live {
        let (old_path, mut tail) = previous
            .remove(&session)
            .unwrap_or_else(|| (path.clone(), SubtaskTail::default()));
        let before = tail.log().running();
        if old_path != path {
            tail = SubtaskTail::default();
        }
        // No transcript (yet, or any more): not read, so not counted —
        // never a guessed 0.
        let Ok(read) = tail.read(&path) else {
            changed |= before > 0;
            continue;
        };
        changed |= tail.log().running() != before;
        if read || old_path != path || !state.subtasks.published(&session) {
            updates.push((session.clone(), tail.log().clone()));
        }
        work.insert(session, (path, tail));
    }
    // Sessions that ended or went away.
    changed |= previous.values().any(|(_, tail)| tail.log().running() > 0);
    let live_ids: std::collections::HashSet<SessionId> = work.keys().cloned().collect();
    drop(work);
    if let Ok(mut view) = state.subtasks.view.lock() {
        view.retain(|session, _| live_ids.contains(session));
        view.extend(updates);
    }
    changed
}

impl Subtasks {
    fn published(&self, session: &SessionId) -> bool {
        self.view
            .lock()
            .is_ok_and(|view| view.contains_key(session))
    }
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
        if juggler_adapters::is_compressed(&path) {
            // Bounded by `SESSION_FILE_MAX`.
            let bytes = juggler_adapters::read_session_file(&path).ok()?;
            return Some(juggler_adapters::subtasks_of(&bytes));
        }
        // 16 MiB at a time: memory stays bounded by what the log keeps.
        let mut tail = SubtaskTail::default();
        tail.read_all(&path).ok()?;
        Some(tail.log().clone())
    });
    Ok(dto::SubtasksInfo::new(log.as_ref(), live))
}
