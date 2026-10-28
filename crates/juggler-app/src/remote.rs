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
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use juggler_link::client::LinkPeer;
use juggler_link::proto;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A machine's values are `live` while heard within this window.
const LIVE_WITHIN_MS: i64 = 5_000;
/// An Instance's live numbers (CPU, memory) are dropped after this long
/// without an update; its durable row stays (AJ-E708).
const FORGET_LIVE_AFTER_MS: i64 = 10_000;
/// The replica kind for Instance rows.
const INSTANCE_KIND: &str = "instance";
/// At most this many of a machine's Instances are shown (newest first).
const SHOW_INSTANCES: usize = 50;
/// A clock difference past this is flagged (ordering uses sequences, so
/// it is only a display caveat).
pub const CLOCK_SKEW_FLAG_MS: i64 = 5 * 60 * 1000;
/// The app setting holding this console's command sequence per peer.
const SEQUENCE_KEY: &str = "fleet_console_sequence";
/// How long a remote command may take end to end (the node may wait up
/// to 20 s for its user to Allow an "Ask each time" request).
const COMMAND_TIMEOUT: Duration = Duration::from_secs(25);

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
    /// Latest live numbers per Instance and when they arrived (the
    /// durable rows live in the replica, AJ-E708).
    live: HashMap<String, (proto::SessionDelta, i64)>,
    /// Durable rows for a node without a sync epoch (an older build):
    /// kept in memory only, as before AJ-E708.
    unsynced: HashMap<String, (proto::SessionDelta, i64)>,
    /// That machine's clock minus this Mac's, from its Hello.
    clock_skew_ms: Option<i64>,
    /// When a durable change from it was last applied.
    synced_at_ms: Option<i64>,
    /// When anything last arrived.
    last_heard_ms: Option<i64>,
    /// The machine's recent alerts (newest first, at most 10).
    alerts: Vec<proto::AlertDelta>,
    /// Its managed processes and when they arrived (AJ-E707).
    cleanup: Option<(proto::CleanupDelta, i64)>,
    /// What it can take work into (AJ-E706), as last heard.
    catalog: Option<(proto::CatalogDelta, i64)>,
}

/// The cache, shared by the reconcile loop and the commands.
pub type RemoteCache = Arc<Mutex<HashMap<String, RemoteMachine>>>;

/// Subscribes to `machine_id`'s telemetry on a fresh connection,
/// catching up from this Mac's durable replica of it (AJ-E708): only the
/// changes after its high-water mark are sent, and a new sync epoch
/// (that machine was reset) starts the replica afresh.
pub(crate) fn subscribe(
    state: &Weak<AppState>,
    cache: &RemoteCache,
    machine_id: &str,
    peer: &LinkPeer,
) -> tokio::task::JoinHandle<()> {
    let now = now_ms();
    let epoch = peer.hello.sync_epoch.clone();
    if let Ok(mut map) = cache.lock() {
        let entry = map.entry(machine_id.to_owned()).or_default();
        entry.permission = Some(peer.hello.permission.clone());
        entry.peer = Some(peer.clone());
        entry.clock_skew_ms = (peer.hello.sent_at_ms > 0).then(|| peer.hello.sent_at_ms - now);
    }
    let since = if epoch.is_empty() {
        0
    } else {
        state
            .upgrade()
            .and_then(|app| {
                let storage = app.storage.lock().ok()?;
                let sync = storage.fleet_sync();
                sync.replica_set_epoch(machine_id, &epoch).ok()?;
                sync.replica_high_water(machine_id, &epoch).ok()
            })
            .unwrap_or(0)
    };
    let durable = !epoch.is_empty();
    let state = state.clone();
    let cache = cache.clone();
    let id = machine_id.to_owned();
    let mut client = peer.client.clone();
    tokio::spawn(async move {
        let Ok(response) = client
            .telemetry(proto::TelemetryRequest {
                since_sequence: since,
            })
            .await
        else {
            return;
        };
        let mut stream = response.into_inner();
        while let Ok(Some(delta)) = stream.message().await {
            if from_connected_origin(&delta, &id) {
                apply(&state, &cache, &id, durable, delta);
            }
        }
    })
}

