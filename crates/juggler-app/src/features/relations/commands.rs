//! The relation counts' commands.

use std::sync::Arc;

use super::dto::{EntityKind, EntityRef, RelatedIdsInfo, RelationCountsInfo};
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

/// AJ-E3104: which `of` `entity`'s sessions are on (a page's Pools or
/// Apps tab lists them), from the same cache as its count.
///
/// # Errors
///
/// As [`super::related`].
#[tauri::command]
pub async fn relations_related(
    state: tauri::State<'_, Arc<AppState>>,
    entity: EntityRef,
    of: EntityKind,
) -> AppResult<RelatedIdsInfo> {
    crate::ipc::blocking(state.inner(), "the related", move |state| {
        super::related(state, &entity, of)
    })
    .await
}
