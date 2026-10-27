//! The transcript-tree watcher Claude Code's and Codex's adapters share
//! (spec §4.1): filesystem events become a line-count diff per `.jsonl`
//! file, never a parse. Moved out of `claude_code.rs` by feature.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use juggler_adapter::{AdapterError, SessionEvent, SessionWatcher};

use crate::claude_code::{collect_jsonl, file_stem, newlines_from};

/// notify-based watcher over the account's transcript tree
/// (`<config>/projects/…`). Filesystem events trigger a line-count diff per
/// `.jsonl` file; new files emit [`SessionEvent::Discovered`], growth emits
/// [`SessionEvent::MessageAppended`]. Content is never parsed here — the
/// transcript parser runs downstream on demand (spec §4.1).
pub(crate) struct ClaudeTranscriptWatcher {
    root: PathBuf,
    /// The vendor session id of a transcript file (Claude: the stem;
    /// Codex: the thread UUID inside the rollout name).
    id_of: fn(&Path) -> String,
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    seen: BTreeMap<PathBuf, usize>,
    /// (size, mtime) at the last read: an unchanged file is not read
    /// again (Codex keeps every past rollout under `sessions/`).
    stamps: BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>)>,
    /// How far each file's lines are counted (bytes, lines): a file that
    /// grew is counted from there, never read whole again.
    counted: BTreeMap<PathBuf, (u64, usize)>,
    pending: std::collections::VecDeque<SessionEvent>,
}

impl ClaudeTranscriptWatcher {
    pub(crate) fn new(root: &Path) -> Result<Self, AdapterError> {
        Self::with_ids(root, file_stem)
    }

    /// A watcher whose native ids come from `id_of` (shared by Codex).
    pub(crate) fn with_ids(root: &Path, id_of: fn(&Path) -> String) -> Result<Self, AdapterError> {
        use notify::Watcher;
        let (tx, events) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|error| AdapterError::Watch(format!("cannot create fs watcher: {error}")))?;
        watcher
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|error| {
                AdapterError::Watch(format!("cannot watch {}: {error}", root.display()))
            })?;
        Ok(Self {
            root: root.to_path_buf(),
            id_of,
            _watcher: watcher,
            events,
            seen: BTreeMap::new(),
            stamps: BTreeMap::new(),
            counted: BTreeMap::new(),
            pending: std::collections::VecDeque::new(),
        })
    }

    /// Waits up to `timeout` for a filesystem event (returning early when
    /// one arrives), then rescans. The scan is unconditional: fs backends
    /// can drop or delay events (sleep/wake, coalescing), so the bounded
    /// walk is the self-healing source of truth and notify is the wake-up
    /// signal (spec §5: watching degrades gracefully).
    fn wait_and_rescan(&mut self, timeout: std::time::Duration) {
        if !timeout.is_zero() {
            let _ = self.events.recv_timeout(timeout);
            // Coalesce bursts before scanning once.
            while self.events.try_recv().is_ok() {}
        }
        self.scan();
    }

    /// Walks the watched tree and queues Discovered/MessageAppended diffs.
    fn scan(&mut self) {
        let mut files = Vec::new();
        collect_jsonl(&self.root, &mut files, 0);
        files.sort();
        for path in files {
            let stamp = std::fs::metadata(&path)
                .map(|meta| (meta.len(), meta.modified().ok()))
                .unwrap_or_default();
            if self.stamps.get(&path) == Some(&stamp) {
                continue;
            }
            self.stamps.insert(path.clone(), stamp);
            // Only what was appended since the last count; a file that
            // shrank (rewritten) is counted again from its start.
            let (before, base) = self
                .counted
                .get(&path)
                .copied()
                .filter(|(at, _)| *at <= stamp.0)
                .unwrap_or((0, 0));
            let lines = newlines_from(&path, before).map_or(0, |(end, more)| {
                self.counted.insert(path.clone(), (end, base + more));
                base + more
            });
            let known = self.seen.entry(path.clone()).or_insert(0);
            if *known == 0 && lines > 0 {
                self.pending.push_back(SessionEvent::Discovered {
                    native_id: (self.id_of)(&path),
                    transcript_path: path.clone(),
                });
            } else if lines > *known {
                self.pending.push_back(SessionEvent::MessageAppended {
                    native_id: (self.id_of)(&path),
                    transcript_path: path.clone(),
                });
            }
            *known = lines;
        }
    }
}

impl SessionWatcher for ClaudeTranscriptWatcher {
    fn poll(&mut self, timeout: std::time::Duration) -> Result<Option<SessionEvent>, AdapterError> {
        if self.pending.is_empty() {
            self.wait_and_rescan(timeout);
        }
        Ok(self.pending.pop_front())
    }
}
