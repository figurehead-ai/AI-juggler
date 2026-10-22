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
//!   Install. The app never restarts by itself, and offers "Restart to
//!   update"
//!   only when it is safe: no session running, no switch in flight and
//!   no bring-in under way. Until then the prompt is held back, never
//!   forced.
//! - **Verified.** The package is checked against the release signature
//!   (minisign) before it is installed. A failed download, signature or
//!   install leaves the running version untouched and reports the reason
//!   in plain words ([`FailureKind`]), with Try again.
//! - **Reversible.** Before installing, a backup (AJ-E1009) of AI
//!   Juggler's data is written to `backups/`, so going back to the
//!   previous version is: install it, then restore that backup.
//! - **Not mid-switch.** Install waits while a switch is in flight.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::{Arc, Mutex};

use crate::edition::Edition;
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
/// The channel a main install follows until the user chooses one: the
/// one it was published to, recorded once (see [`first_channel`]).
const CHANNEL_DEFAULT_KEY: &str = "update_channel_default";

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

    /// The channels an edition can follow (D-23): AI Juggler chooses
    /// between stable and beta; AI Juggler Nightly follows nightly; AI
    /// Juggler Dev never updates.
    #[must_use]
    pub fn offered(edition: Edition) -> &'static [Self] {
        match edition {
            Edition::Main => &[Self::Stable, Self::Beta],
            Edition::Nightly => &[Self::Nightly],
            Edition::Dev => &[],
        }
    }

    /// The channel an edition follows until the user chooses one. A Dev
    /// build is built from the newest code, so it reports nightly.
    #[must_use]
    pub fn default_for(edition: Edition) -> Self {
        match edition {
            Edition::Main => Self::DEFAULT,
            Edition::Nightly | Edition::Dev => Self::Nightly,
        }
    }

    /// The channel's update feed. The nightly feed carries AI Juggler
    /// Nightly; beta and stable carry AI Juggler.
    #[must_use]
    pub fn endpoint(self) -> String {
        PINNED_ENDPOINT.map_or_else(
            || format!("{FEED_BASE}/{}-channel/latest.json", self.as_str()),
            str::to_owned,
        )
    }
}

/// The channel this install follows (the user's choice, or the
/// edition's default; a stored choice this edition doesn't offer falls
/// back to the default). Each read also refreshes [`current_channel`].
pub(crate) fn channel_of(state: &Arc<AppState>) -> Channel {
    let edition = crate::edition::current();
    let stored = |key: &str| {
        state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(key).ok().flatten())
            .and_then(|raw| Channel::parse(&raw))
            .filter(|channel| Channel::offered(edition).contains(channel))
    };
    let channel = stored(CHANNEL_KEY)
        .or_else(|| stored(CHANNEL_DEFAULT_KEY))
        .unwrap_or_else(|| Channel::default_for(edition));
    CURRENT.store(channel as u8, std::sync::atomic::Ordering::Relaxed);
    channel
}

/// Not decided yet: a fresh install whose first check hasn't read the
/// stable feed.
const UNDECIDED: &str = "undecided";

/// At startup: a fresh install is marked undecided, to learn its channel
/// from the feeds ([`first_channel`]); an install already in use when
/// this rule arrived keeps the channel it followed (beta, the default
/// until then).
pub(crate) fn note_install(storage: &juggler_core::storage::Storage, fresh: bool) {
    let settings = storage.app_settings();
    if settings.get(CHANNEL_DEFAULT_KEY).ok().flatten().is_none() {
        let value = if fresh {
            UNDECIDED
        } else {
            Channel::Beta.as_str()
        };
        let _ = settings.set(CHANNEL_DEFAULT_KEY, value);
    }
}

