//! The switch commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::launch::dto::LaunchResult;
use crate::features::switch::back::{
    switch_back_execute_to_impl, switch_back_plan_to_impl, switch_back_suggestions_impl,
};
use crate::features::switch::dto::{
    PreflightInfo, SwitchBackChoiceIn, SwitchBackPlanInfo, SwitchBackSuggestionInfo,
    SwitchExecutedInfo, SwitchTargetInfo, WorkGraphInfo,
};
use crate::features::switch::execute::switch_execute_core;
use crate::features::switch::graph::switch_graph_overview_impl;
use crate::features::switch::preflight::{switch_door_count_impl, switch_preflight_impl};
use crate::features::switch::resume::{epoch_resume_impl, session_resume_impl};
use crate::features::switch::targets::switch_targets_impl;
use crate::features::switch::undo::switch_undo_impl;
use crate::state::AppState;

/// AJ-E205: preflight for a switch — ordered checks stop BEFORE any
/// byte moves; warnings render for the door (E206).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when resolution fails or a hard
/// gate stops the switch.
#[tauri::command]
pub async fn switch_preflight(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
) -> AppResult<PreflightInfo> {
    crate::ipc::blocking(state.inner(), "check the switch", move |state| {
        switch_preflight_impl(state, &session_id, &target_profile_id)
    })
    .await
}

/// AJ-E206 §5 telemetry: the switch screen counts the door it shows,
/// from the preflight's own answer (`door`, and `steps` for a wizard).
/// Local KV diagnostics only, never networked. Anything that isn't a
/// door the preflight can choose is refused.
///
/// # Errors
///
/// [`AppError::Validation`](crate::error::AppError::Validation) for an unknown door or step.
#[tauri::command]
pub async fn switch_door_count(
    state: tauri::State<'_, Arc<AppState>>,
    door: String,
    steps: Vec<String>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "count the switch screen", move |state| {
        switch_door_count_impl(state, &door, &steps)
    })
    .await
}

/// AJ-E205: executes a switch with the user's freeze choice — the
/// pipeline stages + verifies + launches (via the supervisor), and
/// every attempt records its edge in the Switch Graph (E201).
/// `approved`: the set-up check's offers the user ticked that the switch
/// itself carries out (D-51: this project's notes quoted in the handover,
/// for a tool that keeps none); what is set up beforehand goes through
/// `switch_setup_apply`.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on any failure; the source
/// is exactly resumable in all of them (S-F contract).
#[allow(
    clippy::too_many_arguments,
    reason = "the Tauri command's flat wire args"
)]
#[tauri::command]
pub async fn switch_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    target_profile_id: String,
    freeze_choice: String,
    redaction_choice: Option<String>,
    redaction_unredact: Option<Vec<u32>>,
    trigger: Option<String>,
    approved: Option<Vec<String>>,
    destination: Option<String>,
    cut: Option<crate::features::switch_cut::SwitchCutArgs>,
) -> AppResult<SwitchExecutedInfo> {
    // AJ-E1103: why the user switched (a closed word; anything else is
    // a manual switch).
    let trigger = match trigger.as_deref() {
        Some("quota_wall") => "quota_wall",
        Some("suggestion") => "suggestion",
        _ => "manual",
    };
    // Off the main thread (T1): "Finish this turn" can wait minutes for
    // the source to go quiet, and the screen must keep drawing.
    crate::ipc::blocking(state.inner(), "switch", move |state| {
        switch_execute_core(
            state,
            &session_id,
            &target_profile_id,
            &freeze_choice,
            redaction_choice.as_deref(),
            &redaction_unredact.unwrap_or_default(),
            trigger,
            &approved.unwrap_or_default(),
            destination.as_deref(),
            cut.and_then(crate::features::switch_cut::SwitchCutArgs::cut),
        )
    })
    .await
}

/// AJ-E206 §8.2: undoes a completed switch — stops the continuation
/// instance, resumes the source account's native session, and records
/// the compensating edge (`parent_edge_a` = the original). User-click
/// only; the graph keeps both edges so the travel stays explainable.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the edge cannot be compensated
/// (not confirmed, missing epochs/native ids) or stop/resume fails —
/// a failed stop aborts BEFORE the source resumes (never two live
/// instances on one work item).
#[tauri::command]
pub async fn switch_undo(
    state: tauri::State<'_, Arc<AppState>>,
    edge_id: String,
) -> AppResult<LaunchResult> {
    crate::ipc::blocking(state.inner(), "switch undo", move |state| {
        switch_undo_impl(state, &edge_id)
    })
    .await
}

