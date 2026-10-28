//! The app's Fleet link manager (AJ-E704; ADR-0013 contract).
//!
//! Link is **off by default** and only ever turned on by the user. While
//! on, this Mac:
//!
//! - serves the link (`juggler_link::node`) on TCP 47821 (AI Juggler;
//!   Nightly 47822, Dev 47823 — D-23), or an ephemeral port when that is
//!   taken, TLS 1.3 + pinned keys only;
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

/// AI Juggler's preferred listening port (fixed so a VPN peer can be
/// told it). Each edition has its own (D-23): see [`preferred_port`].
pub const PREFERRED_PORT: u16 = 47_821;

/// The running edition's preferred listening port, also the default port
/// of a typed address (a Fleet joins machines of the same edition).
#[must_use]
pub fn preferred_port() -> u16 {
    crate::edition::current().preferred_link_port()
}
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
    /// Every live connection's tasks, which `stop` ends and waits for.
    watches: Watches,
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
    /// The join in flight (this Mac pairing with another), which `stop`
    /// ends: Link off, nothing pairs after.
    join_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    telemetry: OnceLock<tokio::sync::broadcast::Sender<proto::TelemetryDelta>>,
    last_error: Mutex<Option<String>>,
    /// What paired machines report, as this console last heard it.
    pub(crate) remote: crate::remote::RemoteCache,
    /// The last durable row synced per Instance (AJ-E708), so a sync
    /// pass only touches the outbox for what changed.
    synced: Mutex<HashMap<String, Vec<u8>>>,
    /// Per console: the outbox sequence it holds (AJ-E708), so this Mac
    /// can say how many changes wait for it. Saved by the ticker.
    delivered: Mutex<Option<Delivered>>,
    /// Harness versions installed here (AJ-E706), probed at most once a
    /// minute (`--version` spawns a process).
    pub(crate) versions: VersionCache,
    /// Cancel switches for handovers this console is pushing, by target.
    pub(crate) handover_cancel: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
    /// "Ask each time" commands waiting for this Mac's user.
    prompts: Prompts,
    /// Peers to reconnect at once (a re-pair changed what they grant).
    reconnect: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Test seam: skip mDNS (tests must not advertise on a real LAN).
    no_discovery: std::sync::atomic::AtomicBool,
    /// Held through every start and stop (and a join's start), so two
    /// never interleave: a click during the launch start waits for it,
    /// and the setting always matches what runs.
    lifecycle: Mutex<()>,
    /// The run Link is on for (each start is a new one; 0 when off), for
    /// whatever a peer causes on this Mac: a launch, a command, a pin.
    /// Each effect belongs to the run it came in on and runs holding a
    /// read guard ([`link_open`]) only while that run is the current one:
    /// `stop` closes it first, waiting for the ones under way, and an
    /// effect from before an off-then-on is refused too.
    open: std::sync::RwLock<u64>,
    /// The last run number given out.
    runs: std::sync::atomic::AtomicU64,
    /// Remote commands this console has sent and still waits on; `stop`
    /// ends them ([`run_outbound`]).
    outbound: Mutex<Vec<tokio::task::AbortHandle>>,
    /// Test seam: how long a pairing waits for the other machine's first
    /// Hello before its row says it didn't finish. Never a stored setting,
    /// so nothing in a real install can hide or flash the line.
    pub(crate) unfinished_wait_ms: Mutex<Option<i64>>,
    /// Reconcile loops running (one per run), for tests.
    loops: Arc<std::sync::atomic::AtomicUsize>,
}

/// Counts a reconcile loop for as long as it runs (its future is dropped
/// when the loop ends or is ended).
struct LoopAlive(Arc<std::sync::atomic::AtomicUsize>);

impl LoopAlive {
    fn new(loops: &Arc<std::sync::atomic::AtomicUsize>) -> Self {
        loops.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self(loops.clone())
    }
}

impl Drop for LoopAlive {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// A start's run number: what its node and host may cause belongs to it
/// ([`link_open`]).
fn next_run(manager: &LinkManager) -> u64 {
    manager
        .runs
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 1
}

/// `work`, counted in `loops` while it runs: the count drops with its
/// future, ended or aborted.
fn counted(
    loops: Arc<std::sync::atomic::AtomicUsize>,
    work: impl std::future::Future<Output = ()>,
) -> impl std::future::Future<Output = ()> {
    let alive = LoopAlive::new(&loops);
    async move {
        let _alive = alive;
        work.await;
    }
}

/// How work this console started on another machine ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outbound<T> {
    /// It ran to the end.
    Done(T),
    /// Link was off: it never started, so nothing was sent.
    NotSent,
    /// Link went off while it ran: it may have reached the other machine.
    Ended,
}

/// Runs `work` this console started on another machine on Link's runtime,
/// where Link off (`stop`) ends it.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the runtime is unavailable or the work
/// panicked.
pub(crate) async fn run_outbound<T: Send + 'static>(
    state: &AppState,
    work: impl std::future::Future<Output = T> + Send + 'static,
) -> AppResult<Outbound<T>> {
    // Started and recorded only while Link is on, so `stop` (which closes
    // the gate before it ends these) finds it, or it never goes out.
    let task = {
        let Some(_open) = link_open(state, link_run(state)) else {
            return Ok(Outbound::NotSent);
        };
        let task = state.link.runtime()?.spawn(work);
        if let Ok(mut outbound) = state.link.outbound.lock() {
            outbound.retain(|task| !task.is_finished());
            outbound.push(task.abort_handle());
        }
        task
    };
    match task.await {
        Ok(done) => Ok(Outbound::Done(done)),
        Err(error) if error.is_cancelled() => Ok(Outbound::Ended),
        Err(error) => Err(AppError::Io {
            context: "remote action",
            message: error.to_string(),
        }),
    }
}

/// The run Link is on for now, 0 when off.
pub(crate) fn link_run(state: &AppState) -> u64 {
    state.link.open.read().map_or(0, |run| *run)
}

/// The "Link is on" guard for an effect that came in on `run`, or `None`
/// when Link is off, going off, or was turned off and on since. Held only
/// around the effect itself (never across a wait for this Mac's user or
/// the network).
pub(crate) fn link_open(state: &AppState, run: u64) -> Option<std::sync::RwLockReadGuard<'_, u64>> {
    state
        .link
        .open
        .read()
        .ok()
        .filter(|open| run != 0 && **open == run)
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
            .get_or_init(|| tokio::sync::broadcast::channel(1024).0)
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