/// A fresh main install's channel, recorded on its first check: stable
/// when the stable feed carries this exact version (someone downloaded
/// the stable release), else beta (a beta release; no stable feed yet).
/// Promotion doesn't rebuild, so the build can't know this itself. When
/// the stable feed can't be read, nothing is recorded and beta is used
/// until a later check can. The user's own choice always wins.
fn first_channel(state: &Arc<AppState>, feed: &dyn UpdateFeed) {
    if crate::edition::current() != Edition::Main {
        return;
    }
    let read = |key: &str| {
        state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(key).ok().flatten())
    };
    if read(CHANNEL_KEY).is_some() || read(CHANNEL_DEFAULT_KEY).as_deref() != Some(UNDECIDED) {
        return;
    }
    let Ok(on_stable) = feed.carries_this_version(Channel::Stable) else {
        return;
    };
    let channel = if on_stable {
        Channel::Stable
    } else {
        Channel::Beta
    };
    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .app_settings()
            .set(CHANNEL_DEFAULT_KEY, channel.as_str());
    }
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
    /// This edition (D-23): `main` | `nightly` | `dev`.
    pub edition: String,
    /// The edition's app name (AI Juggler, AI Juggler Nightly, …).
    pub app_name: String,
    /// The channels this edition can choose between (empty: none).
    pub channels: Vec<String>,
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
    /// Why the last step failed, in plain words (the running version is
    /// untouched).
    pub error: Option<String>,
    /// What kind of failure it was ([`FailureKind::as_str`]).
    pub error_kind: Option<String>,
    /// The technical reason behind `error` (shown on request).
    pub error_detail: Option<String>,
    /// Installed, but restarting has to wait — why, in plain words. `None`
    /// once it is safe (or before anything is installed).
    pub restart_wait: Option<String>,
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

/// Why an update step failed — each kind has its own plain-words
/// message; the technical reason travels beside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailureKind {
    /// The server couldn't be reached, or the download stopped.
    Network,
    /// The feed's answer couldn't be read (format, version, platform).
    Feed,
    /// The package failed its signature check: damaged on the way, or not
    /// signed with AI Juggler's release key.
    Signature,
    /// The package is signed for another version than the one offered
    /// (a downgrade attempt, or a mixed-up feed).
    WrongVersion,
    /// The new version couldn't be put in place; the old one is back.
    Install,
    /// Installing failed and the previous app couldn't be moved back.
    NotRestored,
    /// The pre-update backup couldn't be written.
    Disk,
}

impl FailureKind {
    /// The wire name (`error_kind`) and the usage-statistics reason.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Feed => "feed",
            Self::Signature => "signature",
            Self::WrongVersion => "wrong_version",
            Self::Install => "install",
            Self::NotRestored => "not_restored",
            Self::Disk => "disk",
        }
    }

    /// AJ-E1103's closed reason words for `update.flow`.
    fn stats_reason(self) -> &'static str {
        match self {
            Self::Network | Self::Feed => "network",
            Self::Signature | Self::WrongVersion => "signature",
            Self::Disk => "disk",
            Self::Install | Self::NotRestored => "other",
        }
    }
}

/// A failed feed step: what kind, and the technical reason.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FeedError {
    /// What kind of failure.
    pub kind: FailureKind,
    /// The technical reason (for "Details").
    pub detail: String,
}

