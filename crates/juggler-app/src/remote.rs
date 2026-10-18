//! The console side of Fleet (AJ-E705; ADR-0013 contract): what each
//! paired machine reports over the link, as this Mac last heard it.
//!
//! Every value carries its source machine and when it was measured
//! (received), and the machine's freshness is explicit — `live` (heard
//! in the last 5 s on a live connection), `stale`, or `offline` — never
//! hidden to look real-time. Fields a machine doesn't send stay `None`
//! and the UI says "Unavailable", never an estimate.
//!
//! Remote actions go through the link's command pipeline (nonce,
//! sequence, 30 s expiry, node-side permission check): a console can
//! only ever ask; the owning machine decides.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use juggler_link::client::LinkPeer;
use juggler_link::proto;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A machine's values are `live` while heard within this window.
const LIVE_WITHIN_MS: i64 = 5_000;
/// An Instance a machine stopped reporting is dropped after this long.
const FORGET_INSTANCE_AFTER_MS: i64 = 10_000;
/// The app setting holding this console's command sequence per peer.
const SEQUENCE_KEY: &str = "fleet_console_sequence";
/// How long a remote command may take end to end.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

fn now_ms() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

/// What one paired machine reported.
#[derive(Debug, Default)]
pub struct RemoteMachine {
    /// What that machine lets THIS Mac do ("view" | "control" | …), as
    /// its Hello said.
    permission: Option<String>,
    /// The live connection (for commands), while connected.
    peer: Option<LinkPeer>,
    /// Latest machine resources and when they arrived.
    resources: Option<(proto::MachineResources, i64)>,
    /// Latest row per Instance and when it arrived.
    instances: HashMap<String, (proto::SessionDelta, i64)>,
    /// When anything last arrived.
    last_heard_ms: Option<i64>,
    /// The machine's recent alerts (newest first, at most 10).
    alerts: Vec<proto::AlertDelta>,
}

/// The cache, shared by the reconcile loop and the commands.
pub type RemoteCache = Arc<Mutex<HashMap<String, RemoteMachine>>>;

/// Subscribes to `machine_id`'s telemetry on a fresh connection.
pub(crate) fn subscribe(
    cache: &RemoteCache,
    machine_id: &str,
    peer: &LinkPeer,
) -> tokio::task::JoinHandle<()> {
    if let Ok(mut map) = cache.lock() {
        let entry = map.entry(machine_id.to_owned()).or_default();
        entry.permission = Some(peer.hello.permission.clone());
        entry.peer = Some(peer.clone());
    }
    let cache = cache.clone();
    let id = machine_id.to_owned();
    let mut client = peer.client.clone();
    tokio::spawn(async move {
        let Ok(response) = client
            .telemetry(proto::TelemetryRequest { since_sequence: 0 })
            .await
        else {
            return;
        };
        let mut stream = response.into_inner();
        while let Ok(Some(delta)) = stream.message().await {
            // v1 never relays: a delta about another machine is dropped.
            if delta.origin_machine_id == id {
                apply(&cache, &id, delta);
            }
        }
    })
}

fn apply(cache: &RemoteCache, machine_id: &str, delta: proto::TelemetryDelta) {
    let now = now_ms();
    let Ok(mut map) = cache.lock() else {
        return;
    };
    let entry = map.entry(machine_id.to_owned()).or_default();
    entry.last_heard_ms = Some(now);
    match delta.payload {
        Some(proto::telemetry_delta::Payload::Resources(resources)) => {
            entry.resources = Some((resources, now));
        }
        Some(proto::telemetry_delta::Payload::Session(session)) => {
            entry
                .instances
                .insert(session.instance_id.clone(), (session, now));
        }
        Some(proto::telemetry_delta::Payload::Alert(alert)) => {
            let duplicate = entry.alerts.iter().any(|seen| {
                seen.at_ms == alert.at_ms
                    && seen.kind == alert.kind
                    && seen.account_label == alert.account_label
            });
            if !duplicate {
                entry.alerts.insert(0, alert);
                entry.alerts.truncate(10);
            }
        }
        None => {}
    }
    entry
        .instances
        .retain(|_, (_, at)| now - *at <= FORGET_INSTANCE_AFTER_MS);
}

