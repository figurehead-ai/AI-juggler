//! AJ-E1601: Add account › More › Through another service, and a new key
//! for such an account. Both take the key, so both are writes: never over
//! the read-only test bridge, and the key is never sent back.

use std::sync::Arc;

use juggler_adapter::endpoint::{plausible_key, plausible_model};
use juggler_core::model::AccountId;

use super::{AccountServiceInfo, LINK_PREFIX, Link};
use crate::commands::{AccountInfo, lock_storage_arc};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One tool a service can run, with the models its guide names.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServiceToolInfo {
    /// The tool's id.
    pub tool: String,
    /// The models the service's guide names for it, the usual one first.
    /// Empty: the user types the one they want.
    pub models: Vec<String>,
    /// The service's guide for this tool.
    pub guide: String,
}

/// One service an account can be added through.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServiceInfo {
    /// The service, as the screens show it.
    pub service: AccountServiceInfo,
    /// Who runs it.
    pub vendor: String,
    /// Its plans.
    pub plans: Vec<String>,
    /// The tools installed here that can run through it.
    pub tools: Vec<ServiceToolInfo>,
}

/// One profile to make for the new account.
#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServiceProfileRequest {
    /// The tool it runs.
    pub tool: String,
    /// Its name.
    pub label: String,
    /// The model it starts with.
    pub model: String,
}

/// A new account through another service. Deliberately not `Debug`: the
/// key is in it.
#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServiceAccountRequest {
    /// The service's id.
    pub service: String,
    /// Its plan, one of the service's.
    pub plan: Option<String>,
    /// The key, pasted once.
    pub key: String,
    /// The profiles to make, one per tool.
    pub profiles: Vec<ServiceProfileRequest>,
}

/// Every service, with the registered tools that can run through it.
#[must_use]
pub fn services_list_impl(state: &Arc<AppState>) -> Vec<ServiceInfo> {
    juggler_adapters::services::presets()
        .iter()
        .filter_map(|preset| {
            let tools: Vec<ServiceToolInfo> = state
                .registry
                .ids()
                .into_iter()
                .filter_map(|id| {
                    let wire = state.registry.get(&id)?.endpoint_wire()?;
                    let endpoint = preset.endpoint(wire)?;
                    Some(ServiceToolInfo {
                        tool: id.to_string(),
                        models: endpoint.models.clone(),
                        guide: endpoint.guide.clone(),
                    })
                })
                .collect();
            (!tools.is_empty()).then(|| ServiceInfo {
                service: AccountServiceInfo::of(preset),
                vendor: preset.vendor.clone(),
                plans: preset.plans.clone(),
                tools,
            })
        })
        .collect()
}

/// The services an account can be added through (no key, no secret).
///
/// # Errors
///
/// Only when the blocking pool can't run it.
#[tauri::command]
pub async fn service_accounts_catalog(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<ServiceInfo>> {
    crate::ipc::blocking(state.inner(), "list the services", |state| {
        Ok(services_list_impl(state))
    })
    .await
}

/// Adds an account through another service: keeps the key in the
/// Keychain, then makes each profile in its own folder, pointed at the
/// service's published address. Nothing is left behind when a step fails.
///
/// # Errors
///
/// [`AppError::Refused`] for an unknown service or plan, a key that isn't
/// one, a tool that can't run through it, a model or name that won't do,
/// or no Keychain; the folder's or the Keychain's error.
pub fn service_account_add_impl(
    state: &Arc<AppState>,
    request: &ServiceAccountRequest,
) -> AppResult<Vec<AccountInfo>> {
    let store = state.service_keys.store()?;
    let preset = juggler_adapters::services::preset(&request.service)
        .ok_or_else(|| AppError::Refused("AI Juggler doesn't know that service".into()))?;
    let key = request.key.trim();
    if !plausible_key(key) {
        return Err(AppError::Refused(format!(
            "That doesn't look like a key: paste your {} again, on one line",
            preset.key_name
        )));
    }
    let plan = request
        .plan
        .as_deref()
        .map(str::trim)
        .filter(|plan| !plan.is_empty());
    if let Some(plan) = plan
        && !preset.plans.iter().any(|known| known == plan)
    {
        return Err(AppError::Refused(format!(
            "{} has no plan called «{plan}»",
            preset.name
        )));
    }
    let profiles = checked_profiles(state, preset, &request.profiles)?;
    let account = format!("svc-{}", crate::ids::unique_suffix());
    store
        .set(&account, key)
        .map_err(|error| AppError::Refused(error.to_string()))?;
    let mut made: Vec<AccountInfo> = Vec::new();
    for (tool, label, model, endpoint) in &profiles {
        let step = make_profile(state, &account, preset, tool, label, model, endpoint, plan);
        match step {
            Ok(info) => made.push(info),
            Err(error) => {
                undo(state, &made, &account, store.as_ref());
                return Err(error);
            }
        }
    }
    let ids: Vec<String> = made.into_iter().map(|info| info.id).collect();
    Ok(crate::commands::accounts_info(state)?
        .into_iter()
        .filter(|info| ids.contains(&info.id))
        .collect())
}

/// The request's profiles, checked: a tool once each, one that runs
/// through `preset`, a model and a free name.
fn checked_profiles<'a>(
    state: &Arc<AppState>,
    preset: &'a juggler_adapter::endpoint::ServicePreset,
    requested: &[ServiceProfileRequest],
) -> AppResult<
    Vec<(
        String,
        String,
        String,
        &'a juggler_adapter::endpoint::ServiceEndpoint,
    )>,