impl FeedError {
    /// A failure of `kind` with its technical reason.
    pub fn new(kind: FailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

/// The update feed: a seam so the policy is testable without a server.
pub trait UpdateFeed: Send + Sync {
    /// Asks the feed for a newer version.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] (network, feed format).
    fn check(&self) -> Result<Option<AnnouncedInfo>, FeedError>;
    /// Downloads the announced package and verifies its signature.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] (network, signature mismatch).
    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, FeedError>;
    /// Installs a downloaded, verified package.
    ///
    /// # Errors
    ///
    /// A [`FeedError`]; the running version is untouched.
    fn install(&self, bytes: &[u8]) -> Result<(), FeedError>;
    /// Whether `channel`'s feed carries this exact version (a fresh
    /// install's channel, recorded on its first check). A feed that can't tell says
    /// no.
    ///
    /// # Errors
    ///
    /// A [`FeedError`] when the feed can't be read.
    fn carries_this_version(&self, channel: Channel) -> Result<bool, FeedError> {
        let _ = channel;
        Ok(false)
    }
}

/// The in-memory update state (one per app).
#[derive(Debug)]
pub struct UpdateManager {
    status: Mutex<UpdateStatusInfo>,
    /// Bring-ins (account imports) under way right now.
    bring_ins: std::sync::atomic::AtomicUsize,
    /// When the last bring-in ended, epoch ms: a first-run bring-in is a
    /// row of imports, so a short gap between two still counts as running.
    bring_in_ended_ms: std::sync::atomic::AtomicI64,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self {
            status: Mutex::new(UpdateStatusInfo {
                current_version: env!("CARGO_PKG_VERSION").to_owned(),
                channel: Channel::default_for(crate::edition::current())
                    .as_str()
                    .to_owned(),
                edition: crate::edition::current().as_str().to_owned(),
                app_name: crate::edition::current().app_name().to_owned(),
                channels: Channel::offered(crate::edition::current())
                    .iter()
                    .map(|channel| channel.as_str().to_owned())
                    .collect(),
                configured: configured(),
                check_on_launch: true,
                state: "idle".to_owned(),
                available: None,
                progress: None,
                error: None,
                error_kind: None,
                error_detail: None,
                restart_wait: None,
                backup_path: None,
                checked_at_ms: None,
            }),
            bring_ins: std::sync::atomic::AtomicUsize::new(0),
            bring_in_ended_ms: std::sync::atomic::AtomicI64::new(0),
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

    /// Marks a bring-in (an account import) as under way until the
    /// returned guard drops. A restart is not offered meanwhile.
    #[must_use]
    pub fn bring_in(&self) -> BringInGuard<'_> {
        self.bring_ins
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        BringInGuard { manager: self }
    }

    fn bring_in_running(&self, now_ms: i64) -> bool {
        self.bring_ins.load(std::sync::atomic::Ordering::SeqCst) > 0
            || now_ms
                - self
                    .bring_in_ended_ms
                    .load(std::sync::atomic::Ordering::SeqCst)
                < BRING_IN_GAP_MS
    }
}

/// A bring-in under way (see [`UpdateManager::bring_in`]).
#[derive(Debug)]
pub struct BringInGuard<'a> {
    manager: &'a UpdateManager,
}

impl Drop for BringInGuard<'_> {
    fn drop(&mut self) {
        self.manager.bring_in_ended_ms.store(
            juggler_core::model::Timestamp::now().millis(),
            std::sync::atomic::Ordering::SeqCst,
        );
        self.manager
            .bring_ins
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// First run brings accounts in one after another; a gap shorter than
/// this between two imports is still the same bring-in.
const BRING_IN_GAP_MS: i64 = 10_000;

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

/// Core: where updating stands — with, once something is installed,
/// whether a restart has to wait and why.
#[must_use]
pub fn update_status_impl(state: &Arc<AppState>) -> UpdateStatusInfo {
    let on = check_on_launch(state);
    let channel = channel_of(state);
    let wait = restart_wait(state);
    state.updates.update(|status| {
        status.check_on_launch = on;
        channel.as_str().clone_into(&mut status.channel);
        status.restart_wait = wait;
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
    let edition = crate::edition::current();
    let offered = Channel::offered(edition);
    let Some(parsed) = Channel::parse(channel).filter(|parsed| offered.contains(parsed)) else {
        let message = match edition {
            Edition::Main if channel == "nightly" => {
                "nightly builds are a separate app, AI Juggler Nightly, installed next to this one"
                    .to_owned()
            }
            Edition::Main => format!("unknown channel {channel:?} — choose stable or beta"),
            Edition::Nightly => "AI Juggler Nightly always follows the nightly channel".to_owned(),
            Edition::Dev => "AI Juggler Dev is built locally and doesn't update".to_owned(),
        };
        return Err(AppError::Validation {
            subject: "update channel",
            message,
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
            clear_error(status);
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

fn clear_error(status: &mut UpdateStatusInfo) {
    status.error = None;
    status.error_kind = None;
    status.error_detail = None;
}

/// The plain words for a failed check.
fn check_failed_message(kind: FailureKind) -> String {
    match kind {
        FailureKind::Network => {
            "Couldn't reach the update server. Check your internet connection, then try again."
                .to_owned()
        }
        _ => {
            "The update server's answer couldn't be read. Try again later; nothing was downloaded."
                .to_owned()
        }
    }
}

/// The plain words for a failed install; `current` is the running
/// version, which every one of these leaves in place.
fn install_failed_message(kind: FailureKind, app: &str, current: &str) -> String {
    match kind {
        FailureKind::Network => format!(
            "The download stopped before it finished, so nothing was installed. {app} {current} is untouched. Check your internet connection, then try again."
        ),
        FailureKind::Feed => format!(
            "The update server's answer couldn't be read, so nothing was installed. {app} {current} is untouched. Try again later."
        ),
        FailureKind::Signature => format!(
            "The download didn't pass the signature check — it was damaged on the way or isn't from AI Juggler — so it wasn't installed. {app} {current} is untouched."
        ),
        FailureKind::WrongVersion => format!(
            "The download is signed for a different version than the one offered, so it wasn't installed. {app} {current} is untouched."
        ),
        FailureKind::Install => format!(
            "The new version couldn't be put in place, so {app} {current} is still the one installed. Try again; if it keeps failing, download the new version from GitHub."
        ),
        FailureKind::NotRestored => format!(
            "The update didn't finish, and {app} couldn't be put back where it was. The previous app was kept (see Details): move it back to Applications, or download the new version from GitHub."
        ),
        FailureKind::Disk => format!(
            "Couldn't write the backup that comes before every update, so nothing was installed. {app} {current} is untouched. Check there is free space, then try again."
        ),
    }
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
        clear_error(status);
    });
    first_channel(state, feed);
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
            Err(failure) => {
                status.set_state("failed");
                status.available = None;
                status.error = Some(check_failed_message(failure.kind));
                status.error_kind = Some(failure.kind.as_str().to_owned());
                status.error_detail = Some(failure.detail);
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
    switch_in_flight_in(&*crate::commands::lock_storage_arc(state)?)
}

fn switch_in_flight_in(storage: &juggler_core::storage::Storage) -> AppResult<bool> {
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
    Ok(in_flight > 0)
}

/// Why installing has to wait, if it does.
fn install_blocker(state: &Arc<AppState>) -> AppResult<Option<String>> {
    Ok(switch_in_flight(state)?
        .then(|| "a switch is in progress — finish it first, then install".to_owned()))
}

fn fail(state: &Arc<AppState>, failure: FeedError) -> UpdateStatusInfo {
    crate::usage_stats::count(
        "update.flow",
        &[("step", "failed"), ("reason", failure.kind.stats_reason())],
    );
    state.updates.update(|status| {
        status.error = Some(install_failed_message(
            failure.kind,
            &status.app_name,
            &status.current_version,
        ));
        status.set_state("failed");
        status.progress = None;
        status.error_kind = Some(failure.kind.as_str().to_owned());
        status.error_detail = Some(failure.detail);
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
/// errors, so the UI can offer Try again.
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
        clear_error(status);
    }));
    let backups = state.data_dir.join("backups");
    if let Err(e) = std::fs::create_dir_all(&backups) {
        return Ok(fail(
            state,
            FeedError::new(FailureKind::Disk, e.to_string()),
        ));
    }
    let current = env!("CARGO_PKG_VERSION");
    let dest = backups.join(format!(
        "before-update-{current}-to-{}-{}.ajbackup",
        announced.version,
        juggler_core::model::Timestamp::now().millis()
    ));
    if let Err(e) = crate::backup::backup_export_impl(state, &dest, &mut |_, _| {}) {
        return Ok(fail(
            state,
            FeedError::new(FailureKind::Disk, e.to_string()),
        ));
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
        Err(failure) => return Ok(fail(state, failure)),
    };
    if let Err(failure) = feed.install(&bytes) {
        return Ok(fail(state, failure));
    }
    crate::usage_stats::count("update.flow", &[("step", "installed"), ("reason", "none")]);
    state.updates.update(|status| {
        status.set_state("installed");
        status.progress = Some(100);
    });
    Ok(update_status_impl(state))
}

// --- the real feed (the Tauri updater plugin) ------------------------------

/// Sorts the plugin's error into a [`FailureKind`]; `otherwise` is the
/// kind for anything the step doesn't name (an I/O error while
/// downloading is the network's; while installing, the install's).
fn classify(error: &tauri_plugin_updater::Error, otherwise: FailureKind) -> FailureKind {
    use tauri_plugin_updater::Error as E;
    match error {
        E::Minisign(_) | E::Base64(_) | E::SignatureUtf8(_) | E::MissingSignedVersion => {
            FailureKind::Signature
        }
        E::SignedVersionMismatch { .. } => FailureKind::WrongVersion,
        E::Reqwest(_) | E::Network(_) | E::Http(_) => FailureKind::Network,
        E::ReleaseNotFound
        | E::Serialization(_)
        | E::Semver(_)
        | E::UrlParse(_)
        | E::TargetNotFound(_)
        | E::TargetsNotFound(_)
        | E::InvalidUpdaterFormat
        | E::FormatDate
        | E::EmptyEndpoints
        | E::InsecureTransportProtocol => FailureKind::Feed,
        E::PreviousAppNotRestored(_) => FailureKind::NotRestored,
        _ => otherwise,
    }
}

fn feed_error(error: &tauri_plugin_updater::Error, otherwise: FailureKind) -> FeedError {
    FeedError::new(classify(error, otherwise), error.to_string())
}

/// The feed backed by the Tauri updater plugin (release builds; tests
/// pin a local feed, key and install path).
pub struct PluginFeed<R: tauri::Runtime = tauri::Wry> {
    app: tauri::AppHandle<R>,
    pubkey: String,
    endpoint: Option<String>,
    executable: Option<std::path::PathBuf>,
    found: Mutex<Option<tauri_plugin_updater::Update>>,
}

impl<R: tauri::Runtime> PluginFeed<R> {
    /// The feed for this app: its channel's feed, the baked-in key.
    #[must_use]
    pub fn new(app: tauri::AppHandle<R>) -> Self {
        Self {
            app,
            pubkey: PUBKEY.unwrap_or_default().to_owned(),
            endpoint: None,
            executable: None,
            found: Mutex::new(None),
        }
    }

    /// A feed pinned to one endpoint and key, installing over the app
    /// whose executable is `executable` (fixtures: a local server, a
    /// throwaway app bundle).
    #[must_use]
    pub fn pinned(
        app: tauri::AppHandle<R>,
        pubkey: &str,
        endpoint: &str,
        executable: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            app,
            pubkey: pubkey.to_owned(),
            endpoint: Some(endpoint.to_owned()),
            executable,
            found: Mutex::new(None),
        }
    }

    fn updater(&self) -> Result<tauri_plugin_updater::Updater, FeedError> {
        use tauri::Manager as _;
        use tauri_plugin_updater::UpdaterExt as _;
        let endpoint = self.endpoint.clone().unwrap_or_else(|| {
            self.app
                .try_state::<Arc<AppState>>()
                .map_or(Channel::DEFAULT, |state| channel_of(state.inner()))
                .endpoint()
        });
        let url = endpoint.parse().map_err(|e| {
            FeedError::new(
                FailureKind::Feed,
                format!("the update feed address is invalid ({e})"),
            )
        })?;
        let mut builder = self
            .app
            .updater_builder()
            .pubkey(self.pubkey.clone())
            .endpoints(vec![url])
            .map_err(|e| feed_error(&e, FailureKind::Feed))?
            .timeout(std::time::Duration::from_secs(20));
        if let Some(executable) = &self.executable {
            builder = builder.executable_path(executable);
        }
        builder
            .build()
            .map_err(|e| feed_error(&e, FailureKind::Feed))
    }

    fn found(&self) -> Result<tauri_plugin_updater::Update, FeedError> {
        self.found
            .lock()
            .map_err(|_| FeedError::new(FailureKind::Install, "busy"))?
            .clone()
            .ok_or_else(|| FeedError::new(FailureKind::Feed, "check for updates first"))
    }
}

impl<R: tauri::Runtime> UpdateFeed for PluginFeed<R> {
    fn carries_this_version(&self, channel: Channel) -> Result<bool, FeedError> {
        use tauri_plugin_updater::UpdaterExt as _;
        // A build pinned to one feed (fixtures, local tests) can't tell.
        if self.endpoint.is_some() || PINNED_ENDPOINT.is_some() {
            return Ok(false);
        }
        let url = channel.endpoint().parse().map_err(|e| {
            FeedError::new(
                FailureKind::Feed,
                format!("the update feed address is invalid ({e})"),
            )
        })?;
        // Found only when the feed's version is this one.
        let updater = self
            .app
            .updater_builder()
            .pubkey(self.pubkey.clone())
            .endpoints(vec![url])
            .map_err(|e| feed_error(&e, FailureKind::Feed))?
            .version_comparator(|current, remote| remote.version == current)
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| feed_error(&e, FailureKind::Feed))?;
        let found = tauri::async_runtime::block_on(updater.check())
            .map_err(|e| feed_error(&e, FailureKind::Network))?;
        Ok(found.is_some())
    }

    fn check(&self) -> Result<Option<AnnouncedInfo>, FeedError> {
        let updater = self.updater()?;
        let found = tauri::async_runtime::block_on(updater.check())
            .map_err(|e| feed_error(&e, FailureKind::Network))?;
        let announced = found.as_ref().map(|update| AnnouncedInfo {
            version: update.version.clone(),
            notes: update.body.clone().unwrap_or_default(),
            date_ms: update
                .date
                .map(|date| i64::try_from(date.unix_timestamp_nanos() / 1_000_000).unwrap_or(0)),
        });
        *self
            .found
            .lock()
            .map_err(|_| FeedError::new(FailureKind::Feed, "busy"))? = found;
        Ok(announced)
    }

    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, FeedError> {
        let update = self.found()?;
        // `download` verifies the minisign signature before returning.
        tauri::async_runtime::block_on(
            update.download(|chunk, total| progress(chunk as u64, total), || {}),
        )
        .map_err(|e| feed_error(&e, FailureKind::Network))
    }

    fn install(&self, bytes: &[u8]) -> Result<(), FeedError> {
        let update = self.found()?;
        // `install` verifies again before replacing anything.
        update
            .install(bytes)
            .map_err(|e| feed_error(&e, FailureKind::Install))
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

/// After an install: re-reads whether a restart has to wait every few
/// seconds and tells the window when that changes, so "Restart to
/// update" appears once it is safe — and is held back again while a
/// switch or a bring-in starts. Ends when the app restarts.
fn watch_restart(app: tauri::AppHandle, state: Arc<AppState>) {
    static WATCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if WATCHING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let mut last = update_status_impl(&state).restart_wait;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let status = update_status_impl(&state);
            if status.state != "installed" {
                break;
            }
            if status.restart_wait != last {
                last.clone_from(&status.restart_wait);
                emit(&app, &status);
            }
        }
        WATCHING.store(false, std::sync::atomic::Ordering::SeqCst);
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
        if status.state == "installed" {
            watch_restart(app.clone(), Arc::clone(&state));
        }
        Ok(status)
    })
    .await
    .map_err(|e| AppError::Io {
        context: "update",
        message: e.to_string(),
    })?
}

/// Why "Restart to update" has to wait right now, if it does — `None`
/// before anything is installed (there is nothing to restart into) and
/// once it is safe.
fn restart_wait(state: &Arc<AppState>) -> Option<String> {
    match restart_blocker(state) {
        Ok(Some(_)) if state.updates.update(|_| {}).state != "installed" => None,
        Ok(wait) => wait,
        Err(_) => {
            Some("AI Juggler couldn't read what is running — try again in a moment.".to_owned())
        }
    }
}

/// Core: why a restart into the installed version has to wait, if it
/// does — nothing installed yet, a switch in flight, a bring-in under
/// way, or sessions still live (a restart would stop them).
///
/// # Errors
///
/// Returns the typed storage error.
pub fn restart_blocker(state: &Arc<AppState>) -> AppResult<Option<String>> {
    restart_blocker_at(state, juggler_core::model::Timestamp::now().millis())
}

/// [`restart_blocker`] as of `now_ms` (tests step past the bring-in gap).
///
/// # Errors
///
/// Returns the typed storage error.
pub fn restart_blocker_at(state: &Arc<AppState>, now_ms: i64) -> AppResult<Option<String>> {
    if state.updates.update(|_| {}).state != "installed" {
        return Ok(Some("nothing has been installed yet".to_owned()));
    }
    Ok(busy_at(state, now_ms)?.map(|busy| busy.words("Restart to update is offered")))
}

/// What would be cut short if AI Juggler stopped now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Busy {
    /// A switch is staged, verified or launched.
    Switch,
    /// Accounts are being brought in.
    BringIn,
    /// This many sessions are live (stopping would stop them).
    Sessions(usize),
}

impl Busy {
    /// Plain words: what is busy, then "`offered` once it finishes".
    #[must_use]
    pub fn words(self, offered: &str) -> String {
        match self {
            Self::Switch => format!("A switch is in progress. {offered} once it finishes."),
            Self::BringIn => {
                format!("Accounts are being brought in. {offered} once that's done.")
            }
            Self::Sessions(live) => format!(
                "{live} {} still running — stopping AI Juggler would stop {}. {offered} once {} finished.",
                if live == 1 {
                    "session is"
                } else {
                    "sessions are"
                },
                if live == 1 { "it" } else { "them" },
                if live == 1 { "it has" } else { "they have" },
            ),
        }
    }
}

/// Core: whether stopping AI Juggler now would cut something short — a
/// switch in flight, a bring-in under way, live sessions — as of `now_ms`.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn busy_at(state: &Arc<AppState>, now_ms: i64) -> AppResult<Option<Busy>> {
    busy_with(
        &*crate::commands::lock_storage_arc(state)?,
        &state.updates,
        now_ms,
    )
}

/// [`busy_at`] with the storage lock already held (so a caller can act
/// on the answer before anything else starts).
///
/// # Errors
///
/// Returns the typed storage error.
pub fn busy_with(
    storage: &juggler_core::storage::Storage,
    updates: &UpdateManager,
    now_ms: i64,
) -> AppResult<Option<Busy>> {
    if switch_in_flight_in(storage)? {
        return Ok(Some(Busy::Switch));
    }
    if updates.bring_in_running(now_ms) {
        return Ok(Some(Busy::BringIn));
    }
    let live = storage
        .sessions()
        .list(None)?
        .iter()
        .filter(|record| !crate::commands::is_terminal(record.status))
        .count();
    Ok((live > 0).then_some(Busy::Sessions(live)))
}

/// AJ-E1010: restarts into the installed version — only when it is safe
/// (no switch in flight, no bring-in, no live session).
///
/// # Errors
///
/// Returns [`AppError::Validation`] while a restart has to wait or
/// nothing was installed.
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
