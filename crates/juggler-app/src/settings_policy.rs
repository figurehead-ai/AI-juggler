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

/// Accounts left out of the digest (and of system notifications).
#[must_use]
pub fn muted_accounts(storage: &Storage) -> Vec<String> {
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

// --- AJ-E1505: Settings › Sessions -----------------------------------------

/// "Stuck after" (minutes of no output, with low CPU).
const STUCK_AFTER_KEY: &str = "stuck_after_minutes";
/// The terminal's font size (points).
const TERMINAL_FONT_KEY: &str = "terminal_font_size";
/// The terminal's scrollback (lines kept above the screen).
const TERMINAL_SCROLLBACK_KEY: &str = "terminal_scrollback_lines";

/// AJ-E1509: restart a session that crashes (off unless the user asks).
const RESTART_ON_CRASH_KEY: &str = "sessions.restart_on_crash";
/// AJ-E1509: keep the Mac awake while sessions work, on power.
const AWAKE_ON_POWER_KEY: &str = "sessions.keep_awake_on_power";
/// AJ-E1509: …and on battery too.
const AWAKE_ON_BATTERY_KEY: &str = "sessions.keep_awake_on_battery";
/// AJ-E1509: reopen AI Juggler when it quits unexpectedly.
const REOPEN_KEY: &str = "app.reopen_after_crash";

/// The defaults: SPEC §5.1's 5 minutes, and the terminal as it shipped.
pub const STUCK_AFTER_DEFAULT: u32 = 5;
/// The terminal font size as it shipped.
pub const TERMINAL_FONT_DEFAULT: f64 = 12.5;
/// The scrollback as it shipped.
pub const TERMINAL_SCROLLBACK_DEFAULT: u32 = 5_000;

/// The ranges the sheet offers (and the backend holds to).
pub const STUCK_AFTER_RANGE: (u32, u32) = (1, 120);
/// Readable to roomy.
pub const TERMINAL_FONT_RANGE: (f64, f64) = (9.0, 24.0);
/// A screenful to a long day.
pub const TERMINAL_SCROLLBACK_RANGE: (u32, u32) = (500, 100_000);

/// Settings › Sessions.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(clippy::struct_excessive_bools)] // settings: each switch is its own choice
pub struct SessionPrefsInfo {
    /// A session with no output for this many minutes (and low CPU) is
    /// marked Stuck.
    pub stuck_after_minutes: u32,
    /// The terminal's font size, in points.
    pub terminal_font_size: f64,
    /// Lines the terminal keeps above the screen.
    pub terminal_scrollback_lines: u32,
    /// AJ-E1509: a session that crashes is started again and resumed
    /// (a launch setting can override it). Off by default.
    pub restart_on_crash: bool,
    /// AJ-E1509: the Mac stays awake while sessions work, when plugged
    /// in. On by default.
    pub keep_awake_on_power: bool,
    /// AJ-E1509: …on battery too (with the switch above on). Off by
    /// default.
    pub keep_awake_on_battery: bool,
    /// AJ-E1509: AI Juggler is opened again when it quits unexpectedly
    /// (a crash; never a quit the user asked for). On by default.
    pub reopen_after_crash: bool,
}

impl Default for SessionPrefsInfo {
    fn default() -> Self {
        Self {
            stuck_after_minutes: STUCK_AFTER_DEFAULT,
            terminal_font_size: TERMINAL_FONT_DEFAULT,
            terminal_scrollback_lines: TERMINAL_SCROLLBACK_DEFAULT,
            restart_on_crash: false,
            keep_awake_on_power: true,
            keep_awake_on_battery: false,
            reopen_after_crash: true,
        }
    }
}

/// AJ-E1509: what Settings › Sessions says about keeping work going,
/// read where it applies (the restarter, the awake keeper).
#[must_use]
pub fn always_on(storage: &Storage) -> SessionPrefsInfo {
    let defaults = SessionPrefsInfo::default();
    SessionPrefsInfo {
        restart_on_crash: stored(storage, RESTART_ON_CRASH_KEY)
            .unwrap_or(defaults.restart_on_crash),
        keep_awake_on_power: stored(storage, AWAKE_ON_POWER_KEY)
            .unwrap_or(defaults.keep_awake_on_power),
        keep_awake_on_battery: stored(storage, AWAKE_ON_BATTERY_KEY)
            .unwrap_or(defaults.keep_awake_on_battery),
        reopen_after_crash: stored(storage, REOPEN_KEY).unwrap_or(defaults.reopen_after_crash),
        ..defaults
    }
}

fn stored<T: std::str::FromStr>(storage: &Storage, key: &str) -> Option<T> {
    storage
        .app_settings()
        .get(key)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse().ok())
}

/// The stored "Stuck after", when the user set one.
#[must_use]
pub fn stuck_after_minutes(storage: &Storage) -> Option<u32> {
    stored(storage, STUCK_AFTER_KEY)
}

/// The thresholds with the user's "Stuck after" applied: the no-output
/// window and the transcript's, which SPEC §5.1 calls the same window.
#[must_use]
pub fn thresholds_with_stuck_after(
    thresholds: juggler_supervisor::HeuristicThresholds,
    minutes: u32,
) -> juggler_supervisor::HeuristicThresholds {
    let secs = u64::from(minutes) * 60;
    juggler_supervisor::HeuristicThresholds {
        stuck_no_output_secs: secs,
        stuck_transcript_stale_secs: secs,
        ..thresholds
    }
}