/// v1 never relays: a delta is accepted only when its origin is the
/// authenticated machine on this connection (a spoofed origin is
/// dropped — ADR-0013 "Spoofed delta origin").
fn from_connected_origin(delta: &proto::TelemetryDelta, connected: &str) -> bool {
    delta.origin_machine_id == connected
}

/// Applies one delta. A sequenced Instance row from a node with a sync
/// epoch goes to the durable replica (newest sequence wins, so repeats
/// and reordering are harmless); everything else is live, in memory.
fn apply(
    state: &Weak<AppState>,
    cache: &RemoteCache,
    machine_id: &str,
    durable: bool,
    delta: proto::TelemetryDelta,
) {
    use prost::Message as _;
    let now = now_ms();
    let mut synced = false;
    if durable
        && delta.sequence > 0
        && let Some(proto::telemetry_delta::Payload::Session(session)) = &delta.payload
    {
        // Storage first, alone — never under the cache lock.
        synced = state.upgrade().is_some_and(|app| {
            app.storage.lock().is_ok_and(|storage| {
                let entry = juggler_core::storage::SyncEntry {
                    seq: delta.sequence,
                    kind: INSTANCE_KIND.to_owned(),
                    record_id: session.instance_id.clone(),
                    payload: session.encode_to_vec(),
                    wall_ms: session.updated_at_ms,
                };
                storage
                    .fleet_sync()
                    .replica_apply(machine_id, &entry, now)
                    .is_ok()
            })
        });
    }
    let Ok(mut map) = cache.lock() else {
        return;
    };
    let entry = map.entry(machine_id.to_owned()).or_default();
    entry.last_heard_ms = Some(now);
    if synced {
        entry.synced_at_ms = Some(now);
        return;
    }
    match delta.payload {
        Some(proto::telemetry_delta::Payload::Resources(resources)) => {
            entry.resources = Some((resources, now));
        }
        Some(proto::telemetry_delta::Payload::Session(session)) => {
            if delta.sequence > 0 {
                // A node without a sync epoch (older build).
                entry
                    .unsynced
                    .insert(session.instance_id.clone(), (session, now));
            } else {
                entry
                    .live
                    .insert(session.instance_id.clone(), (session, now));
            }
        }
        Some(proto::telemetry_delta::Payload::Cleanup(cleanup)) => {
            entry.cleanup = Some((cleanup, now));
        }
        Some(proto::telemetry_delta::Payload::Catalog(catalog)) => {
            entry.catalog = Some((catalog, now));
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
        .live
        .retain(|_, (_, at)| now - *at <= FORGET_LIVE_AFTER_MS);
    entry
        .unsynced
        .retain(|_, (_, at)| now - *at <= FORGET_LIVE_AFTER_MS);
}

/// Resubscribes to a connected machine whose telemetry stream ended (it
/// lagged, or its permission came back) — `None` when there is no live
/// connection or it grants nothing.
pub(crate) fn resubscribe(
    state: &Weak<AppState>,
    cache: &RemoteCache,
    machine_id: &str,
) -> Option<tokio::task::JoinHandle<()>> {
    let peer = cache.lock().ok().and_then(|map| {
        map.get(machine_id).and_then(|entry| {
            let grants = entry
                .permission
                .as_deref()
                .is_some_and(|permission| permission == "view" || permission == "control");
            entry.peer.clone().filter(|_| grants)
        })
    })?;
    Some(subscribe(state, cache, machine_id, &peer))
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

/// AJ-E706: one paired machine as a switch target — its catalog, what
/// it lets this Mac do, and its live connection (for the push).
#[derive(Clone, Debug, Default)]
pub(crate) struct TargetView {
    pub catalog: Option<proto::CatalogDelta>,
    pub permission: Option<String>,
    pub peer: Option<LinkPeer>,
}

/// AJ-E706: every paired machine this console has heard from.
pub(crate) fn target_views(cache: &RemoteCache) -> Vec<(String, TargetView)> {
    cache
        .lock()
        .map(|map| {
            map.iter()
                .map(|(id, entry)| {
                    (
                        id.clone(),
                        TargetView {
                            catalog: entry.catalog.as_ref().map(|(catalog, _)| catalog.clone()),
                            permission: entry.permission.clone(),
                            peer: entry.peer.clone(),
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// AJ-E1515: each paired machine's last-day usage, as it last reported
/// it with its catalog, and when.
pub(crate) fn reported_usage(cache: &RemoteCache) -> Vec<(String, proto::UsageTotals, i64)> {
    cache
        .lock()
        .map(|map| {
            map.iter()
                .filter_map(|(id, entry)| {
                    let (catalog, at) = entry.catalog.as_ref()?;
                    Some((id.clone(), catalog.last_day?, *at))
                })
                .collect()
        })
        .unwrap_or_default()
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
    /// `terminal` | `desktop` (empty = not reported).
    pub kind: String,
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

/// One managed process on a paired machine (AJ-E707), as its Cleanup
/// Centre lists it — never a command line.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteProcessInfo {
    /// The owning Instance.
    pub instance_id: String,
    /// Provider id of the owning Instance.
    pub provider: String,
    /// Account label of the owning Instance.
    pub account_label: String,
    /// OS process id on that machine.
    pub pid: u32,
    /// `root` | `helper`.
    pub role: String,
    /// `running` | `orphaned` | `unverifiable` | …
    pub state: String,
    /// The plain-words state.
    pub state_label: String,
    /// That machine verified it as owned (pid + start time + exe + boot)
    /// and it may be cleaned; otherwise it is never touched.
    pub verified: bool,
    /// The executable's file name.
    pub name: String,
    /// Age, ms.
    #[ts(type = "number | null")]
    pub age_ms: Option<i64>,
    /// Memory, bytes.
    #[ts(type = "number | null")]
    pub rss_bytes: Option<u64>,
    /// CPU, percent.
    pub cpu_percent: Option<f64>,
    /// Listening ports, when any.
    pub ports: String,
    /// When this console received it, epoch ms.
    #[ts(type = "number")]
    pub measured_at_ms: i64,
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
    /// Its managed processes (AJ-E707), when reported.
    pub processes: Vec<RemoteProcessInfo>,
    /// Its clock minus this Mac's, ms (from its last Hello). Order never
    /// depends on it — sync uses that machine's sequence — but a large
    /// difference makes its times misleading, so the UI says so.
    #[ts(type = "number | null")]
    pub clock_skew_ms: Option<i64>,
    /// When a durable change from it was last applied here, epoch ms.
    #[ts(type = "number | null")]
    pub synced_at_ms: Option<i64>,
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
    /// AJ-E1515: the session a launch started there, to follow.
    #[ts(optional)]
    pub started_instance_id: Option<String>,
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
    let replicas: HashMap<String, Vec<(proto::SessionDelta, i64)>> = {
        use prost::Message as _;
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        let sync = storage.fleet_sync();
        let mut replicas = HashMap::new();
        for peer in &peers {
            let rows = sync
                .replica_list(&peer.machine_id, INSTANCE_KIND)?
                .into_iter()
                .filter_map(|(entry, received)| {
                    proto::SessionDelta::decode(entry.payload.as_slice())
                        .ok()
                        .map(|row| (row, received))
                })
                .collect();
            replicas.insert(peer.machine_id.clone(), rows);
        }
        replicas
    };
    let map = state
        .link
        .remote
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    Ok(peers
        .into_iter()
        .map(|peer| {
            let entry = map.get(&peer.machine_id);
            let replica = replicas
                .get(&peer.machine_id)
                .map_or(&[][..], Vec::as_slice);
            machine_info(peer, entry, replica, now)
        })
        .collect())
}

fn machine_info(
    peer: crate::fleet::FleetPeerInfo,
    entry: Option<&RemoteMachine>,
    replica: &[(proto::SessionDelta, i64)],
    now: i64,
) -> RemoteMachineInfo {
    let last_heard_ms = entry.and_then(|entry| entry.last_heard_ms);
    let freshness = freshness(&peer.connection, last_heard_ms, now);
    let mut alerts = entry.map(alerts_of).unwrap_or_default();
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
        processes: entry.map(processes_of).unwrap_or_default(),
        instances: instances_of(entry, replica, now),
        clock_skew_ms: entry.and_then(|entry| entry.clock_skew_ms),
        synced_at_ms: entry.and_then(|entry| entry.synced_at_ms),
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
        machine_id: peer.machine_id,
        label: peer.label,
        connection: peer.connection,
        last_heard_ms,
    }
}

/// A machine's Instances: the durable replica (last known, kept while
/// offline and across restarts) merged with the live numbers still fresh.
fn instances_of(
    entry: Option<&RemoteMachine>,
    replica: &[(proto::SessionDelta, i64)],
    now: i64,
) -> Vec<RemoteInstanceInfo> {
    let fresh = |at: i64| now - at <= FORGET_LIVE_AFTER_MS;
    let mut rows: HashMap<&str, (&proto::SessionDelta, i64)> = replica
        .iter()
        .map(|(row, at)| (row.instance_id.as_str(), (row, *at)))
        .collect();
    if let Some(entry) = entry {
        for (id, (row, at)) in &entry.unsynced {
            if fresh(*at) {
                rows.insert(id.as_str(), (row, *at));
            }
        }
    }
    let mut instances: Vec<RemoteInstanceInfo> = rows
        .into_iter()
        .map(|(id, (row, received))| {
            let live = entry
                .and_then(|entry| entry.live.get(id))
                .filter(|(_, at)| fresh(*at));
            let numbers = live.map_or(row, |(numbers, _)| numbers);
            RemoteInstanceInfo {
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
                cpu_percent: numbers.cpu_percent,
                rss_bytes: numbers.rss_bytes,
                process_count: numbers.process_count,
                kind: row.kind.clone(),
                measured_at_ms: live.map_or(received, |(_, at)| (*at).max(received)),
            }
        })
        .collect();
    instances.sort_by_key(|row| std::cmp::Reverse(row.started_at_ms));
    instances.truncate(SHOW_INSTANCES);
    instances
}

fn alerts_of(entry: &RemoteMachine) -> Vec<RemoteAlertInfo> {
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
}

fn processes_of(entry: &RemoteMachine) -> Vec<RemoteProcessInfo> {
    entry
        .cleanup
        .as_ref()
        .map(|(cleanup, at)| {
            cleanup
                .processes
                .iter()
                .map(|row| RemoteProcessInfo {
                    instance_id: row.instance_id.clone(),
                    provider: row.provider.clone(),
                    account_label: row.account_label.clone(),
                    pid: row.pid,
                    role: row.role.clone(),
                    state: row.state.clone(),
                    state_label: row.state_label.clone(),
                    verified: row.verified,
                    name: row.name.clone(),
                    age_ms: row.age_ms,
                    rss_bytes: row.rss_bytes,
                    cpu_percent: row.cpu_percent,
                    ports: row.ports.clone(),
                    measured_at_ms: *at,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The next command sequence for `machine_id`: above both the stored
/// one and a wall-clock seed, so a reset console never reuses one the
/// node already saw.
pub(crate) fn next_command_sequence(state: &Arc<AppState>, machine_id: &str) -> AppResult<u64> {
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

/// Sends one control action to `machine_id` and returns the node's
/// verdict. Logged on THIS Mac too (who was asked, the outcome), so a
/// remote action is audited on both machines.
/// Test seam: sends any control action as a console would (the node's
/// checks are the thing under test — forged actions must be refused).
///
/// # Errors
///
/// Returns the typed [`AppError`] per the console command path.
#[doc(hidden)]
pub async fn send_action_for_test(
    state: Arc<AppState>,
    machine_id: String,
    action: proto::control_command::Action,
) -> AppResult<RemoteCommandOutcomeInfo> {
    send_action(state, machine_id, action, "view").await
}

/// The command's answer, or what the user hears when Link off kept it
/// from finishing: it never went out, or it went out and the answer never
/// came.
fn finished_or_link_off<T>(sent: crate::link::Outbound<T>) -> Result<T, RemoteCommandOutcomeInfo> {
    let message = match sent {
        crate::link::Outbound::Done(result) => return Ok(result),
        crate::link::Outbound::NotSent => crate::link::NOTHING_SENT,
        crate::link::Outbound::Ended => {
            "Link went off before that machine answered — it may still have run there."
        }
    };
    Err(RemoteCommandOutcomeInfo {
        accepted: false,
        reason_code: Some("link_off".to_owned()),
        message: message.to_owned(),
        started_instance_id: None,
    })
}

pub(crate) async fn send_action(
    state: Arc<AppState>,
    machine_id: String,
    action: proto::control_command::Action,
    kind: &'static str,
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
        crate::usage_stats::count(
            "fleet.remote_action",
            &[("kind", kind), ("outcome", "denied")],
        );
        return Ok(RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("permission_denied".to_owned()),
            message: "View-only pairing — change it in Fleet on that machine.".to_owned(),
            started_instance_id: None,
        });
    }
    let console = crate::commands::local_machine_id(&state).to_string();
    let sequence = next_command_sequence(&state, &machine_id)?;
    // The node may wait for its user ("Ask each time"), up to 20 s.
    let sent = crate::link::run_outbound(&state, async move {
        tokio::time::timeout(
            COMMAND_TIMEOUT,
            juggler_link::client::send_command(&peer, &console, sequence, action),
        )
        .await
    })
    .await?;
    let result = match finished_or_link_off(sent) {
        Ok(result) => result,
        Err(link_off) => return Ok(link_off),
    };
    let outcome = match result {
        Err(_) => RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("timeout".to_owned()),
            message: "That machine didn't answer in time — nothing was confirmed.".to_owned(),
            started_instance_id: None,
        },
        Ok(Err(error)) => RemoteCommandOutcomeInfo {
            accepted: false,
            reason_code: Some("unreachable".to_owned()),
            message: error.to_string(),
            started_instance_id: None,
        },
        Ok(Ok(result)) => RemoteCommandOutcomeInfo {
            accepted: result.outcome == "accepted",
            reason_code: (!result.reason_code.is_empty()).then_some(result.reason_code),
            message: result.message,
            started_instance_id: result.started_instance_id.filter(|id| !id.is_empty()),
        },
    };
    // AJ-E1103: the kind and how it went, never what or where.
    let counted = if outcome.accepted {
        "ok"
    } else if outcome.reason_code.as_deref() == Some("permission_denied") {
        "denied"
    } else {
        "failed"
    };
    crate::usage_stats::count(
        "fleet.remote_action",
        &[("kind", kind), ("outcome", counted)],
    );
    crate::fleet::append_audit(
        &state,
        &juggler_link::node::LinkAuditEvent {
            peer: machine_id,
            kind: "command_sent".to_owned(),
            reason_code: outcome
                .reason_code
                .clone()
                .unwrap_or_else(|| "accepted".to_owned()),
            sequence: Some(sequence),
            count: None,
        },
    );
    Ok(outcome)
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
    send_action(
        state,
        machine_id,
        proto::control_command::Action::StopInstance(proto::StopInstance { instance_id }),
        "cleanup",
    )
    .await
}

/// Core (AJ-E711): asks `machine_id` to bring one of its desktop
/// Instances' windows forward — executed there, with its permissions;
/// the answer is that machine's honest focus outcome.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when that machine isn't connected.
pub async fn fleet_remote_focus_impl(
    state: Arc<AppState>,
    machine_id: String,
    instance_id: String,
) -> AppResult<RemoteCommandOutcomeInfo> {
    send_action(
        state,
        machine_id,
        proto::control_command::Action::FocusInstance(proto::FocusInstance { instance_id }),
        "view",
    )
    .await
}

/// AJ-E705: every paired machine with what it last reported.
///
/// # Errors
///
/// Returns the typed [`AppError`] when the registry can't load.
#[tauri::command]
pub async fn fleet_remote(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<RemoteMachineInfo>> {
    crate::ipc::blocking(state.inner(), "read the other Macs", fleet_remote_impl).await
}

/// Core (AJ-E707): asks `machine_id` to clean one process it owns. That
/// machine re-verifies ownership first and runs its own ladder; the
/// answer lists each process's outcome in its words.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when that machine isn't connected.
pub async fn fleet_remote_clean_impl(
    state: Arc<AppState>,
    machine_id: String,
    instance_id: String,
    pid: u32,
) -> AppResult<RemoteCommandOutcomeInfo> {
    send_action(
        state,
        machine_id,
        proto::control_command::Action::CleanProcess(proto::CleanProcess { instance_id, pid }),
        "cleanup",
    )
    .await
}

/// AJ-E707: asks a paired machine to clean one of its processes.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_remote_clean_impl`].
#[tauri::command]
pub async fn fleet_remote_clean(
    state: tauri::State<'_, Arc<AppState>>,
    machine_id: String,
    instance_id: String,
    pid: u32,
) -> AppResult<RemoteCommandOutcomeInfo> {
    fleet_remote_clean_impl(Arc::clone(state.inner()), machine_id, instance_id, pid).await
}

/// AJ-E711: asks a paired machine to focus one of its desktop windows.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`fleet_remote_focus_impl`].
#[tauri::command]
pub async fn fleet_remote_focus(
    state: tauri::State<'_, Arc<AppState>>,
    machine_id: String,
    instance_id: String,
) -> AppResult<RemoteCommandOutcomeInfo> {
    fleet_remote_focus_impl(Arc::clone(state.inner()), machine_id, instance_id).await
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

    #[test]
    fn a_delta_naming_another_machine_is_never_applied() {
        let delta = |origin: &str| proto::TelemetryDelta {
            origin_machine_id: origin.to_owned(),
            ..proto::TelemetryDelta::default()
        };
        assert!(from_connected_origin(&delta("mac-a"), "mac-a"));
        assert!(
            !from_connected_origin(&delta("mac-b"), "mac-a"),
            "a relayed / spoofed origin"
        );
        assert!(!from_connected_origin(&delta(""), "mac-a"));
    }
}

#[cfg(test)]
mod link_off_tests {
    use super::finished_or_link_off;
    use crate::link::Outbound;

    /// A command Link off kept from going out says nothing was sent; one
    /// it cut off after sending says it may still have run there; one that
    /// finished gives its answer.
    #[test]
    fn link_off_says_whether_anything_was_sent() {
        let never = finished_or_link_off(Outbound::<u8>::NotSent).unwrap_err();
        assert_eq!(
            never.message,
            "Link is off on this computer — nothing was sent."
        );
        let cut = finished_or_link_off(Outbound::<u8>::Ended).unwrap_err();
        assert_eq!(
            cut.message,
            "Link went off before that machine answered — it may still have run there."
        );
        assert_eq!(never.reason_code.as_deref(), Some("link_off"));
        assert!(!never.accepted && !cut.accepted);
        assert_eq!(finished_or_link_off(Outbound::Done(7_u8)).ok(), Some(7));
    }
}
