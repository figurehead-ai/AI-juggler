//! AJ-E2501 — the notification model over IPC: where it stands, its lists
//! a page at a time, the Needs you list kept in step, and the user's marks
//! (read, snooze, dismiss, mute). Every write answers with the summary
//! and brings the views along ([`super::changed`]). None of these acts on
//! a session: a notice's own action is the window's, on the user's click.

use std::sync::Arc;

use super::centre::{self, NeedInput, NoticeInfo, NoticeMuteInfo, NoticeSummaryInfo};
use super::views::OpenNowInput;
use crate::error::{AppError, AppResult};
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

fn locked<T>(
    state: &Arc<AppState>,
    body: impl FnOnce(&juggler_core::storage::Storage) -> AppResult<T>,
) -> AppResult<T> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    body(&storage)
}

/// Writes through `body`, then brings every view along and answers with
/// the summary.
fn write(
    state: &Arc<AppState>,
    body: impl FnOnce(&juggler_core::storage::Storage, i64) -> AppResult<()>,
) -> AppResult<NoticeSummaryInfo> {
    let now = crate::commands::now_millis();
    locked(state, |storage| body(storage, now))?;
    super::changed(state);
    Ok(super::views::summary_now(state))
}

/// Where the model stands (the bell, the centre's header).
///
/// # Errors
///
/// The typed storage error.
#[tauri::command]
pub async fn notifications_summary(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", |state| {
        let clock = state.notifier.clock();
        locked(state, |storage| centre::summary(storage, clock))
    })
    .await
}

/// One page of a list: `needs`, `earlier` or `banners`.
///
/// # Errors
///
/// An unknown list or cursor; the typed storage error.
#[tauri::command]
pub async fn notifications_page(
    state: tauri::State<'_, Arc<AppState>>,
    section: String,
    request: PageRequest,
) -> AppResult<Page<NoticeInfo>> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        let now = crate::commands::now_millis();
        locked(state, |storage| {
            centre::page(storage, &section, &request, now)
        })
    })
    .await
}

/// The Needs you list now, and what is open (the window keeps them in
/// step: new items are written, gone ones end).
///
/// # Errors
///
/// A list this app's window didn't make; the typed storage error.
#[tauri::command]
pub async fn notifications_sync(
    state: tauri::State<'_, Arc<AppState>>,
    needs: Vec<NeedInput>,
    open: OpenNowInput,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        state.notifier.set_open_now(open);
        write(state, |storage, now| {
            centre::sync_needs(storage, &needs, now).map(|_| ())
        })
    })
    .await
}

/// Marks one notice read, or (no id) all of them.
///
/// # Errors
///
/// The typed storage error.
#[tauri::command]
pub async fn notifications_read(
    state: tauri::State<'_, Arc<AppState>>,
    id: Option<i64>,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        write(state, |storage, now| centre::read(storage, id, now))
    })
    .await
}

/// Snoozes a notice until a time ahead, or wakes it (no time).
///
/// # Errors
///
/// A time already past, a notice that is gone; the typed storage error.
#[tauri::command]
pub async fn notifications_snooze(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
    until_ms: Option<i64>,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        write(state, |storage, now| {
            centre::snooze(storage, id, until_ms, now)
        })
    })
    .await
}

/// Closes a banner (the notice stays in the centre).
///
/// # Errors
///
/// The typed storage error.
#[tauri::command]
pub async fn notifications_dismiss(
    state: tauri::State<'_, Arc<AppState>>,
    id: i64,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        write(state, |storage, now| centre::dismiss(storage, id, now))
    })
    .await
}

/// Mutes a project or a kind: it stops interrupting (still counted).
///
/// # Errors
///
/// An unknown scope or kind; the typed storage error.
#[tauri::command]
pub async fn notifications_mute(
    state: tauri::State<'_, Arc<AppState>>,
    scope: String,
    value: String,
    label: String,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        write(state, |storage, now| {
            centre::mute(storage, &scope, &value, &label, now)
        })
    })
    .await
}

/// Lets a project or a kind interrupt again.
///
/// # Errors
///
/// The typed storage error.
#[tauri::command]
pub async fn notifications_unmute(
    state: tauri::State<'_, Arc<AppState>>,
    scope: String,
    value: String,
) -> AppResult<NoticeSummaryInfo> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        write(state, |storage, _| centre::unmute(storage, &scope, &value))
    })
    .await
}

/// The mutes, a page at a time (Settings › Notifications).
///
/// # Errors
///
/// A bad cursor; the typed storage error.
#[tauri::command]
pub async fn notifications_mutes(
    state: tauri::State<'_, Arc<AppState>>,
    request: PageRequest,
) -> AppResult<Page<NoticeMuteInfo>> {
    crate::ipc::blocking(state.inner(), "notifications", move |state| {
        locked(state, |storage| centre::mutes(storage, &request))
    })
    .await
}
