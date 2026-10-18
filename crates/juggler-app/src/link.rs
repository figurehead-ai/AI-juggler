//! The app's Fleet link manager (AJ-E704; ADR-0013 contract).
//!
//! Link is **off by default** and only ever turned on by the user. While
//! on, this Mac:
//!
//! - serves the link (`juggler_link::node`) on TCP 47821 (or an
//!   ephemeral port when that is taken), TLS 1.3 + pinned keys only;
//! - advertises a minimal mDNS record and browses for other nodes;
//! - keeps a connection (with heartbeats) to every pinned peer it can
//!   find, so Fleet shows `Connected · mTLS · v1.0` / `Degraded` /
//!   `Offline` honestly;
//! - runs pairing: as **host** ("Add a machine" — shows a code) or as
//!   **joiner** ("Join a machine" — enters it). Both users confirm the
//!   other machine's label and fingerprint before anything is pinned.
//!
//! Turning Link off withdraws the advertisement, stops the node, and
//! drops every connection; local behaviour is unaffected either way.
//!
//! Everything here holds `Weak<AppState>` (never a cycle), runs on its
//! own tokio runtime, and never holds the fleet lock together with the
//! storage lock.

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use juggler_link::cert::NodeCertificate;
use juggler_link::discovery::{Advertiser, Browser, DiscoveredNode, new_instance_id};
use juggler_link::node::{
    CommandRejection, LinkAuditEvent, LinkNode, NodeHandler, PairCandidate, PairingHost,
    PeerAccess, PeerDirectory, ReplayStore,
};
use juggler_link::pairing::{
    GuardState, GuardStore, PAIRING_WINDOW_MS, PairingCode, PairingError, PairingGuard,
};
use juggler_link::pairing_rpc::{JoinError, PairingInput, pair_with};
use juggler_link::{Permission, PublicIdentity, proto};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The preferred listening port (fixed so a VPN peer can be told it).
pub const PREFERRED_PORT: u16 = 47_821;
/// How often pinned peers are (re)connected.
const RECONCILE_EVERY: Duration = Duration::from_secs(3);

const LINK_ENABLED_KEY: &str = "fleet_link_enabled";
const GUARD_KEY: &str = "fleet_pairing_guard";
const REPLAY_KEY: &str = "fleet_replay_high_water";
const ADDRESSES_KEY: &str = "fleet_peer_addresses";
/// How long one connection attempt may take before the next peer.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

fn now_ms() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

// ---------------------------------------------------------------- state

/// One pinned peer's live link state.
#[derive(Clone, Debug, Default)]
pub struct PeerLink {
    /// `connecting` | `connected` | `degraded` | `offline`.
    pub state: String,
    /// `v1.0` when connected or degraded.
    pub protocol: Option<String>,
    /// Plain words for degraded/offline.
    pub detail: Option<String>,
    /// Where it was last reached.
    pub address: Option<SocketAddr>,
}

/// The host-side pairing phase the UI shows.
#[derive(Debug, Default)]
enum HostPhase {
    #[default]
    Idle,
    Waiting,
    Confirm {
        candidate: PairCandidate,
        respond: Option<tokio::sync::oneshot::Sender<Option<Permission>>>,
    },
    Paired {
        label: String,
    },
    /// Couldn't open (locked / disabled).
    Failed {
        reason: String,
        message: String,
    },
}

/// The joiner-side pairing phase the UI shows.
#[derive(Debug, Default)]
enum JoinPhase {
    #[default]
    Idle,
    Connecting,
    Confirm {
        candidate: PairCandidate,
        respond: Option<tokio::sync::oneshot::Sender<Option<Permission>>>,
    },
    Paired {
        label: String,
    },
    Failed {
        reason: String,
        message: String,
    },
}

struct Running {
    node: LinkNode,
    /// `None` when mDNS isn't available here (pairing by address still
    /// works — e.g. over the user's Tailscale/WireGuard).
    advertiser: Option<Advertiser>,
    browser: Option<Arc<Browser>>,
    port: u16,
    reconcile: tokio::task::JoinHandle<()>,
    ticker: tokio::task::JoinHandle<()>,
}

/// The link manager (one per app).
#[derive(Default)]
pub struct LinkManager {
    runtime: OnceLock<tokio::runtime::Runtime>,
    running: Mutex<Option<Running>>,
    peers: Arc<Mutex<HashMap<String, PeerLink>>>,
    host: Arc<Mutex<HostPhase>>,
    /// The last notice for the host: (reason code, plain words).
    host_message: Arc<Mutex<Option<(String, String)>>>,
    guard: Arc<Mutex<Option<PairingGuard<SettingsGuardStore>>>>,
    join: Arc<Mutex<JoinPhase>>,
    telemetry: OnceLock<tokio::sync::broadcast::Sender<proto::TelemetryDelta>>,
    last_error: Mutex<Option<String>>,
    /// This node's telemetry sequence (AJ-E705).
    sequence: Arc<std::sync::atomic::AtomicU64>,
    /// What paired machines report, as this console last heard it.
    pub(crate) remote: crate::remote::RemoteCache,
    /// Peers to reconnect at once (a re-pair changed what they grant).
    reconnect: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Test seam: skip mDNS (tests must not advertise on a real LAN).
    no_discovery: std::sync::atomic::AtomicBool,
}

impl std::fmt::Debug for LinkManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkManager").finish_non_exhaustive()
    }
}