/// What a paired machine asks this Mac to do (AJ-E707, AJ-E711): split
/// from this file (the size budget).
mod here;

// ------------------------------------------------------- node handlers

/// Looks a TLS key up in the signed peer registry.
struct AppDirectory(Weak<AppState>);

impl PeerDirectory for AppDirectory {
    fn lookup(&self, key: &[u8; 32]) -> Option<PeerAccess> {
        let state = self.0.upgrade()?;
        let identity = PublicIdentity::from_bytes(key).ok()?;
        crate::fleet::lookup_peer(&state, &identity)
    }

    fn bind_account_peer(&self, key: &[u8; 32], machine_id: &str) -> Option<PeerAccess> {
        let state = self.0.upgrade()?;
        let identity = PublicIdentity::from_bytes(key).ok()?;
        crate::fleet::bind_account_peer(&state, &identity, machine_id)
    }

    fn hello_accepted(&self, _key: &[u8; 32], machine_id: &str) {
        if let Some(state) = self.0.upgrade() {
            crate::fleet::pairing_finished(&state, machine_id);
        }
    }
}

/// What this node serves and executes.
struct AppNode {
    state: Weak<AppState>,
    live: tokio::sync::broadcast::Sender<proto::TelemetryDelta>,
    prompts: Prompts,
    /// The run this node serves ([`link_open`]).
    run: u64,
}

/// How long an "Ask each time" command waits for this Mac's user (well
/// inside the 30 s command expiry).
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(20);

/// What a peer hears when Link was off (or went off) on this Mac before
/// its command or handover could run.
pub(crate) const NOTHING_RAN: &str = "Link is off on that computer — nothing ran.";

/// What this Mac's user hears when Link off kept a command or handover
/// from going out.
pub(crate) const NOTHING_SENT: &str = "Link is off on this computer — nothing was sent.";

/// How a question to this Mac's user ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Allowed,
    /// Denied, or no answer in time.
    Denied,
    /// Link is going off or off: nobody was asked.
    LinkOff,
}

/// A control command from an "Ask each time" machine, waiting for this
/// Mac's user to Allow or Deny it.
#[derive(Debug)]
struct PendingPrompt {
    id: u64,
    peer_label: String,
    action: String,
    expires_at_ms: i64,
    respond: std::sync::mpsc::Sender<bool>,
}

/// The questions waiting, and whether new ones may be asked (not while
/// Link is going off or off: one would hold the off for its 20 s).
#[derive(Debug, Default)]
struct PromptQueue {
    waiting: Vec<PendingPrompt>,
    closed: bool,
}

/// The waiting prompts, and the next prompt id.
#[derive(Debug, Default, Clone)]
struct Prompts {
    pending: Arc<Mutex<PromptQueue>>,
    next: Arc<std::sync::atomic::AtomicU64>,
}

impl Prompts {
    /// Answers every question still waiting "no" and drops them.
    /// Closes the queue and answers every question still waiting "no":
    /// none waits, and none can be asked until it opens again.
    fn close(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.closed = true;
            for prompt in pending.waiting.drain(..) {
                let _ = prompt.respond.send(false);
            }
        }
    }

    /// Lets questions be asked again (Link is on).
    fn open(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.closed = false;
        }
    }

    /// Asks this Mac's user; blocks the command (never the runtime) for
    /// at most [`PROMPT_TIMEOUT`]. Anything but an explicit Allow is a
    /// Deny.
    fn ask(&self, peer_label: &str, action: &str) -> Answer {
        let (respond, decision) = std::sync::mpsc::channel();
        let id = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if let Ok(mut pending) = self.pending.lock() {
            // Link going off or off: refused without asking.
            if pending.closed {
                return Answer::LinkOff;
            }
            pending.waiting.push(PendingPrompt {
                id,
                peer_label: peer_label.to_owned(),
                action: action.to_owned(),
                expires_at_ms: now_ms()
                    + i64::try_from(PROMPT_TIMEOUT.as_millis()).unwrap_or(20_000),
                respond,
            });
        }
        let allowed =
            tokio::task::block_in_place(|| decision.recv_timeout(PROMPT_TIMEOUT)).unwrap_or(false);
        if let Ok(mut pending) = self.pending.lock() {
            pending.waiting.retain(|prompt| prompt.id != id);
        }
        if allowed {
            Answer::Allowed
        } else {
            Answer::Denied
        }
    }
}

/// Live values carry no sequence (AJ-E708: only durable record changes
/// are sequenced, through the outbox).
const UNSEQUENCED: u64 = 0;
/// The app setting holding each console's delivery mark.
const DELIVERED_KEY: &str = "fleet_delivered";

/// The cached harness-version probe: when, and `provider → version`.
pub(crate) type VersionCache = Mutex<Option<(std::time::Instant, Vec<(String, String)>)>>;

/// Per-console delivery marks and whether they changed since saved.
#[derive(Debug, Default)]
struct Delivered {
    marks: HashMap<String, u64>,
    dirty: bool,
}

/// Runs `body` on the delivery marks, loading them on first use.
fn with_delivered<T>(state: &Arc<AppState>, body: impl FnOnce(&mut Delivered) -> T) -> Option<T> {
    let mut cell = state.link.delivered.lock().ok()?;
    if cell.is_none() {
        let marks = state
            .storage
            .lock()
            .ok()
            .and_then(|storage| storage.app_settings().get(DELIVERED_KEY).ok().flatten())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        *cell = Some(Delivered {
            marks,
            dirty: false,
        });
    }
    cell.as_mut().map(body)
}

/// Saves the delivery marks when they changed.
fn save_delivered(state: &Arc<AppState>) {
    let Some(Some(raw)) = with_delivered(state, |delivered| {
        let raw = delivered
            .dirty
            .then(|| serde_json::to_string(&delivered.marks).ok())
            .flatten();
        delivered.dirty = false;
        raw
    }) else {
        return;
    };
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(DELIVERED_KEY, &raw);
    }
}

/// How many durable changes wait for `console` on this Mac (what it
/// will receive on reconnect).
pub(crate) fn queued_for(state: &Arc<AppState>, console: &str) -> Option<u64> {
    let mark =
        with_delivered(state, |delivered| delivered.marks.get(console).copied())?.unwrap_or(0);
    let storage = state.storage.lock().ok()?;
    storage.fleet_sync().outbox_count_since(mark).ok()
}

