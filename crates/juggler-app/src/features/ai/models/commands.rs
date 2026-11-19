//! The Models screen's commands (AJ-E3803, AJ-E3813; the screen is
//! AJ-E3812). One read, [`ai_models_installed`]; the rest are the user's
//! clicks: Download, Cancel, Remove, and the approval card's Add and
//! Not now. Each change tells every window (`ai-models://changed`); a
//! running download reports `ai-models://progress` with a
//! [`DownloadInfo`]; an install or a removal re-answers the AI jobs'
//! query (`ai://jobs-changed`), since a removed model's jobs hide.

use std::sync::{Arc, Weak};

use super::download::{self, Downloads, Event, Notify, Outcome};
use super::dto::{ApprovalInfo, DownloadInfo, InstalledModelInfo, ModelsInfo, StartInfo};
use super::store::{self, Refusal, Store};
use super::{Engine, Listing, ModelEntry};
use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EmitEvent as _};
use crate::state::AppState;

/// The longest model id the section allows.
const MAX_ID: usize = 64;

fn checked(model_id: &str) -> AppResult<()> {
    if model_id.is_empty() || model_id.len() > MAX_ID {
        return Err(AppError::Validation {
            subject: "model",
            message: "not a model id".to_owned(),
        });
    }
    Ok(())
}

fn listing() -> Listing {
    super::listing(&crate::features::remote_config::current().lists)
}

/// The downloadable model `id` the app offers now.
fn offered(listing: &Listing, id: &str) -> Option<ModelEntry> {
    super::offered(listing)
        .into_iter()
        .find(|entry| entry.id == id && entry.engine == Engine::GgufLocal)
}

/// The models on this Mac, the download, and the declines (pure but for
/// the store's index).
#[must_use]
pub fn models_info(store: &Store, downloads: Option<&Downloads>, listing: &Listing) -> ModelsInfo {
    let installed = store.installed();
    let size_of = |id: &str| {
        installed
            .iter()
            .map(|m| &m.entry)
            .chain(super::offered(listing).iter())
            .find(|entry| entry.id == id)
            .and_then(|entry| entry.download.as_ref())
            .map_or(0, |download| download.size_bytes)
    };
    let download = downloads.and_then(|downloads| {
        downloads
            .running()
            .map(|(id, progress)| DownloadInfo::running(&id, progress))
            .or_else(|| {
                downloads
                    .last()
                    .map(|(id, outcome)| DownloadInfo::ended(&id, &outcome, size_of(&id)))
            })
    });
    ModelsInfo {
        installed: installed
            .iter()
            .map(|m| InstalledModelInfo::of(m, listing))
            .collect(),
        total_bytes: store.total_bytes(),
        download,
        declined: store.declined(),
    }
}

fn info_now(state: &AppState) -> ModelsInfo {
    models_info(
        &store::shared(&state.data_dir),
        download::shared_if_made(),
        &listing(),
    )
}

/// After an install or a removal: tell the windows, and re-answer the AI
/// jobs' query.
fn changed(app: &tauri::AppHandle, state: &AppState, jobs_too: bool) {
    app.emit_event(AppEvent::AiModelsChanged, ());
    if jobs_too && crate::features::ai::service::recheck(state).is_some() {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
}

fn notifier(app: tauri::AppHandle, state: Weak<AppState>) -> Notify {
    Arc::new(move |event| match event {
        Event::Progress { id, progress } => {
            app.emit_event(
                AppEvent::AiModelsProgress,
                DownloadInfo::running(&id, progress),
            );
        }
        Event::Ended { outcome, .. } => {
            if let Some(state) = state.upgrade() {
                changed(&app, &state, outcome == Outcome::Installed);
            }
        }
    })
}

/// The models on this Mac: each one's size and standing, the total disk
/// they use (from the index), the running download or how the last one
/// ended, and the declined approval cards.
///
/// # Errors
///
/// None today; the result keeps the command's shape.
#[tauri::command]
pub async fn ai_models_installed(state: tauri::State<'_, Arc<AppState>>) -> AppResult<ModelsInfo> {
    crate::ipc::blocking(state.inner(), "the AI models", |state| Ok(info_now(state))).await
}

/// The user's Download click on `model_id`: starts it, or says why not
/// (a further model answers `needs_approval` with its size and the new
/// total, for the approval card). `approval` is the card's token.
///
/// # Errors
///
/// A model id that can't be one.
#[tauri::command]
pub async fn ai_model_download_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    model_id: String,
    approval: Option<String>,
) -> AppResult<StartInfo> {
    checked(&model_id)?;
    let info = crate::ipc::blocking(state.inner(), "the AI models", move |state| {
        let Some(entry) = offered(&listing(), &model_id) else {
            return Ok(StartInfo::refused(&Refusal::NotOffered));
        };
        let notify = notifier(app.clone(), Arc::downgrade(state));
        let started = download::shared(state).start(&entry, approval.as_deref(), notify);
        app.emit_event(AppEvent::AiModelsChanged, ());
        Ok(match started {
            Ok(()) => StartInfo::started(),
            Err(refusal) => StartInfo::refused(&refusal),
        })
    })
    .await?;
    Ok(info)
}

