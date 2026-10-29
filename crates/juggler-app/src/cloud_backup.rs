//! AJ-E1205: Cloud backup — upload on demand or on a schedule, and
//! restore on any linked Mac through AJ-E1009's verified restore.
//!
//! - **Nothing is sent unless Cloud backup is on.** Every call here first
//!   checks the user's toggle and the plan (`feature_on`); off means no
//!   `/backups` request at all.
//! - **Nothing is sent until the recovery code is saved** (AJ-E1204).
//! - An upload builds an AJ-E1009 backup, seals it (ADR-0017) and sends the
//!   ciphertext. It runs one at a time, never during a switch, and never
//!   over the plan's size limit. `413` and `402` stop it with a reason.
//!   Local data is never deleted.
//! - A restore lists the account's backups, downloads one, checks its
//!   checksum, opens it with this Mac's key or the recovery code, and hands
//!   the rebuilt `.ajbackup` to AJ-E1009's verify step. The user then
//!   confirms in the same restore dialog as a local restore.
//! - The service keeps a bounded number of backups, so nothing here
//!   assumes an old one is still there.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use juggler_account::{BackupRecord, SecretStore, VaultKey};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

const FEATURE: &str = "cloudBackup";
const SCHEDULE_KEY: &str = "cloud_backup_schedule";
const LAST_KEY: &str = "cloud_backup_last";
const DAY_MS: i64 = 24 * 3_600_000;

fn refuse(message: impl Into<String>) -> AppError {
    AppError::Validation {
        subject: "Cloud backup",
        message: message.into(),
    }
}

/// The last upload's outcome (persisted).
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LastBackupInfo {
    /// When it finished (or failed), epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// It was stored.
    pub ok: bool,
    /// Its size, when stored.
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    /// Why it failed, when it did.
    pub error: Option<String>,
}

/// Where Cloud backup stands.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CloudBackupStatusInfo {
    /// The toggle is on and the plan includes it.
    pub enabled: bool,
    /// The recovery code is saved (uploads may run).
    pub code_saved: bool,
    /// `off` | `daily` | `weekly` (default).
    pub schedule: String,
    /// The last upload, if any.
    pub last: Option<LastBackupInfo>,
    /// An upload is running now.
    pub uploading: bool,
    /// The plan's size limit, bytes.
    #[ts(type = "number")]
    pub limit_bytes: u64,
}

/// One cloud backup in the list.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CloudBackupInfo {
    /// The service's id.
    pub id: String,
    /// Its label.
    pub label: Option<String>,
    /// Its size.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// When it was made (RFC 3339, as the service writes it).
    pub created_at: String,
    /// It came from this Mac.
    pub this_mac: bool,
}

fn setting(state: &Arc<AppState>, key: &str) -> Option<String> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
        .filter(|value| !value.is_empty())
}

fn last(state: &Arc<AppState>) -> Option<LastBackupInfo> {
    setting(state, LAST_KEY).and_then(|raw| serde_json::from_str(&raw).ok())
}

fn record_last(state: &Arc<AppState>, info: &LastBackupInfo) {
    if let (Ok(raw), Ok(storage)) = (serde_json::to_string(info), state.storage.lock()) {
        let _ = storage.app_settings().set(LAST_KEY, &raw);
    }
}

fn schedule(state: &Arc<AppState>) -> String {
    setting(state, SCHEDULE_KEY).unwrap_or_else(|| "weekly".to_owned())
}

/// Core: where Cloud backup stands.
#[must_use]
pub fn cloud_backup_status_impl(
    state: &Arc<AppState>,
    vault: &dyn SecretStore,
) -> CloudBackupStatusInfo {
    CloudBackupStatusInfo {
        enabled: crate::figurehead::feature_on(state, FEATURE),
        code_saved: crate::vault::vault_status_impl(state, vault).code_saved,
        schedule: schedule(state),
        last: last(state),
        uploading: state.cloud_uploading.try_lock().is_err(),
        limit_bytes: crate::figurehead::backup_limit(state),
    }
}

/// Core: sets the schedule (`off` | `daily` | `weekly`).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for another value.
pub fn cloud_backup_set_schedule_impl(
    state: &Arc<AppState>,
    vault: &dyn SecretStore,
    value: &str,
) -> AppResult<CloudBackupStatusInfo> {
    if !matches!(value, "off" | "daily" | "weekly") {
        return Err(refuse("choose off, daily or weekly"));
    }
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SCHEDULE_KEY, value)?;
    Ok(cloud_backup_status_impl(state, vault))
}