/// At launch: `heuristics.json`'s thresholds, with the stored "Stuck
/// after" over them when there is one.
#[must_use]
pub fn with_stored_stuck_after(
    thresholds: juggler_supervisor::HeuristicThresholds,
    storage: &Storage,
) -> juggler_supervisor::HeuristicThresholds {
    stuck_after_minutes(storage).map_or(thresholds, |minutes| {
        thresholds_with_stuck_after(thresholds, minutes)
    })
}

/// Core: Settings › Sessions as it applies now. "Stuck after" is the
/// engine's own window (so a `heuristics.json` value shows as it is).
///
/// # Errors
///
/// Returns the typed storage error.
pub fn session_prefs_get_impl(state: &Arc<AppState>) -> AppResult<SessionPrefsInfo> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let secs = state.heuristics.thresholds().stuck_no_output_secs;
    Ok(SessionPrefsInfo {
        stuck_after_minutes: u32::try_from(secs.div_ceil(60)).unwrap_or(u32::MAX),
        terminal_font_size: stored(&storage, TERMINAL_FONT_KEY).unwrap_or(TERMINAL_FONT_DEFAULT),
        terminal_scrollback_lines: stored(&storage, TERMINAL_SCROLLBACK_KEY)
            .unwrap_or(TERMINAL_SCROLLBACK_DEFAULT),
        ..always_on(&storage)
    })
}

/// Core: saves Settings › Sessions. "Stuck after" reaches the running
/// heuristics at once (their next tick reads it); nothing restarts.
///
/// # Errors
///
/// [`AppError::Validation`] for a value outside its range; the typed
/// storage error otherwise.
pub fn session_prefs_set_impl(
    state: &Arc<AppState>,
    prefs: &SessionPrefsInfo,
) -> AppResult<SessionPrefsInfo> {
    let out_of = |subject: &'static str, message: String| AppError::Validation { subject, message };
    if !(STUCK_AFTER_RANGE.0..=STUCK_AFTER_RANGE.1).contains(&prefs.stuck_after_minutes) {
        return Err(out_of(
            "stuck after",
            format!(
                "use {} to {} minutes",
                STUCK_AFTER_RANGE.0, STUCK_AFTER_RANGE.1
            ),
        ));
    }
    if !(TERMINAL_FONT_RANGE.0..=TERMINAL_FONT_RANGE.1).contains(&prefs.terminal_font_size) {
        return Err(out_of(
            "terminal font size",
            format!(
                "use {} to {} points",
                TERMINAL_FONT_RANGE.0, TERMINAL_FONT_RANGE.1
            ),
        ));
    }
    if !(TERMINAL_SCROLLBACK_RANGE.0..=TERMINAL_SCROLLBACK_RANGE.1)
        .contains(&prefs.terminal_scrollback_lines)
    {
        return Err(out_of(
            "scrollback",
            format!(
                "use {} to {} lines",
                TERMINAL_SCROLLBACK_RANGE.0, TERMINAL_SCROLLBACK_RANGE.1
            ),
        ));
    }
    {
        let storage = crate::commands::lock_storage_arc(state)?;
        let settings = storage.app_settings();
        settings.set(STUCK_AFTER_KEY, &prefs.stuck_after_minutes.to_string())?;
        settings.set(TERMINAL_FONT_KEY, &prefs.terminal_font_size.to_string())?;
        settings.set(
            TERMINAL_SCROLLBACK_KEY,
            &prefs.terminal_scrollback_lines.to_string(),
        )?;
        settings.set(RESTART_ON_CRASH_KEY, &prefs.restart_on_crash.to_string())?;
        settings.set(AWAKE_ON_POWER_KEY, &prefs.keep_awake_on_power.to_string())?;
        settings.set(
            AWAKE_ON_BATTERY_KEY,
            &prefs.keep_awake_on_battery.to_string(),
        )?;
        settings.set(REOPEN_KEY, &prefs.reopen_after_crash.to_string())?;
    }
    // AJ-E1509: the watchdog follows the switch at once.
    crate::features::app::crash::follow_setting(state, prefs.reopen_after_crash);
    state.heuristics.set_thresholds(thresholds_with_stuck_after(
        state.heuristics.thresholds(),
        prefs.stuck_after_minutes,
    ));
    session_prefs_get_impl(state)
}

/// Settings › Sessions.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub async fn session_prefs_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<SessionPrefsInfo> {
    crate::ipc::blocking(state.inner(), "session settings", session_prefs_get_impl).await
}

/// Saves Settings › Sessions (live: no restart).
///
/// # Errors
///
/// See [`session_prefs_set_impl`].
#[tauri::command]
pub async fn session_prefs_set(
    state: tauri::State<'_, Arc<AppState>>,
    prefs: SessionPrefsInfo,
) -> AppResult<SessionPrefsInfo> {
    crate::ipc::blocking(state.inner(), "session settings", move |state| {
        session_prefs_set_impl(state, &prefs)
    })
    .await
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
