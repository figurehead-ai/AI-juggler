//! The optional Figurehead account in the app (AJ-E1202; D-18,
//! ADR-0014). Named `figurehead` throughout: in AI Juggler an
//! "Account" is a Provider Account.
//!
//! - **Signed out is silent.** No call to the service happens unless
//!   the user starts linking; the refresh loop only ever runs while a
//!   device id is stored.
//! - **The device token lives only in the Keychain** (`ai.figurehead.juggler`,
//!   account = the device id). Settings hold the device id, the last
//!   account answer (no secret) and the cloud toggles.
//! - **`401` anywhere = unlinked on the website:** the token goes, every
//!   cloud toggle goes off, local data stays, and the app says so once.
//!
//! Every network call runs off the main thread (`spawn_blocking`).

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_account::model::DeviceToken;
use juggler_account::{
    AccountClient, AccountView, ApiError, DeviceFields, KeychainStore, LinkRequest, PollOutcome,
    SecretStore, Transport, UreqTransport,
};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

const DEVICE_KEY: &str = "figurehead_device_id";
const CACHE_KEY: &str = "figurehead_account_cache";
const REFRESHED_KEY: &str = "figurehead_account_refreshed_ms";
const NOTICE_KEY: &str = "figurehead_notice";
/// The cloud toggles + consent record (AJ-E1203 writes them).
pub const CLOUD_KEY: &str = "figurehead_cloud";
/// The consent screen's version: bump it when what a feature sends
/// changes, and the "Turn on Cloud for this Mac" screen shows again.
pub const CONSENT_VERSION: u32 = 1;
/// Cloud features Account & Cloud shows but doesn't switch: the usage
/// sync is set in Settings › Privacy, beside usage statistics (AJ-E1109).
const NOT_YET_SWITCHABLE: [&str; 1] = ["usageSync"];
/// How often the account is refreshed while linked.
const REFRESH_EVERY_MS: i64 = 6 * 60 * 60 * 1000;

fn now_ms() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

/// A browser link in progress (memory only: the poll token is a secret).
#[derive(Debug)]
struct LinkFlow {
    request: LinkRequest,
    started_at_ms: i64,
    next_poll_at_ms: i64,
    interval_s: u64,
    /// `waiting` | `denied` | `expired` | `error`.
    phase: &'static str,
    message: Option<String>,
}

/// The Figurehead side of the app.
pub struct FigureheadManager {
    transport: Mutex<Option<Arc<dyn Transport>>>,
    secrets: Mutex<Arc<dyn SecretStore>>,
    base: Mutex<String>,
    link: Mutex<Option<LinkFlow>>,
    /// Whether the last refresh failed for lack of a connection.
    offline: Mutex<bool>,
    refresher: std::sync::Once,
}

impl Default for FigureheadManager {
    fn default() -> Self {
        Self {
            transport: Mutex::new(None),
            secrets: Mutex::new(Arc::new(KeychainStore)),
            base: Mutex::new(juggler_account::base_url()),
            link: Mutex::new(None),
            offline: Mutex::new(false),
            refresher: std::sync::Once::new(),
        }
    }
}

impl std::fmt::Debug for FigureheadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FigureheadManager").finish_non_exhaustive()
    }
}

impl FigureheadManager {
    /// The client — built on first use, so an app that never links
    /// never even constructs a network agent.
    fn client(&self) -> AppResult<AccountClient> {
        let mut transport = self.transport.lock().map_err(|_| AppError::StatePoisoned)?;
        let transport = transport
            .get_or_insert_with(|| Arc::new(UreqTransport::new(env!("CARGO_PKG_VERSION"))))
            .clone();
        let base = self
            .base
            .lock()
            .map_err(|_| AppError::StatePoisoned)?
            .clone();
        Ok(AccountClient::new(&base, transport))
    }

    fn secrets(&self) -> Arc<dyn SecretStore> {
        self.secrets.lock().map_or_else(
            |_| Arc::new(KeychainStore) as Arc<dyn SecretStore>,
            |s| s.clone(),
        )
    }

    pub(crate) fn base(&self) -> String {
        self.base.lock().map(|b| b.clone()).unwrap_or_default()
    }
}

// ------------------------------------------------------------ settings

fn setting(state: &Arc<AppState>, key: &str) -> Option<String> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .filter(|value| !value.is_empty())
}