/// Why an upload can't start, if it can't (no request is made then).
fn upload_blocker(state: &Arc<AppState>, vault: &dyn SecretStore) -> AppResult<Option<String>> {
    if !crate::figurehead::feature_on(state, FEATURE) {
        return Ok(Some("Cloud backup is off".to_owned()));
    }
    if !crate::vault::vault_status_impl(state, vault).code_saved || VaultKey::load(vault).is_none()
    {
        return Ok(Some(
            "save your recovery code first — backups wait for it".to_owned(),
        ));
    }
    if crate::updates::switch_in_flight(state)? {
        return Ok(Some(
            "a switch is in progress — the backup waits until it finishes".to_owned(),
        ));
    }
    Ok(None)
}

/// Core: makes, seals and uploads one backup now.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Cloud backup is off, the code
/// isn't saved, a switch is in flight, another upload runs, or the
/// service refuses (too large, plan, network) — with the reason. Local
/// data is never deleted.
pub fn cloud_backup_now_impl(
    state: &Arc<AppState>,
    vault: &dyn SecretStore,
) -> AppResult<LastBackupInfo> {
    if let Some(reason) = upload_blocker(state, vault)? {
        return Err(refuse(reason));
    }
    let Ok(_one) = state.cloud_uploading.try_lock() else {
        return Err(refuse("a backup is already uploading"));
    };
    let key = VaultKey::load(vault).ok_or_else(|| refuse("save your recovery code first"))?;
    let work = state.data_dir.join(format!(
        "cloud-backup-work-{}",
        juggler_core::model::Timestamp::now().millis()
    ));
    std::fs::create_dir_all(&work).map_err(|e| AppError::Io {
        context: "cloud backup",
        message: e.to_string(),
    })?;
    let result = (|| {
        let archive = work.join("upload.ajbackup");
        crate::backup::backup_export_impl(state, &archive, &mut |_, _| {})?;
        let sealed = crate::vault::seal_backup(&archive, &key)?;
        let (client, token) = crate::figurehead::session(state)?;
        let label = format!(
            "AI Juggler · {}",
            sysinfo::System::host_name().unwrap_or_else(|| "this computer".to_owned())
        );
        client
            .upload_backup(
                &token,
                Some(&label),
                &sealed,
                crate::figurehead::backup_limit(state),
            )
            .map_err(|e| crate::figurehead::handle_error(state, e))
    })();
    let _ = std::fs::remove_dir_all(&work); // only our own scratch files
    let now = juggler_core::model::Timestamp::now().millis();
    let info = match result {
        Ok(stored) => LastBackupInfo {
            at_ms: now,
            ok: true,
            size_bytes: Some(stored.size_bytes),
            error: None,
        },
        Err(e) => {
            let failed = LastBackupInfo {
                at_ms: now,
                ok: false,
                size_bytes: None,
                error: Some(e.to_string()),
            };
            record_last(state, &failed);
            return Err(e);
        }
    };
    record_last(state, &info);
    Ok(info)
}

/// Core: the scheduled backup, when one is due (the background tick).
/// Returns whether an upload was attempted.
pub fn cloud_backup_tick(state: &Arc<AppState>, vault: &dyn SecretStore) -> bool {
    let every = match schedule(state).as_str() {
        "daily" => DAY_MS,
        "weekly" => 7 * DAY_MS,
        _ => return false,
    };
    if !matches!(upload_blocker(state, vault), Ok(None)) {
        return false;
    }
    let now = juggler_core::model::Timestamp::now().millis();
    // A failure is retried after an hour, not hammered.
    let due = last(state).is_none_or(|last| {
        if last.ok {
            now - last.at_ms >= every
        } else {
            now - last.at_ms >= 3_600_000
        }
    });
    if due {
        let _ = cloud_backup_now_impl(state, vault);
    }
    due
}

/// Core: the account's backups, newest first.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Cloud backup is off (no request
/// is made) or the service refuses.
pub fn cloud_backup_list_impl(state: &Arc<AppState>) -> AppResult<Vec<CloudBackupInfo>> {
    if !crate::figurehead::feature_on(state, FEATURE) {
        return Err(refuse("Cloud backup is off"));
    }
    let (client, token) = crate::figurehead::session(state)?;
    let this = crate::figurehead::this_device_id(state);
    let rows: Vec<BackupRecord> = client
        .list_backups(&token)
        .map_err(|e| crate::figurehead::handle_error(state, e))?;
    Ok(rows
        .into_iter()
        .map(|row| CloudBackupInfo {
            this_mac: row.device_id.is_some() && row.device_id == this,
            id: row.id,
            label: row.label,
            size_bytes: row.size_bytes,
            created_at: row.created_at,
        })
        .collect())
}

