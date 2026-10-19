//! AJ-E1102: usage statistics in the app (D-19, ADR-0015).
//!
//! The choice is stored per Mac as `usage_stats`
//! (`{mode, decided_at_ms, consent_version}`; never in a backup). At
//! launch the overrides are applied and, unless the result is off, the
//! recorder starts. Turning stats off deletes the whole `stats/` folder.
//! Built without the `stats` feature, the crate isn't in the binary and
//! every answer says "not included in this build".

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The setting key.
pub const SETTING_KEY: &str = "usage_stats";
/// The consent text's version (bump it to ask again — AJ-E1105).
pub const CONSENT_VERSION: u32 = 1;

/// The stored choice.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageStatsChoice {
    /// `off` | `local` | `share`.
    pub mode: String,
    /// When the user chose, epoch ms (`None`: never asked).
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered.
    pub consent_version: u32,
}

/// What Settings ▸ Privacy shows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsInfo {
    /// This build includes usage statistics.
    pub included: bool,
    /// The user's choice: `off` | `local` | `share`.
    pub chosen: String,
    /// What applies after the overrides.
    pub effective: String,
    /// Reports may be uploaded.
    pub can_upload: bool,
    /// Why `effective` differs from `chosen`, in plain words.
    pub reason: Option<String>,
    /// When the user chose, epoch ms.
    #[ts(type = "number | null")]
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered (0: never asked).
    pub consent_version: u32,
    /// The current consent version.
    pub current_consent_version: u32,
}

/// The result of changing the mode.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsSetResult {
    /// The new state.
    pub info: UsageStatsInfo,
    /// What happened, in plain words.
    pub message: String,
}

/// The app's `stats/` folder.
#[must_use]
pub fn stats_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("stats")
}

/// The stored choice (off when unset or unreadable).
#[must_use]
pub fn choice(state: &Arc<AppState>) -> UsageStatsChoice {
    let stored = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<UsageStatsChoice>(&raw).ok());
    match stored {
        Some(choice) if matches!(choice.mode.as_str(), "off" | "local" | "share") => choice,
        _ => UsageStatsChoice {
            mode: "off".to_owned(),
            ..UsageStatsChoice::default()
        },
    }
}

fn save(state: &Arc<AppState>, choice: &UsageStatsChoice) -> AppResult<()> {
    let raw = serde_json::to_string(choice).map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })?;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.app_settings().set(SETTING_KEY, &raw)?;
    Ok(())
}

#[cfg(feature = "stats")]
mod enabled {
    use super::{AppError, AppResult, AppState, Arc, UsageStatsChoice, UsageStatsInfo};
    use juggler_stats::{Environment, Mode, effective};

    pub fn mode_of(raw: &str) -> Mode {
        match raw {
            "local" => Mode::Local,
            "share" => Mode::Share,
            _ => Mode::Off,
        }
    }

    pub fn info(choice: &UsageStatsChoice, env: &Environment) -> UsageStatsInfo {
        let applied = effective(mode_of(&choice.mode), env);
        UsageStatsInfo {
            included: true,
            chosen: choice.mode.clone(),
            effective: applied.mode.wire().to_owned(),
            can_upload: applied.can_upload,
            reason: applied.reason.map(|r| r.words().to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: super::CONSENT_VERSION,
        }
    }

    /// Starts (or stops) the recorder for the current choice.
    pub fn apply(state: &Arc<AppState>, env: &Environment) -> AppResult<()> {
        let applied = effective(mode_of(&super::choice(state).mode), env);
        let dir = super::stats_dir(state);
        if applied.mode == Mode::Off {
            juggler_stats::shutdown();
            return Ok(());
        }
        let recorder = juggler_stats::Stats::start(&dir, applied).map_err(|e| AppError::Io {
            context: "usage stats",
            message: e.to_string(),
        })?;
        juggler_stats::install(recorder);
        Ok(())
    }
}

/// Core: the current choice and what applies.
#[must_use]
pub fn usage_stats_info_impl(state: &Arc<AppState>) -> UsageStatsInfo {
    let choice = choice(state);
    #[cfg(feature = "stats")]
    {
        enabled::info(&choice, &juggler_stats::Environment::from_process())
    }
    #[cfg(not(feature = "stats"))]
    {
        UsageStatsInfo {
            included: false,
            chosen: choice.mode,
            effective: "off".to_owned(),
            can_upload: false,
            reason: Some("Not included in this build".to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: CONSENT_VERSION,
        }
    }
}

/// Core: records the user's choice and applies it. Choosing `off` stops
/// the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an unknown mode or a build
/// without usage statistics, and [`AppError::Io`] when the folder can't
/// be deleted or the store can't be opened.
pub fn usage_stats_set_impl(state: &Arc<AppState>, mode: &str) -> AppResult<UsageStatsSetResult> {
    if !matches!(mode, "off" | "local" | "share") {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: format!("unknown mode {mode:?}"),
        });
    }
    if !cfg!(feature = "stats") && mode != "off" {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: "usage statistics are not included in this build".to_owned(),
        });
    }
    save(
        state,
        &UsageStatsChoice {
            mode: mode.to_owned(),
            decided_at_ms: Some(juggler_core::model::Timestamp::now().millis()),
            consent_version: CONSENT_VERSION,
        },
    )?;
    let message = if mode == "off" {
        delete_local(state)?;
        "Usage stats are off. Local stats deleted.".to_owned()
    } else {
        #[cfg(feature = "stats")]
        enabled::apply(state, &juggler_stats::Environment::from_process())?;
        if mode == "local" {
            "Usage stats are counted on this Mac only.".to_owned()
        } else {
            "Usage stats are on. One small report a week will be sent.".to_owned()
        }
    };
    Ok(UsageStatsSetResult {
        info: usage_stats_info_impl(state),
        message,
    })
}