/// The app setting holding the port this node last listened on.
const LAST_PORT_KEY: &str = "fleet_link_last_port";
/// The app setting holding this node's sync epoch.
const SYNC_EPOCH_KEY: &str = "fleet_sync_epoch";

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

/// The durable part of an Instance row — what changes rarely (status,
/// model, project, …) and is synced through the outbox. Only what the link
/// contract allows: ids, Provider, Account label, status, model and the
/// project folder's name (never a path). Live numbers (CPU, memory,
/// processes) travel separately, unsequenced.
fn durable_delta(row: &crate::commands::InstanceInfo) -> proto::SessionDelta {
    let project_name = std::path::Path::new(&row.cwd)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty());
    proto::SessionDelta {
        instance_id: row.id.clone(),
        provider_id: row.provider.clone(),
        account_label: row.account_label.clone(),
        status: row.status.clone(),
        updated_at_ms: row.last_activity_ms.unwrap_or(row.started_at_ms),
        model: row.model.clone(),
        model_source: row.model_source.clone(),
        project_name,
        started_at_ms: row.started_at_ms,
        last_activity_ms: row.last_activity_ms,
        cpu_percent: None,
        rss_bytes: None,
        process_count: None,
        kind: row.kind.clone(),
    }
}

/// An Instance's live numbers only (unsequenced).
fn live_delta(row: &crate::commands::InstanceInfo) -> Option<proto::SessionDelta> {
    (row.cpu_pct.is_some() || row.rss_bytes.is_some()).then(|| proto::SessionDelta {
        instance_id: row.id.clone(),
        cpu_percent: row.cpu_pct,
        rss_bytes: row.rss_bytes,
        process_count: row.child_count.map(|children| children + 1),
        ..proto::SessionDelta::default()
    })
}

/// This node's sync epoch: generated once, kept with its database (a
/// reset machine gets a new one, so consoles never trust old sequences).
fn sync_epoch(state: &Arc<AppState>) -> String {
    let Ok(storage) = state.storage.lock() else {
        return String::new();
    };
    if let Ok(Some(epoch)) = storage.app_settings().get(SYNC_EPOCH_KEY)
        && !epoch.is_empty()
    {
        return epoch;
    }
    let mut bytes = [0_u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        return String::new();
    }
    let epoch = bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    });
    let _ = storage.app_settings().set(SYNC_EPOCH_KEY, &epoch);
    epoch
}

/// One sync pass: every Instance whose durable state changed is appended
/// to the outbox (and returned, sequenced); live numbers are returned
/// unsequenced.
fn sync_instances(
    state: &Arc<AppState>,
) -> (Vec<proto::TelemetryDelta>, Vec<proto::TelemetryDelta>) {
    use prost::Message as _;
    let origin = crate::commands::local_machine_id(state).to_string();
    let Ok(rows) = crate::commands::instances_shared_snapshot(state) else {
        return (Vec::new(), Vec::new());
    };
    let mut changed = Vec::new();
    let mut live = Vec::new();
    let mut synced = state
        .link
        .synced
        .lock()
        .map(|map| map.clone())
        .unwrap_or_default();
    for row in &rows {
        let durable = durable_delta(row);
        let bytes = durable.encode_to_vec();
        if synced.get(&row.id) != Some(&bytes) {
            if let Some(seq) = record_change(state, &row.id, &bytes) {
                changed.push(delta(
                    &origin,
                    seq,
                    proto::telemetry_delta::Payload::Session(durable),
                ));
            }
            synced.insert(row.id.clone(), bytes);
        }
        if let Some(numbers) = live_delta(row) {
            live.push(delta(
                &origin,
                UNSEQUENCED,
                proto::telemetry_delta::Payload::Session(numbers),
            ));
        }
    }
    if let Ok(mut map) = state.link.synced.lock() {
        *map = synced;
    }
    (changed, live)
}

/// Appends an Instance's durable row to the outbox unless it is already
/// the latest there (a restart re-reads the outbox, never re-sends);
/// returns the new sequence.
fn record_change(state: &Arc<AppState>, id: &str, bytes: &[u8]) -> Option<u64> {
    let storage = state.storage.lock().ok()?;
    let sync = storage.fleet_sync();
    let same = sync
        .outbox_latest("instance", id)
        .ok()
        .flatten()
        .is_some_and(|latest| latest.payload == bytes);
    if same {
        return None;
    }
    sync.outbox_append("instance", id, bytes, now_ms()).ok()
}

