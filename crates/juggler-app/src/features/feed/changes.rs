//! AJ-E3713 R6a (0.5.2 G8): a post's diff stat: what the turn behind a
//! session's item changed, read from the session's own transcript
//! through its adapter ([`juggler_adapters::turn_changes`]: its edit and
//! write calls, never git in the user's projects).
//!
//! Only a tool whose adapter reads its transcripts' work
//! (`reports_activity`: Claude Code, Codex) is read; another tool's posts
//! carry no stat. A read is kept by the file's stamp, so a page read again
//! reads only what changed, and a page spends at most `PAGE_BUDGET` (250 ms) on
//! files it hasn't read (each read is at most 8 MB back from a file's
//! end, most a 256 KB window).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use juggler_adapters::turn_changes::{TurnChanges, turn_changes};
use juggler_core::model::{HarnessId, SessionId};

use super::dto::{FeedChangesInfo, FeedItemInfo};
use crate::state::AppState;

/// The kinds told as a turn's end: a session's or another app's
/// conversation's.
pub const TURN_KINDS: &[&str] = &[
    "finished", "ended", "stopped", "failed", "crashed", "worked",
];

/// The most reads kept.
const KEPT_MAX: usize = 512;

/// The most time a page spends reading transcripts it hasn't read
/// before (instant-and-paged): past it, the rest of the page's posts go
/// without a stat until a later read finds them kept or reads them.
const PAGE_BUDGET: std::time::Duration = std::time::Duration::from_millis(250);

/// A read, by its file and turn: the file's stamp when read, and what it
/// said.
type Kept = HashMap<(PathBuf, i64), (String, Option<FeedChangesInfo>)>;

static KEPT: LazyLock<Mutex<Kept>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Gives each turn's item of `items` its diff stat, when its tool's
/// transcript says one. Takes the storage lock for a moment (the
/// sessions' transcripts), so the caller holds none; a file that can't be
/// read is passed.
pub fn attach(state: &Arc<AppState>, items: &mut [FeedItemInfo]) {
    let wanted: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| TURN_KINDS.contains(&item.kind.as_str()) && read_here(state, item))
        .map(|(at, _)| at)
        .collect();
    if wanted.is_empty() {
        return;
    }
    let sessions = session_transcripts(state, wanted.iter().map(|at| &items[*at]));
    let started = std::time::Instant::now();
    for at in wanted {
        let item = &mut items[at];
        if let Some(path) = path_of(state, item, &sessions) {
            item.facts.changes = read(&path, item.at_ms, started.elapsed() < PAGE_BUDGET);
        }
    }
}

/// Whether the item's tool's transcripts are read for their work.
fn read_here(state: &AppState, item: &FeedItemInfo) -> bool {
    item.author.provider.as_deref().is_some_and(|provider| {
        state
            .registry
            .get(&HarnessId::new(provider))
            .is_some_and(|adapter| adapter.reports_activity())
    })
}

/// The sessions' linked transcripts, by session id (one storage read).
fn session_transcripts<'a>(
    state: &Arc<AppState>,
    items: impl Iterator<Item = &'a FeedItemInfo>,
) -> HashMap<String, PathBuf> {
    let ids: Vec<SessionId> = items
        .filter_map(|item| item.target.session_id.as_deref())
        .map(SessionId::new)
        .collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let Ok(storage) = state.storage.lock() else {
        return HashMap::new();
    };
    storage
        .sessions()
        .get_many(&ids)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|record| {
            let path = record.transcript_ref?.path;
            Some((record.id.as_str().to_owned(), path))
        })
        .collect()
}

/// The transcript an item's turn is in: another app's conversation's
/// file (the index's), or the session's linked transcript.
fn path_of(
    state: &AppState,
    item: &FeedItemInfo,
    sessions: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    let target = &item.target;
    target
        .account_id
        .as_deref()
        .zip(target.conversation_id.as_deref())
        .and_then(|(account, id)| state.conversations.path_of(account, id))
        .or_else(|| sessions.get(target.session_id.as_deref()?).cloned())
}

/// What the turn that ended by `until` changed, read again only when the
/// file changed since; `fresh`: the file may be read (else only what was
/// kept is said).
fn read(path: &Path, until: i64, fresh: bool) -> Option<FeedChangesInfo> {
    let stamp = juggler_adapters::last_turns::stamp(path).ok()?;
    let key = (path.to_path_buf(), until);
    if let Ok(kept) = KEPT.lock()
        && let Some((was, said)) = kept.get(&key)
        && *was == stamp
    {
        return said.clone();
    }
    if !fresh {
        return None;
    }
    let said = turn_changes(path, Some(until))
        .ok()
        .flatten()
        .map(|changes| info_of(path, changes));
    if let Ok(mut kept) = KEPT.lock() {
        if kept.len() >= KEPT_MAX {
            kept.clear();
        }
        kept.insert(key, (stamp, said.clone()));
    }
    said
}

/// A turn's changes as the window shows them, keyed by FNV-1a of its
/// file's path and its start (the path itself never leaves).
#[must_use]
pub fn info_of(path: &Path, changes: TurnChanges) -> FeedChangesInfo {
    let count = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    FeedChangesInfo {
        turn: format!("{hash:016x}:{}", changes.start),
        files: changes.files,
        names: changes.names,
        added: changes.lines.map(|lines| count(lines.added)),
        removed: changes.lines.map(|lines| count(lines.removed)),
    }
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod tests;
