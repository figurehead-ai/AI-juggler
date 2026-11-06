//! The cleanup automation's status, pause, Tier 2 decisions, resuming after a
//! crash, and each account's tiers.

use std::sync::Arc;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::features::account::options::account_set_cleanup_arc;
use crate::features::cleanup::dto::{CleanupStatusInfo, Tier2NoticeInfo};
use crate::features::cleanup::sweep::{cleanup_paused, cleanup_suspended_until};
use crate::state::AppState;

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
pub fn cleanup_status_inner(state: &Arc<AppState>) -> AppResult<CleanupStatusInfo> {
    Ok(cleanup_status_of(state))
}

/// The status read (shared by the command and the sweep's tests).
fn cleanup_status_of(state: &Arc<AppState>) -> CleanupStatusInfo {
    let tier2_pending = state
        .tier2_pending
        .lock()
        .map(|pending| {
            pending
                .values()
                .filter(|notice| !notice.keep)
                .map(|notice| Tier2NoticeInfo {
                    pid: notice.pid,
                    session: notice.session.clone(),
                    cmdline_trunc: notice.cmdline_trunc.clone(),
                    deadline_ms: notice.deadline_ms,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    CleanupStatusInfo {
        paused: cleanup_paused(state),
        suspended: cleanup_suspended_until(state).is_some(),
        suspended_until_ms: cleanup_suspended_until(state),
        tier2_pending,
    }
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_pause_set_arc(state: &Arc<AppState>, paused: bool) -> AppResult<CleanupStatusInfo> {
    {
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set("cleanup_paused", if paused { "1" } else { "0" })
            .map_err(AppError::from)?;
    }
    Ok(cleanup_status_of(state))
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on lock failure.
pub fn cleanup_tier2_decide_arc(
    state: &Arc<AppState>,
    pid: u32,
    keep: bool,
) -> AppResult<CleanupStatusInfo> {
    if let Ok(mut pending) = state.tier2_pending.lock()
        && let Some(notice) = pending.get_mut(&pid)
    {
        notice.keep = keep;
    }
    Ok(cleanup_status_of(state))
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_resume_after_crash_arc(state: &Arc<AppState>) -> AppResult<CleanupStatusInfo> {
    {
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set("cleanup_suspended_until", "0")
            .map_err(AppError::from)?;
    }
    Ok(cleanup_status_of(state))
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on read failure.
pub fn cleanup_tiers_get_arc(
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    Ok(state.cleanup_settings.get(account_id).unwrap_or_default())
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on write failure.
pub fn cleanup_tiers_set_arc(
    state: &Arc<AppState>,
    account_id: &str,
    tier1_opt_in: bool,
    tier2_opt_in: bool,
) -> AppResult<juggler_core::storage::CleanupTierSettings> {
    let settings = juggler_core::storage::CleanupTierSettings {
        tier1_opt_in,
        tier2_opt_in,
    };
    state
        .cleanup_settings
        .set(account_id, &settings)
        .map_err(AppError::from)?;
    // Mirror Tier 1 onto the prototype column (one honest source:
    // the tier table; the account row follows).
    account_set_cleanup_arc(state, account_id, tier1_opt_in).ok();
    Ok(settings)
}
