//! AJ-E1520 (0.3): the Claude Code subagent reader's checkpoint — what
//! it has read, as JSON the app saves (its index) and hands back on the
//! next open, so a session with hundreds of subagents and hundreds of
//! megabytes of their transcripts opens at once and reads only what grew.
//!
//! A save holds no line still being written (each transcript's offset
//! goes back to that line's start) and passes every word it keeps through
//! the app's filter first (its redaction, D-14; a word it drops is not
//! saved). A checkpoint of another version or another session is refused:
//! the reader starts from the beginning, as before.

use std::path::Path;

use juggler_adapter::subagents::{SubagentText, TextFilter};
use serde::{Deserialize, Serialize};

use super::Read;

/// The checkpoint's version: a reader refuses any other.
const VERSION: u32 = 1;

#[derive(Serialize)]
struct SavedRef<'a> {
    version: u32,
    session: &'a str,
    read: &'a Read,
}

#[derive(Deserialize)]
struct Saved {
    version: u32,
    session: String,
    read: Read,
}

/// `read` as a checkpoint for the session `session`.
pub(super) fn save(session: &Path, read: &Read, text: &TextFilter<'_>) -> Option<Vec<u8>> {
    let mut copy = Read {
        tails: read
            .tails
            .iter()
            .map(|(id, tail)| (id.clone(), tail.rewound()))
            .collect(),
        metas: read.metas.clone(),
        files: read.files.clone(),
    };
    for tail in copy.tails.values_mut() {
        tail.log_mut().filter_texts(text);
    }
    for seen in copy.metas.values_mut() {
        let meta = &mut seen.meta;
        meta.description = meta
            .description
            .take()
            .and_then(|words| text(SubagentText::Task, &words));
        meta.worktree_path = meta
            .worktree_path
            .take()
            .and_then(|path| text(SubagentText::File, &path));
    }
    serde_json::to_vec(&SavedRef {
        version: VERSION,
        session: &session.to_string_lossy(),
        read: &copy,
    })
    .ok()
}

/// What a checkpoint for `session` holds; `None` for another version or
/// session, or one that can't be read.
pub(super) fn load(session: &Path, bytes: &[u8]) -> Option<Read> {
    let saved: Saved = serde_json::from_slice(bytes).ok()?;
    (saved.version == VERSION && saved.session == session.to_string_lossy()).then_some(saved.read)
}
