//! AJ-E1204: the vault on this Mac — making the key and showing its
//! recovery code once, and sealing or opening the Cloud backup container
//! (ADR-0017).
//!
//! The container's plaintext is a **zstd-compressed tar of the AJ-E1009
//! backup set**: the same entries, with the same bytes, as a local
//! `.ajbackup` (so the same allowlist and scrub apply, and `fleet/` is
//! never in it). Opening a container rebuilds the `.ajbackup`, which then
//! goes through AJ-E1009's verified restore unchanged.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use juggler_account::{SecretStore, VaultKey};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Set once the user ticks "I've saved my recovery code".
const SAVED_KEY: &str = "vault_code_saved";

/// Where the vault stands on this Mac.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct VaultStatusInfo {
    /// A vault key exists on this Mac.
    pub has_key: bool,
    /// The user confirmed saving the recovery code (uploads may start).
    pub code_saved: bool,
}

/// The recovery code, shown until the user confirms saving it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RecoveryCodeInfo {
    /// 14 groups of four characters.
    pub code: String,
}

fn vault_error(e: &juggler_account::VaultError) -> AppError {
    AppError::Validation {
        subject: "vault",
        message: e.to_string(),
    }
}

fn io_error(context: &'static str, error: &impl std::fmt::Display) -> AppError {
    AppError::Io {
        context,
        message: error.to_string(),
    }
}

fn code_saved(state: &Arc<AppState>) -> bool {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SAVED_KEY).ok().flatten())
        .is_some_and(|raw| raw == "1")
}

/// Core: where the vault stands.
#[must_use]
pub fn vault_status_impl(state: &Arc<AppState>, store: &dyn SecretStore) -> VaultStatusInfo {
    VaultStatusInfo {
        has_key: VaultKey::load(store).is_some(),
        code_saved: code_saved(state),
    }
}

/// Core: makes the vault key on first use and returns its recovery code
/// — only until the user confirms saving it; after that it is never shown
/// again.
///
/// # Errors
///
/// Returns [`AppError::Validation`] once the code has been confirmed, or
/// when the Keychain refuses.
pub fn vault_setup_impl(
    state: &Arc<AppState>,
    store: &dyn SecretStore,
) -> AppResult<RecoveryCodeInfo> {
    if code_saved(state) {
        return Err(AppError::Validation {
            subject: "vault",
            message: "your recovery code was shown once and you saved it — it isn't shown again"
                .to_owned(),
        });
    }
    let key = if let Some(key) = VaultKey::load(store) {
        key
    } else {
        let key = VaultKey::generate().map_err(|e| vault_error(&e))?;
        key.save(store).map_err(|e| vault_error(&e))?;
        key
    };
    Ok(RecoveryCodeInfo {
        code: key.recovery_code(),
    })
}

/// Core: the user confirmed "I've saved my recovery code".
///
/// # Errors
///
/// Returns [`AppError::Validation`] when no key exists yet.
pub fn vault_confirm_saved_impl(
    state: &Arc<AppState>,
    store: &dyn SecretStore,
) -> AppResult<VaultStatusInfo> {
    if VaultKey::load(store).is_none() {
        return Err(AppError::Validation {
            subject: "vault",
            message: "there is no vault key yet".to_owned(),
        });
    }
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SAVED_KEY, "1")?;
    Ok(vault_status_impl(state, store))
}

/// Core: a new Mac takes the vault key from its recovery code.
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming a malformed code or a typo.
pub fn vault_enter_code_impl(
    state: &Arc<AppState>,
    store: &dyn SecretStore,
    code: &str,
) -> AppResult<VaultStatusInfo> {
    let key = VaultKey::from_recovery_code(code).map_err(|e| vault_error(&e))?;
    key.save(store).map_err(|e| vault_error(&e))?;
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(SAVED_KEY, "1")?;
    Ok(vault_status_impl(state, store))
}

/// Seals a verified `.ajbackup` into the Cloud backup container: its
/// entries re-packed as a zstd tar, then encrypted.
///
/// # Errors
///
/// Returns the typed IO error for an unreadable archive.
pub fn seal_backup(archive: &Path, key: &VaultKey) -> AppResult<Vec<u8>> {
    let file = std::fs::File::open(archive).map_err(|e| io_error("cloud backup", &e))?;
    let mut source = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let encoder =
        zstd::stream::Encoder::new(Vec::new(), 9).map_err(|e| io_error("cloud backup", &e))?;
    let mut builder = tar::Builder::new(encoder);
    for entry in source.entries().map_err(|e| io_error("cloud backup", &e))? {
        let mut entry = entry.map_err(|e| io_error("cloud backup", &e))?;
        let name = entry
            .path()
            .map_err(|e| io_error("cloud backup", &e))?
            .to_string_lossy()
            .into_owned();
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|e| io_error("cloud backup", &e))?;
        append(&mut builder, &name, &body)?;
    }
    let packed = builder
        .into_inner()
        .and_then(zstd::stream::Encoder::finish)
        .map_err(|e| io_error("cloud backup", &e))?;
    juggler_account::vault::seal(key, &packed).map_err(|e| vault_error(&e))
}

