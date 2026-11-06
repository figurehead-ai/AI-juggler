//! 0.3.5 (ADR-0024): a wake-up when a file in one of a few folders
//! changes, for the conversation feed: only the folders of the
//! transcripts a page follows are watched, not recursively, and only
//! while it follows them. Events are hints, never the truth: the feed
//! also checks its files on a slow timer, so a dropped or late event
//! (sleep and wake, a busy disk) only delays a change.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Watches a set of folders and calls back with the paths that changed.
pub struct FolderWake {
    watcher: Option<notify::RecommendedWatcher>,
    folders: BTreeSet<PathBuf>,
}

impl FolderWake {
    /// A watcher calling `on_change` with the changed paths. When the
    /// system's watcher can't be made, nothing is watched (the feed's
    /// timer still notices changes).
    pub fn new(on_change: impl Fn(Vec<PathBuf>) + Send + 'static) -> Self {
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if let Ok(event) = event
                && !event.paths.is_empty()
            {
                on_change(event.paths);
            }
        })
        .ok();
        Self {
            watcher,
            folders: BTreeSet::new(),
        }
    }

    /// Watches exactly `folders` from now on (each not recursively).
    pub fn watch(&mut self, folders: &BTreeSet<PathBuf>) {
        use notify::Watcher as _;
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        for gone in self.folders.difference(folders) {
            let _ = watcher.unwatch(gone);
        }
        let mut now_watched = BTreeSet::new();
        for folder in folders {
            if self.folders.contains(folder)
                || watcher
                    .watch(folder, notify::RecursiveMode::NonRecursive)
                    .is_ok()
            {
                now_watched.insert(folder.clone());
            }
        }
        self.folders = now_watched;
    }

    /// The folders watched now.
    #[must_use]
    pub fn folders(&self) -> &BTreeSet<PathBuf> {
        &self.folders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_in_a_watched_folder_wakes_and_an_unwatched_one_stops() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut wake = FolderWake::new(move |paths| {
            let _ = tx.send(paths);
        });
        let folder = dir.path().canonicalize().unwrap();
        wake.watch(&BTreeSet::from([folder.clone()]));
        assert_eq!(wake.folders().len(), 1);
        std::fs::write(folder.join("s.jsonl"), b"{}\n").unwrap();
        // Events are hints: they may come in any order (one about the
        // folder itself, made just before the watch, can come first), late,
        // or not at all on a busy system. So the file is changed again
        // every 2 s until a wake-up names it.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut named = false;
        while !named {
            assert!(
                std::time::Instant::now() < deadline,
                "no wake-up named the file"
            );
            match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(paths) => named = paths.iter().any(|path| path.ends_with("s.jsonl")),
                Err(_) => std::fs::write(folder.join("s.jsonl"), b"{}\n{}\n").unwrap(),
            }
        }
        wake.watch(&BTreeSet::new());
        assert!(wake.folders().is_empty());
    }
}
