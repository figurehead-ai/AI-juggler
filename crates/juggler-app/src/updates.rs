//! AJ-E1010: updates for the beta — announced, user-initiated,
//! verified and reversible.
//!
//! - **Off unless configured.** The updater exists only in builds that
//!   carry the release signing public key (`AIJ_UPDATER_PUBKEY`, set by
//!   the release workflow). Development builds have none and never make
//!   an update request. With the "check when AI Juggler starts" setting
//!   off (it is on by default), no request is made either.
//! - **Never silent.** A check only reports: the version and its release
//!   notes are shown, and nothing is installed until the user chooses
//!   Install. The app never restarts by itself, and offers a restart only
//!   when no Instance is live.
//! - **Verified.** The package is checked against the release signature
//!   (minisign) before it is installed. A failed download, signature or
//!   install leaves the running version untouched and reports the reason,
//!   with Retry.
//! - **Reversible.** Before installing, a backup (AJ-E1009) of AI
//!   Juggler's data is written to `backups/`, so going back to the
//!   previous version is: install it, then restore that backup.
//! - **Not mid-switch.** Install waits while a switch is in flight.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::{Arc, Mutex};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The release signing public key baked into release builds.
pub const PUBKEY: Option<&str> = option_env!("AIJ_UPDATER_PUBKEY");
/// Where the channel feeds live: each channel's `latest.json` is an
/// asset of the `<channel>-channel` release (D-20). Overridable at build
/// time; `AIJ_UPDATER_ENDPOINT` pins one feed for every channel (tests).
pub const FEED_BASE: &str = match option_env!("AIJ_UPDATER_FEED_BASE") {
    Some(base) => base,
    None => "https://github.com/figurehead-ai/AI-juggler/releases/download",
};
const PINNED_ENDPOINT: Option<&str> = option_env!("AIJ_UPDATER_ENDPOINT");
const CHECK_ON_LAUNCH_KEY: &str = "update_check_on_launch";
const CHANNEL_KEY: &str = "update_channel";

/// D-20: the update channels. One version stream: a build is made once
/// (nightly) and promoted, so moving to a steadier channel never
/// downgrades — it waits until that channel passes this version.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    /// Well-tested releases, promoted from beta with the maintainer's
    /// approval.
    Stable,
    /// Newer features, promoted from nightly with the maintainer's
    /// approval.
    Beta,
    /// Built from `main` every day, no approval.
    Nightly,
}

impl Channel {
    /// Before the first stable release, installs follow beta (D-20).
    pub const DEFAULT: Self = Self::Beta;

    /// The wire and settings name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }

    /// Parses a stored or requested name.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "stable" => Some(Self::Stable),
            "beta" => Some(Self::Beta),
            "nightly" => Some(Self::Nightly),
            _ => None,
        }
    }

    /// The channel's update feed.
    #[must_use]
    pub fn endpoint(self) -> String {
        PINNED_ENDPOINT.map_or_else(
            || format!("{FEED_BASE}/{}-channel/latest.json", self.as_str()),
            str::to_owned,
        )
    }
}

/// The channel this install follows (the user's choice, or the default).
/// Each read also refreshes [`current_channel`].
pub(crate) fn channel_of(state: &Arc<AppState>) -> Channel {
    let channel = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(CHANNEL_KEY).ok().flatten())
        .and_then(|raw| Channel::parse(&raw))
        .unwrap_or(Channel::DEFAULT);
    CURRENT.store(channel as u8, std::sync::atomic::Ordering::Relaxed);
    channel
}

static CURRENT: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(Channel::DEFAULT as u8);

/// The channel last read from settings, for reports built without the
/// app state (usage-stats envelope, crash reports).
#[must_use]
pub fn current_channel() -> Channel {
    match CURRENT.load(std::sync::atomic::Ordering::Relaxed) {
        x if x == Channel::Stable as u8 => Channel::Stable,
        x if x == Channel::Nightly as u8 => Channel::Nightly,
        _ => Channel::Beta,
    }
}
/// A switch edge newer than this, still staged/verified/launched, is in
/// flight.
const SWITCH_IN_FLIGHT_MS: i64 = 30 * 60 * 1000;

/// Whether this build can update itself at all.
#[must_use]
pub fn configured() -> bool {
    PUBKEY.is_some_and(|key| !key.trim().is_empty())
}