/// The outbox after `since` as deltas — a reconnecting console's
/// catch-up: only what it doesn't have yet.
fn outbox_deltas(state: &Arc<AppState>, since: u64) -> Vec<proto::TelemetryDelta> {
    use prost::Message as _;
    let origin = crate::commands::local_machine_id(state).to_string();
    let entries = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.fleet_sync().outbox_since(since, 10_000).ok())
        .unwrap_or_default();
    entries
        .into_iter()
        .filter_map(|entry| {
            proto::SessionDelta::decode(entry.payload.as_slice())
                .ok()
                .map(|session| {
                    delta(
                        &origin,
                        entry.seq,
                        proto::telemetry_delta::Payload::Session(session),
                    )
                })
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

    fn sync_epoch(&self) -> String {
        self.state
            .upgrade()
            .map(|state| sync_epoch(&state))
            .unwrap_or_default()
    }

    fn caught_up_from(&self, peer: &str, since: u64) {
        if let Some(state) = self.state.upgrade() {
            with_delivered(&state, |delivered| {
                delivered.marks.insert(peer.to_owned(), since);
                delivered.dirty = true;
            });
        }
    }

    fn delivered(&self, peer: &str, sequence: u64) {
        if let Some(state) = self.state.upgrade() {
            with_delivered(&state, |delivered| {
                let mark = delivered.marks.entry(peer.to_owned()).or_default();
                if sequence > *mark {
                    *mark = sequence;
                    delivered.dirty = true;
                }
            });
        }
    }

    fn telemetry_since(&self, since: u64) -> Vec<proto::TelemetryDelta> {
        // AJ-E708: bring the outbox up to date, then send only what the
        // console doesn't have, plus the current live values.
        let Some(state) = self.state.upgrade() else {
            return Vec::new();
        };
        let (_, live) = sync_instances(&state);
        let origin = crate::commands::local_machine_id(&state).to_string();
        let mut backlog = outbox_deltas(&state, since);
        backlog.extend(live);
        backlog.push(delta(
            &origin,
            UNSEQUENCED,
            proto::telemetry_delta::Payload::Cleanup(here::cleanup_delta(&state)),
        ));
        // AJ-E706: what this Mac can take work into.
        backlog.push(delta(
            &origin,
            UNSEQUENCED,
            proto::telemetry_delta::Payload::Catalog(crate::crossmachine::catalog(&state)),
        ));
        backlog
    }

    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta> {
        self.live.subscribe()
    }

    fn execute(
        &self,
        peer: &PeerAccess,
        action: &proto::control_command::Action,
    ) -> Result<juggler_link::node::CommandDone, CommandRejection> {
        let reject = |code: &str, message: &str| CommandRejection {
            reason_code: code.to_owned(),
            message: message.to_owned(),
        };
        let state = self
            .state
            .upgrade()
            .ok_or_else(|| reject("unavailable", "This computer is shutting down."))?;
        if crate::fleet::peer_asks_each_time(&state, &peer.machine_id) {
            // "Ask each time": this Mac's user decides, on this Mac.
            let label = crate::fleet::peer_label(&state, &peer.machine_id)
                .unwrap_or_else(|| "A paired machine".to_owned());
            let what = here::describe_action(&state, action);
            match self.prompts.ask(&label, &what) {
                Answer::Allowed => {}
                Answer::Denied => {
                    return Err(reject(
                        "not_confirmed",
                        "Not allowed on that computer — nothing ran.",
                    ));
                }
                Answer::LinkOff => return Err(reject("link_off", NOTHING_RAN)),
            }
        }
        // Held through the action, never through the question above.
        let _open = link_open(&state, self.run).ok_or_else(|| reject("link_off", NOTHING_RAN))?;
        match action {
            proto::control_command::Action::StopInstance(stop) => {
                let known = state
                    .storage
                    .lock()
                    .ok()
                    .and_then(|storage| crate::commands::session_infos_snapshot(&storage).ok())
                    .is_some_and(|rows| rows.iter().any(|row| row.id == stop.instance_id));
                if !known {
                    return Err(reject(
                        "unknown_record",
                        "No such session on this computer.",
                    ));
                }
                state
                    .supervisor
                    .stop(
                        &juggler_core::model::SessionId::new(stop.instance_id.clone()),
                        Duration::from_secs(5),
                    )
                    .map_err(|e| reject("stop_failed", &e.to_string()))?;
                Ok("Stopping".to_owned().into())
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
                        .ok_or_else(|| {
                            reject("unknown_record", "No such project on this computer.")
                        })?;
                    Some(project.root)
                };
                let launched = crate::commands::launch_profile(&state, &launch.recipe_id, cwd)
                    .map_err(|e| match e {
                        AppError::NotFound { .. } => reject(
                            "unknown_record",
                            "No such Launch settings on this computer.",
                        ),
                        other => reject("launch_failed", &other.wire().user_message),
                    })?;
                // AJ-E1515: the console follows the new session by its id.
                Ok(juggler_link::node::CommandDone {
                    message: "Started a session".to_owned(),
                    started_instance_id: Some(launched.session_id),
                })
            }
            proto::control_command::Action::FocusInstance(focus) => {
                here::focus_here(&state, &focus.instance_id).map(Into::into)
            }
            proto::control_command::Action::CleanProcess(clean) => {
                here::clean_here(&state, &peer.machine_id, &clean.instance_id, clean.pid)
                    .map(Into::into)
            }
            // The link only delivers these through Handover.
            proto::control_command::Action::TakeHandover(_)
            | proto::control_command::Action::SwitchBack(_) => Err(reject(
                "malformed",
                "Handovers go through the Handover call.",
            )),
        }
    }

    fn take_handover(
        &self,
        peer: &PeerAccess,
        action: &proto::control_command::Action,
        bundle: &[u8],
    ) -> Result<juggler_link::node::HandoverAccepted, juggler_link::node::HandoverRejection> {
        let refuse = |code: &str, message: &str| juggler_link::node::HandoverRejection {
            reason_code: code.to_owned(),
            message: message.to_owned(),
            divergence: None,
        };
        let state = self
            .state
            .upgrade()
            .ok_or_else(|| refuse("unavailable", "This computer is shutting down."))?;
        if crate::fleet::peer_asks_each_time(&state, &peer.machine_id) {
            let label = crate::fleet::peer_label(&state, &peer.machine_id)
                .unwrap_or_else(|| "A paired machine".to_owned());
            let what = here::describe_action(&state, action);
            match self.prompts.ask(&label, &what) {
                Answer::Allowed => {}
                Answer::Denied => {
                    return Err(refuse(
                        "not_confirmed",
                        "Not allowed on that computer — nothing ran.",
                    ));
                }
                Answer::LinkOff => return Err(refuse("link_off", NOTHING_RAN)),
            }
        }
        // Held through the take (Link going off waits for it), never
        // through the question above.
        let _open = link_open(&state, self.run).ok_or_else(|| refuse("link_off", NOTHING_RAN))?;
        crate::crossmachine::take_handover_here(&state, peer, action, bundle)
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
    /// The run this host serves ([`link_open`]).
    run: u64,
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
        crate::usage_stats::fleet_pairing(Some("mismatch"));
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

    fn succeeded(&self, candidate: &PairCandidate, permission: Permission) -> bool {
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        // Only while this run's Link is on: the pin is the effect. Refused,
        // the joiner is told and pins nothing either.
        let Some(_open) = link_open(&state, self.run) else {
            if let Ok(mut phase) = self.phase.lock() {
                *phase = HostPhase::Idle;
            }
            return false;
        };
        let _ = self.with_guard(PairingGuard::succeed);
        let _ = crate::fleet::pin_candidate(&state, candidate, permission);
        // Until its first Hello here: the joiner may have stopped before it
        // pinned this Mac.
        crate::fleet::mark_pairing_unfinished(&state, &candidate.machine_id, now_ms());
        SettingsReplayStore(self.state.clone()).reset(&candidate.machine_id);
        mark_reconnect(&state, &candidate.machine_id);
        if let Some(reach) = candidate.reach {
            remember_address(&state, &candidate.machine_id, reach);
        }
        if let Ok(mut phase) = self.phase.lock() {
            *phase = HostPhase::Paired {
                label: candidate.label.clone(),
            };
        }
        crate::usage_stats::fleet_pairing(None);
        true
    }
}

/// Pairing: this Mac hosting a pairing code, and joining another Mac's
/// (split from this file: the size budget).
pub mod pairing;
pub use pairing::{
    pairing_host_close_impl, pairing_host_decide_impl, pairing_host_open_impl,
    pairing_host_state_impl, pairing_join_decide_impl, pairing_join_reset_impl,
    pairing_join_start_impl, pairing_join_state_impl, pairing_re_enable_impl,
};

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
    let lifecycle = state
        .link
        .lifecycle
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
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
    drop(lifecycle);
    Ok(link_status_impl(state))
}

