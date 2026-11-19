//! The Models screen's commands (AJ-E3803, AJ-E3813; the screen is
//! AJ-E3812). Two reads, [`ai_models_installed`] and the screen's list
//! [`ai_models_catalog`]; the rest are the user's clicks: Download,
//! Cancel, Remove, and the approval card's Add and Not now. Each change tells every window (`ai-models://changed`); a
//! running download reports `ai-models://progress` with a
//! [`DownloadInfo`]; an install or a removal re-answers the AI jobs'
//! query (`ai://jobs-changed`), since a removed model's jobs hide.

use std::sync::{Arc, Weak};

use super::catalog::{ModelsCatalog, catalog};
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

/// The licence text of the listed or installed model `id`: the address
/// the signed list carries (on its allowed licence hosts), for the Models
/// screen's licence link.
#[must_use]
pub fn licence_url(state: &AppState, id: &str) -> Option<String> {
    let listing = listing();
    let installed = super::installed_entries(&store::shared(&state.data_dir).installed(), &listing);
    super::offered(&listing)
        .iter()
        .chain(installed.iter())
        .find(|model| model.id == id)?
        .download
        .as_ref()
        .map(|download| download.licence_url.clone())
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

/// What Settings › AI › Models shows besides the installed rows: this
/// Mac's facts, the recommendation (the one Automatic uses), and the
/// models this Mac can use with their verdicts (AJ-E3812, D-127). It may
/// ask the engines again first when their last look is stale.
///
/// # Errors
///
/// None today; the result keeps the command's shape.
#[tauri::command]
pub async fn ai_models_catalog(state: tauri::State<'_, Arc<AppState>>) -> AppResult<ModelsCatalog> {
    crate::ipc::blocking(state.inner(), "the AI models", |state| {
        let listing = listing();
        let offered = super::offered(&listing);
        let installed =
            super::installed_entries(&store::shared(&state.data_dir).installed(), &listing);
        // Free disk read now, not at the engines' last look: Check again
        // after freeing space shows it at once.
        let mac = crate::features::ai::service::mac_now(state).map(|mut mac| {
            if let Ok(free) = juggler_platform::room::free_space(&state.data_dir) {
                mac.free_disk_bytes = free;
            }
            mac
        });
        let usable = crate::features::ai::service::usable(&offered, &installed);
        let recommendation = mac
            .as_ref()
            .map(|mac| crate::features::ai::service::recommendation(mac, &usable, &installed));
        Ok(catalog(
            mac.as_ref(),
            juggler_platform::room::total_memory(),
            &listing,
            &offered,
            &installed,
            recommendation.as_ref(),
        ))
    })
    .await
}

/// A System Settings page the Models screen links to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum SystemPage {
    /// General › Storage: to free disk for a download.
    Storage,
    /// Apple Intelligence & Siri: to turn Apple's model on.
    AppleIntelligence,
}

impl SystemPage {
    /// Its fixed address (never a caller's URL).
    #[must_use]
    pub const fn url(self) -> &'static str {
        match self {
            Self::Storage => "x-apple.systempreferences:com.apple.settings.Storage",
            Self::AppleIntelligence => {
                "x-apple.systempreferences:com.apple.Siri-Settings.extension"
            }
        }
    }
}

/// Opens a System Settings page the Models screen names: Storage (not
/// enough free disk) or Apple Intelligence (its tip).
///
/// # Errors
///
/// The page can't be opened.
#[tauri::command]
pub async fn ai_open_system_page(app: tauri::AppHandle, page: SystemPage) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_url(page.url(), None::<&str>)
        .map_err(|e| AppError::Io {
            context: "open System Settings",
            message: e.to_string(),
        })
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
        // A job picked for it goes back to Automatic, as the confirmation
        // says (AJ-E3812).
        crate::features::ai::service::clear_picks_of(state, &model_id)?;
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
