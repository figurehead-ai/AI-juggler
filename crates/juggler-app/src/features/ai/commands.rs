//! The AI jobs' commands (ADR-0034 §3.4, AJ-E3802): the one query every
//! AI surface reads, a job's Beta switch, and its model pick (AJ-E3812).
//! Each tells every window when the answer changed (`ai://jobs-changed`).

use std::sync::Arc;

use super::dto::{AiJob, AiJobInfo};
use crate::error::AppResult;
use crate::events::{AppEvent, EmitEvent as _};
use crate::state::AppState;

/// Which AI jobs are available now, per job with the model chosen and why
/// (asks the engines again when the last probe is stale).
///
/// # Errors
///
/// As [`super::service::jobs_now`].
#[tauri::command]
pub async fn ai_jobs(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<AiJobInfo>> {
    let (jobs, changed) = crate::ipc::blocking(state.inner(), "the AI jobs", |state| {
        super::service::jobs_now(state)
    })
    .await?;
    if changed {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
    Ok(jobs)
}

/// Turns a job's Beta on or off; every job's answer after it.
///
/// # Errors
///
/// As [`super::service::set_beta`].
#[tauri::command]
pub async fn ai_beta_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    job: AiJob,
    on: bool,
) -> AppResult<Vec<AiJobInfo>> {
    let (jobs, changed) = crate::ipc::blocking(state.inner(), "the AI jobs", move |state| {
        super::service::set_beta(state, job, on)
    })
    .await?;
    if changed {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
    Ok(jobs)
}

/// Picks the model `model_id` for `job`, or Automatic (`None`); every
/// job's answer after it (AJ-E3812's per-job override). Only a model that
/// passed the job may be picked.
///
/// # Errors
///
/// As [`super::service::set_pick`].
#[tauri::command]
pub async fn ai_job_pick_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    job: AiJob,
    model_id: Option<String>,
) -> AppResult<Vec<AiJobInfo>> {
    let (jobs, changed) = crate::ipc::blocking(state.inner(), "the AI jobs", move |state| {
        super::service::set_pick(state, job, model_id.as_deref())
    })
    .await?;
    if changed {
        app.emit_event(AppEvent::AiJobsChanged, ());
    }
    Ok(jobs)
}

/// `job`'s engine status off the window's thread, started first when
/// `start` says so (AJ-E3825).
async fn engine(
    state: &Arc<AppState>,
    job: AiJob,
    start: bool,
) -> AppResult<super::run::AiEngineStatusInfo> {
    crate::ipc::blocking(state, "the AI engine", move |state| {
        let service = super::service::shared();
        Ok(if start {
            super::run::start_on(service, state, job)
        } else {
            super::run::status_on(service, state, job)
        })
    })
    .await
}

/// A job's engine now (AJ-E3825): started, running, idle, answering or
/// failed with why, its model and today's use. Starts nothing.
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn ai_engine_status(
    state: tauri::State<'_, Arc<AppState>>,
    job: AiJob,
) -> AppResult<super::run::AiEngineStatusInfo> {
    engine(state.inner(), job, false).await
}

/// Starts a job's engine when it isn't running (Bot opening with an AI
/// chosen), checks its hello, and says how it stands after (AJ-E3825).
///
/// # Errors
///
/// The work could not finish off the main thread.
#[tauri::command]
pub async fn ai_engine_start(
    state: tauri::State<'_, Arc<AppState>>,
    job: AiJob,
) -> AppResult<super::run::AiEngineStatusInfo> {
    engine(state.inner(), job, true).await
}