/// Starts Link at launch when the user left it on (off the main thread).
pub fn resume_if_enabled(state: Arc<AppState>) {
    if link_settings_enabled(&state) {
        let _ = std::thread::Builder::new()
            .name("aij-link-resume".to_owned())
            .spawn(move || {
                let Ok(_lifecycle) = state.link.lifecycle.lock() else {
                    return;
                };
                // Turned off while this waited: stays off.
                if !link_settings_enabled(&state) {
                    return;
                }
                if let Err(error) = start(&state)
                    && let Ok(mut last) = state.link.last_error.lock()
                {
                    *last = Some(error.wire().user_message);
                }
            });
    }
}

/// Opens the Fleet listener: the port this Mac used last, else the
/// edition's preferred port, else any free one.
async fn bind_listener(last_port: Option<u16>) -> std::io::Result<tokio::net::TcpListener> {
    // Retried briefly: right after Link off the old listener may not be
    // released yet, and falling back to another port would strand peers
    // paired by address.
    if let Some(port) = last_port.filter(|port| *port != 0) {
        for _ in 0..10 {
            if let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
                return Ok(listener);
            }
            // poll: re-binding the last port after Link off; 10 tries, 2 s at most
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    if let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", preferred_port())).await {
        return Ok(listener);
    }
    tokio::net::TcpListener::bind(("0.0.0.0", 0)).await
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
    // The port this Mac used last (peers paired by address — no
    // discovery — still find it after a restart), else the preferred
    // one, else any free one.
    let last_port: Option<u16> = state.storage.lock().ok().and_then(|storage| {
        storage
            .app_settings()
            .get(LAST_PORT_KEY)
            .ok()
            .flatten()
            .and_then(|raw| raw.parse().ok())
    });
    let listener = runtime
        .block_on(bind_listener(last_port))
        .map_err(|e| AppError::io("open the Fleet link port", &e))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::io("Fleet link port", &e))?
        .port();
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(LAST_PORT_KEY, &port.to_string());
    }
    let run = next_run(manager);
    let pairing = Arc::new(AppPairing {
        state: weak.clone(),
        guard: manager.guard.clone(),
        phase: manager.host.clone(),
        message: manager.host_message.clone(),
        key,
        run,
    });
    let node = {
        let _enter = runtime.enter();
        LinkNode::start(
            listener,
            &certificate,
            Arc::new(AppNode {
                state: weak.clone(),
                live: live.clone(),
                prompts: manager.prompts.clone(),
                run,
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
    let watches = Watches::default();
    let reconcile = runtime.spawn(counted(
        manager.loops.clone(),
        reconcile_loop(
            weak.clone(),
            browser.clone(),
            certificate,
            manager.peers.clone(),
            manager.remote.clone(),
            manager.reconnect.clone(),
            watches.clone(),
        ),
    ));
    let ticker = runtime.spawn(resource_ticker(weak, live));
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
        watches,
    });
    manager.prompts.open();
    if let Ok(mut open) = manager.open.write() {
        *open = run;
    }
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
    // Closed first: an effect a peer caused that is under way finishes
    // (taking the write side waits for it), and none starts after.
    if let Ok(mut open) = manager.open.write() {
        *open = 0;
    }
    // A question a peer's command is waiting on is answered "no" now:
    // the command ends at once (it would wait up to 20 s), and an answer
    // after Link is on again can't let it run.
    manager.prompts.close();
    // What this console started on other machines: a command it still
    // waits on ends, and a handover it is pushing is cancelled.
    let outbound: Vec<_> = manager
        .outbound
        .lock()
        .map(|mut tasks| tasks.drain(..).collect())
        .unwrap_or_default();
    for task in &outbound {
        task.abort();
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while outbound.iter().any(|task| !task.is_finished()) && std::time::Instant::now() < deadline {
        // poll: ended commands this console sent finishing after their
        // abort (an AbortHandle can't be awaited); 1 s at most.
        std::thread::sleep(Duration::from_millis(5));
    }
    if let Ok(cancels) = manager.handover_cancel.lock() {
        for cancel in cancels.values() {
            let _ = cancel.send(true);
        }
    }
    let running = manager
        .running
        .lock()
        .ok()
        .and_then(|mut running| running.take());
    if let Some(running) = running {
        // Off means off: every task Link started ends before this returns,
        // so nothing from a peer is applied after (a dropped JoinHandle
        // only detaches its task, and an abort lands at the next await).
        if let Ok(runtime) = manager.runtime() {
            runtime.block_on(end_tasks(
                running.reconcile,
                running.ticker,
                &running.watches,
            ));
        }
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
    // A join in flight ends too, and is waited for: its writes (the pin,
    // the address) hold no await, so they either finished before this or
    // never happen.
    let join = manager
        .join_task
        .lock()
        .ok()
        .and_then(|mut task| task.take());
    if let Some(join) = join {
        join.abort();
        if let Ok(runtime) = manager.runtime() {
            let _ = runtime.block_on(join);
        }
    }
    if let Ok(mut phase) = manager.join.lock()
        && matches!(*phase, JoinPhase::Connecting | JoinPhase::Confirm { .. })
    {
        *phase = JoinPhase::Idle;
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

/// The live connections by machine id, shared by the reconcile loop (which
/// holds it through a pass) and `stop` (which ends them).
type Watches = Arc<tokio::sync::Mutex<HashMap<String, Watch>>>;

/// Ends the reconcile loop and the ticker, then every connection's
/// heartbeat and telemetry task, waiting for each: a task in the middle of
/// applying a change finishes it, and none starts another.
async fn end_tasks(
    reconcile: tokio::task::JoinHandle<()>,
    ticker: tokio::task::JoinHandle<()>,
    watches: &Watches,
) {
    reconcile.abort();
    ticker.abort();
    // The loop holds the connections through a pass: once it has ended,
    // no new one can be added.
    let _ = reconcile.await;
    let _ = ticker.await;
    let mut live = watches.lock().await;
    for (_, (_, heartbeat, telemetry)) in live.drain() {
        heartbeat.abort();
        telemetry.abort();
        let _ = heartbeat.await;
        let _ = telemetry.await;
    }
}

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
    watches: Watches,
) {
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
        let mut live = watches.lock().await;
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
                tend_live(&state, &remote, &mut live, &machine_id, &own, tick).await;
                continue;
            }
            if let Some((_, heartbeat, telemetry)) = live.remove(&machine_id) {
                heartbeat.abort();
                telemetry.abort();
                crate::remote::disconnected(&remote, &machine_id);
                crate::usage_stats::count("fleet.link_degraded", &[("reason", "network")]);
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
                    let machine_id = bound_machine_id(&state, machine_id, &key, &peer);
                    let watch =
                        adopt_connection(&state, &peers, &remote, &machine_id, address, &peer);
                    live.insert(machine_id.clone(), watch);
                }
                Err(detail) => set_peer(&peers, &machine_id, "offline", None, Some(detail), None),
            }
        }
        drop(live);
        tokio::time::sleep(RECONCILE_EVERY).await;
    }
}