impl LinkManager {
    pub(crate) fn runtime(&self) -> AppResult<&tokio::runtime::Runtime> {
        if let Some(runtime) = self.runtime.get() {
            return Ok(runtime);
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("aij-link")
            .enable_all()
            .build()
            .map_err(|e| AppError::io("start the Fleet link", &e))?;
        Ok(self.runtime.get_or_init(|| runtime))
    }

    fn telemetry(&self) -> tokio::sync::broadcast::Sender<proto::TelemetryDelta> {
        self.telemetry
            .get_or_init(|| tokio::sync::broadcast::channel(64).0)
            .clone()
    }

    /// The live state for `machine_id` (`link_off` when Link is off).
    #[must_use]
    pub fn peer_state(&self, machine_id: &str) -> PeerLink {
        let on = self.running.lock().is_ok_and(|running| running.is_some());
        if !on {
            return PeerLink {
                state: "link_off".to_owned(),
                ..PeerLink::default()
            };
        }
        self.peers
            .lock()
            .ok()
            .and_then(|peers| peers.get(machine_id).cloned())
            .unwrap_or(PeerLink {
                state: "offline".to_owned(),
                detail: Some("not found on this network yet".to_owned()),
                ..PeerLink::default()
            })
    }
}

// ------------------------------------------------- persisted link stores

/// Lockout state persisted in app settings (survives restarts).
struct SettingsGuardStore(Weak<AppState>);

impl GuardStore for SettingsGuardStore {
    fn load(&self) -> GuardState {
        self.0
            .upgrade()
            .and_then(|state| {
                let storage = state.storage.lock().ok()?;
                storage.app_settings().get(GUARD_KEY).ok().flatten()
            })
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save(&self, guard: &GuardState) {
        if let (Some(state), Ok(raw)) = (self.0.upgrade(), serde_json::to_string(guard))
            && let Ok(storage) = state.storage.lock()
        {
            let _ = storage.app_settings().set(GUARD_KEY, &raw);
        }
    }
}

/// Per-console replay high-water marks, persisted in app settings.
struct SettingsReplayStore(Weak<AppState>);

impl SettingsReplayStore {
    fn map(&self) -> HashMap<String, u64> {
        self.0
            .upgrade()
            .and_then(|state| {
                let storage = state.storage.lock().ok()?;
                storage.app_settings().get(REPLAY_KEY).ok().flatten()
            })
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write(&self, map: &HashMap<String, u64>) {
        if let (Some(state), Ok(raw)) = (self.0.upgrade(), serde_json::to_string(map))
            && let Ok(storage) = state.storage.lock()
        {
            let _ = storage.app_settings().set(REPLAY_KEY, &raw);
        }
    }

    /// Re-pairing resets a console's mark (a restored console re-pairs
    /// rather than replays).
    fn reset(&self, console: &str) {
        let mut map = self.map();
        map.remove(console);
        self.write(&map);
    }
}

impl ReplayStore for SettingsReplayStore {
    fn high_water(&self, console: &str) -> u64 {
        self.map().get(console).copied().unwrap_or(0)
    }

    fn record(&self, console: &str, sequence: u64) {
        let mut map = self.map();
        map.insert(console.to_owned(), sequence);
        self.write(&map);
    }
}

// ------------------------------------------------------- node handlers

/// Looks a TLS key up in the signed peer registry.
struct AppDirectory(Weak<AppState>);

impl PeerDirectory for AppDirectory {
    fn lookup(&self, key: &[u8; 32]) -> Option<PeerAccess> {
        let state = self.0.upgrade()?;
        let identity = PublicIdentity::from_bytes(key).ok()?;
        crate::fleet::lookup_peer(&state, &identity)
    }
}

/// What this node serves and executes.
struct AppNode {
    state: Weak<AppState>,
    live: tokio::sync::broadcast::Sender<proto::TelemetryDelta>,
    sequence: Arc<std::sync::atomic::AtomicU64>,
}

/// The per-node telemetry sequence: seeded from the wall clock at start
/// (so it keeps rising across restarts), then strictly increasing.
fn next_sequence(sequence: &std::sync::atomic::AtomicU64) -> u64 {
    let seed = u64::try_from(now_ms()).unwrap_or(0).saturating_mul(1000);
    let _ = sequence.fetch_max(seed, std::sync::atomic::Ordering::Relaxed);
    sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}

fn delta(
    origin: &str,
    sequence: u64,
    payload: proto::telemetry_delta::Payload,
) -> proto::TelemetryDelta {
    proto::TelemetryDelta {
        origin_machine_id: origin.to_owned(),
        sequence,
        hlc_ms: now_ms(),
        hlc_counter: 0,
        payload: Some(payload),
    }
}

/// This Mac's Instances as session deltas — only what the link contract
/// allows: ids, Provider, Account label, status, model, the project
/// folder's name (never a path) and live process numbers.
fn session_deltas(
    state: &Arc<AppState>,
    sequence: &std::sync::atomic::AtomicU64,
) -> Vec<proto::TelemetryDelta> {
    let origin = crate::commands::local_machine_id(state).to_string();
    let Ok(rows) = crate::commands::instances_shared_snapshot(state) else {
        return Vec::new();
    };
    let now = now_ms();
    rows.into_iter()
        .map(|row| {
            let project_name = std::path::Path::new(&row.cwd)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .filter(|name| !name.is_empty());
            delta(
                &origin,
                next_sequence(sequence),
                proto::telemetry_delta::Payload::Session(proto::SessionDelta {
                    instance_id: row.id,
                    provider_id: row.provider,
                    account_label: row.account_label,
                    status: row.status,
                    updated_at_ms: now,
                    model: row.model,
                    model_source: row.model_source,
                    project_name,
                    started_at_ms: row.started_at_ms,
                    last_activity_ms: row.last_activity_ms,
                    cpu_percent: row.cpu_pct,
                    rss_bytes: row.rss_bytes,
                    process_count: row.child_count.map(|children| children + 1),
                }),
            )
        })
        .collect()
}

impl NodeHandler for AppNode {
    fn local_machine_id(&self) -> String {
        self.state
            .upgrade()
            .map(|state| crate::commands::local_machine_id(&state).to_string())
            .unwrap_or_default()
    }

    fn telemetry_since(&self, _since: u64) -> Vec<proto::TelemetryDelta> {
        // v1 backlog = the current snapshot of this node's Instances
        // (sequence-vector catch-up arrives with AJ-E708).
        self.state
            .upgrade()
            .map(|state| session_deltas(&state, &self.sequence))
            .unwrap_or_default()
    }

    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta> {
        self.live.subscribe()
    }