fn set_setting(state: &Arc<AppState>, key: &str, value: &str) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.app_settings().set(key, value)?;
    Ok(())
}

fn device_id(state: &Arc<AppState>) -> Option<String> {
    setting(state, DEVICE_KEY)
}

fn token(state: &Arc<AppState>, device: &str) -> Option<DeviceToken> {
    state.figurehead.secrets().get(device).map(DeviceToken::new)
}

fn cached_account(state: &Arc<AppState>) -> Option<AccountView> {
    setting(state, CACHE_KEY).and_then(|raw| serde_json::from_str(&raw).ok())
}

/// The stored cloud choices (AJ-E1203 consent screen).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloudChoices {
    /// Feature key → on.
    #[serde(default)]
    pub features: std::collections::BTreeMap<String, bool>,
    /// The consent screen version the user confirmed (0 = never).
    #[serde(default)]
    pub consent_version: u32,
    /// When they confirmed, epoch ms.
    #[serde(default)]
    pub consented_at_ms: Option<i64>,
}

/// The stored cloud choices.
#[must_use]
pub fn cloud_choices(state: &Arc<AppState>) -> CloudChoices {
    setting(state, CLOUD_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Stores the cloud choices.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
pub fn save_cloud_choices(state: &Arc<AppState>, choices: &CloudChoices) -> AppResult<()> {
    let raw = serde_json::to_string(choices).map_err(|e| AppError::Io {
        context: "cloud choices",
        message: e.to_string(),
    })?;
    set_setting(state, CLOUD_KEY, &raw)
}

/// Whether a cloud feature is both entitled and turned on — the ONLY
/// gate a cloud call may pass through.
#[must_use]
pub fn feature_on(state: &Arc<AppState>, feature: &str) -> bool {
    device_id(state).is_some()
        && cached_account(state).is_some_and(|account| account.allows(feature))
        && cloud_choices(state)
            .features
            .get(feature)
            .copied()
            .unwrap_or(false)
}

/// Local cleanup after an unlink (from here, or `401` from the
/// website): token gone, every toggle off, local data untouched.
fn forget_link(state: &Arc<AppState>, notice: Option<&str>) {
    if let Some(device) = device_id(state) {
        state.figurehead.secrets().delete(&device);
    }
    let mut choices = cloud_choices(state);
    for on in choices.features.values_mut() {
        *on = false;
    }
    let _ = save_cloud_choices(state, &choices);
    for key in [DEVICE_KEY, CACHE_KEY, REFRESHED_KEY] {
        let _ = set_setting(state, key, "");
    }
    let _ = set_setting(state, NOTICE_KEY, notice.unwrap_or(""));
    if let Ok(mut link) = state.figurehead.link.lock() {
        *link = None;
    }
}

/// Maps a call's error, doing the `401` cleanup centrally.
fn handle(state: &Arc<AppState>, error: ApiError) -> AppError {
    if error == ApiError::Unlinked {
        forget_link(state, Some("This Mac was unlinked from Figurehead."));
    }
    if matches!(error, ApiError::Network(_))
        && let Ok(mut offline) = state.figurehead.offline.lock()
    {
        *offline = true;
    }
    AppError::Validation {
        subject: "Figurehead account",
        message: error.to_string(),
    }
}

// ------------------------------------------------------------- the DTO

/// One cloud feature as Settings shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CloudFeatureInfo {
    /// The service's feature key (`cloudBackup`, …).
    pub key: String,
    /// `included` (the account may use it) | `upgrade` (paid and not
    /// included: a disabled toggle with Upgrade) | `not_included`.
    /// Features the service marks unavailable are left out entirely.
    pub access: String,
    /// `free` | `paid`.
    pub tier: String,
    /// The user's toggle.
    pub on: bool,
}

/// A browser link in progress.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FigureheadLinkInfo {
    /// The code to check in the browser (`QVM3-GABM`).
    pub user_code: String,
    /// `waiting` | `denied` | `expired` | `error`.
    pub phase: String,
    /// When the request expires, epoch ms.
    #[ts(type = "number")]
    pub expires_at_ms: i64,
    /// Plain words for `error`.
    pub message: Option<String>,
}