/// An update the feed announced.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AnnouncedInfo {
    /// Its version.
    pub version: String,
    /// What changed (the release notes).
    pub notes: String,
    /// When it was published, epoch ms, when known.
    #[ts(type = "number | null")]
    pub date_ms: Option<i64>,
}

/// Where updating stands.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UpdateStatusInfo {
    /// This build's version.
    pub current_version: String,
    /// The channel this install follows: `stable` | `beta` | `nightly`.
    pub channel: String,
    /// This build can update itself (it carries the signing key).
    pub configured: bool,
    /// Check when AI Juggler starts (user setting; on by default).
    pub check_on_launch: bool,
    /// `idle` | `checking` | `up_to_date` | `available` | `backing_up` |
    /// `downloading` | `installed` | `failed`.
    pub state: String,
    /// The announced update, when there is one.
    pub available: Option<AnnouncedInfo>,
    /// Download progress, 0–100, while downloading.
    pub progress: Option<u8>,
    /// Why the last step failed (the running version is untouched).
    pub error: Option<String>,
    /// The backup written before installing, when there is one.
    pub backup_path: Option<String>,
    /// When the last check finished, epoch ms.
    #[ts(type = "number | null")]
    pub checked_at_ms: Option<i64>,
}

impl UpdateStatusInfo {
    fn set_state(&mut self, state: &str) {
        state.clone_into(&mut self.state);
    }
}

/// The update feed: a seam so the policy is testable without a server.
pub trait UpdateFeed: Send + Sync {
    /// Asks the feed for a newer version.
    ///
    /// # Errors
    ///
    /// A plain reason (network, feed format).
    fn check(&self) -> Result<Option<AnnouncedInfo>, String>;
    /// Downloads the announced package and verifies its signature.
    ///
    /// # Errors
    ///
    /// A plain reason (network, signature mismatch).
    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String>;
    /// Installs a downloaded, verified package.
    ///
    /// # Errors
    ///
    /// A plain reason; the running version is untouched.
    fn install(&self, bytes: &[u8]) -> Result<(), String>;
}

/// The in-memory update state (one per app).
#[derive(Debug)]
pub struct UpdateManager {
    status: Mutex<UpdateStatusInfo>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self {
            status: Mutex::new(UpdateStatusInfo {
                current_version: env!("CARGO_PKG_VERSION").to_owned(),
                channel: Channel::DEFAULT.as_str().to_owned(),
                configured: configured(),
                check_on_launch: true,
                state: "idle".to_owned(),
                available: None,
                progress: None,
                error: None,
                backup_path: None,
                checked_at_ms: None,
            }),
        }
    }
}

impl UpdateManager {
    /// A manager for tests (any build), as if configured or not.
    #[must_use]
    pub fn for_test(configured: bool) -> Self {
        let manager = Self::default();
        if let Ok(mut status) = manager.status.lock() {
            status.configured = configured;
        }
        manager
    }

    fn update(&self, change: impl FnOnce(&mut UpdateStatusInfo)) -> UpdateStatusInfo {
        let mut status = self
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        change(&mut status);
        status.clone()
    }
}

pub(crate) fn check_on_launch(state: &Arc<AppState>) -> bool {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| {
            storage
                .app_settings()
                .get(CHECK_ON_LAUNCH_KEY)
                .ok()
                .flatten()
        })
        .is_none_or(|raw| raw != "0")
}

/// Core: where updating stands.
#[must_use]
pub fn update_status_impl(state: &Arc<AppState>) -> UpdateStatusInfo {
    let on = check_on_launch(state);
    let channel = channel_of(state);
    state.updates.update(|status| {
        status.check_on_launch = on;
        channel.as_str().clone_into(&mut status.channel);
    })
}

/// Core: chooses the update channel (D-20). A different feed, so what a
/// previous check found is forgotten; nothing is downloaded until the
/// next check and the user's Install.
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown channel; the typed storage
/// error otherwise.
pub fn update_set_channel_impl(
    state: &Arc<AppState>,
    channel: &str,
) -> AppResult<UpdateStatusInfo> {
    let Some(parsed) = Channel::parse(channel) else {
        return Err(AppError::Validation {
            subject: "update channel",
            message: format!("unknown channel {channel:?} — choose stable, beta or nightly"),
        });
    };
    let busy = state.updates.update(|_| {}).state;
    if ["backing_up", "downloading"].contains(&busy.as_str()) {
        return Err(AppError::Validation {
            subject: "update channel",
            message: "an update is being installed — change the channel when it finishes"
                .to_owned(),
        });
    }
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(CHANNEL_KEY, parsed.as_str())?;
    state.updates.update(|status| {
        if !["installed"].contains(&status.state.as_str()) {
            status.set_state("idle");
            status.available = None;
            status.error = None;
        }
    });
    Ok(update_status_impl(state))
}

