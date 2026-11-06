//! Shared helpers the features' commands use: the storage lock, the local
//! machine id, now, and finding an account or profile. AJ-E2017 moved
//! every command into its feature (`features/<x>/commands.rs`); what is
//! left here is used across them.

use std::sync::Arc;

use juggler_core::model::{Account, AccountId, MachineId, Profile, ProfileId};
use juggler_core::storage::Storage;

use crate::error::AppError;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The stable LOCAL machine id (AJ-E501 Req.6, D-13): generated once,
/// persisted in app settings — never a hard-coded `"this-mac"` literal
/// at call sites.
pub fn local_machine_id(state: &Arc<AppState>) -> MachineId {
    let stored = {
        let storage = lock_storage_arc(state);
        storage
            .ok()
            .and_then(|storage| storage.app_settings().get("local_machine_id").ok())
            .flatten()
    };
    if let Some(id) = stored {
        return MachineId::new(id);
    }
    let generated = format!("machine-{}", unique_suffix());
    if let Ok(storage) = lock_storage_arc(state) {
        let _ = storage.app_settings().set("local_machine_id", &generated);
    }
    MachineId::new(generated)
}

/// The ledger spawn context for an account launch (root role).
pub(crate) fn ledger_ctx(
    state: &Arc<AppState>,
    account: &Account,
    project_id: Option<String>,
) -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: account.harness.to_string(),
        account_id: Some(account.id.to_string()),
        project_id,
        machine: local_machine_id(state),
        role: juggler_core::storage::ROLE_ROOT,
    }
}

/// Maps a core validation rejection to the coded `validation` wire
/// (not retryable); everything else stays a storage failure.
pub(crate) fn core_to_app(error: juggler_core::CoreError) -> AppError {
    match error {
        juggler_core::CoreError::Validation { subject, message } => {
            AppError::Validation { subject, message }
        }
        other => AppError::Core(other),
    }
}

// ---------------------------------------------------------------------------
// helpers

pub(crate) fn lock_storage_arc(
    state: &Arc<AppState>,
) -> Result<std::sync::MutexGuard<'_, Storage>, AppError> {
    state.storage.lock().map_err(|_| AppError::StatePoisoned)
}

/// Core account lookup shared with the terminal module and tests.
///
/// # Errors
///
/// Returns [`AppError::NotFound`] when the account does not exist.
pub fn find_account_arc(state: &Arc<AppState>, account_id: &str) -> Result<Account, AppError> {
    lock_storage_arc(state)?
        .accounts()
        .get(&AccountId::new(account_id))
        .map_err(|error| not_found_or_core(error, "account", account_id))
}

pub(crate) fn find_profile_with_account_arc(
    state: &Arc<AppState>,
    profile_id: &str,
) -> Result<(Profile, Account), AppError> {
    let storage = lock_storage_arc(state)?;
    let profile = storage
        .profiles()
        .get(&ProfileId::new(profile_id))
        .map_err(|error| not_found_or_core(error, "profile", profile_id))?;
    let account = storage
        .accounts()
        .get(&profile.account)
        .map_err(|error| not_found_or_core(error, "account", &profile.account.to_string()))?;
    Ok((profile, account))
}

pub(crate) fn not_found_or_core(
    error: juggler_core::CoreError,
    entity: &'static str,
    id: &str,
) -> AppError {
    match error {
        juggler_core::CoreError::NotFound { .. } => AppError::NotFound {
            entity,
            id: id.to_string(),
        },
        other => AppError::Core(other),
    }
}

// --- AJ-E505: the port occupancy map ----------------------------------

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| i64::try_from(dur.as_millis()).unwrap_or_default())
        .unwrap_or_default()
}

// features/floor still reads these through `crate::commands` (another
// change owns those files); they move with its next pass (AJ-E2017).
pub use crate::features::sessions::dto::{InstanceInfo, NeedsAttention};
pub use crate::features::sessions::list::instances_shared_snapshot;