    fn execute(
        &self,
        peer: &PeerAccess,
        action: &proto::control_command::Action,
    ) -> Result<String, CommandRejection> {
        let reject = |code: &str, message: &str| CommandRejection {
            reason_code: code.to_owned(),
            message: message.to_owned(),
        };
        let state = self
            .state
            .upgrade()
            .ok_or_else(|| reject("unavailable", "This Mac is shutting down."))?;
        if crate::fleet::peer_asks_each_time(&state, &peer.machine_id) {
            // The local confirmation prompt arrives with AJ-E709; until
            // then an "Ask each time" peer is refused, never auto-run.
            return Err(reject(
                "needs_local_confirmation",
                "This Mac asks before acting for that machine — confirm it here.",
            ));
        }
        match action {
            proto::control_command::Action::StopInstance(stop) => {
                let known = state
                    .storage
                    .lock()
                    .ok()
                    .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
                    .is_some_and(|rows| rows.iter().any(|row| row.id == stop.instance_id));
                if !known {
                    return Err(reject("unknown_record", "No such Instance on this Mac."));
                }
                state
                    .supervisor
                    .stop(
                        &juggler_core::model::SessionId::new(stop.instance_id.clone()),
                        Duration::from_secs(5),
                    )
                    .map_err(|e| reject("stop_failed", &e.to_string()))?;
                Ok("Stopping".to_owned())
            }
            proto::control_command::Action::LaunchRecipe(launch) => {
                // Only a Launch-settings id and a project this Mac
                // already knows — never a path, binary or environment.
                let cwd = if launch.project_id.is_empty() {
                    None
                } else {
                    let projects = crate::commands::projects_list_impl(&state)
                        .map_err(|e| reject("unavailable", &e.to_string()))?;
                    let project = projects
                        .into_iter()
                        .find(|project| project.id == launch.project_id)
                        .ok_or_else(|| reject("unknown_record", "No such project on this Mac."))?;
                    Some(project.root)
                };
                let launched = crate::commands::launch_profile(&state, &launch.recipe_id, cwd)
                    .map_err(|e| match e {
                        AppError::NotFound { .. } => {
                            reject("unknown_record", "No such Launch settings on this Mac.")
                        }
                        other => reject("launch_failed", &other.wire().user_message),
                    })?;
                Ok(format!("Started {}", launched.session_id))
            }
        }
    }

    fn audit(&self, event: LinkAuditEvent) {
        if let Some(state) = self.state.upgrade() {
            crate::fleet::append_audit(&state, &event);
        }
    }
}

/// The host side of pairing, driven by the UI.
struct AppPairing {
    state: Weak<AppState>,
    guard: Arc<Mutex<Option<PairingGuard<SettingsGuardStore>>>>,
    phase: Arc<Mutex<HostPhase>>,
    message: Arc<Mutex<Option<(String, String)>>>,
    key: [u8; 32],
}

impl AppPairing {
    fn with_guard<T>(
        &self,
        body: impl FnOnce(&mut PairingGuard<SettingsGuardStore>) -> T,
    ) -> Option<T> {
        let mut guard = self.guard.lock().ok()?;
        let guard =
            guard.get_or_insert_with(|| PairingGuard::new(SettingsGuardStore(self.state.clone())));
        Some(body(guard))
    }
}

impl PairingHost for AppPairing {
    fn begin(&self) -> Result<PairingCode, PairingError> {
        self.with_guard(|guard| guard.begin_session(now_ms()))
            .unwrap_or(Err(PairingError::NotOpen))
    }

    fn failed(&self) {
        let _ = self.with_guard(|guard| guard.fail(now_ms()));
        if let Ok(mut message) = self.message.lock() {
            *message = Some((
                "mismatch".to_owned(),
                "That code didn't match — here's a new one.".to_owned(),
            ));
        }
        if let Ok(mut phase) = self.phase.lock()
            && matches!(*phase, HostPhase::Confirm { .. } | HostPhase::Waiting)
        {
            *phase = HostPhase::Waiting;
        }
    }

    fn released(&self) {
        let _ = self.with_guard(PairingGuard::release);
        if let Ok(mut phase) = self.phase.lock()
            && matches!(*phase, HostPhase::Confirm { .. })
        {
            *phase = HostPhase::Waiting;
            if let Ok(mut message) = self.message.lock() {
                *message = Some((
                    "cancelled".to_owned(),
                    "Pairing was cancelled on one of the machines — the code still works."
                        .to_owned(),
                ));
            }
        }
    }

    fn local_label(&self) -> String {
        self.state
            .upgrade()
            .and_then(|state| crate::fleet::fleet_self_impl(&state).ok())
            .map_or_else(|| "This Mac".to_owned(), |info| info.label)
    }

    fn local_key(&self) -> [u8; 32] {
        self.key
    }

    fn confirm(
        &self,
        candidate: PairCandidate,
    ) -> tokio::sync::oneshot::Receiver<Option<Permission>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Ok(mut phase) = self.phase.lock() {
            *phase = HostPhase::Confirm {
                candidate,
                respond: Some(tx),
            };
        }
        rx
    }

    fn succeeded(&self, candidate: &PairCandidate, permission: Permission) {
        let _ = self.with_guard(PairingGuard::succeed);
        if let Some(state) = self.state.upgrade() {
            let _ = crate::fleet::pin_candidate(&state, candidate, permission);
            SettingsReplayStore(self.state.clone()).reset(&candidate.machine_id);
            mark_reconnect(&state, &candidate.machine_id);
            if let Some(reach) = candidate.reach {
                remember_address(&state, &candidate.machine_id, reach);
            }
        }
        if let Ok(mut phase) = self.phase.lock() {
            *phase = HostPhase::Paired {
                label: candidate.label.clone(),
            };
        }
    }
}

// ------------------------------------------------------------ commands

/// Link status for Fleet's header.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LinkStatusInfo {
    /// Whether Link is on.
    pub enabled: bool,
    /// The listening port, when on.
    pub port: Option<u16>,
    /// Why Link couldn't start, when it couldn't.
    pub error: Option<String>,
}

/// A machine seen on the network that isn't paired.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiscoveredMachineInfo {
    /// Its per-boot instance id.
    pub instance_id: String,
    /// "Unnamed machine (a1b2)" — names arrive only inside pairing.
    pub label: String,
    /// The advertised protocol version.
    pub version: String,
}

/// The pairing state either side shows.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct PairingStateInfo {
    /// `idle` | `waiting` | `connecting` | `confirm` | `paired` | `failed`.
    pub phase: String,
    /// The code to show (host, while waiting).
    pub code: Option<String>,
    /// The pairing link (host, while waiting): code + fingerprint.
    pub link: Option<String>,
    /// When the window closes, epoch ms (host).
    #[ts(type = "number | null")]
    pub expires_at_ms: Option<i64>,
    /// The other machine's label (confirm / paired).
    pub peer_label: Option<String>,
    /// The other machine's fingerprint (confirm).
    pub peer_fingerprint: Option<String>,
    /// Plain words (a failed code, a refusal, why it closed).
    pub message: Option<String>,
    /// Why, as a stable code: `mismatch` | `cancelled` | `expired` |
    /// `locked_out` | `disabled` | `busy` | `not_open` | `unreachable` |
    /// `fingerprint` | `not_found` | …
    pub reason: Option<String>,
}

