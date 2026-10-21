//! AJ-E1529: every account's conversations, the ones AI Juggler started
//! and the ones it didn't, read from each tool's own files through its
//! adapter (`conversation_files`, `read_conversation`) and kept in memory.
//!
//! The index is keyed by file: a refresh lists each account's files, and
//! re-reads only those whose size or modification time changed. Nothing
//! is written: not into a setup, and not into AI Juggler's database
//! (there is no migration; a restart rebuilds it from the files).

pub mod commands;
pub mod dto;
pub mod live;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use juggler_adapter::conversation::ConversationSummary;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One conversation in the index: what its file says, and whose it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    /// What the file says about itself.
    pub summary: ConversationSummary,
    /// The account whose folder it's in.
    pub account_id: String,
    /// That account's tool.
    pub tool: String,
    /// The file.
    pub path: PathBuf,
    /// Its folder resolved on this Mac (as a running process reports
    /// its own), or as recorded when it's gone.
    pub folder: Option<PathBuf>,
}

/// A file as last read: its size and modification time decide whether
/// it's read again.
#[derive(Clone, Debug)]
struct Entry {
    len: u64,
    modified: Option<SystemTime>,
    conversation: Option<Conversation>,
}

/// The in-memory index (one per app).
#[derive(Debug, Default)]
pub struct ConversationIndex {
    files: Mutex<HashMap<PathBuf, Entry>>,
    /// `(tool, id)` of the conversations a running session is on.
    live: Mutex<std::collections::HashSet<(String, String)>>,
}

impl ConversationIndex {
    /// Every conversation now in the index, newest activity first.
    ///
    /// # Errors
    ///
    /// [`AppError::StatePoisoned`] when the index's lock is poisoned.
    pub fn all(&self) -> AppResult<Vec<Conversation>> {
        let files = self.files.lock().map_err(|_| AppError::StatePoisoned)?;
        let mut out: Vec<Conversation> = files
            .values()
            .filter_map(|entry| entry.conversation.clone())
            .collect();
        out.sort_by(|a, b| {
            b.summary
                .last_ms
                .cmp(&a.summary.last_ms)
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(out)
    }

    /// Whether a running session is on `(tool, id)`.
    #[must_use]
    pub fn is_live(&self, tool: &str, id: &str) -> bool {
        self.live
            .lock()
            .is_ok_and(|live| live.contains(&(tool.to_owned(), id.to_owned())))
    }
}

/// One refresh of the files and of which conversations are live; `true`
/// when anything the UI shows changed.
///
/// # Errors
///
/// As [`refresh_impl`].
pub fn tick(state: &Arc<AppState>) -> AppResult<bool> {
    let files_changed = refresh_impl(state)?;
    if files_changed {
        // New folders become projects without the UI asking.
        crate::commands::projects_backfill_impl(state)?;
    }
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let running = live::running_sessions(state, &accounts);
    let now = live::live_ids(&state.conversations.all()?, &running);
    let mut live = state
        .conversations
        .live
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let live_changed = *live != now;
    *live = now;
    Ok(files_changed || live_changed)
}

/// Brings the index up to date with every account's files: new and
/// changed files are read, gone ones dropped. `true` when anything
/// changed (the UI is told then, and only then).
///
/// # Errors
///
/// Storage errors listing the accounts; a poisoned lock.
pub fn refresh_impl(state: &Arc<AppState>) -> AppResult<bool> {
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut changed = false;
    for account in &accounts {
        let Some(adapter) = state.registry.get(&account.harness) else {
            continue;
        };
        for path in adapter.conversation_files(&account.state_dir) {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let (len, modified) = (meta.len(), meta.modified().ok());
            seen.push(path.clone());
            let fresh = state
                .conversations
                .files
                .lock()
                .map_err(|_| AppError::StatePoisoned)?
                .get(&path)
                .is_some_and(|entry| entry.len == len && entry.modified == modified);
            if fresh {
                continue;
            }
            // Read outside the lock: a big transcript never holds up a reader.
            // A conversation that records no time was last written when
            // its file was.
            let written = modified
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|since| i64::try_from(since.as_millis()).ok());
            let conversation = adapter
                .read_conversation(&path)
                .map(|mut summary| {
                    summary.last_ms = summary.last_ms.or(written);
                    summary
                })
                .map(|summary| Conversation {
                    folder: summary
                        .cwd
                        .as_ref()
                        .map(|cwd| std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.clone())),
                    summary,
                    account_id: account.id.to_string(),
                    tool: account.harness.to_string(),
                    path: path.clone(),
                });
            state
                .conversations
                .files
                .lock()
                .map_err(|_| AppError::StatePoisoned)?
                .insert(
                    path,
                    Entry {
                        len,
                        modified,
                        conversation,
                    },
                );
            changed = true;
        }
    }
    let mut files = state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let before = files.len();
    let seen: std::collections::HashSet<PathBuf> = seen.into_iter().collect();
    files.retain(|path, _| seen.contains(path));
    Ok(changed || files.len() != before)
}
