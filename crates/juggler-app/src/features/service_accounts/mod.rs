//! AJ-E1601 (D-34): accounts through another service. An account here is
//! a subscription to a service that publishes a Claude- or
//! Codex-compatible address (OpenCode Go, Kimi); its profiles are ordinary
//! Claude Code or Codex profiles, each in its own folder AI Juggler made,
//! pointed at the service's published address by the tool's adapter.
//!
//! The key:
//! - is pasted once in Add account and kept in AI Juggler's own Keychain
//!   item, one per account ([`ServiceKeys`]);
//! - is handed only to that account's sessions, when they start, in the
//!   variable the tool reads (`session_env`, from the one spawn path
//!   for an account's sessions, `features::sessions::spawn`);
//! - is never written to a file, a log, the search index or a backup,
//!   never sent back to the window, and goes with the account's last
//!   profile (`profile_removed`).
//!
//! Which profile belongs to which account is a small record in AI
//! Juggler's settings (`service_profile:<profile id>`): the service, the
//! account's id and the model. No key is in it.

pub mod commands;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use juggler_account::SecretStore;
use juggler_adapter::endpoint::{ServiceEndpoint, ServicePreset};
use juggler_core::model::Account;
use juggler_core::storage::Storage;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The settings key prefix of a profile's link to its account.
const LINK_PREFIX: &str = "service_profile:";

/// Where the keys are kept: AI Juggler's own Keychain items in the app
/// (installed at startup), an in-memory store in tests. Until one is
/// installed, no key is kept or read, and no such account can be added.
#[derive(Default)]
pub struct ServiceKeys(RwLock<Option<Arc<dyn SecretStore>>>);

impl ServiceKeys {
    /// Keeps keys in `store` from now on.
    pub fn install(&self, store: Arc<dyn SecretStore>) {
        if let Ok(mut slot) = self.0.write() {
            *slot = Some(store);
        }
    }

    /// The store, when there is one.
    fn store(&self) -> AppResult<Arc<dyn SecretStore>> {
        self.0
            .read()
            .ok()
            .and_then(|slot| slot.clone())
            .ok_or_else(|| {
                AppError::Refused(
                    "Keys for accounts through another service are kept in the macOS Keychain, \
                     which isn't available here"
                        .into(),
                )
            })
    }
}

/// A profile's link to its account: no key in it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Link {
    /// The account (its Keychain item's name): `svc-…`.
    pub account: String,
    /// The service's id.
    pub service: String,
    /// The model the profile was set up with.
    pub model: String,
}

/// Every profile's link, by profile id.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn links(state: &Arc<AppState>) -> AppResult<BTreeMap<String, Link>> {
    let rows = lock_storage_arc(state)?.app_settings().list()?;
    Ok(rows
        .into_iter()
        .filter_map(|(key, value)| {
            let profile = key.strip_prefix(LINK_PREFIX)?.to_owned();
            serde_json::from_str::<Link>(&value)
                .ok()
                .map(|link| (profile, link))
        })
        .collect())
}

/// `profile`'s link, if it runs through another service.
pub(crate) fn link_of(state: &Arc<AppState>, profile: &str) -> AppResult<Option<Link>> {
    let storage = lock_storage_arc(state)?;
    Ok(link_in(&storage, profile))
}

/// [`link_of`] with the storage already in hand.
fn link_in(storage: &Storage, profile: &str) -> Option<Link> {
    storage
        .app_settings()
        .get(&format!("{LINK_PREFIX}{profile}"))
        .ok()
        .flatten()
        .and_then(|value| serde_json::from_str(&value).ok())
}

/// The side of a switch `account` is on (the validation matrix's): its
/// tool, or its tool through another service, whose pairs are never a
/// proven switch (AJ-E1601).
pub(crate) fn switch_side_in(storage: &Storage, account: &Account) -> String {
    if link_in(storage, account.id.as_str()).is_some() {
        juggler_handover::validation::through_service(account.harness.as_str())
    } else {
        account.harness.to_string()
    }
}

/// [`switch_side_in`], taking the storage.
pub(crate) fn switch_side(state: &Arc<AppState>, account: &Account) -> AppResult<String> {
    let storage = lock_storage_arc(state)?;
    Ok(switch_side_in(&storage, account))
}

/// For a profile through another service, whether it can start (its key
/// is in the Keychain, asked of the item's attributes only); `None` for
/// every other profile.
pub(crate) fn ready_in(
    state: &Arc<AppState>,
    storage: &Storage,
    account: &Account,
) -> Option<bool> {
    let link = link_in(storage, account.id.as_str())?;
    Some(
        state
            .service_keys
            .store()
            .is_ok_and(|store| store.has(&link.account)),
    )
}

/// The service and its address for the tool `account` runs.
fn endpoint_for<'a>(
    state: &Arc<AppState>,
    account: &Account,
    preset: &'a ServicePreset,
) -> AppResult<&'a ServiceEndpoint> {
    state
        .registry
        .get(&account.harness)
        .and_then(|adapter| adapter.endpoint_wire())
        .and_then(|wire| preset.endpoint(wire))
        .ok_or_else(|| {
            AppError::Refused(format!(
                "«{}» can't run through {}",
                account.label, preset.name
            ))
        })
}

