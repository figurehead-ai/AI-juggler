//! The feed over IPC (D-118): one page at a time, newest first.

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
    // The conversation index first, with no storage lock held: it takes
    // its own.
    let more = super::conversations::read(state, before, &Ask::of(filters), limit)?;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    page::page(&storage, request, filters, more)
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
