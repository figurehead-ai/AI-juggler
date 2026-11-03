//! AJ-E3102: the relation counts as one of D-58's local caches: named in
//! Settings › Advanced › Local cache with its size, emptied by Clear
//! cache and by turning the cache off (nothing is kept while it's off),
//! and trimmed whole like any entry when the budget asks. It lives in
//! memory, so it writes no file and survives no restart; the next read
//! builds it again from the records.

use std::path::PathBuf;
use std::sync::{Arc, Weak};

use crate::features::cache::{About, Item, LocalCache, Trim};
use crate::state::AppState;

/// Its key.
pub const ID: &str = "relation-counts";

/// The cache to register.
#[must_use]
pub fn local_cache(state: &Arc<AppState>) -> Arc<dyn LocalCache> {
    Arc::new(RelationCache {
        state: Arc::downgrade(state),
        location: state.data_dir.clone(),
        about: About {
            id: ID,
            name: "Counts on pages",
            speeds_up: "Every page's counts (a project's sessions, an app's, an account's) show at once, however many sessions you have.",
            slower: "Counts are worked out again from AI Juggler's records and the tools' files, a moment after a page opens.",
            trim: Trim::Partly,
            own_switch: None,
            max_entries: None,
            shaped_by: &[],
        },
    })
}

struct RelationCache {
    state: Weak<AppState>,
    location: PathBuf,
    about: About,
}

impl LocalCache for RelationCache {
    fn about(&self) -> &About {
        &self.about
    }

    fn location(&self) -> PathBuf {
        self.location.clone()
    }

    fn items(&self, _limit: usize) -> Vec<Item> {
        // One entry: the counts, kept in memory.
        self.state
            .upgrade()
            .and_then(|state| state.relations.kept())
            .map(|(bytes, used)| Item {
                path: self.location.join(ID),
                bytes,
                used,
            })
            .into_iter()
            .collect()
    }

    fn remove(&self, _item: &Item) -> u64 {
        self.state
            .upgrade()
            .map_or(0, |state| state.relations.clear())
    }

    fn clear(&self) -> std::io::Result<()> {
        if let Some(state) = self.state.upgrade() {
            state.relations.clear();
        }
        Ok(())
    }
}
