//! AJ-E1206: your own linked Macs find each other through your Figurehead
//! account, without a pairing code.
//!
//! - **A directory, not trust.** With Fleet discovery on, Fleet reads
//!   `GET /api/v1/devices` (at most once a minute). Each Mac there comes
//!   with the public key it registered. That key is a candidate: a
//!   compromised account could offer an attacker's key.
//! - **Two-sided confirmation is the trust step.** This Mac pins a listed
//!   Mac only when the user confirms its fingerprint here; the other Mac
//!   pins this one only when its user does the same there. Until both
//!   have, mutual TLS refuses the connection, because each side only
//!   accepts a key it pinned.
//! - **The handshake still proves possession.** A pin is made under a
//!   provisional id (`acct-<device>`); the peer's real `MachineId` is
//!   bound on its first Hello, which only the holder of the pinned key
//!   can send.
//! - **Machine to machine.** Traffic stays on the LAN or the user's VPN
//!   over mutual TLS; Figurehead relays nothing. Such peers start at View
//!   and are labelled "via your Figurehead account".
//! - **Off means off.** With Fleet discovery off, no `/devices` call.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use juggler_account::DeviceDirectory;
use juggler_link::{PairedVia, Peer, Permission, PublicIdentity};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

const FEATURE: &str = "fleetDiscovery";
/// The directory is read at most this often.
pub const REFRESH_MS: i64 = 60_000;

fn refuse(message: impl Into<String>) -> AppError {
    AppError::Validation {
        subject: "Fleet discovery",
        message: message.into(),
    }
}

/// One of your Macs in the account directory.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountMacInfo {
    /// The service's device id.
    pub device_id: String,
    /// Its name.
    pub name: String,
    /// Its platform, when known.
    pub platform: Option<String>,
    /// The fingerprint of the key it registered (to compare with what
    /// that Mac shows), or `null` when it registered none.
    pub fingerprint: Option<String>,
    /// `paired` (both confirmed and connected once) | `waiting` (this Mac
    /// confirmed; the other hasn't yet, or hasn't connected) |
    /// `not_paired` | `no_key` (it can't be paired this way).
    pub state: String,
    /// When it was last seen, as the service wrote it.
    pub last_seen_at: Option<String>,
}

/// The directory, with this Mac's own fingerprint for the comparison.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountMacsInfo {
    /// This Mac's fingerprint (the other Mac shows it to compare).
    pub this_fingerprint: String,
    /// Your other linked Macs.
    pub macs: Vec<AccountMacInfo>,
    /// When the directory was read, epoch ms.
    #[ts(type = "number")]
    pub read_at_ms: i64,
}

/// The Ed25519 key in a directory entry (`ed25519:<b64>;x25519:<b64>`,
/// or a bare base64 key).
fn ed25519_of(public_key: &str) -> Option<PublicIdentity> {
    let b64 = public_key
        .split(';')
        .find_map(|part| part.trim().strip_prefix("ed25519:"))
        .unwrap_or(public_key)
        .trim();
    PublicIdentity::from_base64(b64).ok()
}

fn directory(state: &Arc<AppState>) -> AppResult<(DeviceDirectory, i64)> {
    if !crate::figurehead::feature_on(state, FEATURE) {
        return Err(refuse("Fleet discovery is off"));
    }
    let now = juggler_core::model::Timestamp::now().millis();
    if let Ok(cache) = state.account_directory.lock()
        && let Some((at, cached)) = cache.as_ref()
        && now - *at < REFRESH_MS
    {
        return Ok((cached.clone(), *at));
    }
    let (client, token) = crate::figurehead::session(state)?;
    let fresh = client
        .list_devices(&token)
        .map_err(|e| crate::figurehead::handle_error(state, e))?;
    if let Ok(mut cache) = state.account_directory.lock() {
        *cache = Some((now, fresh.clone()));
    }
    Ok((fresh, now))
}