fn append<W: Write>(builder: &mut tar::Builder<W>, name: &str, body: &[u8]) -> AppResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o600);
    header.set_mtime(0);
    header.set_cksum();
    builder
        .append_data(&mut header, name, body)
        .map_err(|e| io_error("cloud backup", &e))
}

/// Opens a Cloud backup container into a local `.ajbackup` at `dest`
/// (then verified and restored by AJ-E1009 as usual).
///
/// # Errors
///
/// Returns [`AppError::Validation`] naming a wrong key, a wrong format or
/// a damaged container; or the typed IO error.
pub fn open_backup(container: &[u8], key: &VaultKey, dest: &Path) -> AppResult<()> {
    let packed = juggler_account::vault::open(key, container).map_err(|e| vault_error(&e))?;
    let decoder =
        zstd::stream::Decoder::new(&packed[..]).map_err(|e| io_error("cloud backup", &e))?;
    let mut source = tar::Archive::new(decoder);
    let file = std::fs::File::create(dest).map_err(|e| io_error("cloud backup", &e))?;
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        file,
        flate2::Compression::fast(),
    ));
    for entry in source.entries().map_err(|e| io_error("cloud backup", &e))? {
        let mut entry = entry.map_err(|e| io_error("cloud backup", &e))?;
        let name = entry
            .path()
            .map_err(|e| io_error("cloud backup", &e))?
            .to_string_lossy()
            .into_owned();
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|e| io_error("cloud backup", &e))?;
        append(&mut builder, &name, &body)?;
    }
    builder
        .into_inner()
        .and_then(flate2::write::GzEncoder::finish)
        .map_err(|e| io_error("cloud backup", &e))?;
    Ok(())
}

/// Core: writes the recovery code to a file the user chose (their own
/// copy — "Save"), only while it is still being shown.
///
/// # Errors
///
/// Returns [`AppError::Validation`] once the code has been confirmed
/// saved or when no key exists; or the typed IO error.
pub fn vault_save_code_impl(
    state: &Arc<AppState>,
    store: &dyn SecretStore,
    dest: &Path,
) -> AppResult<()> {
    if code_saved(state) {
        return Err(AppError::Validation {
            subject: "vault",
            message: "your recovery code isn't shown again once saved".to_owned(),
        });
    }
    let key = VaultKey::load(store).ok_or_else(|| AppError::Validation {
        subject: "vault",
        message: "there is no vault key yet".to_owned(),
    })?;
    let text = format!(
        "AI Juggler recovery code\n\n{}\n\nWithout this code, a new Mac can't open your backups. We can't recover it for you.\nKeep it somewhere safe and private.\n",
        key.recovery_code()
    );
    std::fs::write(dest, text).map_err(|e| io_error("recovery code", &e))
}

/// AJ-E1204: where the vault stands.
#[tauri::command]
#[must_use]
pub fn vault_status(state: tauri::State<'_, Arc<AppState>>) -> VaultStatusInfo {
    vault_status_impl(state.inner(), &crate::edition::current().vault_keychain())
}

/// AJ-E1204: makes the vault key (first time) and returns the recovery
/// code until it is confirmed saved.
///
/// # Errors
///
/// Per [`vault_setup_impl`].
#[tauri::command]
pub fn vault_setup(state: tauri::State<'_, Arc<AppState>>) -> AppResult<RecoveryCodeInfo> {
    vault_setup_impl(state.inner(), &crate::edition::current().vault_keychain())
}

/// AJ-E1204: "I've saved my recovery code".
///
/// # Errors
///
/// Per [`vault_confirm_saved_impl`].
#[tauri::command]
pub fn vault_confirm_saved(state: tauri::State<'_, Arc<AppState>>) -> AppResult<VaultStatusInfo> {
    vault_confirm_saved_impl(state.inner(), &crate::edition::current().vault_keychain())
}

/// AJ-E1204: saves the recovery code to a file the user chose.
///
/// # Errors
///
/// Per [`vault_save_code_impl`].
#[tauri::command]
pub fn vault_save_code(state: tauri::State<'_, Arc<AppState>>, dest: String) -> AppResult<()> {
    vault_save_code_impl(
        state.inner(),
        &crate::edition::current().vault_keychain(),
        Path::new(&dest),
    )
}

/// AJ-E1204: enters a recovery code on a new Mac.
///
/// # Errors
///
/// Per [`vault_enter_code_impl`].
#[tauri::command]
pub fn vault_enter_code(
    state: tauri::State<'_, Arc<AppState>>,
    code: String,
) -> AppResult<VaultStatusInfo> {
    vault_enter_code_impl(
        state.inner(),
        &crate::edition::current().vault_keychain(),
        &code,
    )
}
