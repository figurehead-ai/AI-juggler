//! The quota display commands and the usage sources' health.

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::quota::QuotaDisplayInfo;

/// AJ-E610: the two-clock display for one account (by id).
///
/// # Errors
///
/// Returns [`AppError::NotFound`](crate::error::AppError::NotFound) when the account is unknown.
#[tauri::command]
pub async fn quota_display(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: String,
) -> AppResult<QuotaDisplayInfo> {
    crate::ipc::blocking(state.inner(), "quota display", move |state| {
        let storage = lock_storage_arc(state)?;
        let record = storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(account.clone()))?;
        let now = juggler_core::model::Timestamp::now().millis();
        let display = crate::features::quota::shown(state, &storage, &record, now);
        // AJ-E1103: the Usage tab is showing an exhaustion forecast.
        if display.forecast.tte_min.is_some() {
            crate::usage_stats::count(
                "usage.forecast_shown",
                &[(
                    "provider",
                    crate::usage_stats::provider(record.harness.as_str()),
                )],
            );
        }
        Ok(display)
    })
    .await
}

/// AJ-E610: the two-clock display for EVERY account (the rail chip /
/// `NewSession` row source; one query, no per-row round trips).
///
/// # Errors
///
/// [`AppError::Io`](crate::error::AppError::Io) only when the background task fails; a storage that
/// can't be read gives an empty list, as before.
#[tauri::command]
pub async fn quota_display_all(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> AppResult<Vec<QuotaDisplayInfo>> {
    crate::ipc::blocking(state.inner(), "show the usage", |state| {
        let Ok(storage) = state.storage.lock() else {
            return Ok(Vec::new());
        };
        let now = juggler_core::model::Timestamp::now().millis();
        Ok(storage
            .accounts()
            .list()
            .unwrap_or_default()
            .iter()
            .map(|account| crate::features::quota::shown(state, &storage, account, now))
            .collect())
    })
    .await
}

/// AJ-E607 Req.5: every ingest source's observable health — the typed
/// state the AJ-E610 "Feed stale" banner binds to. A source silent
/// for more than 10 min carries `last_event_at` older than that (the
/// ENGINE demotes its Measured readings to Stale; data is never
/// deleted).
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn usage_source_health(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> AppResult<Vec<crate::usage_pipeline::SourceHealthInfo>> {
    crate::ipc::blocking(state.inner(), "usage source health", move |state| {
        Ok(crate::usage_pipeline::source_health(state))
    })
    .await
}