/// The account, as Settings → Account & Cloud shows it. Holds no secret.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FigureheadStatusInfo {
    /// `signed_out` | `linking` | `linked`.
    pub state: String,
    /// The account e-mail (linked).
    pub email: Option<String>,
    /// The plan name (linked).
    pub plan: Option<String>,
    /// This Mac's name on the service (linked), else the local default.
    pub device_name: String,
    /// The cloud features (linked; never hard-coded free or paid).
    pub features: Vec<CloudFeatureInfo>,
    /// The browser link in progress.
    pub link: Option<FigureheadLinkInfo>,
    /// When the account was last refreshed, epoch ms.
    #[ts(type = "number | null")]
    pub refreshed_at_ms: Option<i64>,
    /// The last refresh couldn't reach the service.
    pub offline: bool,
    /// A one-time notice ("This Mac was unlinked from Figurehead.").
    pub notice: Option<String>,
    /// The consent version the user last confirmed (0 = never).
    pub consent_version: u32,
    /// The current consent version (show the consent screen while
    /// `consent_version` is below it).
    pub consent_current: u32,
}

/// The feature keys the app knows how to show, in display order.
pub const KNOWN_FEATURES: [&str; 6] = [
    "cloudBackup",
    "fleetDiscovery",
    "usageSync",
    "alerts",
    "configSync",
    "transcriptVault",
];

