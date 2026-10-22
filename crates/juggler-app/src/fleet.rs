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

use std::io::Write as _;
use std::sync::{Arc, Mutex};

use juggler_link::cert::NodeCertificate;
use juggler_link::node::{LinkAuditEvent, PairCandidate, PeerAccess};
use juggler_link::{
    MachineIdentity, PairedVia, Peer, PeerRegistry, Permission, PublicIdentity, TrustState,
};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The app setting mirroring the registry generation (rollback check).
const GENERATION_KEY: &str = "fleet_registry_generation";
/// The app setting holding this Mac's local display label.
const LABEL_KEY: &str = "machine_label";
/// The link attempt log (hash-chained JSON lines) in the fleet dir.
const AUDIT_FILE: &str = "link-audit.jsonl";
/// Rotate the attempt log past this size (one `.1` generation kept).
const AUDIT_MAX_BYTES: u64 = 2 * 1024 * 1024;
/// Serialises appends (the chain needs the previous line's hash).
static AUDIT_LOCK: Mutex<()> = Mutex::new(());

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
    /// The link state (AJ-E703): `link_off` | `connecting` | `connected`
    /// | `degraded` | `offline`. `link_off` until Link is turned on
    /// (AJ-E704) — never a guessed state.
    pub connection: String,
    /// The negotiated protocol (`v1.0`) when connected or degraded.
    pub protocol: Option<String>,
    /// Plain words for degraded/offline (the banner text).
    pub connection_detail: Option<String>,
    /// Durable changes on THIS Mac waiting for that machine (AJ-E708) —
    /// what it receives when it reconnects.
    #[ts(type = "number | null")]
    pub queued_changes: Option<u64>,
    /// This Mac finished pairing with it, but it may not have (it never
    /// said Hello here since): "Pair again to finish it."
    pub pairing_unfinished: bool,
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

/// AJ-E1009: this Mac's Fleet fingerprint (a backup records it to
/// recognise where it came from; it never carries the key).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn this_machine_fingerprint(state: &Arc<AppState>) -> AppResult<String> {
    with_fleet(state, |fleet| Ok(fleet.identity.public().fingerprint()))
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
pub(crate) fn self_label(state: &Arc<AppState>) -> AppResult<String> {
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
    // Read the registry first, then the link state: the fleet lock is
    // never held together with the link manager's locks.
    let mut peers = with_fleet(state, |fleet| {
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
                connection: "link_off".to_owned(),
                protocol: None,
                connection_detail: None,
                queued_changes: None,
                pairing_unfinished: false,
            })
            .collect::<Vec<_>>())
    })?;
    let now = crate::commands::now_millis();
    for peer in &mut peers {
        peer.pairing_unfinished = pairing_unfinished(state, &peer.machine_id, now);
        let link = state.link.peer_state(&peer.machine_id);
        peer.connection = link.state;
        peer.protocol = link.protocol;
        peer.connection_detail = link.detail;
        peer.queued_changes = crate::link::queued_for(state, &peer.machine_id);
    }
    Ok(peers)
}

/// Core implementation shared with tests.
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown peer, broken trust, or
/// a storage failure.
pub fn fleet_unpair_impl(state: &Arc<AppState>, machine_id: &str) -> AppResult<()> {
    pairing_finished(state, machine_id);
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
    })?;
    // AJ-E708: what was replicated from it goes too (after the fleet
    // lock is released — storage is never taken under it).
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.fleet_sync().replica_forget(machine_id)?;
    Ok(())
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

/// A pinned peer's local label.
#[must_use]
pub fn peer_label(state: &Arc<AppState>, machine_id: &str) -> Option<String> {
    with_fleet(state, |fleet| {
        Ok(fleet
            .registry
            .peers()
            .iter()
            .find(|peer| peer.machine_id == machine_id)
            .map(|peer| peer.label.clone()))
    })
    .ok()
    .flatten()
}

/// Core: changes what `machine_id` may do on THIS Mac. Only ever called
/// from this Mac's own UI (the confirmation is the user's click here);
/// no link call can change a permission (ADR-0013).
///
/// # Errors
///
/// Returns the typed [`AppError`] for an unknown peer or broken trust.
pub fn fleet_set_permission_impl(
    state: &Arc<AppState>,
    machine_id: &str,
    permission: &str,
    ask_each_time: bool,
) -> AppResult<Vec<FleetPeerInfo>> {
    let permission = match permission {
        "view" => Permission::View,
        "control" => Permission::Control,
        "no_access" => Permission::NoAccess,
        _ => {
            return Err(AppError::Validation {
                subject: "permission",
                message: "Choose View only or View + control.".to_owned(),
            });
        }
    };
    with_fleet(state, |fleet| {
        fleet
            .registry
            .set_permission(
                &fleet.identity,
                machine_id,
                permission,
                ask_each_time && permission == Permission::Control,
            )
            .map_err(link_error)
    })?;
    fleet_peers_impl(state)
}

