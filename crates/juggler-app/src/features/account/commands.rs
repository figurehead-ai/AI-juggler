//! The account commands. Each runs off the main thread.

use std::sync::Arc;

use juggler_core::model::AccountId;

use crate::commands::{find_account_arc, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::features::account::dto::{AccountInfo, ImportResult, SourceInspectInfo};
use crate::features::account::import::{
    ImportRequest, account_import_mode_impl, account_use_original_impl, source_inspect_impl,
};
use crate::features::account::options::{account_set_cleanup_arc, account_set_renewal_arc};
use crate::features::account::{account_info_of, accounts_info, login_state_name, login_state_of};
use crate::state::AppState;

/// AJ-E610 Req.3/E609 Req.4: the user-entered flat subscription fee
/// (USD/month) behind the dual view's ratio. Explicit set/clear only;
/// no plan-tier inference ever writes this. D-62: on the folder's Account.
///
/// # Errors
///
/// Returns the typed [`AppError`] on validation or storage failure.
#[tauri::command]
pub async fn account_set_fee(
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
    account: String,
    fee_usd: Option<f64>,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "account set fee", move |state| {
        let fee = fee_usd.map(|fee| fee.to_string());
        let term = juggler_core::storage::Term::Fee;
        crate::features::subscriptions::terms::set_for_folder(state, &account, term, fee.as_deref())
    })
    .await
}

/// Lists accounts with their login state.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn accounts_list(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<AccountInfo>> {
    crate::ipc::blocking(state.inner(), "list the accounts", accounts_info).await
}

/// Creates a managed account: prepares the isolated harness state dir and
/// persists the account.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn account_create(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    label: String,
    plan: Option<String>,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "account create", move |state| {
        crate::features::account::create_account_for_person(state, &harness, &label, plan)
    })
    .await
}

/// Re-probes an account's login state (metadata-only; ADR-0003).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn account_login_state(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "account login state", move |state| {
        let account = find_account_arc(state, &account_id)?;
        Ok(login_state_name(login_state_of(&state.registry, &account)).to_string())
    })
    .await
}

/// The copyable shell command that runs the vendor's own login against this
/// account's isolated config dir (external-terminal fallback, spec §4.1).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn account_login_command(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<String> {
    crate::ipc::blocking(state.inner(), "account login command", move |state| {
        let account = find_account_arc(state, &account_id)?;
        let adapter = state
            .registry
            .get(&account.harness)
            .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
        juggler_adapter::adapter::shell_snippet(
            adapter.as_ref(),
            &account.state_dir,
            &std::collections::BTreeMap::new(),
        )
        .ok_or_else(|| AppError::Validation {
            subject: "sign-in",
            message: format!("{} has no terminal sign-in command", account.harness),
        })
    })
    .await
}

/// Inspects a candidate import source BEFORE any copy (metadata-only).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] (unknown harness / unreadable path).
pub async fn source_inspect(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    source_path: String,
) -> AppResult<SourceInspectInfo> {
    crate::ipc::blocking(state.inner(), "source inspect", move |state| {
        crate::picked::vouch(&source_path, crate::picked::WORK_FOLDER, |_| false)?;
        source_inspect_impl(state, &harness, &source_path)
    })
    .await
}

/// Brings a discovered setup in as a new account, and creates a default
/// "Main" profile so it is launchable at once. `in_place` (ADR-0018):
/// use the folder where it is — nothing copied or written; otherwise a
/// byte copy into an isolated dir (source untouched).
#[allow(
    clippy::too_many_arguments,
    reason = "the Tauri command's flat wire args"
)]
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn account_import(
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
    source_path: String,
    label: String,
    plan: Option<String>,
    origin: Option<String>,
    degraded: Option<bool>,
    in_place: Option<bool>,
) -> AppResult<ImportResult> {
    crate::picked::vouch(&source_path, crate::picked::WORK_FOLDER, |_| false)?;
    // AJ-E2014: a copy can take seconds; never on the main thread.
    crate::ipc::blocking(state.inner(), "import the account", move |state| {
        let request = ImportRequest {
            plan,
            origin,
            degraded: degraded.unwrap_or(false),
            in_place: in_place.unwrap_or(false),
            // The window's bring-in: the name was on screen (first run's
            // found step) or typed by the user.
            accepted_name: true,
        };
        let imported = account_import_mode_impl(state, &harness, &source_path, &label, request);
        crate::usage_stats::import_finished(&harness, imported.as_ref().err());
        imported
    })
    .await
}

/// ADR-0018: a copied account switches to its original folder, used
/// where it is. The copy stays on disk untouched (nothing is deleted);
/// sessions started in it stay in it.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn account_use_original(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "account use original", move |state| {
        account_use_original_impl(state, &account_id)
    })
    .await
}

/// Opens the vendor's own web sign-in page in the system browser — the
/// maximum login aid allowed (ADR-0003): the user completes the vendor's
/// flow; credentials never touch AI Juggler.
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub async fn login_open_web(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    harness: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "login open web", move |state| {
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(harness.as_str()))
            .ok_or_else(|| AppError::UnknownHarness(harness.clone()))?;
        let Some(url) = adapter.login_url() else {
            return Err(AppError::Io {
                context: "open login page",
                message: format!("{harness} has no web sign-in page"),
            });
        };
        crate::webview_guard::open_web(&app, url, crate::webview_guard::Opened::ByApp)
    })
    .await
}

/// Sets the per-Account live-preview opt-in (Req.3's permanent off
/// switch; enabling is the Screen-Recording consent moment — the first
/// start triggers the system prompt at that point, never before).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown.
pub async fn account_set_preview_opt_in(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "account set preview opt in", move |state| {
        let account = {
            let storage = lock_storage_arc(state)?;
            let mut account = storage
                .accounts()
                .get(&AccountId::new(account_id.clone()))?;
            account.preview_opt_in = opt_in;
            storage.accounts().update(&account)?;
            account
        };
        Ok(account_info_of(state, &account))
    })
    .await
}

/// Sets the per-Account Tier-1 cleanup opt-in (D-08: off by default;
/// the toggle is the only way automatic kills ever happen).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown.
#[tauri::command]
pub async fn account_set_cleanup(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    opt_in: bool,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "account set cleanup", move |state| {
        account_set_cleanup_arc(state, &account_id, opt_in)
    })
    .await
}

/// Sets or clears the optional user-entered renewal date (AJ-E411,
/// D-12). `date` must be a valid local calendar `YYYY-MM-DD` (strictly
/// validated — honest rejection, never a repair); `None` clears it. D-62:
/// it's set on the folder's Account when it has one.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the Account is unknown
/// or the date format is invalid.
#[tauri::command]
pub async fn account_set_renewal(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    date: Option<String>,
) -> AppResult<AccountInfo> {
    crate::ipc::blocking(state.inner(), "account set renewal", move |state| {
        account_set_renewal_arc(state, &account_id, date.as_deref())
    })
    .await
}
