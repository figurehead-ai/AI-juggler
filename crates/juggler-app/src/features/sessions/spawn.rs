//! The one way an account's session is started: every launch, resume,
//! switch and continue goes through [`spawn_account_session`], so what a
//! session starts with is decided in one place.
//!
//! AJ-E1601 (D-34): a profile added through another service starts with
//! its account's key, read from the Keychain into memory for this launch
//! and put in the variable its tool reads. Never in its arguments (the
//! spawn ledger records those), never in the launch spec the caller keeps,
//! and never for any other account's session.

use std::sync::Arc;

use juggler_adapter::LaunchSpec;
use juggler_core::model::{Account, SessionId};
use juggler_supervisor::SpawnedProcess;

use crate::error::AppResult;
use crate::state::AppState;

/// `spec` as `account`'s session starts it: with the account's key in its
/// environment when it runs through another service, else as it is.
///
/// # Errors
///
/// As [`crate::features::service_accounts::session_env`]: an account
/// whose key isn't in this Mac's Keychain starts nothing.
pub fn launched_spec(
    state: &Arc<AppState>,
    account: &Account,
    spec: &LaunchSpec,
) -> AppResult<LaunchSpec> {
    let mut launched = spec.clone();
    if let Some((name, key)) = crate::features::service_accounts::session_env(state, account)? {
        launched.env.insert(name, key);
    }
    Ok(launched)
}

/// Starts `spec` as `account`'s session `session`, in `project`, under
/// the app's supervisor.
///
/// # Errors
///
/// As [`launched_spec`], and the supervisor's spawn error.
pub(crate) fn spawn_account_session(
    state: &Arc<AppState>,
    session: &SessionId,
    spec: &LaunchSpec,
    account: &Account,
    project: Option<String>,
) -> AppResult<SpawnedProcess> {
    let launched = launched_spec(state, account, spec)?;
    let context = crate::commands::ledger_ctx(state, account, project);
    Ok(state.supervisor.spawn(session, &launched, &context)?)
}
