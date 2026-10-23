//! AJ-E1510 Req.1: which kinds may become a system notification. Each is
//! off until the user turns it on.

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

/// Settings › Notifications › Tell me outside AI Juggler.
#[derive(
    Clone, Copy, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, ts_rs::TS,
)]
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
        }
    }
}

const KEYS: [&str; 5] = [
    WAITING_KEY,
    STUCK_KEY,
    LIMIT_KEY,
    FINISHED_KEY,
    THIS_MAC_KEY,
];

fn on(storage: &Storage, key: &str) -> bool {
    storage.app_settings().get(key).ok().flatten().as_deref() == Some("true")
}

/// The kinds as stored (all off when unset).
#[must_use]
pub fn read(storage: &Storage) -> SystemNotificationsInfo {
    let [waiting, stuck, limit, finished, this_mac] = KEYS.map(|key| on(storage, key));
    SystemNotificationsInfo {
        waiting,
        stuck,
        limit,
        finished,
        this_mac,
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
