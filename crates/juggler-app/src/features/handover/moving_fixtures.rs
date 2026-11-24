//! The move's test fixtures, shared by its test files (one way, never
//! copies): a fresh app state and the scripted systems' common parts.

use std::path::PathBuf;
use std::sync::Arc;

use crate::features::places::book::{self, KnownPlace};
use crate::state::AppState;

/// A fresh app state in a temp folder: the folder, its path, the state.
pub(super) fn lab_state() -> (tempfile::TempDir, PathBuf, Arc<AppState>) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let state = Arc::new(AppState::init(root.join("data")).unwrap());
    (tmp, root, state)
}

/// `place`'s id, as the book keys it.
pub(super) fn id_of(place: &KnownPlace) -> String {
    book::place_id(&place.app_id, place.instance_dir.as_deref())
}

/// A scripted [`super::MoveSystem`]'s parts its tests never look at: a
/// link goes nowhere, nothing is open elsewhere, no settling, and `$wait`
/// for the apps to quit and their writers to end.
macro_rules! quiet_parts {
    ($wait:expr) => {
        fn send(&self, _: u32, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn open_elsewhere(&self, _: &Arc<AppState>, _: &[String], _: &[u32]) -> usize {
            0
        }
        fn settle(&self, _: Duration) {}
        fn quit_wait(&self) -> Duration {
            $wait
        }
        fn writer_wait(&self) -> Duration {
            $wait
        }
    };
}
pub(super) use quiet_parts;
