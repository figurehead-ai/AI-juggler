//! The Apps' commands (D-62, AJ-E3204).

// Tauri command signatures take State/String by value.
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use super::StoredAppInfo;
use crate::error::AppResult;
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

/// The Apps, the CLI ones first, then by id (one per folder and per
/// desktop profile seen: a page holds them).
///
/// # Errors
///
/// As [`super::list_impl`]; a cursor this app didn't make.
#[tauri::command]
pub async fn apps_list(
    state: tauri::State<'_, Arc<AppState>>,
    page: Option<PageRequest>,
) -> AppResult<Page<StoredAppInfo>> {
    crate::ipc::blocking(state.inner(), "the apps", move |state| {
        let rows = super::list_impl(state)?;
        crate::paging::page(
            rows,
            |row| SortKey::new(vec![i64::from(row.kind != "cli")], row.id.clone()),
            &page.unwrap_or_default(),
        )
    })
    .await
}

/// Names an App (`null` or blank: its default name).
///
/// # Errors
///
/// As [`super::rename_impl`].
#[tauri::command]
pub async fn apps_rename(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    label: Option<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "naming the app", move |state| {
        super::rename_impl(state, &id, label.as_deref())
    })
    .await
}
