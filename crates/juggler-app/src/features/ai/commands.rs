//! The AI jobs' commands (ADR-0034 §3.4, AJ-E3802): the one query every
//! AI surface reads, and a job's Beta switch. Each tells every window when
//! the answer changed (`ai://jobs-changed`).

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
