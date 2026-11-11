//! An account's own options: its fee (AJ-E611), cleanup opt-in (D-08),
//! renewal date (D-12) and live preview (AJ-E415).

use std::sync::Arc;

use juggler_core::model::AccountId;

use crate::commands::{find_account_arc, lock_storage_arc};
use crate::error::AppResult;
use crate::features::account::account_info_of;
use crate::features::account::dto::AccountInfo;
use crate::state::AppState;

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message when the Account is unknown.
pub fn account_set_cleanup_arc(
    state: &Arc<AppState>,
    account_id: &str,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    let account = {
        let storage = lock_storage_arc(state)?;
        let mut account = storage
            .accounts()
            .get(&AccountId::new(account_id.to_string()))?;
        account.cleanup_opt_in = opt_in;
        storage.accounts().update(&account)?;
        account
    };
    Ok(account_info_of(state, &account))
}

/// The Arc-based core (tests call this directly).
///
/// # Errors
///
/// Returns the typed [`AppError`](crate::error::AppError) message when the Account is unknown
/// or the date format is invalid.
pub fn account_set_renewal_arc(
    state: &Arc<AppState>,
    account_id: &str,
    date: Option<&str>,
) -> AppResult<AccountInfo> {
    let term = juggler_core::storage::Term::Renewal;
    crate::features::subscriptions::terms::set_for_folder(state, account_id, term, date)?;
    Ok(account_info_of(
        state,
        &find_account_arc(state, account_id)?,
    ))
}