/// Cancels the download of `model_id`; nothing of it is left behind.
///
/// # Errors
///
/// A model id that can't be one.
#[tauri::command]
pub async fn ai_model_download_cancel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    model_id: String,
) -> AppResult<ModelsInfo> {
    checked(&model_id)?;
    crate::ipc::blocking(state.inner(), "the AI models", move |state| {
        if let Some(downloads) = download::shared_if_made() {
            downloads.cancel(&model_id);
        }
        app.emit_event(AppEvent::AiModelsChanged, ());
        Ok(info_now(state))
    })
    .await
}

/// Removes the model `model_id`: its file and its installed copy. The
/// jobs only it served hide (the AI jobs' query is told).
///
/// # Errors
///
/// It isn't on this Mac, or a file couldn't be removed.
#[tauri::command]
pub async fn ai_model_remove(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    model_id: String,
) -> AppResult<ModelsInfo> {
    checked(&model_id)?;
    crate::ipc::blocking(state.inner(), "the AI models", move |state| {
        let removed = store::shared(&state.data_dir)
            .remove(&model_id)
            .map_err(|error| AppError::Io {
                context: "remove the AI model",
                message: error.to_string(),
            })?;
        if removed.is_none() {
            return Err(AppError::NotFound {
                entity: "model",
                id: model_id,
            });
        }
        changed(&app, state, true);
        Ok(info_now(state))
    })
    .await
}

/// The approval card's Add for a further model: a token for one
/// Download of `model_id` within 10 minutes (never a standing yes); it
/// clears a remembered decline.
///
/// # Errors
///
/// Not a downloadable model the app offers.
#[tauri::command]
pub async fn ai_model_approve_extra(
    state: tauri::State<'_, Arc<AppState>>,
    model_id: String,
) -> AppResult<ApprovalInfo> {
    checked(&model_id)?;
    crate::ipc::blocking(state.inner(), "the AI models", move |state| {
        let entry = offered(&listing(), &model_id).ok_or_else(|| AppError::NotFound {
            entity: "model",
            id: model_id.clone(),
        })?;
        let size_bytes = entry.download.as_ref().map_or(0, |d| d.size_bytes);
        let store = store::shared(&state.data_dir);
        let token = store.approve(&entry.id);
        if token.is_empty() {
            return Err(AppError::Io {
                context: "approve the AI model",
                message: "no secure random numbers".to_owned(),
            });
        }
        Ok(ApprovalInfo {
            id: entry.id,
            token,
            size_bytes,
            total_after_bytes: store.total_bytes().saturating_add(size_bytes),
        })
    })
    .await
}

/// The approval card's Not now: remembered, so it isn't offered again
/// unasked.
///
/// # Errors
///
/// A model id that can't be one.
#[tauri::command]
pub async fn ai_model_decline_extra(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    model_id: String,
) -> AppResult<ModelsInfo> {
    checked(&model_id)?;
    crate::ipc::blocking(state.inner(), "the AI models", move |state| {
        store::shared(&state.data_dir).decline(&model_id);
        app.emit_event(AppEvent::AiModelsChanged, ());
        Ok(info_now(state))
    })
    .await
}