impl PairingStateInfo {
    fn idle() -> Self {
        Self {
            phase: "idle".to_owned(),
            code: None,
            link: None,
            expires_at_ms: None,
            peer_label: None,
            peer_fingerprint: None,
            message: None,
            reason: None,
        }
    }
}

fn link_settings_enabled(state: &Arc<AppState>) -> bool {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(LINK_ENABLED_KEY).ok().flatten())
        .is_some_and(|value| value == "1")
}

/// Core: the status.
#[must_use]
pub fn link_status_impl(state: &Arc<AppState>) -> LinkStatusInfo {
    let running = state.link.running.lock();
    LinkStatusInfo {
        enabled: running.as_ref().is_ok_and(|running| running.is_some()),
        port: running
            .ok()
            .and_then(|running| running.as_ref().map(|r| r.port)),
        error: state
            .link
            .last_error
            .lock()
            .ok()
            .and_then(|error| error.clone()),
    }
}

/// Core: turns Link on or off (a user click; remembered).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the identity can't load.
pub fn link_set_enabled_impl(state: &Arc<AppState>, enabled: bool) -> AppResult<LinkStatusInfo> {
    {
        let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
        storage
            .app_settings()
            .set(LINK_ENABLED_KEY, if enabled { "1" } else { "0" })?;
    }
    if enabled {
        start(state)?;
    } else {
        stop(state);
    }
    Ok(link_status_impl(state))
}

/// Starts Link at launch when the user left it on (off the main thread).
pub fn resume_if_enabled(state: Arc<AppState>) {
    if link_settings_enabled(&state) {
        let _ = std::thread::Builder::new()
            .name("aij-link-resume".to_owned())
            .spawn(move || {
                if let Err(error) = start(&state)
                    && let Ok(mut last) = state.link.last_error.lock()
                {
                    *last = Some(error.wire().user_message);
                }
            });
    }
}

fn start(state: &Arc<AppState>) -> AppResult<()> {
    let manager = &state.link;
    if manager
        .running
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .is_some()
    {
        return Ok(());
    }
    let runtime = manager.runtime()?;
    let certificate = crate::fleet::node_certificate(state)?;
    let key = crate::fleet::local_public_key(state)?;
    let weak = Arc::downgrade(state);
    let live = manager.telemetry();
    let listener = runtime
        .block_on(async {
            match tokio::net::TcpListener::bind(("0.0.0.0", PREFERRED_PORT)).await {
                Ok(listener) => Ok(listener),
                Err(_) => tokio::net::TcpListener::bind(("0.0.0.0", 0)).await,
            }
        })
        .map_err(|e| AppError::io("open the Fleet link port", &e))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::io("Fleet link port", &e))?
        .port();
    let pairing = Arc::new(AppPairing {
        state: weak.clone(),
        guard: manager.guard.clone(),
        phase: manager.host.clone(),
        message: manager.host_message.clone(),
        key,
    });
    let node = {
        let _enter = runtime.enter();
        LinkNode::start(
            listener,
            &certificate,
            Arc::new(AppNode {
                state: weak.clone(),
                live: live.clone(),
                sequence: manager.sequence.clone(),
            }),
            Arc::new(AppDirectory(weak.clone())),
            Arc::new(SettingsReplayStore(weak.clone())),
            pairing,
        )
        .map_err(|message| AppError::Io {
            context: "start the Fleet link",
            message,
        })?
    };
    let instance_id = new_instance_id().map_err(|e| AppError::Io {
        context: "Fleet discovery",
        message: e.to_string(),
    })?;
    let (advertiser, browser, discovery_error) = start_discovery(manager, port, &key, &instance_id);
    let reconcile = runtime.spawn(reconcile_loop(
        weak.clone(),
        browser.clone(),
        certificate,
        manager.peers.clone(),
        manager.remote.clone(),
        manager.reconnect.clone(),
    ));
    let ticker = runtime.spawn(resource_ticker(weak, live, manager.sequence.clone()));
    if let Ok(mut last) = manager.last_error.lock() {
        *last = discovery_error;
    }
    *manager
        .running
        .lock()
        .map_err(|_| AppError::StatePoisoned)? = Some(Running {
        node,
        advertiser,
        browser,
        port,
        reconcile,
        ticker,
    });
    Ok(())
}

/// Starts mDNS advertising + browsing. Discovery is a convenience,
/// never a requirement: on failure the node still serves pinned peers
/// and pairing by address, and the reason is returned for the UI.
fn start_discovery(
    manager: &LinkManager,
    port: u16,
    key: &[u8; 32],
    instance_id: &str,
) -> (Option<Advertiser>, Option<Arc<Browser>>, Option<String>) {
    let discovery = if manager
        .no_discovery
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        Err(juggler_link::discovery::DiscoveryError::Daemon(
            "discovery is off for this test".to_owned(),
        ))
    } else {
        Advertiser::start(port, key, instance_id)
            .and_then(|advertiser| Browser::start(instance_id).map(|browser| (advertiser, browser)))
    };
    match discovery {
        Ok((advertiser, browser)) => (Some(advertiser), Some(Arc::new(browser)), None),
        Err(error) => (
            None,
            None,
            Some(format!(
                "{error} — machines on this network won't be listed; pair by address instead."
            )),
        ),
    }
}

fn stop(state: &Arc<AppState>) {
    let manager = &state.link;
    let running = manager
        .running
        .lock()
        .ok()
        .and_then(|mut running| running.take());
    if let Some(running) = running {
        running.reconcile.abort();
        running.ticker.abort();
        if let Some(advertiser) = running.advertiser {
            advertiser.stop();
        }
        if let Some(browser) = running
            .browser
            .and_then(|browser| Arc::try_unwrap(browser).ok())
        {
            browser.stop();
        }
        if let Ok(runtime) = manager.runtime() {
            runtime.block_on(running.node.stop());
        }
    }
    if let Ok(mut peers) = manager.peers.lock() {
        peers.clear();
    }
    crate::remote::clear(&manager.remote);
    if let Ok(mut last) = manager.last_error.lock() {
        *last = None;
    }
    if let Ok(mut guard) = manager.guard.lock()
        && let Some(guard) = guard.as_mut()
    {
        guard.close();
    }
    if let Ok(mut phase) = manager.host.lock() {
        *phase = HostPhase::Idle;
    }
}