/// Core: the current status (local only — never a network call).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn status_impl(state: &Arc<AppState>) -> AppResult<FigureheadStatusInfo> {
    let device = device_id(state);
    let account = device.as_ref().and_then(|_| cached_account(state));
    let choices = cloud_choices(state);
    let link = state
        .figurehead
        .link
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .as_ref()
        .map(|flow| FigureheadLinkInfo {
            user_code: flow.request.user_code.clone(),
            phase: flow.phase.to_owned(),
            expires_at_ms: flow.started_at_ms
                + i64::try_from(flow.request.expires_in_s).unwrap_or(600) * 1000,
            message: flow.message.clone(),
        });
    let features = account
        .as_ref()
        .map(|account| {
            KNOWN_FEATURES
                .iter()
                .filter_map(|key| {
                    let entitlement = account
                        .entitlements
                        .get(*key)
                        .filter(|entitlement| entitlement.available)?;
                    Some(CloudFeatureInfo {
                        key: (*key).to_owned(),
                        access: if entitlement.enabled {
                            "included"
                        } else if entitlement.needs_upgrade() {
                            "upgrade"
                        } else {
                            "not_included"
                        }
                        .to_owned(),
                        tier: entitlement.tier.clone(),
                        on: choices.features.get(*key).copied().unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let local_name = crate::fleet::self_label(state).unwrap_or_else(|_| "This Mac".to_owned());
    Ok(FigureheadStatusInfo {
        state: if device.is_some() {
            "linked"
        } else if link.is_some() {
            "linking"
        } else {
            "signed_out"
        }
        .to_owned(),
        email: account.as_ref().and_then(|a| a.email().map(str::to_owned)),
        plan: account
            .as_ref()
            .and_then(|a| a.plan_name().map(str::to_owned)),
        device_name: account
            .as_ref()
            .and_then(|a| a.device_name().map(str::to_owned))
            .unwrap_or(local_name),
        features,
        link,
        refreshed_at_ms: setting(state, REFRESHED_KEY).and_then(|raw| raw.parse().ok()),
        offline: state
            .figurehead
            .offline
            .lock()
            .is_ok_and(|offline| *offline),
        notice: setting(state, NOTICE_KEY),
        consent_version: choices.consent_version,
        consent_current: CONSENT_VERSION,
    })
}

fn device_fields(state: &Arc<AppState>, name: Option<&str>) -> AppResult<DeviceFields> {
    let name = name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map_or_else(
            || crate::fleet::self_label(state),
            |name| Ok(name.to_owned()),
        )?;
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    };
    let os = sysinfo::System::os_version().unwrap_or_default();
    let platform = match std::env::consts::OS {
        "macos" => format!("macOS {os} · {arch}"),
        other => format!("{other} {os} · {arch}"),
    };
    Ok(DeviceFields {
        name,
        platform: platform.chars().take(32).collect(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        public_key: crate::fleet::link_public_key(state)?,
    })
}

/// Stores a freshly delivered token and fetches the account.
fn adopt(state: &Arc<AppState>, device: &str, token: &DeviceToken) -> AppResult<()> {
    state
        .figurehead
        .secrets()
        .set(device, token.expose())
        .map_err(|e| AppError::Validation {
            subject: "Figurehead account",
            message: e.to_string(),
        })?;
    set_setting(state, DEVICE_KEY, device)?;
    set_setting(state, NOTICE_KEY, "")?;
    if let Ok(mut link) = state.figurehead.link.lock() {
        *link = None;
    }
    refresh_impl(state)?;
    start_refresher(state);
    Ok(())
}

/// Core: starts a browser link; the caller opens the verify page.
///
/// # Errors
///
/// Returns the typed [`AppError`] for a bad name or a service error.
pub fn link_start_impl(
    state: &Arc<AppState>,
    name: Option<&str>,
) -> AppResult<FigureheadStatusInfo> {
    let fields = device_fields(state, name)?;
    let request = state
        .figurehead
        .client()?
        .start_link(&fields)
        .map_err(|e| handle(state, e))?;
    let now = now_ms();
    *state
        .figurehead
        .link
        .lock()
        .map_err(|_| AppError::StatePoisoned)? = Some(LinkFlow {
        interval_s: request.interval_s.max(1),
        next_poll_at_ms: now + i64::try_from(request.interval_s.max(1)).unwrap_or(3) * 1000,
        request,
        started_at_ms: now,
        phase: "waiting",
        message: None,
    });
    status_impl(state)
}

/// Core: polls the browser link — at most once per `interval` (an
/// early call just answers the current status).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn link_poll_impl(state: &Arc<AppState>) -> AppResult<FigureheadStatusInfo> {
    let request = {
        let link = state
            .figurehead
            .link
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        match link.as_ref() {
            Some(flow) if flow.phase == "waiting" && now_ms() >= flow.next_poll_at_ms => {
                Some(flow.request.clone())
            }
            _ => None,
        }
    };
    let Some(request) = request else {
        return status_impl(state);
    };
    let outcome = state.figurehead.client()?.poll_link(&request);
    let mut adopt_token = None;
    {
        let mut link = state
            .figurehead
            .link
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        if let Some(flow) = link.as_mut() {
            let wait = |seconds: u64| now_ms() + i64::try_from(seconds).unwrap_or(3) * 1000;
            match outcome {
                Ok(PollOutcome::Pending) => flow.next_poll_at_ms = wait(flow.interval_s),
                Ok(PollOutcome::SlowDown { interval_s }) => {
                    flow.interval_s = interval_s.max(flow.interval_s);
                    flow.next_poll_at_ms = wait(flow.interval_s);
                }
                Ok(PollOutcome::Approved { device_id, token }) => {
                    adopt_token = Some((device_id, token));
                }
                Ok(PollOutcome::Denied) => flow.phase = "denied",
                Ok(PollOutcome::Expired | PollOutcome::NotFound) => flow.phase = "expired",
                Err(ApiError::Network(_)) => {
                    // Offline for a moment: keep waiting (the request
                    // itself carries on server-side).
                    flow.next_poll_at_ms = wait(flow.interval_s);
                }
                Err(error) => {
                    flow.phase = "error";
                    flow.message = Some(error.to_string());
                }
            }
        }
    }
    if let Some((device, token)) = adopt_token {
        adopt(state, &device, &token)?;
    }
    status_impl(state)
}

/// Core: cancels a browser link in progress.
pub fn link_cancel_impl(state: &Arc<AppState>) {
    if let Ok(mut link) = state.figurehead.link.lock() {
        *link = None;
    }
}

/// Core: links with a dashboard pairing code.
///
/// # Errors
///
/// Returns the typed [`AppError`] (a wrong code says so plainly).
pub fn claim_impl(
    state: &Arc<AppState>,
    code: &str,
    name: Option<&str>,
) -> AppResult<FigureheadStatusInfo> {
    let fields = device_fields(state, name)?;
    let (device, token) = state
        .figurehead
        .client()?
        .claim(code, &fields)
        .map_err(|e| handle(state, e))?;
    adopt(state, &device, &token)?;
    status_impl(state)
}

/// Core: refreshes the account and entitlements (linked only).
///
/// # Errors
///
/// Returns the typed [`AppError`]; `401` unlinks locally.
pub fn refresh_impl(state: &Arc<AppState>) -> AppResult<FigureheadStatusInfo> {
    let Some(device) = device_id(state) else {
        return status_impl(state);
    };
    let Some(token) = token(state, &device) else {
        forget_link(
            state,
            Some("This Mac's Figurehead sign-in was missing — connect again."),
        );
        return status_impl(state);
    };
    let account = state
        .figurehead
        .client()?
        .account(&token)
        .map_err(|e| handle(state, e))?;
    let raw = serde_json::to_string(&account).map_err(|e| AppError::Io {
        context: "account cache",
        message: e.to_string(),
    })?;
    set_setting(state, CACHE_KEY, &raw)?;
    set_setting(state, REFRESHED_KEY, &now_ms().to_string())?;
    if let Ok(mut offline) = state.figurehead.offline.lock() {
        *offline = false;
    }
    status_impl(state)
}

/// Core: renames this Mac on the service.
///
/// # Errors
///
/// Returns the typed [`AppError`].
pub fn rename_impl(state: &Arc<AppState>, name: &str) -> AppResult<FigureheadStatusInfo> {
    let device = device_id(state).ok_or_else(|| AppError::Validation {
        subject: "Figurehead account",
        message: "Connect an account first.".to_owned(),
    })?;
    let token = token(state, &device).ok_or(AppError::Validation {
        subject: "Figurehead account",
        message: "Connect an account first.".to_owned(),
    })?;
    state
        .figurehead
        .client()?
        .rename(&token, name, env!("CARGO_PKG_VERSION"))
        .map_err(|e| handle(state, e))?;
    refresh_impl(state)
}

/// Core: unlinks this Mac — the service call is best-effort; the local
/// cleanup always happens (offline included).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn unlink_impl(state: &Arc<AppState>) -> AppResult<FigureheadStatusInfo> {
    if let Some(device) = device_id(state)
        && let Some(token) = token(state, &device)
        && let Ok(client) = state.figurehead.client()
    {
        let _ = client.unlink(&token);
    }
    forget_link(state, None);
    status_impl(state)
}

/// Core: stores the user's cloud choices (the consent screen's **Turn
/// on selected**, or a toggle later). A feature the plan doesn't
/// include, or this build can't run yet, is stored off.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when not linked.
pub fn save_choices_impl(
    state: &Arc<AppState>,
    features: &std::collections::BTreeMap<String, bool>,
) -> AppResult<FigureheadStatusInfo> {
    if device_id(state).is_none() {
        return Err(AppError::Validation {
            subject: "Figurehead account",
            message: "Connect an account first.".to_owned(),
        });
    }
    let account = cached_account(state);
    let mut choices = cloud_choices(state);
    for key in KNOWN_FEATURES {
        let wanted = features.get(key).copied().unwrap_or(false);
        let allowed = account.as_ref().is_some_and(|account| account.allows(key))
            && !NOT_YET_SWITCHABLE.contains(&key);
        choices.features.insert(key.to_owned(), wanted && allowed);
    }
    choices.consent_version = CONSENT_VERSION;
    choices.consented_at_ms = Some(now_ms());
    save_cloud_choices(state, &choices)?;
    status_impl(state)
}

/// Core: dismisses the one-time notice.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
pub fn dismiss_notice_impl(state: &Arc<AppState>) -> AppResult<FigureheadStatusInfo> {
    set_setting(state, NOTICE_KEY, "")?;
    status_impl(state)
}

/// The URL for `target`, always on the service's own host — the app
/// never opens an address the server or a page made up.
fn page_url(state: &Arc<AppState>, target: &str) -> Option<String> {
    let base = state.figurehead.base();
    match target {
        "verify" => state
            .figurehead
            .link
            .lock()
            .ok()?
            .as_ref()
            .map(|flow| flow.request.verify_url.clone())
            .filter(|url| url.starts_with(&format!("{base}/"))),
        "dashboard" => Some(format!("{base}/dashboard")),
        "devices" => Some(format!("{base}/dashboard/devices")),
        "upgrade" => cached_account(state)
            .and_then(|account| account.upgrade_url)
            .filter(|url| url.starts_with(&format!("{base}/")))
            .or_else(|| Some(format!("{base}/dashboard"))),
        _ => None,
    }
}

/// Starts the 6-hourly refresh (once per process; only while linked).
pub fn start_refresher(state: &Arc<AppState>) {
    let weak = Arc::downgrade(state);
    state.figurehead.refresher.call_once(move || {
        let _ = std::thread::Builder::new()
            .name("aij-account".to_owned())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                    let Some(state) = weak.upgrade() else {
                        return;
                    };
                    if device_id(&state).is_none() {
                        continue;
                    }
                    let last = setting(&state, REFRESHED_KEY)
                        .and_then(|raw| raw.parse::<i64>().ok())
                        .unwrap_or(0);
                    if now_ms() - last >= REFRESH_EVERY_MS {
                        let _ = refresh_impl(&state);
                    }
                }
            });
    });
}