/// Core: your other linked Macs, with how far pairing has got.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Fleet discovery is off (no call
/// is made) or the service refuses.
pub fn fleet_account_macs_impl(state: &Arc<AppState>) -> AppResult<AccountMacsInfo> {
    let (found, read_at_ms) = directory(state)?;
    let peers = crate::fleet::fleet_peers_impl(state)?;
    let macs = found
        .devices
        .into_iter()
        .filter(|device| device.id != found.self_id)
        .map(|device| {
            let key = device.public_key.as_deref().and_then(ed25519_of);
            let fingerprint = key.as_ref().map(PublicIdentity::fingerprint);
            let pinned = fingerprint
                .as_ref()
                .and_then(|fp| peers.iter().find(|peer| &peer.fingerprint == fp));
            let state = match (&key, pinned) {
                (None, _) => "no_key",
                (Some(_), None) => "not_paired",
                (Some(_), Some(peer))
                    if peer
                        .machine_id
                        .starts_with(juggler_link::PROVISIONAL_PREFIX) =>
                {
                    "waiting"
                }
                (Some(_), Some(_)) => "paired",
            };
            AccountMacInfo {
                device_id: device.id,
                name: device.name,
                platform: device.platform,
                fingerprint,
                state: state.to_owned(),
                last_seen_at: device.last_seen_at,
            }
        })
        .collect();
    Ok(AccountMacsInfo {
        this_fingerprint: crate::fleet::this_machine_fingerprint(state)?,
        macs,
        read_at_ms,
    })
}

/// Core: the user confirmed `fingerprint` for `device_id` on this Mac —
/// pin that Mac's key (View, "via your Figurehead account") under a
/// provisional id. It connects once the other Mac confirms this one too.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when discovery is off, the Mac isn't
/// in the directory, it registered no key, or the confirmed fingerprint
/// doesn't match its key now (the directory changed — compare again).
pub fn fleet_account_pin_impl(
    state: &Arc<AppState>,
    device_id: &str,
    fingerprint: &str,
) -> AppResult<()> {
    let (found, _) = directory(state)?;
    let device = found
        .devices
        .into_iter()
        .find(|device| device.id == device_id && device.id != found.self_id)
        .ok_or_else(|| refuse("that Mac isn't linked to your account any more"))?;
    let key = device
        .public_key
        .as_deref()
        .and_then(ed25519_of)
        .ok_or_else(|| refuse("that Mac hasn't shared a key — pair it with a code instead"))?;
    if key.fingerprint() != fingerprint {
        return Err(refuse(
            "the fingerprint changed since you compared it — compare it again on both Macs",
        ));
    }
    let now = juggler_core::model::Timestamp::now().millis();
    crate::fleet::pin_account_peer(
        state,
        Peer {
            machine_id: format!("{}{}", juggler_link::PROVISIONAL_PREFIX, device.id),
            label: if device.name.trim().is_empty() {
                "Your Mac".to_owned()
            } else {
                device.name.trim().to_owned()
            },
            public_key: key.to_base64(),
            fingerprint: key.fingerprint(),
            permission: Permission::View,
            ask_each_time: false,
            paired_via: PairedVia::Account,
            paired_at_ms: now,
            last_seen_ms: None,
        },
    )
}

/// AJ-E1206: your other linked Macs.
///
/// # Errors
///
/// Per [`fleet_account_macs_impl`].
#[tauri::command]
pub async fn fleet_account_macs(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<AccountMacsInfo> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || fleet_account_macs_impl(&state))
        .await
        .map_err(|e| AppError::Io {
            context: "Fleet discovery",
            message: e.to_string(),
        })?
}

/// AJ-E1206: pins a listed Mac after the user confirmed its fingerprint.
///
/// # Errors
///
/// Per [`fleet_account_pin_impl`].
#[tauri::command]
pub async fn fleet_account_pin(
    state: tauri::State<'_, Arc<AppState>>,
    device_id: String,
    fingerprint: String,
) -> AppResult<()> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        fleet_account_pin_impl(&state, &device_id, &fingerprint)
    })
    .await
    .map_err(|e| AppError::Io {
        context: "Fleet discovery",
        message: e.to_string(),
    })?
}
