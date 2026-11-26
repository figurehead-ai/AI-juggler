//! The feed over IPC (D-118): one page at a time, newest first; and a
//! post's More or Less like this ([`prefs`], AJ-E3713).

pub mod prefs;

use std::sync::Arc;

use super::dto::{FeedFiltersInfo, FeedItemInfo};
use super::page::{self, Ask};
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

/// One page of the activity feed past `request`'s cursor, narrowed by
/// `filters` (D-118, AJ-E3702 R1): every journal's items after the start,
/// at most a page from each, merged newest first.
///
/// # Errors
///
/// A cursor this app didn't make; the typed storage error.
pub fn feed_page_impl(
    state: &Arc<AppState>,
    request: &PageRequest,
    filters: &FeedFiltersInfo,
) -> AppResult<Page<FeedItemInfo>> {
    let limit = request.limit.map_or(page::FEED_PAGE, |_| request.limit());
    let before = request.cursor.as_deref().map(page::key_of).transpose()?;
    // AJ-E3713 R5: Running keeps the items of the sessions running now
    // (the read model's rows, read before the storage lock below).
    let mut ask = Ask::of(filters);
    if ask.state.as_deref() == Some("running") {
        let rows = crate::features::sessions::list::instances_shared_snapshot(state)?;
        ask = ask.with_running(
            rows.into_iter()
                .filter(|row| matches!(row.status.as_str(), "starting" | "running"))
                .map(|row| row.id),
        );
    }
    // The conversation index first, with no storage lock held: it takes
    // its own.
    let more = super::conversations::read(state, before, &ask, limit)?;
    let mut page = {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        page::page_for(&storage, request, &ask, more)?
    };
    // AJ-E3713 R6a: each turn's diff stat, with no storage lock held.
    super::changes::attach(state, &mut page.items);
    Ok(page)
}

/// One page of the activity feed (see [`feed_page_impl`]).
///
/// # Errors
///
/// A cursor this app didn't make; the typed storage error.
#[tauri::command]
pub async fn feed_page(
    state: tauri::State<'_, Arc<AppState>>,
    request: PageRequest,
    filters: FeedFiltersInfo,
) -> AppResult<Page<FeedItemInfo>> {
    crate::ipc::blocking(state.inner(), "feed", move |state| {
        feed_page_impl(state, &request, &filters)
    })
    .await
}
