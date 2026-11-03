//! The relation counts' command.

use std::sync::Arc;

use super::dto::{EntityRef, RelationCountsInfo};
use crate::error::AppResult;
use crate::state::AppState;

/// AJ-E3102: the counts each of `entities` heads its tabs with (at most
/// [`super::MAX_ASKED`]), in order: from the cache, built first off the
/// main thread when something changed or it was cleared.
///
/// # Errors
///
/// As [`super::counts_many`].
#[tauri::command]
pub async fn relations_counts(
    state: tauri::State<'_, Arc<AppState>>,
    entities: Vec<EntityRef>,
) -> AppResult<Vec<RelationCountsInfo>> {
    crate::ipc::blocking(state.inner(), "the counts", move |state| {
        super::counts_many(state, &entities)
    })
    .await
}