/// Stops the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the folder can't be deleted.
pub fn delete_local(state: &Arc<AppState>) -> AppResult<()> {
    let dir = stats_dir(state);
    #[cfg(feature = "stats")]
    let deleted = juggler_stats::delete_all(&dir);
    #[cfg(not(feature = "stats"))]
    let deleted = match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    };
    deleted.map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })
}

/// At launch: start the recorder if the choice (after the overrides)
/// isn't off. Off creates nothing.
pub fn start(state: Arc<AppState>) {
    #[cfg(feature = "stats")]
    if let Err(e) = enabled::apply(&state, &juggler_stats::Environment::from_process()) {
        eprintln!("ai-juggler: {e}");
    }
    #[cfg(not(feature = "stats"))]
    let _ = state;
}

/// On quit: write what was counted.
pub fn shutdown() {
    #[cfg(feature = "stats")]
    juggler_stats::shutdown();
}

/// AJ-E1102: the usage-stats choice and what applies.
#[tauri::command]
#[must_use]
pub fn usage_stats_info(state: tauri::State<'_, Arc<AppState>>) -> UsageStatsInfo {
    usage_stats_info_impl(state.inner())
}

/// AJ-E1102: sets the usage-stats mode (`off` deletes local stats).
///
/// # Errors
///
/// Per [`usage_stats_set_impl`].
#[tauri::command]
pub fn usage_stats_set(
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> AppResult<UsageStatsSetResult> {
    usage_stats_set_impl(state.inner(), &mode)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(dir.path().join("data")).unwrap());
        (dir, state)
    }

    #[test]
    fn default_is_off_and_creates_nothing() {
        let (_dir, state) = state();
        let info = usage_stats_info_impl(&state);
        assert_eq!(info.chosen, "off");
        assert_eq!(info.effective, "off");
        assert_eq!(info.consent_version, 0);
        start(Arc::clone(&state));
        assert!(!stats_dir(&state).exists());
    }

    #[cfg(feature = "stats")]
    #[test]
    fn choosing_off_deletes_the_folder_and_says_so() {
        let (_dir, state) = state();
        let on = usage_stats_set_impl(&state, "local").unwrap();
        assert_eq!(on.info.chosen, "local");
        assert_eq!(on.info.consent_version, CONSENT_VERSION);
        assert!(stats_dir(&state).join("counters.sqlite").exists());
        let off = usage_stats_set_impl(&state, "off").unwrap();
        assert_eq!(off.message, "Usage stats are off. Local stats deleted.");
        assert!(!stats_dir(&state).exists());
        assert!(usage_stats_set_impl(&state, "sometimes").is_err());
    }

    #[cfg(feature = "stats")]
    #[test]
    fn overrides_show_their_reason() {
        let choice = UsageStatsChoice {
            mode: "share".to_owned(),
            decided_at_ms: Some(1),
            consent_version: 1,
        };
        let env = juggler_stats::Environment {
            do_not_track: Some("1".to_owned()),
            ..juggler_stats::Environment::default()
        };
        let info = enabled::info(&choice, &env);
        assert_eq!(info.effective, "off");
        assert_eq!(
            info.reason.as_deref(),
            Some("Off because DO_NOT_TRACK is set")
        );
    }
}