/// A live connection: its heartbeat watch, the heartbeat task, and the
/// telemetry subscription.
type Watch = (
    tokio::sync::watch::Receiver<juggler_link::client::Liveness>,
    tokio::task::JoinHandle<()>,
    tokio::task::JoinHandle<()>,
);

/// Keeps a heartbeat connection to every pinned peer it can reach: the
/// addresses mDNS shows for it, then the address it was last reached at
/// (how a peer over the user's VPN stays connected).
async fn reconcile_loop(
    state: Weak<AppState>,
    browser: Option<Arc<Browser>>,
    certificate: NodeCertificate,
    peers: Arc<Mutex<HashMap<String, PeerLink>>>,
    remote: crate::remote::RemoteCache,
    reconnect: Arc<Mutex<std::collections::HashSet<String>>>,
) {
    let mut live: HashMap<String, Watch> = HashMap::new();
    let mut tick = 0_u64;
    loop {
        tick = tick.wrapping_add(1);
        let Some(app) = state.upgrade() else {
            return;
        };
        let own = crate::commands::local_machine_id(&app).to_string();
        let pinned = crate::fleet::pinned_keys(&app);
        let remembered = remembered_addresses(&app);
        drop(app);
        live.retain(|machine_id, (_, heartbeat, telemetry)| {
            let keep = pinned.iter().any(|(id, _)| id == machine_id);
            if !keep {
                heartbeat.abort();
                telemetry.abort();
            }
            keep
        });
        crate::remote::retain(&remote, &pinned);
        let fresh: Vec<String> = reconnect
            .lock()
            .map(|mut set| set.drain().collect())
            .unwrap_or_default();
        for machine_id in fresh {
            if let Some((_, heartbeat, telemetry)) = live.remove(&machine_id) {
                heartbeat.abort();
                telemetry.abort();
                crate::remote::disconnected(&remote, &machine_id);
            }
        }
        if let Ok(mut peers) = peers.lock() {
            peers.retain(|machine_id, _| pinned.iter().any(|(id, _)| id == machine_id));
        }
        let nodes = browser
            .as_ref()
            .map(|browser| browser.nodes())
            .unwrap_or_default();
        for (machine_id, key) in pinned {
            let alive = live.get(&machine_id).is_some_and(|(watch, _, _)| {
                *watch.borrow() == juggler_link::client::Liveness::Live
            });
            if alive {
                // Every ~30 s, re-check what that machine grants.
                if tick.is_multiple_of(10) {
                    crate::remote::refresh_permission(&remote, &machine_id, &own).await;
                }
                continue;
            }
            if let Some((_, heartbeat, telemetry)) = live.remove(&machine_id) {
                heartbeat.abort();
                telemetry.abort();
                crate::remote::disconnected(&remote, &machine_id);
                set_peer(
                    &peers,
                    &machine_id,
                    "offline",
                    None,
                    Some("stopped answering — reconnecting".to_owned()),
                    None,
                );
            }
            let mut addresses: Vec<SocketAddr> = nodes
                .iter()
                .find(|node| node.is(&key))
                .map(ordered)
                .unwrap_or_default();
            if let Some(address) = remembered.get(&machine_id)
                && !addresses.contains(address)
            {
                addresses.push(*address);
            }
            if addresses.is_empty() {
                set_peer(
                    &peers,
                    &machine_id,
                    "offline",
                    None,
                    Some("not found on this network".to_owned()),
                    None,
                );
                continue;
            }
            let outcome = connect_any(&addresses, &certificate, key, &own).await;
            match outcome {
                Ok((address, peer)) => {
                    let watch =
                        adopt_connection(&state, &peers, &remote, &machine_id, address, &peer);
                    live.insert(machine_id.clone(), watch);
                }
                Err(detail) => set_peer(&peers, &machine_id, "offline", None, Some(detail), None),
            }
        }
        tokio::time::sleep(RECONCILE_EVERY).await;
    }
}

/// A fresh connection: record its state, start heartbeats and the
/// telemetry subscription, remember where it was reached.
fn adopt_connection(
    state: &Weak<AppState>,
    peers: &Arc<Mutex<HashMap<String, PeerLink>>>,
    remote: &crate::remote::RemoteCache,
    machine_id: &str,
    address: SocketAddr,
    peer: &juggler_link::client::LinkPeer,
) -> Watch {
    let degraded = peer.hello.service_level == "degraded";
    let version = peer.hello.version.unwrap_or_default();
    set_peer(
        peers,
        machine_id,
        if degraded { "degraded" } else { "connected" },
        Some(format!("v{}.{}", version.major, version.minor)),
        degraded.then(|| peer.hello.degraded_reason.clone()),
        Some(address),
    );
    let (watch, heartbeat) = juggler_link::client::watch_liveness(peer.client.clone());
    let telemetry = crate::remote::subscribe(remote, machine_id, peer);
    if let Some(app) = state.upgrade() {
        crate::fleet::touch_peer(&app, machine_id, now_ms());
        remember_address(&app, machine_id, address);
    }
    (watch, heartbeat, telemetry)
}

/// Tries each address in turn; a refusal (reachable, but it said no)
/// stops the search.
async fn connect_any(
    addresses: &[SocketAddr],
    certificate: &NodeCertificate,
    key: [u8; 32],
    own: &str,
) -> Result<(SocketAddr, juggler_link::client::LinkPeer), String> {
    for address in addresses {
        let attempt = tokio::time::timeout(
            CONNECT_TIMEOUT,
            juggler_link::client::connect(*address, certificate, key, own),
        )
        .await;
        match attempt {
            Ok(Ok(peer)) => return Ok((*address, peer)),
            Ok(Err(juggler_link::client::ConnectError::Refused(reason))) => return Err(reason),
            Ok(Err(_)) | Err(_) => {}
        }
    }
    Err("couldn't reach it".to_owned())
}

/// A (re-)pair changed what `machine_id` grants: reconnect now.
fn mark_reconnect(state: &Arc<AppState>, machine_id: &str) {
    if let Ok(mut set) = state.link.reconnect.lock() {
        set.insert(machine_id.to_owned());
    }
}