/// Core: turns "check when AI Juggler starts" on or off.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn update_set_check_on_launch_impl(
    state: &Arc<AppState>,
    enabled: bool,
) -> AppResult<UpdateStatusInfo> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(CHECK_ON_LAUNCH_KEY, if enabled { "1" } else { "0" })?;
    Ok(update_status_impl(state))
}

/// Core: asks the feed for a newer version. Never installs.
///
/// # Errors
///
/// Returns [`AppError::Validation`] in a build without the updater.
pub fn update_check_impl(
    state: &Arc<AppState>,
    feed: &dyn UpdateFeed,
) -> AppResult<UpdateStatusInfo> {
    if !state.updates.update(|_| {}).configured {
        return Err(AppError::Validation {
            subject: "update",
            message:
                "this build does not update itself (development build) — get releases from GitHub"
                    .to_owned(),
        });
    }
    state.updates.update(|status| {
        status.set_state("checking");
        status.error = None;
    });
    let result = feed.check();
    let now = juggler_core::model::Timestamp::now().millis();
    Ok(state.updates.update(|status| {
        status.checked_at_ms = Some(now);
        match result {
            Ok(Some(announced)) => {
                crate::usage_stats::count(
                    "update.flow",
                    &[("step", "prompt_shown"), ("reason", "none")],
                );
                status.set_state("available");
                status.available = Some(announced);
            }
            Ok(None) => {
                status.set_state("up_to_date");
                status.available = None;
            }
            Err(reason) => {
                status.set_state("failed");
                status.error = Some(format!("Couldn't check for updates: {reason}"));
            }
        }
    }))
}

/// Core: the launch-time check — only when configured and the user's
/// setting is on. Returns whether a request was made.
pub fn launch_check(state: &Arc<AppState>, feed: &dyn UpdateFeed) -> bool {
    if !state.updates.update(|_| {}).configured || !check_on_launch(state) {
        return false;
    }
    let _ = update_check_impl(state, feed);
    true
}

/// A switch edge is still staged, verified or launched (recently): a
/// switch is in flight.
///
/// # Errors
///
/// Returns the typed storage error.
pub(crate) fn switch_in_flight(state: &Arc<AppState>) -> AppResult<bool> {
    Ok(install_blocker(state)?.is_some())
}

/// Why installing has to wait, if it does.
fn install_blocker(state: &Arc<AppState>) -> AppResult<Option<String>> {
    let storage = crate::commands::lock_storage_arc(state)?;
    let since = juggler_core::model::Timestamp::now().millis() - SWITCH_IN_FLIGHT_MS;
    let in_flight: i64 = storage
        .raw_conn()
        .query_row(
            "SELECT COUNT(*) FROM switch_edges
              WHERE state IN ('staged', 'verified', 'launched') AND created_at >= ?1",
            [since],
            |row| row.get(0),
        )
        .map_err(juggler_core::CoreError::from)?;
    Ok((in_flight > 0)
        .then(|| "a switch is in progress — finish it first, then install".to_owned()))
}

fn fail(state: &Arc<AppState>, reason: String) -> UpdateStatusInfo {
    state.updates.update(|status| {
        status.set_state("failed");
        status.progress = None;
        status.error = Some(reason);
    })
}