/// AJ-E709: changes what a paired machine may do on this Mac.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_set_permission_impl`].
#[tauri::command]
pub fn fleet_set_permission(
    state: tauri::State<'_, Arc<AppState>>,
    machine_id: String,
    permission: String,
    ask_each_time: bool,
) -> AppResult<Vec<FleetPeerInfo>> {
    fleet_set_permission_impl(state.inner(), &machine_id, &permission, ask_each_time)
}

/// The peer pinned with `key`, as the link node sees it (per request,
/// so an unpair revokes on the next call).
#[must_use]
pub fn lookup_peer(state: &Arc<AppState>, key: &PublicIdentity) -> Option<PeerAccess> {
    with_fleet(state, |fleet| {
        Ok(fleet.registry.pinned(key).map(|peer| PeerAccess {
            machine_id: peer.machine_id.clone(),
            permission: peer.permission,
        }))
    })
    .ok()
    .flatten()
}

/// Whether control commands from `machine_id` also need a local
/// confirmation (the "Ask each time" choice).
#[must_use]
pub fn peer_asks_each_time(state: &Arc<AppState>, machine_id: &str) -> bool {
    with_fleet(state, |fleet| {
        Ok(fleet
            .registry
            .peers()
            .iter()
            .any(|peer| peer.machine_id == machine_id && peer.ask_each_time))
    })
    .unwrap_or(true)
}

/// Every pinned peer's id and key (the link reconciles against these).
#[must_use]
pub fn pinned_keys(state: &Arc<AppState>) -> Vec<(String, [u8; 32])> {
    with_fleet(state, |fleet| {
        if *fleet.registry.trust() != TrustState::Trusted {
            return Ok(Vec::new());
        }
        Ok(fleet
            .registry
            .peers()
            .iter()
            .filter_map(|peer| {
                PublicIdentity::from_base64(&peer.public_key)
                    .ok()
                    .map(|key| (peer.machine_id.clone(), key.to_bytes()))
            })
            .collect())
    })
    .unwrap_or_default()
}

/// This Mac's TLS certificate (its identity key as the SPKI).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn node_certificate(state: &Arc<AppState>) -> AppResult<NodeCertificate> {
    with_fleet(state, |fleet| {
        juggler_link::cert::node_certificate(&fleet.identity).map_err(link_error)
    })
}

/// This Mac's identity public key.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn local_public_key(state: &Arc<AppState>) -> AppResult<[u8; 32]> {
    with_fleet(state, |fleet| Ok(fleet.identity.public().to_bytes()))
}

/// Pins a machine both users confirmed in the pairing exchange.
///
/// # Errors
///
/// Returns the typed [`AppError`] on broken trust or storage failure.
pub fn pin_candidate(
    state: &Arc<AppState>,
    candidate: &PairCandidate,
    permission: Permission,
) -> AppResult<()> {
    let key = PublicIdentity::from_bytes(&candidate.key).map_err(link_error)?;
    let now = juggler_core::model::Timestamp::now().millis();
    with_fleet(state, |fleet| {
        fleet
            .registry
            .pin(
                &fleet.identity,
                Peer {
                    machine_id: candidate.machine_id.clone(),
                    label: candidate.label.clone(),
                    public_key: key.to_base64(),
                    fingerprint: key.fingerprint(),
                    permission,
                    ask_each_time: false,
                    paired_via: PairedVia::Code,
                    paired_at_ms: now,
                    last_seen_ms: Some(now),
                },
            )
            .map_err(link_error)
    })
}

/// AJ-E1206: pins a Mac from the account directory after the user
/// confirmed its fingerprint (provisional id until its first Hello).
///
/// # Errors
///
/// Returns the typed [`AppError`] on broken trust or storage failure.
pub fn pin_account_peer(state: &Arc<AppState>, peer: Peer) -> AppResult<()> {
    let machine_id = peer.machine_id.clone();
    with_fleet(state, |fleet| {
        fleet
            .registry
            .pin(&fleet.identity, peer)
            .map_err(link_error)
    })?;
    crate::link::reconnect_soon(state, &machine_id);
    Ok(())
}

/// AJ-E1206: binds an account-discovered peer's real `MachineId` (its
/// first TLS-authenticated Hello). `None` when nothing may be bound.
#[must_use]
pub fn bind_account_peer(
    state: &Arc<AppState>,
    key: &PublicIdentity,
    machine_id: &str,
) -> Option<PeerAccess> {
    with_fleet(state, |fleet| {
        fleet
            .registry
            .bind_account_peer(&fleet.identity, key, machine_id)
            .map_err(link_error)
    })
    .ok()
    .flatten()
    .map(|peer| PeerAccess {
        machine_id: peer.machine_id,
        permission: peer.permission,
    })
}

/// A pairing this Mac finished as host that the other machine may not have
/// (it was stopped between its "yes" and the end): the time of the pin,
/// kept until that machine's first Hello here. Display-only: it grants
/// nothing, and it lives outside the signed peer list (which older builds
/// would refuse with an extra field).
const UNFINISHED_PREFIX: &str = "fleet_pairing_unfinished:";

/// How long a pairing may wait for the other machine's first Hello before
/// its row says it didn't finish (a normal pairing's comes in seconds).
/// Fixed: only a test changes it (`link::pairing_wait_for_test`).
const UNFINISHED_AFTER_MS: i64 = 30_000;