/// Where each pinned peer was last reached.
fn remembered_addresses(state: &Arc<AppState>) -> HashMap<String, SocketAddr> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(ADDRESSES_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<HashMap<String, String>>(&raw).ok())
        .map(|map| {
            map.into_iter()
                .filter_map(|(id, address)| address.parse().ok().map(|address| (id, address)))
                .collect()
        })
        .unwrap_or_default()
}

fn remember_address(state: &Arc<AppState>, machine_id: &str, address: SocketAddr) {
    let mut map: HashMap<String, String> = remembered_addresses(state)
        .into_iter()
        .map(|(id, address)| (id, address.to_string()))
        .collect();
    if map.get(machine_id) == Some(&address.to_string()) {
        return;
    }
    map.insert(machine_id.to_owned(), address.to_string());
    if let (Ok(raw), Ok(storage)) = (serde_json::to_string(&map), state.storage.lock()) {
        let _ = storage.app_settings().set(ADDRESSES_KEY, &raw);
    }
}

fn ordered(node: &DiscoveredNode) -> Vec<SocketAddr> {
    let mut addresses = node.addresses.clone();
    addresses.sort_by_key(|address| !address.is_ipv4());
    addresses
}

fn set_peer(
    peers: &Arc<Mutex<HashMap<String, PeerLink>>>,
    machine_id: &str,
    state: &str,
    protocol: Option<String>,
    detail: Option<String>,
    address: Option<SocketAddr>,
) {
    if let Ok(mut peers) = peers.lock() {
        peers.insert(
            machine_id.to_owned(),
            PeerLink {
                state: state.to_owned(),
                protocol,
                detail,
                address,
            },
        );
    }
}

/// Publishes this Mac's resource numbers and Instance rows every 2 s
/// while Link is on.
async fn resource_ticker(
    state: Weak<AppState>,
    live: tokio::sync::broadcast::Sender<proto::TelemetryDelta>,
    sequence: Arc<std::sync::atomic::AtomicU64>,
) {
    let mut system = sysinfo::System::new();
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let Some(app) = state.upgrade() else {
            return;
        };
        let origin = crate::commands::local_machine_id(&app).to_string();
        let sessions = session_deltas(&app, &sequence);
        drop(app);
        system.refresh_cpu_usage();
        system.refresh_memory();
        let _ = live.send(delta(
            &origin,
            next_sequence(&sequence),
            proto::telemetry_delta::Payload::Resources(proto::MachineResources {
                cpu_percent: f64::from(system.global_cpu_usage()),
                memory_used_bytes: system.used_memory(),
                memory_total_bytes: system.total_memory(),
            }),
        ));
        for session in sessions {
            let _ = live.send(session);
        }
    }
}

/// Core: discovered, unpaired machines.
#[must_use]
pub fn discovered_impl(state: &Arc<AppState>) -> Vec<DiscoveredMachineInfo> {
    // The registry first: the fleet lock is never taken under `running`.
    let pinned = crate::fleet::pinned_keys(state);
    let Ok(running) = state.link.running.lock() else {
        return Vec::new();
    };
    let Some(browser) = running
        .as_ref()
        .and_then(|running| running.browser.as_ref())
    else {
        return Vec::new();
    };
    browser
        .nodes()
        .into_iter()
        .filter(|node| !pinned.iter().any(|(_, key)| node.is(key)))
        .map(|node| DiscoveredMachineInfo {
            label: format!("Unnamed machine ({})", node.instance_id),
            instance_id: node.instance_id,
            version: node.version,
        })
        .collect()
}

/// Core: opens the host pairing window ("Add a machine").
///
/// # Errors
///
/// Returns [`AppError::Validation`] when Link is off or pairing is locked.
pub fn pairing_host_open_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    if !link_status_impl(state).enabled {
        return Err(AppError::Validation {
            subject: "pairing",
            message: "Turn Link on first.".to_owned(),
        });
    }
    let manager = &state.link;
    let opened = {
        let mut guard = manager.guard.lock().map_err(|_| AppError::StatePoisoned)?;
        guard
            .get_or_insert_with(|| PairingGuard::new(SettingsGuardStore(Arc::downgrade(state))))
            .open(now_ms())
    };
    if let Err(error) = opened {
        *manager.host.lock().map_err(|_| AppError::StatePoisoned)? = HostPhase::Failed {
            reason: error.code().to_owned(),
            message: error.to_string(),
        };
        return pairing_host_state_impl(state);
    }
    if let Ok(mut message) = manager.host_message.lock() {
        *message = None;
    }
    *manager.host.lock().map_err(|_| AppError::StatePoisoned)? = HostPhase::Waiting;
    pairing_host_state_impl(state)
}

/// Core: the host's pairing state (the UI polls it).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn pairing_host_state_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    let manager = &state.link;
    let message = manager
        .host_message
        .lock()
        .ok()
        .and_then(|message| message.clone());
    let fingerprint = crate::fleet::fleet_self_impl(state)
        .map(|info| info.fingerprint)
        .unwrap_or_default();
    let phase = manager.host.lock().map_err(|_| AppError::StatePoisoned)?;
    let mut info = PairingStateInfo::idle();
    if let Some((reason, message)) = message {
        info.reason = Some(reason);
        info.message = Some(message);
    }
    match &*phase {
        HostPhase::Idle => {}
        HostPhase::Failed { reason, message } => {
            info.phase = String::from("failed");
            info.reason = Some(reason.clone());
            info.message = Some(message.clone());
        }
        HostPhase::Waiting => {
            let (code, opened) = manager
                .guard
                .lock()
                .ok()
                .and_then(|guard| {
                    guard
                        .as_ref()
                        .and_then(|g| g.current_code(now_ms()).map(|c| (c, g.opened_at_ms())))
                })
                .unzip();
            if let Some(code) = code {
                info.phase = String::from("waiting");
                info.link = Some(PairingInput::link(&code, &fingerprint));
                info.code = Some(code.display());
                info.expires_at_ms = opened.flatten().map(|at| at + PAIRING_WINDOW_MS);
            } else {
                info.phase = String::from("failed");
                info.reason = Some("expired".to_owned());
                info.message = Some("The pairing window closed after 2 minutes.".to_owned());
            }
        }
        HostPhase::Confirm { candidate, .. } => {
            info.phase = String::from("confirm");
            info.peer_label = Some(candidate.label.clone());
            info.peer_fingerprint = Some(candidate.fingerprint.clone());
        }
        HostPhase::Paired { label } => {
            info.phase = String::from("paired");
            info.peer_label = Some(label.clone());
        }
    }
    Ok(info)
}