/// Core: installs the announced update — backup first, then download and
/// verify, then install. Any failure leaves the running version
/// untouched. Never restarts.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when there is nothing to install or a
/// switch is in flight. Download, signature and install failures are
/// reported in the returned status (`failed`, with the reason), not as
/// errors, so the UI can offer Retry.
pub fn update_install_impl(
    state: &Arc<AppState>,
    feed: &dyn UpdateFeed,
    on_progress: &mut dyn FnMut(&UpdateStatusInfo),
) -> AppResult<UpdateStatusInfo> {
    let Some(announced) = state.updates.update(|_| {}).available else {
        return Err(AppError::Validation {
            subject: "update",
            message: "check for updates first — nothing is ready to install".to_owned(),
        });
    };
    if let Some(reason) = install_blocker(state)? {
        return Err(AppError::Validation {
            subject: "update",
            message: reason,
        });
    }
    on_progress(&state.updates.update(|status| {
        status.set_state("backing_up");
        status.error = None;
    }));
    let backups = state.data_dir.join("backups");
    if let Err(e) = std::fs::create_dir_all(&backups) {
        flow_failed("disk");
        return Ok(fail(
            state,
            format!("Couldn't make the pre-update backup: {e}. Nothing was installed."),
        ));
    }
    let current = env!("CARGO_PKG_VERSION");
    let dest = backups.join(format!(
        "before-update-{current}-to-{}-{}.ajbackup",
        announced.version,
        juggler_core::model::Timestamp::now().millis()
    ));
    match crate::backup::backup_export_impl(state, &dest, &mut |_, _| {}) {
        Ok(_) => {}
        Err(e) => {
            flow_failed("disk");
            return Ok(fail(
                state,
                format!("Couldn't make the pre-update backup: {e}. Nothing was installed."),
            ));
        }
    }
    let backup_path = dest.display().to_string();
    on_progress(&state.updates.update(|status| {
        status.set_state("downloading");
        status.progress = Some(0);
        status.backup_path = Some(backup_path.clone());
    }));
    let mut received = 0u64;
    let mut last = 0u8;
    let bytes = feed.download(&mut |chunk, total| {
        received += chunk;
        if let Some(total) = total.filter(|t| *t > 0) {
            let pct = u8::try_from((received.min(total) * 100) / total).unwrap_or(100);
            if pct > last {
                last = pct;
                on_progress(&state.updates.update(|status| status.progress = Some(pct)));
            }
        }
    });
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(reason) => {
            let lower = reason.to_ascii_lowercase();
            flow_failed(if lower.contains("signature") {
                "signature"
            } else if lower.contains("checksum") || lower.contains("hash") {
                "checksum"
            } else {
                "network"
            });
            return Ok(fail(
                state,
                format!(
                    "The download didn't finish or didn't verify: {reason}. Your current version is untouched."
                ),
            ));
        }
    };
    if let Err(reason) = feed.install(&bytes) {
        flow_failed("other");
        return Ok(fail(
            state,
            format!("The install didn't finish: {reason}. Your current version is untouched."),
        ));
    }
    crate::usage_stats::count("update.flow", &[("step", "installed"), ("reason", "none")]);
    Ok(state.updates.update(|status| {
        status.set_state("installed");
        status.progress = Some(100);
    }))
}

/// AJ-E1103: an install that didn't finish, and why (a closed word).
fn flow_failed(reason: &'static str) {
    crate::usage_stats::count("update.flow", &[("step", "failed"), ("reason", reason)]);
}

// --- the real feed (the Tauri updater plugin) ------------------------------

/// The feed backed by the Tauri updater plugin (release builds only).
pub struct PluginFeed {
    app: tauri::AppHandle,
    found: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl PluginFeed {
    /// The feed for this app.
    #[must_use]
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            found: Mutex::new(None),
        }
    }

    fn updater(&self) -> Result<tauri_plugin_updater::Updater, String> {
        use tauri::Manager as _;
        use tauri_plugin_updater::UpdaterExt as _;
        let channel = self
            .app
            .try_state::<Arc<AppState>>()
            .map_or(Channel::DEFAULT, |state| channel_of(state.inner()));
        let url = channel
            .endpoint()
            .parse()
            .map_err(|e| format!("the update feed address is invalid ({e})"))?;
        self.app
            .updater_builder()
            .pubkey(PUBKEY.unwrap_or_default())
            .endpoints(vec![url])
            .map_err(|e| e.to_string())?
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())
    }
}

impl UpdateFeed for PluginFeed {
    fn check(&self) -> Result<Option<AnnouncedInfo>, String> {
        let updater = self.updater()?;
        let found = tauri::async_runtime::block_on(updater.check()).map_err(|e| e.to_string())?;
        let announced = found.as_ref().map(|update| AnnouncedInfo {
            version: update.version.clone(),
            notes: update.body.clone().unwrap_or_default(),
            date_ms: update
                .date
                .map(|date| i64::try_from(date.unix_timestamp_nanos() / 1_000_000).unwrap_or(0)),
        });
        *self.found.lock().map_err(|_| "busy".to_owned())? = found;
        Ok(announced)
    }

    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
        let update = self
            .found
            .lock()
            .map_err(|_| "busy".to_owned())?
            .clone()
            .ok_or_else(|| "check for updates first".to_owned())?;
        // `download` verifies the minisign signature before returning.
        tauri::async_runtime::block_on(
            update.download(|chunk, total| progress(chunk as u64, total), || {}),
        )
        .map_err(|e| e.to_string())
    }

    fn install(&self, bytes: &[u8]) -> Result<(), String> {
        let update = self
            .found
            .lock()
            .map_err(|_| "busy".to_owned())?
            .clone()
            .ok_or_else(|| "check for updates first".to_owned())?;
        // `install` verifies again before replacing anything.
        update.install(bytes).map_err(|e| e.to_string())
    }
}