/// Notes a pairing this Mac finished as host at `at_ms`.
pub(crate) fn mark_pairing_unfinished(state: &AppState, machine_id: &str, at_ms: i64) {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(
            &format!("{UNFINISHED_PREFIX}{machine_id}"),
            &at_ms.to_string(),
        );
    }
}

/// The other machine said Hello, or was unpaired: nothing is unfinished.
/// Writes only when something was noted (a Hello comes every connection).
pub(crate) fn pairing_finished(state: &AppState, machine_id: &str) {
    let key = format!("{UNFINISHED_PREFIX}{machine_id}");
    if let Ok(storage) = state.storage.lock()
        && storage.app_settings().get(&key).ok().flatten().is_some()
    {
        let _ = storage.app_settings().set(&key, "");
    }
}

/// Whether the row says the pairing didn't finish: noted, and no Hello for
/// longer than a normal pairing takes.
fn pairing_unfinished(state: &AppState, machine_id: &str, now: i64) -> bool {
    let after = state
        .link
        .unfinished_wait_ms
        .lock()
        .ok()
        .and_then(|wait| *wait)
        .unwrap_or(UNFINISHED_AFTER_MS);
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    storage
        .app_settings()
        .get(&format!("{UNFINISHED_PREFIX}{machine_id}"))
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<i64>().ok())
        .is_some_and(|since| now - since >= after)
}

/// Records that a pinned peer answered.
pub fn touch_peer(state: &Arc<AppState>, machine_id: &str, at_ms: i64) {
    let _ = with_fleet(state, |fleet| {
        fleet
            .registry
            .touch(&fleet.identity, machine_id, at_ms)
            .map_err(link_error)
    });
}

/// Appends one event to the link attempt log. Each line carries the
/// hash of the previous one, so a deleted or edited line breaks the
/// chain. Never holds a code, key, or command payload — only who, what
/// kind, and the reason code.
pub fn append_audit(state: &Arc<AppState>, event: &LinkAuditEvent) {
    let Ok(_serial) = AUDIT_LOCK.lock() else {
        return;
    };
    let dir = state.data_dir.join("fleet");
    let path = dir.join(AUDIT_FILE);
    let previous = last_audit_hash(&path);
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > AUDIT_MAX_BYTES) {
        let _ = std::fs::rename(&path, dir.join(format!("{AUDIT_FILE}.1")));
    }
    let body = serde_json::json!({
        "at_ms": juggler_core::model::Timestamp::now().millis(),
        "peer": event.peer,
        "kind": event.kind,
        "reason": event.reason_code,
        "sequence": event.sequence,
        "count": event.count,
        "prev": previous,
    });
    let body = body.to_string();
    let hash = audit_hash(&previous, &body);
    let line = format!("{{\"entry\":{body},\"hash\":\"{hash}\"}}\n");
    let _ = std::fs::create_dir_all(&dir);
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    if let Ok(mut file) = options.open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn audit_hash(previous: &str, body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(previous.as_bytes());
    hasher.update(body.as_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The last line's hash (the rotated file's, after a rotation), or the
/// all-zero genesis.
fn last_audit_hash(path: &std::path::Path) -> String {
    let rotated = path.with_extension("jsonl.1");
    for candidate in [path, rotated.as_path()] {
        if let Ok(text) = std::fs::read_to_string(candidate)
            && let Some(line) = text.lines().rev().find(|line| !line.trim().is_empty())
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(hash) = value.get("hash").and_then(serde_json::Value::as_str)
        {
            return hash.to_owned();
        }
    }
    "0".repeat(64)
}

/// Checks the attempt log's chain; returns the entries (oldest first)
/// or the line number where the chain breaks.
///
/// # Errors
///
/// Returns the 1-based line number of the first broken link.
pub fn verify_audit(state: &Arc<AppState>) -> Result<Vec<serde_json::Value>, usize> {
    let path = state.data_dir.join("fleet").join(AUDIT_FILE);
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut previous: Option<String> = None;
    let mut entries = Vec::new();
    for (index, line) in text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|_| index + 1)?;
        let entry = value.get("entry").ok_or(index + 1)?;
        let prev = entry
            .get("prev")
            .and_then(serde_json::Value::as_str)
            .ok_or(index + 1)?;
        if previous.as_deref().is_some_and(|expected| expected != prev) {
            return Err(index + 1);
        }
        let hash = value
            .get("hash")
            .and_then(serde_json::Value::as_str)
            .ok_or(index + 1)?;
        if audit_hash(prev, &entry.to_string()) != hash {
            return Err(index + 1);
        }
        previous = Some(hash.to_owned());
        entries.push(entry.clone());
    }
    Ok(entries)
}

/// The account-link `publicKey` (`ed25519:<b64>;x25519:<b64>`; ADR-0014).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity cannot be loaded.
pub fn link_public_key(state: &Arc<AppState>) -> AppResult<String> {
    with_fleet(state, |fleet| Ok(fleet.identity.link_public_key()))
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