/// Re-asks `machine_id` what it lets this Mac do (a permission changed
/// on that machine shows up without a reconnect).
pub(crate) async fn refresh_permission(cache: &RemoteCache, machine_id: &str, own: &str) {
    let peer = cache
        .lock()
        .ok()
        .and_then(|map| map.get(machine_id).and_then(|entry| entry.peer.clone()));
    let Some(peer) = peer else {
        return;
    };
    let mut client = peer.client.clone();
    let hello = proto::HelloRequest {
        version: Some(proto::ProtocolVersion {
            major: juggler_link::protocol::PROTOCOL_MAJOR,
            minor: juggler_link::protocol::PROTOCOL_MINOR,
        }),
        machine_id: own.to_owned(),
        sent_at_ms: now_ms(),
    };
    if let Ok(Ok(response)) =
        tokio::time::timeout(Duration::from_secs(2), client.hello(hello)).await
    {
        let permission = response.into_inner().permission;
        if let Ok(mut map) = cache.lock()
            && let Some(entry) = map.get_mut(machine_id)
        {
            entry.permission = Some(permission);
        }
    }
}

/// Forgets machines that are no longer pinned.
pub(crate) fn retain(cache: &RemoteCache, pinned: &[(String, [u8; 32])]) {
    if let Ok(mut map) = cache.lock() {
        map.retain(|id, _| pinned.iter().any(|(pinned, _)| pinned == id));
    }
}

/// The connection dropped: keep the last values (shown as stale /
/// offline), drop the command handle.
pub(crate) fn disconnected(cache: &RemoteCache, machine_id: &str) {
    if let Ok(mut map) = cache.lock()
        && let Some(entry) = map.get_mut(machine_id)
    {
        entry.peer = None;
    }
}

/// Clears everything (Link off).
pub(crate) fn clear(cache: &RemoteCache) {
    if let Ok(mut map) = cache.lock() {
        map.clear();
    }
}

// ------------------------------------------------------------- the DTOs

/// A paired machine's resources, as last heard.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteResourcesInfo {
    /// Whole-machine CPU, percent.
    pub cpu_percent: f64,
    /// Memory in use, bytes.
    #[ts(type = "number")]
    pub memory_used_bytes: u64,
    /// Total memory, bytes.
    #[ts(type = "number")]
    pub memory_total_bytes: u64,
    /// When this console received it, epoch ms.
    #[ts(type = "number")]
    pub measured_at_ms: i64,
}

/// One Instance on a paired machine. `None` = that machine doesn't
/// report it.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteInstanceInfo {
    /// The Instance id on that machine.
    pub id: String,
    /// Provider id.
    pub provider: String,
    /// Account label (as that machine names it).
    pub account_label: String,
    /// Status word (storage vocabulary).
    pub status: String,
    /// The display model.
    pub model: Option<String>,
    /// How the model is known.
    pub model_source: String,
    /// The project folder's name (never a path).
    pub project_name: Option<String>,
    /// Started at, epoch ms (that machine's clock).
    #[ts(type = "number")]
    pub started_at_ms: i64,
    /// Last activity, epoch ms (that machine's clock).
    #[ts(type = "number | null")]
    pub last_activity_ms: Option<i64>,
    /// Process-tree CPU, percent.
    pub cpu_percent: Option<f64>,
    /// Process-tree memory, bytes.
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// Processes in its tree.
    pub process_count: Option<u32>,
    /// When this console received the row, epoch ms.
    #[ts(type = "number")]
    pub measured_at_ms: i64,
}

/// An alert from (or about) a paired machine.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteAlertInfo {
    /// The alert kind (as local alerts, plus `peer-offline`).
    pub kind: String,
    /// The Account it is about (empty for machine-level alerts).
    pub account: String,
    /// Plain words.
    pub text: String,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// A paired machine is flagged after this long without a word.
const OFFLINE_ALERT_AFTER_MS: i64 = 10 * 60 * 1000;

/// One paired machine, as this console sees it.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteMachineInfo {
    /// Its `MachineId`.
    pub machine_id: String,
    /// The local label for it.
    pub label: String,
    /// The link state (as `FleetPeerInfo.connection`).
    pub connection: String,
    /// `live` (heard < 5 s ago on a live connection) | `stale` |
    /// `offline`.
    pub freshness: String,
    /// When anything last arrived from it, epoch ms.
    #[ts(type = "number | null")]
    pub last_heard_ms: Option<i64>,
    /// What it lets THIS Mac do: `view` | `control` | `no_access`;
    /// `None` until it has answered once.
    pub permission: Option<String>,
    /// Its resources, when reported.
    pub resources: Option<RemoteResourcesInfo>,
    /// Its Instances.
    pub instances: Vec<RemoteInstanceInfo>,
    /// Its recent alerts, newest first (plus "offline for over 10
    /// minutes" when that holds).
    pub alerts: Vec<RemoteAlertInfo>,
}