/// The variable and key `account`'s sessions start with, when it runs
/// through another service: read from the Keychain into memory for this
/// one launch. `None` for every other account.
///
/// # Errors
///
/// [`AppError::Refused`] when the service is no longer known or its key
/// isn't in this Mac's Keychain (a restored backup has no keys).
pub(crate) fn session_env(
    state: &Arc<AppState>,
    account: &Account,
) -> AppResult<Option<(String, String)>> {
    let Some(link) = link_of(state, account.id.as_str())? else {
        return Ok(None);
    };
    let preset = juggler_adapters::services::preset(&link.service).ok_or_else(|| {
        AppError::Refused(format!(
            "«{}» runs through a service this version of AI Juggler doesn't know",
            account.label
        ))
    })?;
    let endpoint = endpoint_for(state, account, preset)?;
    let key = state.service_keys.store()?.get(&link.account).ok_or_else(|| {
        AppError::Refused(format!(
            "«{}»'s {} isn't in this Mac's Keychain. Choose Replace key on its account and paste it again",
            account.label, preset.key_name
        ))
    })?;
    Ok(Some((endpoint.key_env.clone(), key)))
}

/// Why `account`'s sessions may only be started by the user, when its
/// service's terms say so.
pub(crate) fn interactive_only(state: &Arc<AppState>, account: &Account) -> Option<String> {
    let link = link_of(state, account.id.as_str()).ok()??;
    juggler_adapters::services::preset(&link.service)?
        .interactive_only
        .clone()
}

/// After `profile` was removed: its link goes, and with the account's
/// last profile, its key.
pub(crate) fn profile_removed(state: &Arc<AppState>, profile: &str) -> AppResult<()> {
    let Some(link) = link_of(state, profile)? else {
        return Ok(());
    };
    lock_storage_arc(state)?
        .app_settings()
        .set(&format!("{LINK_PREFIX}{profile}"), "")?;
    let active: Vec<String> = lock_storage_arc(state)?
        .accounts()
        .list()?
        .into_iter()
        .map(|account| account.id.to_string())
        .collect();
    let others = links(state)?
        .into_iter()
        .any(|(other, rest)| rest.account == link.account && active.contains(&other));
    if !others && let Ok(store) = state.service_keys.store() {
        store.delete(&link.account);
    }
    Ok(())
}

/// What the screens show about an account's service.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountServiceInfo {
    /// The service's id.
    pub id: String,
    /// Its name (`OpenCode Go`).
    pub name: String,
    /// Its short name (`Go`).
    pub short_name: String,
    /// `estimate` or `unknown`.
    pub usage: String,
    /// How its limits work and what AI Juggler can say, in a line.
    pub usage_words: String,
    /// Where the user sees their usage, on the service's site.
    pub console_url: String,
    /// What its key is called.
    pub key_name: String,
}

impl AccountServiceInfo {
    /// The info for `preset`.
    #[must_use]
    pub fn of(preset: &ServicePreset) -> Self {
        Self {
            id: preset.id.clone(),
            name: preset.name.clone(),
            short_name: preset.short_name.clone(),
            usage: preset.usage.as_str().to_owned(),
            usage_words: preset.usage_words.clone(),
            console_url: preset.console_url.clone(),
            key_name: preset.key_name.clone(),
        }
    }
}

/// What a profile's read finds about its service: its account, the
/// service, and whether the key is in this Mac's Keychain (asked of the
/// item's attributes, never its secret).
#[derive(Clone, Debug)]
pub struct ProfileService {
    /// The account's id.
    pub account: String,
    /// The service.
    pub info: AccountServiceInfo,
    /// The key is there.
    pub key: bool,
}

/// Every linked profile's service, by profile id.
pub(crate) fn profile_services(state: &Arc<AppState>) -> BTreeMap<String, ProfileService> {
    let Ok(links) = links(state) else {
        return BTreeMap::new();
    };
    let store = state.service_keys.store().ok();
    let mut keys: BTreeMap<String, bool> = BTreeMap::new();
    links
        .into_iter()
        .filter_map(|(profile, link)| {
            let preset = juggler_adapters::services::preset(&link.service)?;
            let key = *keys
                .entry(link.account.clone())
                .or_insert_with(|| store.as_ref().is_some_and(|store| store.has(&link.account)));
            Some((
                profile,
                ProfileService {
                    account: link.account,
                    info: AccountServiceInfo::of(preset),
                    key,
                },
            ))
        })
        .collect()
}

/// A profile that runs through another service is ready when its key is
/// in the Keychain: the accounts list says so in its sign-in words.
pub(crate) fn mark_profiles(state: &Arc<AppState>, rows: &mut [crate::commands::AccountInfo]) {
    let services = profile_services(state);
    for row in rows {
        if let Some(found) = services.get(&row.id) {
            row.service = Some(found.info.name.clone());
            let ready = if found.key { "signed_in" } else { "logged_out" };
            ready.clone_into(&mut row.login_state);
        }
    }
}
