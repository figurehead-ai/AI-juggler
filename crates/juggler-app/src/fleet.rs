//! Fleet commands (AJ-E702; ADR-0013 architecture contract): this Mac's
//! machine identity and the signed paired-peer registry.
//!
//! The identity loads lazily on first use, so a damaged key file can
//! never stop the app from starting — Fleet reports it instead. Lock
//! ordering: the fleet lock is never held together with the storage
//! lock (the registry generation is read before, and recorded after,
//! the fleet lock is held).

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::{Arc, Mutex};

use juggler_link::{MachineIdentity, PeerRegistry, Permission, TrustState};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The app setting mirroring the registry generation (rollback check).
const GENERATION_KEY: &str = "fleet_registry_generation";
/// The app setting holding this Mac's local display label.
const LABEL_KEY: &str = "machine_label";

/// The loaded identity + registry.
#[derive(Debug)]
pub struct FleetState {
    identity: MachineIdentity,
    registry: PeerRegistry,
}

/// The lazily loaded fleet state.
pub type FleetCell = Mutex<Option<FleetState>>;

/// This Mac, as Fleet shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FleetSelfInfo {
    /// The stable `MachineId`.
    pub machine_id: String,
    /// The local display label.
    pub label: String,
    /// The identity fingerprint (8 groups of 4 hex digits).
    pub fingerprint: String,
    /// `trusted` | `broken`.
    pub trust: String,
    /// Why trust is broken (for the Details disclosure), when it is.
    pub trust_reason: Option<String>,
    /// Pinned peers.
    pub peer_count: u32,
}

/// One pinned peer, as Fleet lists it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FleetPeerInfo {
    /// The peer's `MachineId`.
    pub machine_id: String,
    /// The local label.
    pub label: String,
    /// The pinned key's fingerprint.
    pub fingerprint: String,
    /// `no_access` | `view` | `control`.
    pub permission: String,
    /// The plain-words permission ("View only").
    pub permission_label: String,
    /// Control commands also need a confirmation on this Mac.
    pub ask_each_time: bool,
    /// `code` | `account`.
    pub paired_via: String,
    /// Paired at, epoch ms.
    #[ts(type = "number")]
    pub paired_at_ms: i64,
    /// Last heard from, epoch ms.
    #[ts(type = "number | null")]
    pub last_seen_ms: Option<i64>,
}

fn link_error(error: impl std::fmt::Display) -> AppError {
    AppError::Io {
        context: "fleet",
        message: error.to_string(),
    }
}

fn recorded_generation(state: &Arc<AppState>) -> AppResult<Option<u64>> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    Ok(storage
        .app_settings()
        .get(GENERATION_KEY)?
        .and_then(|raw| raw.parse().ok()))
}

fn record_generation(state: &Arc<AppState>, generation: u64) -> AppResult<()> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage
        .app_settings()
        .set(GENERATION_KEY, &generation.to_string())?;
    Ok(())
}

/// Runs `body` with the fleet state loaded (loading it on first use),
/// then records the registry generation if it moved.
fn with_fleet<T>(
    state: &Arc<AppState>,
    body: impl FnOnce(&mut FleetState) -> AppResult<T>,
) -> AppResult<T> {
    let recorded = recorded_generation(state)?;
    let (result, generation) = {
        let mut cell = state.fleet.lock().map_err(|_| AppError::StatePoisoned)?;
        if cell.is_none() {
            let dir = state.data_dir.join("fleet");
            let identity = MachineIdentity::load_or_create(&dir).map_err(link_error)?;
            let registry = PeerRegistry::load(&dir, &identity, recorded);
            *cell = Some(FleetState { identity, registry });
        }
        let fleet = cell.as_mut().ok_or(AppError::StatePoisoned)?;
        let result = body(fleet)?;
        (result, fleet.registry.generation())
    };
    if recorded.unwrap_or(0) < generation {
        record_generation(state, generation)?;
    }
    Ok(result)
}

/// This Mac's local label: the stored one, else the computer's name,
/// else "This Mac".
fn self_label(state: &Arc<AppState>) -> AppResult<String> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    if let Some(label) = storage.app_settings().get(LABEL_KEY)?
        && !label.trim().is_empty()
    {
        return Ok(label);
    }
    Ok(sysinfo::System::host_name()
        .map(|name| name.trim_end_matches(".local").to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "This Mac".to_owned()))
}

fn self_info(state: &Arc<AppState>) -> AppResult<FleetSelfInfo> {
    let machine_id = crate::commands::local_machine_id(state).to_string();
    let label = self_label(state)?;
    with_fleet(state, |fleet| {
        let (trust, trust_reason) = match fleet.registry.trust() {
            TrustState::Trusted => ("trusted".to_owned(), None),
            TrustState::Broken { reason } => ("broken".to_owned(), Some(reason.clone())),
        };
        Ok(FleetSelfInfo {
            machine_id,
            label,
            fingerprint: fleet.identity.public().fingerprint(),
            trust,
            trust_reason,
            peer_count: u32::try_from(fleet.registry.peers().len()).unwrap_or(u32::MAX),
        })
    })
}