/// The outcome of a remote command, in the node's words.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteCommandOutcomeInfo {
    /// Whether the node accepted it.
    pub accepted: bool,
    /// The node's reason code when rejected (`permission_denied`, …).
    pub reason_code: Option<String>,
    /// Plain words for the UI.
    pub message: String,
}

fn freshness(connection: &str, last_heard_ms: Option<i64>, now: i64) -> &'static str {
    let live_link = connection == "connected" || connection == "degraded";
    match last_heard_ms {
        Some(at) if live_link && now - at <= LIVE_WITHIN_MS => "live",
        _ if live_link => "stale",
        _ => "offline",
    }
}

/// Core: every paired machine with what it last reported.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the registry can't load.
pub fn fleet_remote_impl(state: &Arc<AppState>) -> AppResult<Vec<RemoteMachineInfo>> {
    // Registry + link state first; the cache lock is taken alone.
    let peers = crate::fleet::fleet_peers_impl(state)?;
    let now = now_ms();
    let map = state
        .link
        .remote
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    Ok(peers
        .into_iter()
        .map(|peer| {
            let entry = map.get(&peer.machine_id);
            let last_heard_ms = entry.and_then(|entry| entry.last_heard_ms);
            let mut instances: Vec<RemoteInstanceInfo> = entry
                .map(|entry| {
                    entry
                        .instances
                        .values()
                        .map(|(row, at)| RemoteInstanceInfo {
                            id: row.instance_id.clone(),
                            provider: row.provider_id.clone(),
                            account_label: row.account_label.clone(),
                            status: row.status.clone(),
                            model: row.model.clone(),
                            model_source: if row.model_source.is_empty() {
                                "unknown".to_owned()
                            } else {
                                row.model_source.clone()
                            },
                            project_name: row.project_name.clone(),
                            started_at_ms: row.started_at_ms,
                            last_activity_ms: row.last_activity_ms,
                            cpu_percent: row.cpu_percent,
                            rss_bytes: row.rss_bytes,
                            process_count: row.process_count,
                            measured_at_ms: *at,
                        })
                        .collect()
                })
                .unwrap_or_default();
            instances.sort_by_key(|row| std::cmp::Reverse(row.started_at_ms));
            let freshness = freshness(&peer.connection, last_heard_ms, now);
            let mut alerts: Vec<RemoteAlertInfo> = entry
                .map(|entry| {
                    entry
                        .alerts
                        .iter()
                        .map(|alert| RemoteAlertInfo {
                            kind: alert.kind.clone(),
                            account: alert.account_label.clone(),
                            text: alert.text.clone(),
                            at_ms: alert.at_ms,
                        })
                        .collect()
                })
                .unwrap_or_default();
            let quiet_since = last_heard_ms.or(peer.last_seen_ms);
            if peer.connection != "link_off"
                && freshness == "offline"
                && quiet_since.is_some_and(|at| now - at > OFFLINE_ALERT_AFTER_MS)
            {
                alerts.insert(
                    0,
                    RemoteAlertInfo {
                        kind: "peer-offline".to_owned(),
                        account: String::new(),
                        text: format!("{} has been offline for over 10 minutes.", peer.label),
                        at_ms: quiet_since.unwrap_or(now),
                    },
                );
            }
            RemoteMachineInfo {
                freshness: freshness.to_owned(),
                alerts,
                machine_id: peer.machine_id,
                label: peer.label,
                connection: peer.connection,
                last_heard_ms,
                permission: entry.and_then(|entry| entry.permission.clone()),
                resources: entry.and_then(|entry| {
                    entry
                        .resources
                        .as_ref()
                        .map(|(resources, at)| RemoteResourcesInfo {
                            cpu_percent: resources.cpu_percent,
                            memory_used_bytes: resources.memory_used_bytes,
                            memory_total_bytes: resources.memory_total_bytes,
                            measured_at_ms: *at,
                        })
                }),
                instances,
            }
        })
        .collect())
}