/// At launch: refresh once if linked (off the main thread) and keep
/// refreshing; signed out, nothing at all happens.
pub fn resume_if_linked(state: Arc<AppState>) {
    if device_id(&state).is_none() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("aij-account-resume".to_owned())
        .spawn(move || {
            let _ = refresh_impl(&state);
            start_refresher(&state);
        });
}

/// AJ-E1205: the client and this Mac's token, for a cloud call that
/// already passed [`feature_on`].
///
/// # Errors
///
/// Returns [`AppError::Validation`] when this Mac is not linked.
pub(crate) fn session(state: &Arc<AppState>) -> AppResult<(AccountClient, DeviceToken)> {
    let not_linked = || AppError::Validation {
        subject: "Figurehead account",
        message: "this Mac isn't linked to Figurehead".to_owned(),
    };
    let device = device_id(state).ok_or_else(not_linked)?;
    let token = token(state, &device).ok_or_else(not_linked)?;
    Ok((state.figurehead.client()?, token))
}

/// AJ-E1109: the personal stats sync (`usageSync`), set only from
/// Settings › Privacy (it rides on usage statistics, so Account & Cloud
/// shows it but doesn't switch it).
///
/// # Errors
///
/// Returns [`AppError::Validation`] when this Mac isn't linked or the
/// account doesn't include it.
pub(crate) fn set_usage_sync(state: &Arc<AppState>, on: bool) -> AppResult<()> {
    if on
        && !(device_id(state).is_some()
            && cached_account(state).is_some_and(|account| account.allows("usageSync")))
    {
        return Err(AppError::Validation {
            subject: "stats sync",
            message: "link this Mac to a Figurehead account that includes it first".to_owned(),
        });
    }
    let mut choices = cloud_choices(state);
    choices.features.insert("usageSync".to_owned(), on);
    save_cloud_choices(state, &choices)
}

