//! ADR-0023: the search index as one of the local caches — shown with
//! its size, cleared when the user clears the cache, but kept whole: it
//! is slow to build, so it is never trimmed for the cache's limit (only
//! a nearly full disk removes it, and its builder waits for room). It
//! keeps its own switch (Settings › Privacy), which the cache's switch
//! leaves alone.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};

use juggler_core::storage::SearchIndexFile;

use crate::features::cache::{About, Item, LocalCache, OwnSwitch, Trim};
use crate::state::AppState;

/// The search index, for the cache's registry.
#[must_use]
pub fn local_cache(state: &Arc<AppState>) -> Arc<dyn LocalCache> {
    Arc::new(SearchIndex {
        state: Arc::downgrade(state),
        path: super::index_path(state),
        about: About {
            id: "search-index",
            name: "Search index",
            speeds_up: "Search the words of every session answers at once.",
            slower: "The index is built again in the background; until it's done, search finds less.",
            trim: Trim::WholeOnly,
            own_switch: Some(OwnSwitch {
                name: "Search the words of every session",
                pane: "Privacy",
                section: "search",
            }),
            max_entries: None,
            shaped_by: &[],
        },
    })
}

struct SearchIndex {
    state: Weak<AppState>,
    path: PathBuf,
    about: About,
}

impl LocalCache for SearchIndex {
    fn about(&self) -> &About {
        &self.about
    }

    fn location(&self) -> PathBuf {
        self.path.clone()
    }

    fn items(&self, _limit: usize) -> Vec<Item> {
        // The index and the files SQLite keeps beside it, as one.
        let mut bytes = 0;
        let mut used = None;
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let mut name = self.path.as_os_str().to_owned();
            name.push(suffix);
            if let Ok(meta) = std::fs::symlink_metadata(PathBuf::from(name))
                && meta.is_file()
            {
                bytes += meta.len();
                used = used.max(meta.modified().ok());
            }
        }
        used.map(|used| Item {
            path: self.path.clone(),
            bytes,
            used,
        })
        .into_iter()
        .collect()
    }

    fn is_on(&self) -> bool {
        self.state
            .upgrade()
            .is_some_and(|state| super::enabled(&state).unwrap_or(false))
    }

    fn remove(&self, item: &Item) -> u64 {
        if self.clear().is_ok() { item.bytes } else { 0 }
    }

    fn clear(&self) -> std::io::Result<()> {
        let Some(state) = self.state.upgrade() else {
            return Ok(());
        };
        clear_index(&state)
    }
}

/// Deletes the index's file; while search is on, it is built again in
/// the background.
///
/// # Errors
///
/// The file couldn't be removed.
pub fn clear_index(state: &Arc<AppState>) -> std::io::Result<()> {
    let mut index = state
        .outside_search
        .index
        .lock()
        .map_err(|_| std::io::Error::other("the search index is unavailable"))?;
    drop(index.take()); // closes the file before it goes
    SearchIndexFile::remove(&super::index_path(state))?;
    drop(index);
    state.outside_search.kick.store(true, Ordering::Release);
    Ok(())
}
