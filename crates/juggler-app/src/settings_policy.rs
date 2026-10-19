//! Settings › the policies the user owns (wireframe 26) that have no
//! other home: notification quiet hours and digest mutes (AJ-E611's
//! anti-fatigue defaults, now the user's to change), and the cleanup
//! default for new Accounts (D-08: still an opt-in — off unless the
//! user chooses it, and only ever applied to an Account created after
//! that choice).

// Tauri command signatures take State/values by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use juggler_core::alerts::QuietHours;
use juggler_core::storage::Storage;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The digest's quiet hours: `start-end` (local hours), e.g. `22-7`.
const QUIET_HOURS_KEY: &str = "alert_quiet_hours";
/// `1` when new Accounts start with left-behind dev servers tidied.
const CLEANUP_DEFAULT_KEY: &str = "cleanup_default_tier1";

/// Settings › Notifications.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NotificationPrefsInfo {
    /// First quiet hour (0–23, local): the hourly digest waits until the
    /// quiet hours end. Equal to `quiet_end`: no quiet hours.
    pub quiet_start: u32,
    /// First hour after the quiet hours (0–23, local).
    pub quiet_end: u32,
    /// Accounts whose lines are left out of the hourly digest.
    pub digest_muted_accounts: Vec<String>,
}

/// The quiet hours the digest honours (the default when unset or
/// unreadable).
#[must_use]
pub fn quiet_hours(storage: &Storage) -> QuietHours {
    storage
        .app_settings()
        .get(QUIET_HOURS_KEY)
        .ok()
        .flatten()
        .and_then(|raw| {
            let (start, end) = raw.split_once('-')?;
            let hours = QuietHours {
                start: start.trim().parse().ok()?,
                end: end.trim().parse().ok()?,
            };
            (hours.start < 24 && hours.end < 24).then_some(hours)
        })
        .unwrap_or_default()
}

fn muted_accounts(storage: &Storage) -> Vec<String> {
    storage
        .app_settings()
        .get(crate::commands::ALERT_MUTES_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default()
}

/// Core: Settings › Notifications as stored.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn notification_prefs_get_impl(state: &Arc<AppState>) -> AppResult<NotificationPrefsInfo> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let quiet = quiet_hours(&storage);
    Ok(NotificationPrefsInfo {
        quiet_start: quiet.start,
        quiet_end: quiet.end,
        digest_muted_accounts: muted_accounts(&storage),
    })
}

/// Core: saves Settings › Notifications.
///
/// # Errors
///
/// [`AppError::Validation`] for an hour outside 0–23; the typed storage
/// error otherwise.
pub fn notification_prefs_set_impl(
    state: &Arc<AppState>,
    prefs: &NotificationPrefsInfo,
) -> AppResult<NotificationPrefsInfo> {
    if prefs.quiet_start > 23 || prefs.quiet_end > 23 {
        return Err(AppError::Validation {
            subject: "quiet hours",
            message: "use hours from 0 to 23".to_owned(),
        });
    }
    let mut muted = prefs.digest_muted_accounts.clone();
    muted.sort();
    muted.dedup();
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        let settings = storage.app_settings();
        settings.set(
            QUIET_HOURS_KEY,
            &format!("{}-{}", prefs.quiet_start, prefs.quiet_end),
        )?;
        let blob = serde_json::to_string(&muted).map_err(|error| AppError::Validation {
            subject: "digest mutes",
            message: error.to_string(),
        })?;
        settings.set(crate::commands::ALERT_MUTES_KEY, &blob)?;
    }
    notification_prefs_get_impl(state)
}

/// Whether new Accounts start with left-behind dev servers tidied (the
/// user's own default; off unless chosen).
#[must_use]
pub fn cleanup_default_on(storage: &Storage) -> bool {
    storage
        .app_settings()
        .get(CLEANUP_DEFAULT_KEY)
        .ok()
        .flatten()
        .is_some_and(|raw| raw == "1")
}

/// Core: reads the cleanup default for new Accounts.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn cleanup_default_get_impl(state: &Arc<AppState>) -> AppResult<bool> {
    let storage = crate::commands::lock_storage_arc(state)?;
    Ok(cleanup_default_on(&storage))
}

/// Core: sets the cleanup default for new Accounts. Existing Accounts
/// keep their own choice.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn cleanup_default_set_impl(state: &Arc<AppState>, on: bool) -> AppResult<bool> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(CLEANUP_DEFAULT_KEY, if on { "1" } else { "0" })?;
    cleanup_default_get_impl(state)
}

/// Applies the cleanup default to an Account just created (Tier 1 only:
/// crash tidying is never a default).
pub(crate) fn apply_cleanup_default(state: &Arc<AppState>, account_id: &str) {
    let on =
        crate::commands::lock_storage_arc(state).is_ok_and(|storage| cleanup_default_on(&storage));
    if on {
        let _ = crate::commands::cleanup_tiers_set_arc(state, account_id, true, false);
    }
}

/// Settings › Notifications.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub fn notification_prefs_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<NotificationPrefsInfo> {
    notification_prefs_get_impl(state.inner())
}

/// Saves Settings › Notifications.
///
/// # Errors
///
/// See [`notification_prefs_set_impl`].
#[tauri::command]
pub fn notification_prefs_set(
    state: tauri::State<'_, Arc<AppState>>,
    prefs: NotificationPrefsInfo,
) -> AppResult<NotificationPrefsInfo> {
    notification_prefs_set_impl(state.inner(), &prefs)
}

/// The cleanup default for new Accounts.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub fn cleanup_default_get(state: tauri::State<'_, Arc<AppState>>) -> AppResult<bool> {
    cleanup_default_get_impl(state.inner())
}

/// Sets the cleanup default for new Accounts.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub fn cleanup_default_set(state: tauri::State<'_, Arc<AppState>>, on: bool) -> AppResult<bool> {
    cleanup_default_set_impl(state.inner(), on)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_past_midnight_and_equal_means_none() {
        let night = QuietHours::DEFAULT;
        assert!(night.contains(22) && night.contains(3) && night.contains(6));
        assert!(!night.contains(7) && !night.contains(12) && !night.contains(21));
        let afternoon = QuietHours { start: 13, end: 15 };
        assert!(afternoon.contains(13) && afternoon.contains(14));
        assert!(!afternoon.contains(15) && !afternoon.contains(12));
        let none = QuietHours { start: 9, end: 9 };
        assert!((0..24).all(|hour| !none.contains(hour)));
    }
}
