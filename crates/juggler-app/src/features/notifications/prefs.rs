//! AJ-E1510 Req.1: which kinds may become a system notification. Each is
//! off until the user turns it on, but one: the watches the user set
//! (AJ-E3817, ADR-0036 §2.5), on by default, since the user asked for
//! each when they confirmed it.

use std::sync::Arc;

use juggler_core::storage::Storage;

use super::engine::Kind;
use crate::error::AppResult;
use crate::state::AppState;

const WAITING_KEY: &str = "notify.waiting";
const STUCK_KEY: &str = "notify.stuck";
const LIMIT_KEY: &str = "notify.limit";
const FINISHED_KEY: &str = "notify.finished";
const THIS_MAC_KEY: &str = "notify.this_mac";
const LEFT_BEHIND_KEY: &str = "notify.left_behind";
const QUOTA_KEY: &str = "notify.quota";
const CRASH_KEY: &str = "notify.crash";
const RESET_KEY: &str = "notify.reset";
const WATCH_KEY: &str = "notify.watch";
const DIGEST_KEY: &str = "notify.digest";

/// Settings › Notifications › Tell me outside AI Juggler.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(clippy::struct_excessive_bools)] // settings: each switch is its own choice
pub struct SystemNotificationsInfo {
    /// A session waits for the user's reply.
    pub waiting: bool,
    /// A session looks stuck.
    pub stuck: bool,
    /// A session hit its usage limit.
    pub limit: bool,
    /// A session finished on its own.
    pub finished: bool,
    /// This Mac is getting tight or under pressure.
    pub this_mac: bool,
    /// AJ-E2405: a session ended and left something running.
    #[serde(default)]
    pub left_behind: bool,
    /// AJ-E2501: an account is almost out of room.
    #[serde(default)]
    pub quota: bool,
    /// AJ-E2501: a session crashed and was started again.
    #[serde(default)]
    pub crash: bool,
    /// AJ-E2501: an account has room again.
    #[serde(default)]
    pub reset: bool,
    /// AJ-E3817: a watch the user set fired ("Watches you set"; on unless
    /// turned off).
    #[serde(default = "on")]
    pub watch: bool,
    /// AJ-E3712: the agent digest, with the finishes it carries instead of
    /// one notification each (off by default, like every other kind).
    #[serde(default)]
    pub digest: bool,
}

const fn on() -> bool {
    true
}

/// As stored before the user chooses: every kind off but the watches.
impl Default for SystemNotificationsInfo {
    fn default() -> Self {
        Self {
            waiting: false,
            stuck: false,
            limit: false,
            finished: false,
            this_mac: false,
            left_behind: false,
            quota: false,
            crash: false,
            reset: false,
            watch: true,
            digest: false,
        }
    }
}

impl SystemNotificationsInfo {
    /// Whether this kind may interrupt.
    #[must_use]
    pub const fn allows(self, kind: Kind) -> bool {
        match kind {
            Kind::Waiting => self.waiting,
            Kind::Stuck => self.stuck,
            Kind::Limit => self.limit,
            Kind::Finished => self.finished,
            Kind::MacTight => self.this_mac,
            Kind::LeftBehind => self.left_behind,
            Kind::Quota => self.quota,
            Kind::Crashed => self.crash,
            Kind::Reset => self.reset,
            Kind::Watch => self.watch,
            Kind::Digest => self.digest,
        }
    }
}

const KEYS: [&str; 11] = [
    WAITING_KEY,
    STUCK_KEY,
    LIMIT_KEY,
    FINISHED_KEY,
    THIS_MAC_KEY,
    LEFT_BEHIND_KEY,
    QUOTA_KEY,
    CRASH_KEY,
    RESET_KEY,
    WATCH_KEY,
    DIGEST_KEY,
];

/// The kinds on until the user turns them off.
const ON_BY_DEFAULT: &[&str] = &[WATCH_KEY];

fn stored(storage: &Storage, key: &str) -> bool {
    match storage.app_settings().get(key).ok().flatten().as_deref() {
        Some(value) => value == "true",
        None => ON_BY_DEFAULT.contains(&key),
    }
}

/// The kinds as stored (all off when unset).
#[must_use]
pub fn read(storage: &Storage) -> SystemNotificationsInfo {
    let [
        waiting,
        stuck,
        limit,
        finished,
        this_mac,
        left_behind,
        quota,
        crash,
        reset,
        watch,
        digest,
    ] = KEYS.map(|key| stored(storage, key));
    SystemNotificationsInfo {
        waiting,
        stuck,
        limit,
        finished,
        this_mac,
        left_behind,
        quota,
        crash,
        reset,
        watch,
        digest,
    }
}

/// Core: saves the kinds; returns them as stored.
///
/// # Errors
///
/// Returns the typed storage error.
pub fn notifications_set_impl(
    state: &Arc<AppState>,
    kinds: SystemNotificationsInfo,
) -> AppResult<SystemNotificationsInfo> {
    let storage = state
        .storage
        .lock()
        .map_err(|_| crate::error::AppError::StatePoisoned)?;
    let values = [
        kinds.waiting,
        kinds.stuck,
        kinds.limit,
        kinds.finished,
        kinds.this_mac,
        kinds.left_behind,
        kinds.quota,
        kinds.crash,
        kinds.reset,
        kinds.watch,
        kinds.digest,
    ];
    for (key, value) in KEYS.into_iter().zip(values) {
        storage.app_settings().set(key, &value.to_string())?;
    }
    Ok(read(&storage))
}

/// Which kinds may become a system notification.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub async fn notifications_get(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<SystemNotificationsInfo> {
    crate::ipc::blocking(state.inner(), "notification settings", |state| {
        let storage = state
            .storage
            .lock()
            .map_err(|_| crate::error::AppError::StatePoisoned)?;
        Ok(read(&storage))
    })
    .await
}

/// Saves which kinds may become a system notification.
///
/// # Errors
///
/// Returns the typed storage error.
#[tauri::command]
pub async fn notifications_set(
    state: tauri::State<'_, Arc<AppState>>,
    kinds: SystemNotificationsInfo,
) -> AppResult<SystemNotificationsInfo> {
    crate::ipc::blocking(state.inner(), "notification settings", move |state| {
        notifications_set_impl(state, kinds)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watches_are_on_and_everything_else_off_until_the_user_says() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::bare_state(dir.path().to_path_buf());
        let kinds = read(&state.storage.lock().unwrap());
        assert_eq!(kinds, SystemNotificationsInfo::default());
        assert!(kinds.watch, "the user asked for each watch");
        assert!(!kinds.digest && !kinds.finished && !kinds.waiting);
        let off = SystemNotificationsInfo {
            watch: false,
            digest: true,
            ..kinds
        };
        let saved = notifications_set_impl(&state, off).unwrap();
        assert!(!saved.watch && saved.digest, "turned off stays off");
        assert!(!saved.allows(Kind::Watch) && saved.allows(Kind::Digest));
    }
}