/// AJ-E1109: whether the personal stats sync could be offered (linked,
/// and the account includes it), and the account's email.
#[cfg(feature = "stats")]
pub(crate) fn usage_sync_offer(state: &Arc<AppState>) -> (bool, Option<String>) {
    let account = cached_account(state);
    (
        device_id(state).is_some() && account.as_ref().is_some_and(|a| a.allows("usageSync")),
        account.as_ref().and_then(|a| a.email().map(str::to_owned)),
    )
}

/// AJ-E1109: the member endpoint for synced usage-stats reports.
#[cfg(feature = "stats")]
pub(crate) fn stats_sync_url(state: &Arc<AppState>) -> String {
    format!("{}/api/v1/stats/reports", state.figurehead.base())
}

/// AJ-E1109: the service said this Mac's token is no longer valid —
/// unlink locally (every cloud toggle off, local data kept), as any
/// other `401` does.
#[cfg(feature = "stats")]
pub(crate) fn unlinked_by_service(state: &Arc<AppState>) {
    forget_link(state, Some("This Mac was unlinked from Figurehead."));
}

/// AJ-E1205: this Mac's device id on Figurehead, when linked.
pub(crate) fn this_device_id(state: &Arc<AppState>) -> Option<String> {
    device_id(state)
}

/// AJ-E1205: maps a cloud call's error (the `401` cleanup included).
pub(crate) fn handle_error(state: &Arc<AppState>, error: ApiError) -> AppError {
    handle(state, error)
}

/// AJ-E1205: the plan's backup size limit (`maxBytes`), else 25 MB.
pub(crate) fn backup_limit(state: &Arc<AppState>) -> u64 {
    cached_account(state)
        .and_then(|account| account.entitlements.get("cloudBackup").cloned())
        .and_then(|entitlement| {
            entitlement
                .limits
                .get("maxBytes")
                .and_then(serde_json::Value::as_u64)
        })
        .unwrap_or(juggler_account::DEFAULT_BACKUP_LIMIT)
}

