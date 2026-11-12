//! 0.4.1: the conversation index, saved — what each conversation file
//! said when it was last read (its size and time, and its summary), in
//! one file under AI Juggler's data folder (`conversation-list/index.cli`),
//! restored before the first pass so a start reads only the files that
//! changed since. The go sitting's startup minute: every start read every
//! transcript on the Mac again (a long Codex history, compressed rollouts
//! included), at 100 % CPU for over half a minute.
//!
//! A cache, not a record (ADR-0023): one of the local caches, listed in
//! Settings, cleared or turned off there; a file it can't read, or one of
//! another format, is dropped and every file read again. Nothing is
//! written while the cache is off or the disk nearly full; written whole
//! and readable by this user only.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{Conversation, Entry};
use crate::features::cache::{self, About, LocalCache, Trim, folder::FolderCache, stamp};
use crate::state::AppState;

/// The folder under AI Juggler's data folder.
pub const DIR: &str = "conversation-list";
/// The cache's id.
pub const CACHE: &str = "conversation-list";
/// The saved index's format: another is not read.
pub const FORMAT: u32 = 1;
/// Its file's extension.
const EXTENSION: &str = "cli";
/// At most this big (a Mac with tens of thousands of conversations).
const MAX_BYTES: u64 = 256 * 1024 * 1024;
/// A pass that changed something saves at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(60);

/// Whether the saved index was restored, and when it was last saved.
#[derive(Debug, Default)]
pub struct Kept {
    restored: bool,
    /// The restore found a saved list (a start that reads only changes).
    restored_any: bool,
    saved_at: Option<Instant>,
}

/// One file as it was last read.
#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    path: PathBuf,
    len: u64,
    /// Its modification time, exactly (seconds and nanoseconds since the
    /// epoch): a file is read again only when its time differs.
    modified: Option<(u64, u32)>,
    complete: bool,
    conversation: Option<Conversation>,
}

/// The conversation list, for the cache's registry.
#[must_use]
pub fn local_cache(state: &Arc<AppState>) -> Arc<dyn LocalCache> {
    Arc::new(FolderCache::new(
        About {
            id: CACHE,
            name: "Conversation list",
            speeds_up: "Your conversations are listed as soon as AI Juggler starts; only the files that changed since are read.",
            slower: "Each start reads every conversation file again, which takes a while on a long history.",
            trim: Trim::WholeOnly,
            own_switch: None,
            max_entries: Some(1),
            shaped_by: &[],
        },
        state.data_dir.join(DIR),
        EXTENSION,
        &[],
        cache::with_state(state, |_| (FORMAT, 0)),
    ))
}

fn path_of(state: &AppState) -> PathBuf {
    state.data_dir.join(DIR).join(format!("index.{EXTENSION}"))
}

fn stamp_of(state: &AppState) -> stamp::Stamp<'static> {
    stamp::Stamp {
        cache: CACHE,
        format: FORMAT,
        settings: readers_print(state),
        source: "index",
    }
}

/// 0.4.3: a print of every tool's conversation reader version
/// (`ToolTranscripts::conversation_reader_version`): a saved index read
/// under another reader is not restored, so every file is read again
/// once. 0.4.2 changed what Claude Code's and Codex's summaries say (who
/// started a session) without it, and the index restored 0.4.1's for
/// every unchanged file: desktop sessions stayed under CLI Apps.
fn readers_print(state: &AppState) -> u64 {
    let mut ids = state.registry.ids();
    ids.sort();
    let parts: Vec<String> = ids
        .iter()
        .filter_map(|id| {
            let adapter = state.registry.get(id)?;
            Some(format!("{id}={}", adapter.conversation_reader_version()))
        })
        .collect();
    stamp::settings_print(&parts.iter().map(String::as_str).collect::<Vec<_>>())
}

fn exact(time: Option<SystemTime>) -> Option<(u64, u32)> {
    let since = time?.duration_since(UNIX_EPOCH).ok()?;
    Some((since.as_secs(), since.subsec_nanos()))
}