/// AJ-E207: the switch-back plan for one away instance — verbatim
/// suggestion, S-F7 verdict, both-live warning, divergence counts,
/// the delta preview, and the conflict list. Suggest-only.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the lineage walk fails.
#[tauri::command]
pub async fn switch_back_plan(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    to_epoch: Option<String>,
) -> AppResult<SwitchBackPlanInfo> {
    crate::ipc::blocking(state.inner(), "plan the switch back", move |state| {
        switch_back_plan_to_impl(state, &session_id, to_epoch.as_deref())
    })
    .await
}

/// AJ-E207: executes the switch-back — applies the user's conflict
/// choices, stops-or-keeps the away instance per the explicit
/// choice, resumes the ORIGINAL native session with the one delta
/// turn, and records the switch-back edge (`resumed_from_epoch` =
/// the original epoch; parent = the fork; choices in the manifest).
/// The original transcript is never written (copy-back has no code
/// path).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) on unresolved conflicts (S-F8 —
/// the work stays forked, both sides resumable), spawn failure, or
/// graph write failure.
#[tauri::command]
pub async fn switch_back_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    choices: Vec<SwitchBackChoiceIn>,
    keep_away_running: bool,
    to_epoch: Option<String>,
) -> AppResult<LaunchResult> {
    crate::ipc::blocking(state.inner(), "switch back", move |state| {
        switch_back_execute_to_impl(
            state,
            &session_id,
            &choices,
            keep_away_running,
            to_epoch.as_deref(),
        )
    })
    .await
}

/// AJ-E207 Req.3: switch-back suggestions — one open excursion whose
/// ORIGINAL account has quota headroom again (the reset trigger),
/// each with the verbatim copy. Suggests only; nothing executes.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when storage reads fail.
#[tauri::command]
pub async fn switch_back_suggestions(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<SwitchBackSuggestionInfo>> {
    crate::ipc::blocking(
        state.inner(),
        "suggest switching back",
        switch_back_suggestions_impl,
    )
    .await
}

/// AJ-E209: the ranked + badged destinations for one source session
/// (§Q6 with the switch extensions; D-04/D-11 badges; suggest-only).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the source session or ranking
/// inputs cannot be resolved.
#[tauri::command]
pub async fn switch_targets(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<Vec<SwitchTargetInfo>> {
    crate::ipc::blocking(state.inner(), "switch targets", move |state| {
        switch_targets_impl(state, &session_id)
    })
    .await
}

/// AJ-E209 Req.4: the per-project Switch Graph — the timeline that
/// answers "how did this work travel?".
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the project or graph reads
/// fail.
#[tauri::command]
pub async fn switch_graph_overview(
    state: tauri::State<'_, Arc<AppState>>,
    project_id: String,
) -> AppResult<Vec<WorkGraphInfo>> {
    crate::ipc::blocking(state.inner(), "switch graph overview", move |state| {
        switch_graph_overview_impl(state, &project_id)
    })
    .await
}

/// AJ-E209 Req.5: resumes an epoch's native session (the graph's
/// Resume action — suggest-chosen, the user's click).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) when the epoch or its account/
/// recipe cannot be resolved or the spawn fails.
#[tauri::command]
pub async fn epoch_resume(
    state: tauri::State<'_, Arc<AppState>>,
    epoch_id: String,
) -> AppResult<LaunchResult> {
    crate::ipc::blocking(state.inner(), "epoch resume", move |state| {
        epoch_resume_impl(state, &epoch_id)
    })
    .await
}

/// AJ-E2908: Resume — an ended session's conversation, continued on its
/// own account and launch settings.
///
/// # Errors
///
/// As [`session_resume_impl`].
#[tauri::command]
pub async fn session_resume(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
) -> AppResult<LaunchResult> {
    crate::ipc::blocking(state.inner(), "session resume", move |state| {
        session_resume_impl(state, &session_id)
    })
    .await
}