/// Test seam: swaps the transport, the secret store and the base URL.
#[doc(hidden)]
pub fn use_for_test(
    state: &Arc<AppState>,
    transport: Arc<dyn Transport>,
    secrets: Arc<dyn SecretStore>,
    base: &str,
) {
    if let Ok(mut slot) = state.figurehead.transport.lock() {
        *slot = Some(transport);
    }
    if let Ok(mut slot) = state.figurehead.secrets.lock() {
        *slot = secrets;
    }
    if let Ok(mut slot) = state.figurehead.base.lock() {
        base.clone_into(&mut slot);
    }
}

/// Test seam: makes the current link request pollable now.
#[doc(hidden)]
pub fn poll_now_for_test(state: &Arc<AppState>) {
    if let Ok(mut link) = state.figurehead.link.lock()
        && let Some(flow) = link.as_mut()
    {
        flow.next_poll_at_ms = 0;
    }
}

// ------------------------------------------------------------ commands

async fn blocking<T: Send + 'static>(
    state: &tauri::State<'_, Arc<AppState>>,
    body: impl FnOnce(&Arc<AppState>) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || body(&state))
        .await
        .map_err(|e| AppError::Io {
            context: "Figurehead account",
            message: e.to_string(),
        })?
}

/// AJ-E1202: the Figurehead status (local; never a network call).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn figurehead_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    status_impl(state.inner())
}

/// AJ-E1202: starts a browser link and opens the approval page.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`link_start_impl`].
#[tauri::command]
pub async fn figurehead_link_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    name: Option<String>,
) -> AppResult<FigureheadStatusInfo> {
    let status = blocking(&state, move |state| link_start_impl(state, name.as_deref())).await?;
    if let Some(url) = page_url(state.inner(), "verify") {
        use tauri_plugin_opener::OpenerExt as _;
        let _ = app.opener().open_url(url, None::<&str>);
    }
    Ok(status)
}

/// AJ-E1202: polls the browser link (at most once per interval).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`link_poll_impl`].
#[tauri::command]
pub async fn figurehead_link_poll(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    blocking(&state, link_poll_impl).await
}

/// AJ-E1202: cancels a browser link.
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn figurehead_link_cancel(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    link_cancel_impl(state.inner());
    status_impl(state.inner())
}

/// AJ-E1202: links with a dashboard pairing code.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`claim_impl`].
#[tauri::command]
pub async fn figurehead_claim(
    state: tauri::State<'_, Arc<AppState>>,
    code: String,
    name: Option<String>,
) -> AppResult<FigureheadStatusInfo> {
    blocking(&state, move |state| {
        claim_impl(state, &code, name.as_deref())
    })
    .await
}

/// AJ-E1202: refreshes the account now.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`refresh_impl`].
#[tauri::command]
pub async fn figurehead_refresh(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    blocking(&state, refresh_impl).await
}

/// AJ-E1202: renames this Mac on the service.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`rename_impl`].
#[tauri::command]
pub async fn figurehead_rename(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
) -> AppResult<FigureheadStatusInfo> {
    blocking(&state, move |state| rename_impl(state, &name)).await
}

/// AJ-E1202: unlinks this Mac.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`unlink_impl`].
#[tauri::command]
pub async fn figurehead_unlink(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    blocking(&state, unlink_impl).await
}

/// AJ-E1203: stores the cloud choices (consent screen / toggles).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`save_choices_impl`].
#[tauri::command]
pub fn figurehead_save_choices(
    state: tauri::State<'_, Arc<AppState>>,
    features: std::collections::BTreeMap<String, bool>,
) -> AppResult<FigureheadStatusInfo> {
    save_choices_impl(state.inner(), &features)
}

/// AJ-E1202: dismisses the one-time notice.
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
#[tauri::command]
pub fn figurehead_dismiss_notice(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<FigureheadStatusInfo> {
    dismiss_notice_impl(state.inner())
}

/// AJ-E1202: opens a Figurehead page (`verify` | `dashboard` | `devices`
/// | `upgrade`) — only ever on the service's own host.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an unknown page.
#[tauri::command]
pub fn figurehead_open_page(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    target: String,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    let url = page_url(state.inner(), &target).ok_or(AppError::Validation {
        subject: "page",
        message: "That page isn't available.".to_owned(),
    })?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Io {
            context: "open Figurehead",
            message: e.to_string(),
        })
}