/// Upkeep for a live connection: every ~30 s re-check what that machine
/// grants; a telemetry stream that ended (lagged, or access came back)
/// resumes from the replica's high-water mark.
async fn tend_live(
    state: &Weak<AppState>,
    remote: &crate::remote::RemoteCache,
    live: &mut HashMap<String, Watch>,
    machine_id: &str,
    own: &str,
    tick: u64,
) {
    if tick.is_multiple_of(10) {
        crate::remote::refresh_permission(remote, machine_id, own).await;
    }
    if let Some((_, _, telemetry)) = live.get_mut(machine_id)
        && telemetry.is_finished()
        && let Some(resumed) = crate::remote::resubscribe(state, remote, machine_id)
    {
        *telemetry = resumed;
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
    if degraded {
        crate::usage_stats::count("fleet.link_degraded", &[("reason", "version")]);
    }
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
    let telemetry = crate::remote::subscribe(state, remote, machine_id, peer);
    if let Some(app) = state.upgrade() {
        crate::fleet::touch_peer(&app, machine_id, now_ms());
        remember_address(&app, machine_id, address);
    }
    (watch, heartbeat, telemetry)
}

/// AJ-E1206: an account-discovered node tells us its real id once (its
/// Hello); mutual TLS already proved it holds the pinned key.
fn bound_machine_id(
    state: &Weak<AppState>,
    machine_id: String,
    key: &[u8; 32],
    peer: &juggler_link::client::LinkPeer,
) -> String {
    if !machine_id.starts_with(juggler_link::PROVISIONAL_PREFIX) {
        return machine_id;
    }
    state
        .upgrade()
        .and_then(|app| {
            let identity = PublicIdentity::from_bytes(key).ok()?;
            crate::fleet::bind_account_peer(&app, &identity, &peer.hello.machine_id)
        })
        .map_or(machine_id, |bound| bound.machine_id)
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

/// AJ-E1206: a newly pinned peer — try it on the next pass.
pub(crate) fn reconnect_soon(state: &Arc<AppState>, machine_id: &str) {
    mark_reconnect(state, machine_id);
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

/// Routes alerts this Mac's alert engine emitted to its paired consoles
/// (nothing is sent while Link is off — no subscriber exists).
pub fn publish_alerts(state: &Arc<AppState>, events: &[crate::commands::AlertEventInfo]) {
    if events.is_empty() || !link_status_impl(state).enabled {
        return;
    }
    let origin = crate::commands::local_machine_id(state).to_string();
    let live = state.link.telemetry();
    for event in events {
        let _ = live.send(delta(
            &origin,
            UNSEQUENCED,
            proto::telemetry_delta::Payload::Alert(proto::AlertDelta {
                kind: event.kind.clone(),
                account_label: event.account.clone(),
                text: event.text.clone(),
                at_ms: event.at_ms,
            }),
        ));
    }
}

/// Publishes this Mac's resource numbers and Instance rows every 2 s
/// while Link is on.
async fn resource_ticker(
    state: Weak<AppState>,
    live: tokio::sync::broadcast::Sender<proto::TelemetryDelta>,
) {
    let mut system = sysinfo::System::new();
    let mut tick = 0_u64;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        tick = tick.wrapping_add(1);
        let Some(app) = state.upgrade() else {
            return;
        };
        let origin = crate::commands::local_machine_id(&app).to_string();
        let (changed, live_numbers) = sync_instances(&app);
        save_delivered(&app);
        // Processes every ~6 s: orphans reach consoles within 10 s.
        let cleanup = tick.is_multiple_of(3).then(|| here::cleanup_delta(&app));
        // The catalog every ~30 s (Launch settings, versions, projects).
        let catalog = tick
            .is_multiple_of(15)
            .then(|| crate::crossmachine::catalog(&app));
        drop(app);
        if let Some(catalog) = catalog {
            let _ = live.send(delta(
                &origin,
                UNSEQUENCED,
                proto::telemetry_delta::Payload::Catalog(catalog),
            ));
        }
        if let Some(cleanup) = cleanup {
            let _ = live.send(delta(
                &origin,
                UNSEQUENCED,
                proto::telemetry_delta::Payload::Cleanup(cleanup),
            ));
        }
        system.refresh_cpu_usage();
        system.refresh_memory();
        let _ = live.send(delta(
            &origin,
            UNSEQUENCED,
            proto::telemetry_delta::Payload::Resources(proto::MachineResources {
                cpu_percent: f64::from(system.global_cpu_usage()),
                memory_used_bytes: system.used_memory(),
                memory_total_bytes: system.total_memory(),
            }),
        ));
        for session in changed.into_iter().chain(live_numbers) {
            let _ = live.send(session);
        }
    }
}

/// Test seam: one sync pass now (instead of waiting for the 2 s tick);
/// returns how many durable changes it appended to the outbox.
#[doc(hidden)]
pub fn link_sync_now_for_test(state: &Arc<AppState>) -> usize {
    sync_instances(state).0.len()
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

/// AJ-E704: Link status.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn link_status(state: tauri::State<'_, Arc<AppState>>) -> AppResult<LinkStatusInfo> {
    crate::ipc::blocking(state.inner(), "link status", move |state| {
        Ok(link_status_impl(state))
    })
    .await
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
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn fleet_discovered(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<DiscoveredMachineInfo>> {
    crate::ipc::blocking(state.inner(), "fleet discovered", move |state| {
        Ok(discovered_impl(state))
    })
    .await
}

/// Test seam: run Link without mDNS (pairing by address only).
#[doc(hidden)]
pub fn link_disable_discovery_for_test(state: &Arc<AppState>) {
    state
        .link
        .no_discovery
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Test seam: how many Link runs are going (their reconcile loops).
#[doc(hidden)]
#[must_use]
pub fn link_runs_for_test(state: &Arc<AppState>) -> usize {
    state.link.loops.load(std::sync::atomic::Ordering::SeqCst)
}

/// Test seam: how long a pairing waits for the other machine's first
/// Hello before its row says it may not have finished (30 s otherwise).
#[doc(hidden)]
pub fn pairing_wait_for_test(state: &Arc<AppState>, wait_ms: i64) {
    if let Ok(mut wait) = state.link.unfinished_wait_ms.lock() {
        *wait = Some(wait_ms);
    }
}

/// A command waiting for this Mac's user.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct FleetPromptInfo {
    /// The prompt id.
    #[ts(type = "number")]
    pub id: u64,
    /// Which machine asks.
    pub peer_label: String,
    /// What it asks for, in plain words ("stop Work Max (claude-code)").
    pub action: String,
    /// When it is denied by default, epoch ms.
    #[ts(type = "number")]
    pub expires_at_ms: i64,
}

/// Core: the waiting prompts (oldest first).
#[must_use]
pub fn prompts_impl(state: &Arc<AppState>) -> Vec<FleetPromptInfo> {
    state
        .link
        .prompts
        .pending
        .lock()
        .map(|pending| {
            pending
                .waiting
                .iter()
                .map(|prompt| FleetPromptInfo {
                    id: prompt.id,
                    peer_label: prompt.peer_label.clone(),
                    action: prompt.action.clone(),
                    expires_at_ms: prompt.expires_at_ms,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Core: this Mac's user allows or denies a waiting command.
pub fn prompt_decide_impl(state: &Arc<AppState>, id: u64, allow: bool) {
    if let Ok(pending) = state.link.prompts.pending.lock()
        && let Some(prompt) = pending.waiting.iter().find(|prompt| prompt.id == id)
    {
        let _ = prompt.respond.send(allow);
    }
}

/// AJ-E709: control commands waiting for this Mac's user.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn fleet_prompts(
    state: tauri::State<'_, Arc<AppState>>,
) -> AppResult<Vec<FleetPromptInfo>> {
    crate::ipc::blocking(state.inner(), "fleet prompts", move |state| {
        Ok(prompts_impl(state))
    })
    .await
}

/// AJ-E709: Allow or Deny a waiting command.
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn fleet_prompt_decide(
    state: tauri::State<'_, Arc<AppState>>,
    id: u64,
    allow: bool,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "fleet prompt decide", move |state| {
        prompt_decide_impl(state, id, allow);
        Ok(())
    })
    .await
}

/// Test seam: re-asks every connected peer what it grants this Mac now
/// (production does this every ~30 s).
#[doc(hidden)]
pub fn refresh_permissions_for_test(state: &Arc<AppState>) {
    let own = crate::commands::local_machine_id(state).to_string();
    let ids: Vec<String> = state
        .link
        .remote
        .lock()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    if let Ok(runtime) = state.link.runtime() {
        runtime.block_on(async {
            for id in ids {
                crate::remote::refresh_permission(&state.link.remote, &id, &own).await;
            }
        });
    }
}

#[cfg(test)]
mod unfinished_pairing_tests {
    use super::*;

    /// A pairing this Mac finished as host: unfinished until the other
    /// machine's first Hello here (it may have stopped before it pinned
    /// this Mac), shown only once that is longer than a pairing takes.
    #[test]
    fn a_hosts_pairing_is_unfinished_until_the_other_machines_first_hello() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(
                dir.path().join("app-data"),
                juggler_adapter::AdapterRegistry::new(),
            )
            .unwrap(),
        );
        if let Ok(mut open) = state.link.open.write() {
            *open = 1;
        }
        let joiner = juggler_link::MachineIdentity::load_or_create(&dir.path().join("joiner"))
            .unwrap()
            .public()
            .to_bytes();
        let host = AppPairing {
            state: Arc::downgrade(&state),
            guard: state.link.guard.clone(),
            phase: state.link.host.clone(),
            message: state.link.host_message.clone(),
            key: [0; 32],
            run: 1,
        };
        host.succeeded(
            &PairCandidate {
                machine_id: "m-joiner".to_owned(),
                label: "Laptop".to_owned(),
                key: joiner,
                reach: None,
                fingerprint: juggler_link::fingerprint_of(&joiner),
            },
            Permission::View,
        );
        let unfinished = || {
            crate::fleet::fleet_peers_impl(&state)
                .unwrap()
                .into_iter()
                .find(|peer| peer.machine_id == "m-joiner")
                .map(|peer| peer.pairing_unfinished)
        };
        assert_eq!(unfinished(), Some(false), "not within a pairing's time");
        pairing_wait_for_test(&state, 0);
        assert_eq!(unfinished(), Some(true), "no Hello from it since");
        AppDirectory(Arc::downgrade(&state)).hello_accepted(&joiner, "m-joiner");
        assert_eq!(unfinished(), Some(false), "its first Hello finishes it");
    }

    /// The note never touches the signed peer list: while a peer is
    /// unfinished, `peers.json` holds only what older builds read (they
    /// refuse an unknown field), noting and finishing never rewrite it,
    /// and a restart keeps both the peer and the note.
    #[test]
    fn an_unfinished_pairing_leaves_the_peer_list_as_older_builds_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("app-data");
        let open = |data: &std::path::Path| {
            let state = Arc::new(
                AppState::init_with_registry(
                    data.to_path_buf(),
                    juggler_adapter::AdapterRegistry::new(),
                )
                .unwrap(),
            );
            if let Ok(mut open) = state.link.open.write() {
                *open = 1;
            }
            pairing_wait_for_test(&state, 0);
            state
        };
        let state = open(&data);
        let joiner = juggler_link::MachineIdentity::load_or_create(&dir.path().join("joiner"))
            .unwrap()
            .public()
            .to_bytes();
        let candidate = PairCandidate {
            machine_id: "m-joiner".to_owned(),
            label: "Laptop".to_owned(),
            key: joiner,
            reach: None,
            fingerprint: juggler_link::fingerprint_of(&joiner),
        };
        let _ = crate::fleet::pin_candidate(&state, &candidate, Permission::View);
        let peers = data.join("fleet/peers.json");
        let pinned = std::fs::read(&peers).unwrap();

        crate::fleet::mark_pairing_unfinished(&state, "m-joiner", 1);
        assert_eq!(
            std::fs::read(&peers).unwrap(),
            pinned,
            "noting never rewrites it"
        );
        let file: serde_json::Value = serde_json::from_slice(&pinned).unwrap();
        let older = [
            "machine_id",
            "label",
            "public_key",
            "fingerprint",
            "permission",
            "ask_each_time",
            "paired_via",
            "paired_at_ms",
            "last_seen_ms",
        ];
        for peer in file["body"]["peers"].as_array().unwrap() {
            for key in peer.as_object().unwrap().keys() {
                assert!(
                    older.contains(&key.as_str()),
                    "a field older builds refuse: {key}"
                );
            }
        }

        drop(state);
        let restarted = open(&data);
        let row = crate::fleet::fleet_peers_impl(&restarted)
            .unwrap()
            .into_iter()
            .find(|peer| peer.machine_id == "m-joiner")
            .expect("the peer list still loads (trust intact)");
        assert!(row.pairing_unfinished, "a restart keeps the note");
        AppDirectory(Arc::downgrade(&restarted)).hello_accepted(&joiner, "m-joiner");
        assert_eq!(
            std::fs::read(&peers).unwrap(),
            pinned,
            "finishing never rewrites it"
        );
    }

    /// A peer set to No access before its first Hello can't say one: the
    /// note goes with the change.
    #[test]
    fn no_access_clears_an_unfinished_pairing() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(
                dir.path().join("app-data"),
                juggler_adapter::AdapterRegistry::new(),
            )
            .unwrap(),
        );
        pairing_wait_for_test(&state, 0);
        let joiner = juggler_link::MachineIdentity::load_or_create(&dir.path().join("joiner"))
            .unwrap()
            .public()
            .to_bytes();
        let candidate = PairCandidate {
            machine_id: "m-joiner".to_owned(),
            label: "Laptop".to_owned(),
            key: joiner,
            reach: None,
            fingerprint: juggler_link::fingerprint_of(&joiner),
        };
        let _ = crate::fleet::pin_candidate(&state, &candidate, Permission::View);
        crate::fleet::mark_pairing_unfinished(&state, "m-joiner", 1);
        let rows = crate::fleet::fleet_set_permission_impl(&state, "m-joiner", "no_access", false)
            .unwrap();
        assert!(rows.iter().all(|row| !row.pairing_unfinished));
    }

    /// Work this console starts on another machine goes out only while
    /// Link is on, and an effect from an earlier run is refused after an
    /// off-then-on.
    #[test]
    fn nothing_starts_or_lands_outside_its_run() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(
                dir.path().join("app-data"),
                juggler_adapter::AdapterRegistry::new(),
            )
            .unwrap(),
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let sent = runtime.block_on(run_outbound(&state, async { 1 })).unwrap();
        assert_eq!(sent, Outbound::NotSent, "Link off: nothing goes out");
        if let Ok(mut open) = state.link.open.write() {
            *open = 2;
        }
        assert_eq!(
            runtime.block_on(run_outbound(&state, async { 1 })).unwrap(),
            Outbound::Done(1)
        );
        assert!(link_open(&state, 1).is_none(), "an earlier run's effect");
        assert!(link_open(&state, 2).is_some());
        assert!(link_open(&state, 0).is_none());
    }

    /// Once Link is going off, a question a peer's command would ask is
    /// answered "no" without waiting (one asked between the drain and the
    /// node's end would hold the off for its 20 s); once on again, it is
    /// asked.
    #[test]
    fn a_closed_queue_answers_no_without_asking() {
        let prompts = Prompts::default();
        prompts.close();
        let started = std::time::Instant::now();
        assert_eq!(prompts.ask("Laptop", "stop a session"), Answer::LinkOff);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(prompts.pending.lock().unwrap().waiting.is_empty());

        prompts.open();
        let asking = {
            let prompts = prompts.clone();
            std::thread::spawn(move || prompts.ask("Laptop", "stop a session"))
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let waiting = prompts
                .pending
                .lock()
                .unwrap()
                .waiting
                .first()
                .map(|prompt| prompt.respond.clone());
            if let Some(respond) = waiting {
                respond.send(true).unwrap();
                break;
            }
            assert!(std::time::Instant::now() < deadline, "never asked");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            asking.join().unwrap(),
            Answer::Allowed,
            "asked again once on"
        );
    }

    /// A peer's "Ask each time" command arriving while Link goes off is
    /// refused as Link off ("nothing ran"), not as a "no" nobody said.
    #[test]
    fn a_question_refused_by_link_off_says_link_off() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(
            AppState::init_with_registry(
                dir.path().join("app-data"),
                juggler_adapter::AdapterRegistry::new(),
            )
            .unwrap(),
        );
        let peer_key = juggler_link::MachineIdentity::load_or_create(&dir.path().join("peer"))
            .unwrap()
            .public()
            .to_bytes();
        let candidate = PairCandidate {
            machine_id: "m-peer".to_owned(),
            label: "Laptop".to_owned(),
            key: peer_key,
            reach: None,
            fingerprint: juggler_link::fingerprint_of(&peer_key),
        };
        let _ = crate::fleet::pin_candidate(&state, &candidate, Permission::Control);
        crate::fleet::fleet_set_permission_impl(&state, "m-peer", "control", true).unwrap();
        let prompts = Prompts::default();
        prompts.close();
        let node = AppNode {
            state: Arc::downgrade(&state),
            live: tokio::sync::broadcast::channel(1).0,
            prompts,
            run: 1,
        };
        let refused = node
            .execute(
                &PeerAccess {
                    machine_id: "m-peer".to_owned(),
                    permission: Permission::Control,
                },
                &proto::control_command::Action::StopInstance(proto::StopInstance {
                    instance_id: "s1".to_owned(),
                }),
            )
            .unwrap_err();
        assert_eq!(refused.reason_code, "link_off");
        assert_eq!(refused.message, NOTHING_RAN);
    }
}
