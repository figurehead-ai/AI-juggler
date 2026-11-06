//! The cleanup commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::cleanup::alerts::cleanup_alerts_inner;
use crate::features::cleanup::dto::{CleanupAlertsInfo, CleanupStatusInfo};
use crate::features::cleanup::inventory::cleanup_audit_export_inner;
use crate::features::cleanup::status::{
    cleanup_pause_set_arc, cleanup_resume_after_crash_arc, cleanup_status_inner,
    cleanup_tier2_decide_arc, cleanup_tiers_get_arc, cleanup_tiers_set_arc,
};
use crate::state::AppState;

/// Exports the 7-day cleanup audit as JSONL (AJ-E509 Req.4). The
/// first line is the redaction note (itself valid JSON): cmdlines are
/// hashed; full command lines never persisted. User-initiated only.
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
#[tauri::command]
pub async fn cleanup_audit_export(state: tauri::State<'_, Arc<AppState>>) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "cleanup audit export", move |state| {
        Ok(cleanup_audit_export_inner(state))
    })
    .await
}

/// Reads the passive Cleanup Centre alerts (AJ-E512 Req.3).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on read failure.
#[tauri::command]
pub async fn cleanup_alerts(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CleanupAlertsInfo> {
    crate::ipc::blocking(state.inner(), "cleanup alerts", move |state| {
        Ok(cleanup_alerts_inner(state))
    })
    .await
}

/// Reads the automation status (Tier 0 is always the fallback — this
/// surface never signals).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on lock failure.
#[tauri::command]
pub async fn cleanup_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CleanupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cleanup status", move |state| {
        cleanup_status_inner(state)
    })
    .await
}

/// Sets the global `Pause automatic cleanup` kill-switch (Req.4):
/// takes effect within one tick (the sweep reads the flag first).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on write failure.
#[tauri::command]
pub async fn cleanup_pause_set(
    state: tauri::State<'_, Arc<AppState>>,
    paused: bool,
) -> AppResult<CleanupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cleanup pause set", move |state| {
        cleanup_pause_set_arc(state, paused)
    })
    .await
}

/// The user's Tier-2 decision on one notice (Req.1): `keep` = Undo /
/// Keep running — the candidate is never signalled and the notice is
/// dismissed (exactly one notification per crash-orphan, by construction).
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on lock failure.
#[tauri::command]
pub async fn cleanup_tier2_decide(
    state: tauri::State<'_, Arc<AppState>>,
    pid: u32,
    keep: bool,
) -> AppResult<CleanupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cleanup tier2 decide", move |state| {
        cleanup_tier2_decide_arc(state, pid, keep)
    })
    .await
}

/// Re-confirms automatic cleanup after the 24 h crash suspension
/// (Req.5 — the Cleanup Centre's button; shown, never silent).
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on write failure.
#[tauri::command]
pub async fn cleanup_resume_after_crash(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CleanupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cleanup resume after crash", move |state| {
        cleanup_resume_after_crash_arc(state)
    })
    .await
}

/// Reads one Account's two tier opt-ins (absent rows = both off).
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on read failure.
#[tauri::command]
pub async fn cleanup_tiers_get(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    crate::ipc::blocking(state.inner(), "cleanup tiers get", move |state| {
        cleanup_tiers_get_arc(state, &account_id)
    })
    .await
}

/// The Account launch-settings area's two SEPARATE toggles (Req.4,
/// exact labels in the UI). Also mirrors Tier 1 onto the legacy
/// `accounts.cleanup_opt_in` column so older readers stay truthful.
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on write failure.
#[tauri::command]
pub async fn cleanup_tiers_set(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    tier1_opt_in: bool,
    tier2_opt_in: bool,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    crate::ipc::blocking(state.inner(), "cleanup tiers set", move |state| {
        cleanup_tiers_set_arc(state, &account_id, tier1_opt_in, tier2_opt_in)
    })
    .await
}