> {
    if requested.is_empty() {
        return Err(AppError::Refused(
            "Choose at least one tool to use it with".into(),
        ));
    }
    let mut taken: Vec<String> = lock_storage_arc(state)?
        .accounts()
        .list()?
        .into_iter()
        .map(|account| account.label.trim().to_lowercase())
        .collect();
    let mut tools: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for profile in requested {
        let tool = profile.tool.as_str();
        let name = crate::features::tools::display_name(&state.registry, tool);
        if tools.contains(&tool) {
            return Err(AppError::Refused(format!("{name} is in the list twice")));
        }
        tools.push(tool);
        let endpoint = state
            .registry
            .get(&juggler_core::model::HarnessId::new(tool))
            .and_then(|adapter| adapter.endpoint_wire())
            .and_then(|wire| preset.endpoint(wire))
            .ok_or_else(|| {
                AppError::Refused(format!("{name} can't run through {}", preset.name))
            })?;
        let model = profile.model.trim();
        if !plausible_model(model) {
            return Err(AppError::Refused(format!(
                "Choose the model {name} starts with, as {} names it",
                preset.name
            )));
        }
        let label = profile.label.trim();
        if label.is_empty() || label.chars().count() > crate::manage::LABEL_MAX {
            return Err(AppError::Refused(format!(
                "Give the {name} profile a name of {} characters or fewer",
                crate::manage::LABEL_MAX
            )));
        }
        if taken.contains(&label.to_lowercase()) {
            return Err(AppError::Refused(format!(
                "Another profile is already called «{label}» — pick another name"
            )));
        }
        taken.push(label.to_lowercase());
        out.push((
            tool.to_owned(),
            label.to_owned(),
            model.to_owned(),
            endpoint,
        ));
    }
    Ok(out)
}

/// Makes one profile: its folder, the service's address in it, and its
/// link to the account.
#[allow(clippy::too_many_arguments)] // one profile's facts, checked together above
fn make_profile(
    state: &Arc<AppState>,
    account: &str,
    preset: &juggler_adapter::endpoint::ServicePreset,
    tool: &str,
    label: &str,
    model: &str,
    endpoint: &juggler_adapter::endpoint::ServiceEndpoint,
    plan: Option<&str>,
) -> AppResult<AccountInfo> {
    let info = crate::commands::create_account(state, tool, label, plan.map(str::to_owned))?;
    let made = || {
        let profile = crate::commands::find_account_arc(state, &info.id)?;
        let adapter = state
            .registry
            .get(&profile.harness)
            .ok_or_else(|| AppError::UnknownHarness(tool.to_owned()))?;
        adapter.configure_endpoint(&profile.state_dir, endpoint, model)?;
        let link = Link {
            account: account.to_owned(),
            service: preset.id.clone(),
            model: model.to_owned(),
        };
        let value = serde_json::to_string(&link).map_err(|error| AppError::Io {
            context: "link the profile to its account",
            message: error.to_string(),
        })?;
        let storage = lock_storage_arc(state)?;
        storage
            .app_settings()
            .set(&format!("{LINK_PREFIX}{}", info.id), &value)?;
        // Launch settings with its model, so it starts in one click (as a
        // folder brought in does).
        storage
            .profiles()
            .insert(&juggler_core::model::LaunchRecipe {
                id: juggler_core::model::ProfileId::new(format!(
                    "prof-{}",
                    crate::ids::unique_suffix()
                )),
                name: "Main".to_owned(),
                account: profile.id.clone(),
                settings_overlay: juggler_core::model::SettingsOverlay {
                    model: Some(model.to_owned()),
                    ..juggler_core::model::SettingsOverlay::default()
                },
                default_cwd: None,
            })?;
        Ok::<(), AppError>(())
    };
    match made() {
        Ok(()) => Ok(info),
        Err(error) => {
            undo(state, std::slice::from_ref(&info), account, &NoKeys);
            Err(error)
        }
    }
}