fn wire(permission: Permission) -> &'static str {
    match permission {
        Permission::NoAccess => "no_access",
        Permission::View => "view",
        Permission::Control => "control",
    }
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn fleet_self_impl(state: &Arc<AppState>) -> AppResult<FleetSelfInfo> {
    self_info(state)
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn fleet_peers_impl(state: &Arc<AppState>) -> AppResult<Vec<FleetPeerInfo>> {
    with_fleet(state, |fleet| {
        Ok(fleet
            .registry
            .peers()
            .iter()
            .map(|peer| FleetPeerInfo {
                machine_id: peer.machine_id.clone(),
                label: peer.label.clone(),
                fingerprint: peer.fingerprint.clone(),
                permission: wire(peer.permission).to_owned(),
                permission_label: peer.permission.label().to_owned(),
                ask_each_time: peer.ask_each_time,
                paired_via: match peer.paired_via {
                    juggler_link::PairedVia::Code => "code".to_owned(),
                    juggler_link::PairedVia::Account => "account".to_owned(),
                },
                paired_at_ms: peer.paired_at_ms,
                last_seen_ms: peer.last_seen_ms,
            })
            .collect())
    })
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown peer, broken trust, or
/// a storage failure.
pub fn fleet_unpair_impl(state: &Arc<AppState>, machine_id: &str) -> AppResult<()> {
    with_fleet(state, |fleet| {
        let removed = fleet
            .registry
            .unpair(&fleet.identity, machine_id)
            .map_err(link_error)?;
        if removed {
            Ok(())
        } else {
            Err(AppError::NotFound {
                entity: "paired machine",
                id: machine_id.to_owned(),
            })
        }
    })
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an empty or over-long label.
pub fn fleet_set_label_impl(state: &Arc<AppState>, label: &str) -> AppResult<FleetSelfInfo> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 64 {
        return Err(AppError::Validation {
            subject: "machine name",
            message: "use 1–64 characters".to_owned(),
        });
    }
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage.app_settings().set(LABEL_KEY, label)?;
    }
    self_info(state)
}

/// Core implementation shared with tests ("Reset trust" after a broken
/// registry — an explicit user click).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
pub fn fleet_reset_trust_impl(state: &Arc<AppState>) -> AppResult<FleetSelfInfo> {
    with_fleet(state, |fleet| {
        fleet
            .registry
            .reset_trust(&fleet.identity)
            .map_err(link_error)
    })?;
    self_info(state)
}

/// Core implementation shared with tests ("Reset this Mac's identity" —
/// a new key; every peer must pair again).
///
/// # Errors
///
/// Returns the typed [`AppError`] on storage failure.
pub fn fleet_reset_identity_impl(state: &Arc<AppState>) -> AppResult<FleetSelfInfo> {
    with_fleet(state, |fleet| {
        let dir = state.data_dir.join("fleet");
        // `regenerate` consumes the old identity; a reload of the same
        // (still-present) key stands in for the moment it takes.
        let old = std::mem::replace(
            &mut fleet.identity,
            MachineIdentity::load_or_create(&dir).map_err(link_error)?,
        );
        fleet.identity = old.regenerate().map_err(link_error)?;
        // Pins signed by the old key would no longer verify; start clean.
        fleet
            .registry
            .reset_trust(&fleet.identity)
            .map_err(link_error)
    })?;
    self_info(state)
}

/// AJ-E702: this Mac's identity (label, fingerprint, trust state).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
#[tauri::command]
pub fn fleet_self(state: tauri::State<'_, Arc<AppState>>) -> AppResult<FleetSelfInfo> {
    fleet_self_impl(state.inner())
}

/// AJ-E702: the pinned peers.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
#[tauri::command]
pub fn fleet_peers(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<FleetPeerInfo>> {
    fleet_peers_impl(state.inner())
}

/// AJ-E702: unpairs one machine (deletes its pin).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_unpair_impl`].
#[tauri::command]
pub fn fleet_unpair(state: tauri::State<'_, Arc<AppState>>, machine_id: String) -> AppResult<()> {
    fleet_unpair_impl(state.inner(), &machine_id)
}

/// AJ-E702: renames this Mac locally (id and key unchanged).
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_set_label_impl`].
#[tauri::command]
pub fn fleet_set_label(
    state: tauri::State<'_, Arc<AppState>>,
    label: String,
) -> AppResult<FleetSelfInfo> {
    fleet_set_label_impl(state.inner(), &label)
}

/// AJ-E702: clears a broken registry so pairing can start again.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_reset_trust_impl`].
#[tauri::command]
pub fn fleet_reset_trust(state: tauri::State<'_, Arc<AppState>>) -> AppResult<FleetSelfInfo> {
    fleet_reset_trust_impl(state.inner())
}

/// AJ-E702: generates a new identity for this Mac; every peer must pair
/// again.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_reset_identity_impl`].
#[tauri::command]
pub fn fleet_reset_identity(state: tauri::State<'_, Arc<AppState>>) -> AppResult<FleetSelfInfo> {
    fleet_reset_identity_impl(state.inner())
}

/// Test seam: pins a peer directly (pairing lands with AJ-E704).
///
/// # Errors
///
/// Returns the typed [`AppError`] on broken trust or storage failure.
#[doc(hidden)]
pub fn fleet_pin_for_test(state: &Arc<AppState>, peer: juggler_link::Peer) -> AppResult<()> {
    with_fleet(state, |fleet| {
        fleet
            .registry
            .pin(&fleet.identity, peer)
            .map_err(link_error)
    })
}

/// Test seam: forgets the in-memory fleet state (the next call reloads
/// from disk, as a restart would).
#[doc(hidden)]
pub fn fleet_unload_for_test(state: &Arc<AppState>) {
    if let Ok(mut cell) = state.fleet.lock() {
        *cell = None;
    }
}