/// The next command sequence for `machine_id`: above both the stored
/// one and a wall-clock seed, so a reset console never reuses one the
/// node already saw.
fn next_command_sequence(state: &Arc<AppState>, machine_id: &str) -> AppResult<u64> {
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    let mut map: HashMap<String, u64> = storage
        .app_settings()
        .get(SEQUENCE_KEY)?
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    let seed = u64::try_from(now_ms()).unwrap_or(0).saturating_mul(1000);
    let next = map
        .get(machine_id)
        .map_or(seed, |last| (last + 1).max(seed));
    map.insert(machine_id.to_owned(), next);
    let raw = serde_json::to_string(&map).map_err(|e| AppError::Io {
        context: "fleet command sequence",
        message: e.to_string(),
    })?;
    storage.app_settings().set(SEQUENCE_KEY, &raw)?;
    Ok(next)
}

/// Core: asks `machine_id` to stop one of its Instances. The node
/// re-checks everything (permission, replay, expiry, that the Instance
/// exists) and answers in its own words.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when that machine isn't connected.
pub async fn fleet_remote_stop_impl(
    state: Arc<AppState>,
    machine_id: String,
    instance_id: String,
) -> AppResult<RemoteCommandOutcomeInfo> {
    let (peer, permission) = {
        let map = state
            .link
            .remote
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        let entry = map.get(&machine_id);
        (
            entry.and_then(|entry| entry.peer.clone()),
            entry.and_then(|entry| entry.permission.clone()),
        )
    };
    let Some(peer) = peer else {
        return Err(AppError::Validation {
            subject: "remote action",
            message: "That machine isn't connected right now.".to_owned(),
        });
    };
    if permission.as_deref() != Some("control") {
        // The UI never offers this; the node would refuse it anyway.
        return Ok(RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("permission_denied".to_owned()),
            message: "View-only pairing — change it in Fleet on that machine.".to_owned(),
        });
    }
    let console = crate::commands::local_machine_id(&state).to_string();
    let sequence = next_command_sequence(&state, &machine_id)?;
    let action = proto::control_command::Action::StopInstance(proto::StopInstance { instance_id });
    let runtime = state.link.runtime()?;
    let task = runtime.spawn(async move {
        tokio::time::timeout(
            COMMAND_TIMEOUT,
            juggler_link::client::send_command(&peer, &console, sequence, action),
        )
        .await
    });
    let result = task.await.map_err(|e| AppError::Io {
        context: "remote action",
        message: e.to_string(),
    })?;
    match result {
        Err(_) => Ok(RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("timeout".to_owned()),
            message: "That machine didn't answer in time — nothing was confirmed.".to_owned(),
        }),
        Ok(Err(error)) => Ok(RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("unreachable".to_owned()),
            message: error.to_string(),
        }),
        Ok(Ok(result)) => Ok(RemoteCommandOutcomeInfo {
            accepted: result.outcome == "accepted",
            reason_code: (!result.reason_code.is_empty()).then_some(result.reason_code),
            message: result.message,
        }),
    }
}

/// AJ-E705: every paired machine with what it last reported.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the registry can't load.
#[tauri::command]
pub fn fleet_remote(state: tauri::State<'_, Arc<AppState>>) -> AppResult<Vec<RemoteMachineInfo>> {
    fleet_remote_impl(state.inner())
}

/// AJ-E705: asks a paired machine to stop one of its Instances.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_remote_stop_impl`].
#[tauri::command]
pub async fn fleet_remote_stop(
    state: tauri::State<'_, Arc<AppState>>,
    machine_id: String,
    instance_id: String,
) -> AppResult<RemoteCommandOutcomeInfo> {
    fleet_remote_stop_impl(Arc::clone(state.inner()), machine_id, instance_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_is_never_live_without_a_live_link() {
        let now = 100_000;
        assert_eq!(freshness("connected", Some(now - 1_000), now), "live");
        assert_eq!(freshness("connected", Some(now - 6_000), now), "stale");
        assert_eq!(freshness("connected", None, now), "stale");
        assert_eq!(freshness("degraded", Some(now - 1_000), now), "live");
        assert_eq!(freshness("offline", Some(now - 1_000), now), "offline");
        assert_eq!(freshness("link_off", Some(now), now), "offline");
    }
}