/// Core: this Mac's user confirms (with the permission to grant) or
/// cancels the machine shown.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when nothing awaits a decision.
pub fn pairing_host_decide_impl(
    state: &Arc<AppState>,
    commit: bool,
    permission: Permission,
) -> AppResult<PairingStateInfo> {
    respond(&state.link.host, commit, permission, |phase| match phase {
        HostPhase::Confirm { respond, .. } => respond.take(),
        _ => None,
    })?;
    pairing_host_state_impl(state)
}

fn respond<T>(
    phase: &Arc<Mutex<T>>,
    commit: bool,
    permission: Permission,
    take: impl FnOnce(&mut T) -> Option<tokio::sync::oneshot::Sender<Option<Permission>>>,
) -> AppResult<()> {
    let sender =
        take(&mut *phase.lock().map_err(|_| AppError::StatePoisoned)?).ok_or_else(|| {
            AppError::Validation {
                subject: "pairing",
                message: "Nothing is waiting for a decision.".to_owned(),
            }
        })?;
    let _ = sender.send(commit.then_some(permission));
    Ok(())
}

/// Core: closes the host window (Cancel / leaving Fleet).
pub fn pairing_host_close_impl(state: &Arc<AppState>) {
    let manager = &state.link;
    if let Ok(mut guard) = manager.guard.lock()
        && let Some(guard) = guard.as_mut()
    {
        guard.close();
    }
    if let Ok(mut phase) = manager.host.lock() {
        if let HostPhase::Confirm { respond, .. } = &mut *phase
            && let Some(sender) = respond.take()
        {
            let _ = sender.send(None);
        }
        *phase = HostPhase::Idle;
    }
}

/// Core: re-enables pairing after the daily disable (a user click).
pub fn pairing_re_enable_impl(state: &Arc<AppState>) {
    let mut guard = state.link.guard.lock().ok();
    if let Some(guard) = guard.as_mut() {
        guard
            .get_or_insert_with(|| PairingGuard::new(SettingsGuardStore(Arc::downgrade(state))))
            .re_enable();
    }
}

/// Where to reach the host: the typed address, or the discovered node's.
fn join_addresses(
    state: &Arc<AppState>,
    instance_id: Option<&str>,
    address: Option<&str>,
) -> AppResult<Vec<SocketAddr>> {
    if let Some(address) = address {
        let parsed = address
            .trim()
            .parse::<SocketAddr>()
            .or_else(|_| format!("{}:{PREFERRED_PORT}", address.trim()).parse())
            .map_err(|_| AppError::Validation {
                subject: "address",
                message: "Use an IP address, optionally with :port (for example 100.64.0.2)."
                    .to_owned(),
            })?;
        Ok(vec![parsed])
    } else {
        let running = state
            .link
            .running
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        let browser = running
            .as_ref()
            .and_then(|running| running.browser.as_ref())
            .ok_or_else(|| AppError::Validation {
                subject: "pairing",
                message: "This Mac can't see the local network — pair by address instead."
                    .to_owned(),
            })?;
        let wanted = instance_id.unwrap_or_default();
        let node = browser
            .nodes()
            .into_iter()
            .find(|node| node.instance_id == wanted)
            .ok_or_else(|| AppError::Validation {
                subject: "pairing",
                message: "That machine is no longer on the network.".to_owned(),
            })?;
        Ok(ordered(&node))
    }
}

/// Core: starts joining a host found on the LAN (by instance id) or at
/// an address (`host:port`, e.g. over the user's VPN).
///
/// # Errors
///
/// Returns [`AppError::Validation`] for a bad code/address or Link off.
pub fn pairing_join_start_impl(
    state: &Arc<AppState>,
    instance_id: Option<&str>,
    address: Option<&str>,
    typed: &str,
) -> AppResult<PairingStateInfo> {
    let input = PairingInput::parse(typed).map_err(|_| AppError::Validation {
        subject: "pairing code",
        message: "Enter the 6-digit code shown on the other Mac.".to_owned(),
    })?;
    let manager = &state.link;
    let Some(own_port) = link_status_impl(state).port else {
        return Err(AppError::Validation {
            subject: "pairing",
            message: "Turn Link on first.".to_owned(),
        });
    };
    let addresses = join_addresses(state, instance_id, address)?;
    let certificate = crate::fleet::node_certificate(state)?;
    let key = crate::fleet::local_public_key(state)?;
    let own = crate::commands::local_machine_id(state).to_string();
    let label = crate::fleet::fleet_self_impl(state)?.label;
    *manager.join.lock().map_err(|_| AppError::StatePoisoned)? = JoinPhase::Connecting;
    let join = manager.join.clone();
    let weak = Arc::downgrade(state);
    manager.runtime()?.spawn(async move {
        let mut last = JoinError::Cancelled;
        for address in addresses {
            let phase = join.clone();
            let result = pair_with(
                address,
                juggler_link::pairing_rpc::Joiner {
                    certificate: &certificate,
                    key,
                    machine_id: &own,
                    label: &label,
                    port: own_port,
                },
                &input,
                move |candidate| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    if let Ok(mut phase) = phase.lock() {
                        *phase = JoinPhase::Confirm {
                            candidate,
                            respond: Some(tx),
                        };
                    }
                    async move { rx.await.ok().flatten() }
                },
            )
            .await;
            match result {
                Ok(joined) => {
                    if let Some(app) = weak.upgrade() {
                        let _ =
                            crate::fleet::pin_candidate(&app, &joined.candidate, joined.permission);
                        SettingsReplayStore(weak.clone()).reset(&joined.candidate.machine_id);
                        remember_address(&app, &joined.candidate.machine_id, address);
                        mark_reconnect(&app, &joined.candidate.machine_id);
                    }
                    if let Ok(mut phase) = join.lock() {
                        *phase = JoinPhase::Paired {
                            label: joined.candidate.label,
                        };
                    }
                    return;
                }
                Err(JoinError::Connect(reason)) => last = JoinError::Connect(reason),
                Err(other) => {
                    last = other;
                    break;
                }
            }
        }
        if let Ok(mut phase) = join.lock() {
            *phase = JoinPhase::Failed {
                reason: last.reason_code().to_owned(),
                message: last.to_string(),
            };
        }
    });
    pairing_join_state_impl(state)
}