fn from_exact(time: Option<(u64, u32)>) -> Option<SystemTime> {
    time.map(|(secs, nanos)| UNIX_EPOCH + Duration::new(secs, nanos))
}

/// Restores the saved index into the (still empty) index, once.
pub fn restore_once(state: &Arc<AppState>) {
    let Ok(mut kept) = state.conversations.saved.lock() else {
        return;
    };
    if kept.restored {
        return;
    }
    kept.restored = true;
    drop(kept);
    let gate = state.cache.gate();
    let Some(bytes) = stamp::read(&gate, &path_of(state), &stamp_of(state), MAX_BYTES) else {
        return;
    };
    let Ok(saved) = serde_json::from_slice::<Vec<Saved>>(&bytes) else {
        return;
    };
    if let Ok(mut kept) = state.conversations.saved.lock() {
        kept.restored_any = !saved.is_empty();
    }
    let Ok(mut files) = state.conversations.files.lock() else {
        return;
    };
    for one in saved {
        // A file a pass has read already is newer than what was saved.
        files.entry(one.path).or_insert(Entry {
            len: one.len,
            modified: from_exact(one.modified),
            conversation: one.conversation,
            complete: one.complete,
            counted_at: None,
        });
    }
}

/// Whether a saved list was restored, or is there to restore: a start
/// with one reads only what changed (seconds), without one every file.
#[must_use]
pub fn has_saved(state: &AppState) -> bool {
    state.conversations.saved.lock().is_ok_and(|kept| {
        if kept.restored {
            kept.restored_any
        } else {
            path_of(state).is_file()
        }
    })
}

/// Saves the index when the last save was over `SAVE_EVERY` ago (a pass
/// changed something).
pub fn save_soon(state: &Arc<AppState>) {
    let due = state
        .conversations
        .saved
        .lock()
        .is_ok_and(|kept| kept.saved_at.is_none_or(|at| at.elapsed() >= SAVE_EVERY));
    if due {
        save(state);
    }
}

/// Saves the index now (nothing while the cache is off).
pub fn save(state: &Arc<AppState>) {
    let snapshot: Vec<Saved> = {
        let Ok(files) = state.conversations.files.lock() else {
            return;
        };
        files
            .iter()
            .map(|(path, entry)| Saved {
                path: path.clone(),
                len: entry.len,
                modified: exact(entry.modified),
                complete: entry.complete,
                conversation: entry.conversation.clone(),
            })
            .collect()
    };
    let Ok(bytes) = serde_json::to_vec(&snapshot) else {
        return;
    };
    let gate = state.cache.gate();
    match stamp::write(&gate, &path_of(state), &stamp_of(state), &bytes) {
        Ok(_) => {
            if let Ok(mut kept) = state.conversations.saved.lock() {
                kept.saved_at = Some(Instant::now());
            }
        }
        Err(error) => tracing::warn!("the conversation list wasn't saved: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.4.3: an index saved before a reader changed isn't restored: 0.4.2
    /// saved with no print (0), and one saved under every reader's first
    /// version prints otherwise than today's.
    #[test]
    fn a_saved_index_from_another_reader_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::init(dir.path().join("data")).unwrap();
        let now = readers_print(&state);
        assert_ne!(now, 0, "0.4.2's saved index (settings 0) is not restored");
        let mut ids = state.registry.ids();
        ids.sort();
        let first: Vec<String> = ids.iter().map(|id| format!("{id}=1")).collect();
        let before = stamp::settings_print(&first.iter().map(String::as_str).collect::<Vec<_>>());
        assert_ne!(
            now, before,
            "a reader's new version invalidates the saved index"
        );
        for (tool, version) in [("claude-code", 3), ("codex", 2)] {
            let adapter = state
                .registry
                .get(&juggler_core::HarnessId::new(tool))
                .expect(tool);
            assert_eq!(
                adapter.conversation_reader_version(),
                version,
                "{tool}'s reader"
            );
        }
    }
}