/// Core: downloads one backup, checks and opens it (this Mac's key, or
/// `recovery_code` on a new Mac), and returns AJ-E1009's verified restore
/// plan. Nothing is restored until the user confirms that plan.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Cloud backup is off (no request
/// is made), the checksum differs, the key or code is wrong, or the
/// backup fails AJ-E1009's verification — each with its reason.
pub fn cloud_backup_prepare_restore_impl(
    state: &Arc<AppState>,
    vault: &dyn SecretStore,
    id: &str,
    recovery_code: Option<&str>,
) -> AppResult<crate::backup::RestorePlanInfo> {
    if !crate::figurehead::feature_on(state, FEATURE) {
        return Err(refuse("Cloud backup is off"));
    }
    let key = match recovery_code.filter(|code| !code.trim().is_empty()) {
        Some(code) => VaultKey::from_recovery_code(code).map_err(|e| refuse(e.to_string()))?,
        None => VaultKey::load(vault).ok_or_else(|| {
            refuse("enter your recovery code — this computer doesn't have the vault key yet")
        })?,
    };
    let (client, token) = crate::figurehead::session(state)?;
    let container = client
        .download_backup(&token, id)
        .map_err(|e| crate::figurehead::handle_error(state, e))?;
    let dest = state.data_dir.join(format!(
        "from-cloud-{}.ajbackup",
        juggler_core::model::Timestamp::now().millis()
    ));
    crate::vault::open_backup(&container, &key, &dest)?;
    if recovery_code.is_some() {
        // The code opened it: keep the key on this Mac for next time.
        let _ =
            crate::vault::vault_enter_code_impl(state, vault, recovery_code.unwrap_or_default());
    }
    crate::backup::backup_verify_impl(state, &dest)
}

/// AJ-E1205: where Cloud backup stands.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn cloud_backup_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<CloudBackupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cloud backup status", move |state| {
        Ok(cloud_backup_status_impl(
            state,
            &crate::edition::current().vault_keychain(),
        ))
    })
    .await
}

/// AJ-E1205: sets the schedule.
///
/// # Errors
///
/// Per [`cloud_backup_set_schedule_impl`].
#[tauri::command]
pub async fn cloud_backup_set_schedule(
    state: tauri::State<'_, Arc<AppState>>,
    schedule: String,
) -> AppResult<CloudBackupStatusInfo> {
    crate::ipc::blocking(state.inner(), "cloud backup set schedule", move |state| {
        cloud_backup_set_schedule_impl(
            state,
            &crate::edition::current().vault_keychain(),
            &schedule,
        )
    })
    .await
}

async fn blocking<T: Send + 'static>(
    state: &tauri::State<'_, Arc<AppState>>,
    body: impl FnOnce(&Arc<AppState>) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || body(&state))
        .await
        .map_err(|e| AppError::Io {
            context: "cloud backup",
            message: e.to_string(),
        })?
}

/// AJ-E1205: backs up now (off the UI thread).
///
/// # Errors
///
/// Per [`cloud_backup_now_impl`].
#[tauri::command]
pub async fn cloud_backup_now(state: tauri::State<'_, Arc<AppState>>) -> AppResult<LastBackupInfo> {
    blocking(&state, |state| {
        cloud_backup_now_impl(state, &crate::edition::current().vault_keychain())
    })
    .await
}

/// AJ-E1205: lists the account's backups.
///
/// # Errors
///
/// Per [`cloud_backup_list_impl`].
#[tauri::command]
pub async fn cloud_backup_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<CloudBackupInfo>> {
    blocking(&state, cloud_backup_list_impl).await
}

/// AJ-E1205: downloads and opens one backup; returns the restore plan.
///
/// # Errors
///
/// Per [`cloud_backup_prepare_restore_impl`].
#[tauri::command]
pub async fn cloud_backup_prepare_restore(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    recovery_code: Option<String>,
) -> AppResult<crate::backup::RestorePlanInfo> {
    blocking(&state, move |state| {
        cloud_backup_prepare_restore_impl(
            state,
            &crate::edition::current().vault_keychain(),
            &id,
            recovery_code.as_deref(),
        )
    })
    .await
}

/// Starts the scheduled-backup tick (hourly; does nothing while Cloud
/// backup is off).
pub fn spawn_schedule(state: Arc<AppState>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3_600));
            let _ = cloud_backup_tick(&state, &crate::edition::current().vault_keychain());
        }
    });
}