/// Takes back what a failed add made: the profiles (their rows, links and
/// fresh folders) and the key.
fn undo(
    state: &Arc<AppState>,
    made: &[AccountInfo],
    account: &str,
    store: &dyn juggler_account::SecretStore,
) {
    let ours = state.accounts_dir();
    for info in made {
        if let Ok(storage) = lock_storage_arc(state) {
            let id = AccountId::new(info.id.clone());
            if let Ok(profile) = storage.accounts().get(&id)
                && profile.state_dir.starts_with(&ours)
                && !profile.in_place
            {
                let _ = std::fs::remove_dir_all(&profile.state_dir);
            }
            let _ = storage.accounts().delete(&id);
            let _ = storage
                .app_settings()
                .set(&format!("{LINK_PREFIX}{}", info.id), "");
        }
    }
    store.delete(account);
}

/// A store with nothing in it: a single profile's undo leaves the key to
/// the caller's.
struct NoKeys;

impl juggler_account::SecretStore for NoKeys {
    fn get(&self, _account: &str) -> Option<String> {
        None
    }

    fn set(
        &self,
        _account: &str,
        _secret: &str,
    ) -> Result<(), juggler_account::secrets::SecretError> {
        Ok(())
    }

    fn delete(&self, _account: &str) {}
}

/// Adds an account through another service (the key goes to the Keychain
/// and is never sent back).
///
/// # Errors
///
/// As [`service_account_add_impl`].
#[tauri::command]
pub async fn service_accounts_add(
    state: tauri::State<'_, Arc<AppState>>,
    request: ServiceAccountRequest,
) -> AppResult<Vec<AccountInfo>> {
    crate::ipc::blocking(state.inner(), "add the account", move |state| {
        service_account_add_impl(state, &request)
    })
    .await
}

/// Replaces the key of `profile`'s account (every profile on it starts
/// with the new one).
///
/// # Errors
///
/// [`AppError::Refused`] for a profile that doesn't run through another
/// service, a key that isn't one, or no Keychain; the Keychain's error.
pub fn service_account_set_key_impl(
    state: &Arc<AppState>,
    profile: &str,
    key: &str,
) -> AppResult<()> {
    let store = state.service_keys.store()?;
    let link = super::link_of(state, profile)?.ok_or_else(|| {
        AppError::Refused("That profile doesn't run through another service".into())
    })?;
    let preset = juggler_adapters::services::preset(&link.service).ok_or_else(|| {
        AppError::Refused("AI Juggler doesn't know that profile's service".into())
    })?;
    let key = key.trim();
    if !plausible_key(key) {
        return Err(AppError::Refused(format!(
            "That doesn't look like a key: paste your {} again, on one line",
            preset.key_name
        )));
    }
    store
        .set(&link.account, key)
        .map_err(|error| AppError::Refused(error.to_string()))
}

/// Replaces the key of a profile's account.
///
/// # Errors
///
/// As [`service_account_set_key_impl`].
#[tauri::command]
pub async fn service_accounts_set_key(
    state: tauri::State<'_, Arc<AppState>>,
    profile_id: String,
    key: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "save the key", move |state| {
        service_account_set_key_impl(state, &profile_id, &key)
    })
    .await
}
