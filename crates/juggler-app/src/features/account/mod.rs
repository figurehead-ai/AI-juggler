//! An account (one tool's sign-in folder): the accounts list's read model,
//! adding one, its sign-in state, importing an existing setup (AJ-E307) and
//! its per-account options (fee, cleanup opt-in, renewal date, live preview).

pub mod commands;
pub mod dto;
pub mod import;
pub mod options;

use std::sync::Arc;

use juggler_core::model::{Account, AccountId};

use crate::commands::{find_account_arc, lock_storage_arc, now_millis};
use crate::error::{AppError, AppResult};
use crate::features::account::dto::AccountInfo;
use crate::ids::unique_suffix;
use crate::state::AppState;

/// The accounts in use, each marked whether it is the default
/// (AJ-E1501).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on storage failures.
pub fn accounts_info(state: &Arc<AppState>) -> AppResult<Vec<AccountInfo>> {
    // The rows under the lock; each account's sign-in check (a Keychain
    // lookup) after it, so no other command waits on those.
    let (default, accounts) = {
        let storage = lock_storage_arc(state)?;
        let default = storage
            .app_settings()
            .get(crate::manage::DEFAULT_ACCOUNT_KEY)?;
        (default, storage.accounts().list()?)
    };
    let mut out = Vec::new();
    for account in accounts {
        let mut info = account_info_of(state, &account);
        info.is_default = Some(default.as_deref() == Some(account.id.as_str()));
        out.push(info);
    }
    crate::features::service_accounts::mark_profiles(state, &mut out);
    Ok(out)
}

/// An Account a person makes (Accounts › Add, a service account):
/// startable at once with "Main" launch settings, as an import makes it
/// (the go sitting's 7b run: an App with none couldn't start). Not in
/// [`create_account`]: its other callers make their own, and two became
/// two switch targets.
///
/// # Errors
///
/// As [`create_account`], and storage failures.
pub fn create_account_for_person(
    state: &Arc<AppState>,
    harness: &str,
    label: &str,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    let account = create_account(state, harness, label, plan)?;
    crate::features::profile::ensure_defaults(&*lock_storage_arc(state)?)?;
    Ok(account)
}

/// Core implementation shared with integration tests (commands need a
/// Tauri runtime; this does not).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn create_account(
    state: &Arc<AppState>,
    harness: &str,
    label: &str,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(harness))
        .ok_or_else(|| AppError::UnknownHarness(harness.to_string()))?;
    let id = AccountId::new(format!("{harness}-{}", unique_suffix()));
    let state_dir = state.accounts_dir().join(harness).join(id.as_str());
    adapter.prepare_account(&state_dir)?;
    let account = new_account(id, harness, label, plan, state_dir);
    {
        let storage = lock_storage_arc(state)?;
        storage.accounts().insert(&account)?;
    }
    crate::settings_policy::apply_cleanup_default(state, account.id.as_str());
    let account = find_account_arc(state, account.id.as_str()).unwrap_or(account);
    Ok(account_info_of(state, &account))
}

/// The folder's row for the UI, with the plan and renewal that apply to
/// it (its Account's once moved, D-62). Takes the storage lock: never
/// call with it held.
pub(crate) fn account_info_of(state: &Arc<AppState>, account: &Account) -> AccountInfo {
    let (terms, owner, app_slot, account_slot) =
        lock_storage_arc(state).map_or((None, None, None, None), |storage| {
            let terms = storage.terms();
            let id = account.id.as_str();
            let owner = terms.owner(id).ok().flatten();
            let app_slot = storage
                .apps()
                .cli_colour_slot(account.harness.as_str(), id, now_millis())
                .ok()
                .flatten();
            let account_slot = owner
                .as_deref()
                .and_then(|owner| storage.subscriptions().colour_slot(owner).ok().flatten());
            (terms.applying(account).ok(), owner, app_slot, account_slot)
        });
    let terms = terms.unwrap_or_else(|| juggler_core::storage::Terms {
        plan: account.plan.as_ref().map(ToString::to_string),
        renewal_date: account.renewal_date.clone(),
        fee_usd: None,
    });
    AccountInfo {
        login_state: login_state_name(login_state_of(&state.registry, account)).to_string(),
        id: account.id.to_string(),
        provider: account.harness.to_string(),
        label: account.label.clone(),
        plan: terms.plan,
        cleanup_opt_in: account.cleanup_opt_in,
        renewal_date: terms.renewal_date,
        preview_opt_in: account.preview_opt_in,
        import_source_path: account.import_source_path.clone(),
        import_origin: account.import_origin.clone(),
        imported_at: account.imported_at,
        in_place: account.in_place,
        is_default: None,
        service: None,
        subscription_id: owner,
        app_colour_slot: app_slot,
        account_colour_slot: account_slot,
        agent_tools_why: crate::features::agent_bridge::tools_why_not(state, account),
    }
}

/// Wire-format name for a scan/import login probe.
pub(crate) fn login_probe_name(probe: juggler_adapter::LoginProbe) -> String {
    match probe {
        juggler_adapter::LoginProbe::LoggedIn => "logged_in",
        juggler_adapter::LoginProbe::LoggedOut => "logged_out",
    }
    .to_string()
}

/// A new Account as AI Juggler records it: not imported, nothing opted
/// in. An import sets its provenance on top of this.
pub(crate) fn new_account(
    id: AccountId,
    harness: &str,
    label: &str,
    plan: Option<String>,
    state_dir: std::path::PathBuf,
) -> Account {
    Account {
        id,
        harness: juggler_core::model::HarnessId::new(harness),
        label: label.to_string(),
        plan: plan.map(juggler_core::model::PlanTier::new),
        state_dir,
        cleanup_opt_in: false,
        renewal_date: None,
        preview_opt_in: false,
        import_source_path: None,
        import_origin: None,
        imported_at: None,
        in_place: false,
    }
}

/// Wire-format name for a login state — the AJ-E308 Req.1 taxonomy
/// (`signed_in|partial|expired|sso_pending|api_key|logged_out|unknown`).
/// `unknown` renders "sign in when online", NEVER as logged out.
pub(crate) fn login_state_name(state: juggler_adapters::LoginState) -> &'static str {
    match state {
        juggler_adapters::LoginState::SignedIn => "signed_in",
        juggler_adapters::LoginState::Partial => "partial",
        juggler_adapters::LoginState::Expired => "expired",
        juggler_adapters::LoginState::SsoPending => "sso_pending",
        juggler_adapters::LoginState::ApiKey => "api_key",
        juggler_adapters::LoginState::LoggedOut => "logged_out",
        juggler_adapters::LoginState::Unknown => "unknown",
    }
}

/// The Account's sign-in state from its own Provider's adapter in the
/// app's registry. Unknown Providers report logged out, as before.
pub(crate) fn login_state_of(
    registry: &juggler_adapter::AdapterRegistry,
    account: &Account,
) -> juggler_adapters::LoginState {
    registry
        .get(&account.harness)
        .map_or(juggler_adapters::LoginState::LoggedOut, |adapter| {
            adapter.login_state(&account.state_dir)
        })
}
