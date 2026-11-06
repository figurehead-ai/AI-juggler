//! The dashboard commands. Each runs off the main thread.

use std::sync::Arc;

use juggler_core::model::SessionStatus;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::features::dashboard::dto::StatsInfo;
use crate::state::AppState;

/// Dashboard counts.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message on failure.
pub async fn dashboard_stats(state: tauri::State<'_, Arc<AppState>>) -> AppResult<StatsInfo> {
    crate::ipc::blocking(state.inner(), "dashboard stats", move |state| {
        let ledger_degraded = state.supervisor.ledger_degraded();
        let storage = lock_storage_arc(state)?;
        let accounts = storage.accounts().list()?;
        let mut profiles = 0_u64;
        for account in &accounts {
            profiles += u64::try_from(storage.profiles().list_by_account(&account.id)?.len())
                .unwrap_or_default();
        }
        let active = storage
            .sessions()
            .list(None)?
            .into_iter()
            .filter(|record| {
                matches!(
                    record.status,
                    SessionStatus::Starting | SessionStatus::Running | SessionStatus::WaitingUser
                )
            })
            .count();
        Ok(StatsInfo {
            accounts: u64::try_from(accounts.len()).unwrap_or_default(),
            profiles,
            active_sessions: u64::try_from(active).unwrap_or_default(),
            ledger_degraded,
        })
    })
    .await
}