/// Core: the joiner's pairing state (the UI polls it).
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
pub fn pairing_join_state_impl(state: &Arc<AppState>) -> AppResult<PairingStateInfo> {
    let phase = state
        .link
        .join
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let mut info = PairingStateInfo::idle();
    match &*phase {
        JoinPhase::Idle => {}
        JoinPhase::Connecting => info.phase = String::from("connecting"),
        JoinPhase::Confirm { candidate, .. } => {
            info.phase = String::from("confirm");
            info.peer_label = Some(candidate.label.clone());
            info.peer_fingerprint = Some(candidate.fingerprint.clone());
        }
        JoinPhase::Paired { label } => {
            info.phase = String::from("paired");
            info.peer_label = Some(label.clone());
        }
        JoinPhase::Failed { reason, message } => {
            info.phase = String::from("failed");
            info.reason = Some(reason.clone());
            info.message = Some(message.clone());
        }
    }
    Ok(info)
}

/// Core: this Mac's user confirms or cancels the host shown.
///
/// # Errors
///
/// Returns [`AppError::Validation`] when nothing awaits a decision.
pub fn pairing_join_decide_impl(
    state: &Arc<AppState>,
    commit: bool,
    permission: Permission,
) -> AppResult<PairingStateInfo> {
    respond(&state.link.join, commit, permission, |phase| match phase {
        JoinPhase::Confirm { respond, .. } => respond.take(),
        _ => None,
    })?;
    pairing_join_state_impl(state)
}

/// Core: resets the joiner state (Done / Start over).
pub fn pairing_join_reset_impl(state: &Arc<AppState>) {
    if let Ok(mut phase) = state.link.join.lock() {
        if let JoinPhase::Confirm { respond, .. } = &mut *phase
            && let Some(sender) = respond.take()
        {
            let _ = sender.send(None);
        }
        *phase = JoinPhase::Idle;
    }
}

fn parse_permission(wire: &str) -> AppResult<Permission> {
    match wire {
        "view" => Ok(Permission::View),
        "control" => Ok(Permission::Control),
        _ => Err(AppError::Validation {
            subject: "permission",
            message: "Choose View only or View + control.".to_owned(),
        }),
    }
}

/// AJ-E704: Link status.
#[tauri::command]
#[must_use]
pub fn link_status(state: tauri::State<'_, Arc<AppState>>) -> LinkStatusInfo {
    link_status_impl(state.inner())
}

/// AJ-E704: turns Link on or off.
///
/// # Errors
///
/// Returns the typed [`AppError`] when Link can't start.
#[tauri::command]
pub async fn link_set_enabled(
    state: tauri::State<'_, Arc<AppState>>,
    enabled: bool,
) -> AppResult<LinkStatusInfo> {
    // Starting/stopping blocks briefly (bind, mDNS goodbye packets):
    // never on the main thread or an async worker.
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || link_set_enabled_impl(&state, enabled))
        .await
        .map_err(|e| AppError::Io {
            context: "Fleet link",
            message: e.to_string(),
        })?
}

/// AJ-E704: machines on the network that aren't paired.
#[tauri::command]
#[must_use]
pub fn fleet_discovered(state: tauri::State<'_, Arc<AppState>>) -> Vec<DiscoveredMachineInfo> {
    discovered_impl(state.inner())
}

/// AJ-E704: opens "Add a machine".
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_host_open_impl`].
#[tauri::command]
pub fn pairing_host_open(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_host_open_impl(state.inner())
}

/// AJ-E704: the host's pairing state.
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn pairing_host_state(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_host_state_impl(state.inner())
}

/// AJ-E704: the host user's decision.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_host_decide_impl`].
#[tauri::command]
pub fn pairing_host_decide(
    state: tauri::State<'_, Arc<AppState>>,
    commit: bool,
    permission: String,
) -> AppResult<PairingStateInfo> {
    pairing_host_decide_impl(state.inner(), commit, parse_permission(&permission)?)
}

/// AJ-E704: closes "Add a machine".
#[tauri::command]
pub fn pairing_host_close(state: tauri::State<'_, Arc<AppState>>) {
    pairing_host_close_impl(state.inner());
}

/// AJ-E704: re-enables pairing after the daily disable.
#[tauri::command]
pub fn pairing_re_enable(state: tauri::State<'_, Arc<AppState>>) {
    pairing_re_enable_impl(state.inner());
}

/// AJ-E704: starts joining a host.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_join_start_impl`].
#[tauri::command]
pub fn pairing_join_start(
    state: tauri::State<'_, Arc<AppState>>,
    instance_id: Option<String>,
    address: Option<String>,
    code: String,
) -> AppResult<PairingStateInfo> {
    pairing_join_start_impl(
        state.inner(),
        instance_id.as_deref(),
        address.as_deref(),
        &code,
    )
}

/// AJ-E704: the joiner's pairing state.
///
/// # Errors
///
/// Returns the typed [`AppError`] when state is unavailable.
#[tauri::command]
pub fn pairing_join_state(state: tauri::State<'_, Arc<AppState>>) -> AppResult<PairingStateInfo> {
    pairing_join_state_impl(state.inner())
}

/// AJ-E704: the joiner user's decision.
///
/// # Errors
///
/// Returns the typed [`AppError`] per [`pairing_join_decide_impl`].
#[tauri::command]
pub fn pairing_join_decide(
    state: tauri::State<'_, Arc<AppState>>,
    commit: bool,
    permission: String,
) -> AppResult<PairingStateInfo> {
    pairing_join_decide_impl(state.inner(), commit, parse_permission(&permission)?)
}

/// AJ-E704: resets the joiner state.
#[tauri::command]
pub fn pairing_join_reset(state: tauri::State<'_, Arc<AppState>>) {
    pairing_join_reset_impl(state.inner());
}

/// Test seam: run Link without mDNS (pairing by address only).
#[doc(hidden)]
pub fn link_disable_discovery_for_test(state: &Arc<AppState>) {
    state
        .link
        .no_discovery
        .store(true, std::sync::atomic::Ordering::Relaxed);
}
