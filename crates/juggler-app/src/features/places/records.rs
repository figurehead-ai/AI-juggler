//! 0.4.3: what the user's linked desktop Apps' own records list, read
//! with their records (`read_records`). Moved out of `places` for its
//! size.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::in_folder;
use crate::state::AppState;

/// 0.4.3: what the user's linked desktop Apps' own records list. A
/// profile on the real Mac had 3 records but about 40 transcripts its app
/// wrote in its folder (lab leftovers, sessions deleted in the app); every
/// one counted as the profile's. An App whose records were read lists
/// only what they list ([`Self::leaves_out`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordIndex {
    /// The places whose records were read (a list, even an empty one).
    pub(super) read: HashSet<String>,
    /// Each conversation they list ([`super::IN_FOLDER`]-qualified, as
    /// [`super::DesktopApps::by_session`]) → archived in every record of it.
    listed: HashMap<String, bool>,
}

impl RecordIndex {
    pub(super) fn list(&mut self, key: String, archived: bool) {
        let all = self.listed.entry(key).or_insert(true);
        *all &= archived;
    }

    /// The records of place `id` were read.
    #[must_use]
    pub fn read(&self, id: &str) -> bool {
        self.read.contains(id)
    }

    /// A read place's records list conversation `key` (`<tool>/<id>`)
    /// whose files are in `folder`.
    #[must_use]
    pub fn listed(&self, folder: &str, key: &str) -> bool {
        self.listed.contains_key(&in_folder(folder, key))
    }

    /// Its records archive it (every record listing it does).
    #[must_use]
    pub fn archived(&self, folder: &str, key: &str) -> bool {
        self.listed
            .get(&in_folder(folder, key))
            .copied()
            .unwrap_or(false)
    }

    /// App `app` is a desktop place whose records were read and don't
    /// list conversation `key` in `folder`: its lists and counts leave it
    /// out (History still has it). Only for one that isn't open: an open
    /// one is the App its process runs in.
    #[must_use]
    pub fn leaves_out(&self, app: &str, folder: &str, key: &str) -> bool {
        app.starts_with("app:") && self.read(app) && !self.listed(folder, key)
    }
}

/// 0.4.3: the record index as the last read left it.
#[must_use]
pub fn record_index(state: &Arc<AppState>) -> Arc<RecordIndex> {
    state
        .places
        .records
        .lock()
        .map(|kept| Arc::clone(&*kept))
        .unwrap_or_default()
}