/// The app's feed, created on first use (release builds only).
static FEED: std::sync::OnceLock<PluginFeed> = std::sync::OnceLock::new();

fn feed(app: &tauri::AppHandle) -> &'static PluginFeed {
    FEED.get_or_init(|| PluginFeed::new(app.clone()))
}

fn emit(app: &tauri::AppHandle, status: &UpdateStatusInfo) {
    use tauri::Emitter as _;
    let _ = app.emit("update://status", status);
}

/// Starts the launch-time check in the background (no wait on launch).
pub fn spawn_launch_check(app: tauri::AppHandle, state: Arc<AppState>) {
    if !configured() {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        if launch_check(&state, feed(&app)) {
            emit(&app, &update_status_impl(&state));
        }
    });
}

/// AJ-E1010: where updating stands.
#[tauri::command]
#[must_use]
pub fn update_status(state: tauri::State<'_, Arc<AppState>>) -> UpdateStatusInfo {
    update_status_impl(state.inner())
}

/// AJ-E1010: turns the launch-time check on or off.
///
/// # Errors
///
/// Per [`update_set_check_on_launch_impl`].
#[tauri::command]
pub fn update_set_check_on_launch(
    state: tauri::State<'_, Arc<AppState>>,
    enabled: bool,
) -> AppResult<UpdateStatusInfo> {
    update_set_check_on_launch_impl(state.inner(), enabled)
}

/// D-20: chooses the update channel.
///
/// # Errors
///
/// Per [`update_set_channel_impl`].
#[tauri::command]
pub fn update_set_channel(
    state: tauri::State<'_, Arc<AppState>>,
    channel: String,
) -> AppResult<UpdateStatusInfo> {
    update_set_channel_impl(state.inner(), &channel)
}

/// AJ-E1010: checks for a newer version now.
///
/// # Errors
///
/// Per [`update_check_impl`].
#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<UpdateStatusInfo> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let status = update_check_impl(&state, feed(&app))?;
        emit(&app, &status);
        Ok(status)
    })
    .await
    .map_err(|e| AppError::Io {
        context: "update",
        message: e.to_string(),
    })?
}

/// AJ-E1010: installs the announced update (backup, download, verify,
/// install; never restarts).
///
/// # Errors
///
/// Per [`update_install_impl`].
#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<UpdateStatusInfo> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let status = update_install_impl(&state, feed(&app), &mut |status| emit(&app, status))?;
        emit(&app, &status);
        Ok(status)
    })
    .await
    .map_err(|e| AppError::Io {
        context: "update",
        message: e.to_string(),
    })?
}

/// Core: why a restart into the installed version has to wait, if it
/// does — nothing installed yet, or Instances still live (a restart
/// would stop them).
///
/// # Errors
///
/// Returns the typed storage error.
pub fn restart_blocker(state: &Arc<AppState>) -> AppResult<Option<String>> {
    if state.updates.update(|_| {}).state != "installed" {
        return Ok(Some("nothing has been installed yet".to_owned()));
    }
    let live = crate::commands::lock_storage_arc(state)?
        .sessions()
        .list(None)?
        .iter()
        .filter(|record| !crate::commands::is_terminal(record.status))
        .count();
    Ok((live > 0).then(|| {
        format!(
            "{live} {} still running — restarting would stop {}. Restart when they finish.",
            if live == 1 {
                "Instance is"
            } else {
                "Instances are"
            },
            if live == 1 { "it" } else { "them" }
        )
    }))
}

/// AJ-E1010: restarts into the installed version — only when no Instance
/// is live (a restart would stop them).
///
/// # Errors
///
/// Returns [`AppError::Validation`] while an Instance is live or nothing
/// was installed.
#[tauri::command]
pub fn update_restart(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<()> {
    if let Some(reason) = restart_blocker(state.inner())? {
        return Err(AppError::Validation {
            subject: "update",
            message: reason,
        });
    }
    app.restart()
}
