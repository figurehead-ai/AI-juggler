//! The watches at run time: the engine fed by the observation bus, one
//! thread that sleeps until the next deadline (or forever, with none
//! armed), the store's watches loaded on first use, and what fired handed
//! to the notification path (AJ-E3817: a `watch` notice, told once, its
//! alert through the batch, quiet hours and the rest).
//!
//! **While AI Juggler was closed** (ADR-0036 §2.3): the time the engine
//! last heard is kept ([`CURSOR_KEY`]); on start the journals past it are
//! replayed once, in order, and what fires is told "while AI Juggler was
//! closed", as one consolidated notification. Today's journal of what a
//! session did is its endings (`instance_lifecycle`); other apps' states
//! "as seen on start" wait for AJ-E3705.
//!
//! Lock order: the runtime's own lock and the store's are never held
//! together. A publisher only queues ([`hear`]), so a caller holding the
//! store's lock can publish.

use std::collections::VecDeque;
use std::sync::{Arc, OnceLock, Weak};

use juggler_adapter::Observes;
use juggler_core::model::{ProfileId, SessionId};
use juggler_core::storage::MemberRef;
use juggler_core::storage::Storage;
use juggler_core::storage::memory::KindWriter;

use super::catalogue;
use super::conditions::Registry;
use super::engine::{EndReason, Engine, Expiry, Outcome};
use super::model::{Watch, WatchTarget};
use super::store::{Stored, WatchStore};
use crate::features::observe::bell::Bell;
use crate::features::observe::{Observation, Pending, Subject};
use crate::state::AppState;

/// The app setting holding when the engine last heard (epoch ms).
pub const CURSOR_KEY: &str = "watches.cursor_ms";
/// The most journal rows a start replays.
const REPLAY_MAX: usize = 500;
/// The most observations queued before the oldest are dropped (a stalled
/// thread must not grow without bound).
const QUEUE_KEPT: usize = 2_000;

#[derive(Default)]
struct Inner {
    engine: Option<Engine>,
    queue: VecDeque<Pending>,
    closed: bool,
}

type Shared = Bell<Inner>;

/// The watches' run-time state, held by the app's state.
#[derive(Default)]
pub struct Watches {
    shared: Arc<Shared>,
    thread: OnceLock<()>,
    writer: OnceLock<Option<KindWriter>>,
}

impl Drop for Watches {
    fn drop(&mut self) {
        self.shared.close(|inner| inner.closed = true);
    }
}

impl std::fmt::Debug for Watches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watches").finish_non_exhaustive()
    }
}

impl Watches {
    /// The watches' writer, taken from the state's writers on first use
    /// (`None` when another state in this process took them: a test).
    pub fn writer(&self, state: &AppState) -> Option<&KindWriter> {
        self.writer
            .get_or_init(|| state.memory_writers.take_watches())
            .as_ref()
    }

    /// The watches the engine hears for now (loaded on first use).
    #[must_use]
    pub fn hearing(&self) -> usize {
        self.shared
            .lock()
            .map_or(0, |inner| inner.engine.as_ref().map_or(0, Engine::len))
    }
}

/// What the app knows about a session, an account and a target, read from
/// the store: what an observation is about, what an App observes, and
/// whether a target still exists.
pub struct Facts<'a> {
    storage: &'a Storage,
    state: &'a AppState,
}

impl<'a> Facts<'a> {
    /// Facts over one storage hold.
    #[must_use]
    pub const fn new(storage: &'a Storage, state: &'a AppState) -> Self {
        Self { storage, state }
    }

    /// The tool an account is for, and its id.
    fn tool_of_profile(&self, profile: &ProfileId) -> Option<(String, String)> {
        let profile = self.storage.profiles().get(profile).ok()?;
        let account = self.storage.accounts().get(&profile.account).ok()?;
        Some((
            account.id.as_str().to_owned(),
            account.harness.as_str().to_owned(),
        ))
    }

    /// An App's tool, by its App id: its stored row's, else (a CLI App on a
    /// folder) its folder's.
    fn tool_of_app(&self, app: &str) -> Option<String> {
        if let Some(row) = self.storage.apps().get(app).ok().flatten() {
            return Some(row.tool).filter(|tool| !tool.is_empty());
        }
        let folder = app.strip_prefix("cli:")?;
        self.storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(folder))
            .ok()
            .map(|account| account.harness.as_str().to_owned())
    }

    /// An App is there: its stored row, or (a CLI App) its folder.
    fn app_exists(&self, app: &str) -> Option<bool> {
        match self.storage.apps().get(app) {
            Ok(Some(_)) => return Some(true),
            Ok(None) => {}
            Err(_) => return None,
        }
        let Some(folder) = app.strip_prefix("cli:") else {
            return Some(false);
        };
        match self
            .storage
            .accounts()
            .get(&juggler_core::model::AccountId::new(folder))
        {
            Ok(_) => Some(true),
            Err(juggler_core::error::CoreError::NotFound { .. }) => Some(false),
            Err(_) => None,
        }
    }

    /// The App id a launch setting's sessions are in: its folder's CLI
    /// App (a watch stored before App ids, AJ-E3816's migration).
    #[must_use]
    pub fn app_of_launch_settings(&self, id: &str) -> Option<String> {
        let profile = self.storage.profiles().get(&ProfileId::new(id)).ok()?;
        Some(juggler_core::storage::cli_app_id(profile.account.as_str()))
    }

    /// The pools whose members name this session, its App, account or tool.
    fn pools_of(
        &self,
        session: &str,
        app: Option<&str>,
        account: Option<&str>,
        tool: Option<&str>,
    ) -> Vec<String> {
        let Ok(members) = self.storage.pools().members() else {
            return Vec::new();
        };
        let mut pools: Vec<String> = members
            .into_iter()
            .filter(|m| match &m.member {
                MemberRef::Session(id) => id.as_str() == session,
                MemberRef::App(id) => Some(id.as_str()) == app,
                MemberRef::Account(id) => Some(id.as_str()) == account,
                MemberRef::Tool(id) => Some(id.as_str()) == tool,
                _ => false,
            })
            .map(|m| m.pool_id)
            .collect();
        pools.sort();
        pools.dedup();
        pools
    }

    /// What an observation about a session is about: the session, its App
    /// (by its App id, `cli:<folder>` or `app:<host>:<instance>`: the one
    /// resolution the pools use, `sessions().pool_facts_of`), project,
    /// account and pools.
    #[must_use]
    pub fn session_subject(&self, session: &str) -> Subject {
        let record = self.storage.sessions().get(&SessionId::new(session)).ok();
        let app = self
            .storage
            .sessions()
            .pool_facts_of(&SessionId::new(session))
            .ok()
            .flatten()
            .and_then(|facts| facts.app);
        let project = record
            .as_ref()
            .and_then(|r| r.project.as_ref())
            .map(|p| p.as_str().to_owned());
        let tool = record
            .as_ref()
            .and_then(|r| self.tool_of_profile(&r.profile));
        let account = tool.as_ref().map(|(account, _)| account.clone());
        let pools = self.pools_of(
            session,
            app.as_deref(),
            account.as_deref(),
            tool.as_ref().map(|(_, tool)| tool.as_str()),
        );
        Subject {
            session: Some(session.to_owned()),
            apps: app.into_iter().collect(),
            project,
            account,
            pools,
        }
    }

    /// What an observation about an account's room is about: the account
    /// and every App on its folder (its CLI App and the desktop ones that
    /// use it), by App id.
    #[must_use]
    pub fn account_subject(&self, account: &str) -> Subject {
        let mut apps: Vec<String> = self
            .storage
            .apps()
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|app| app.folder_id.as_deref() == Some(account))
            .map(|app| app.id)
            .collect();
        apps.push(juggler_core::storage::cli_app_id(account));
        apps.sort();
        apps.dedup();
        Subject {
            account: Some(account.to_owned()),
            apps,
            ..Subject::default()
        }
    }

    fn observes_of_tool(&self, tool: &str) -> Observes {
        let Some(adapter) = self
            .state
            .registry
            .get(&juggler_core::model::HarnessId::new(tool))
        else {
            return Observes::empty();
        };
        let spec = adapter.spec();
        spec.observes.cli | spec.observes.desktop
    }

    /// What AI Juggler can observe for a target: its Apps' declared sets
    /// (every App's for a project, pool, account or any session), and AI
    /// Juggler's own.
    #[must_use]
    pub fn observes(&self, target: &WatchTarget) -> Observes {
        let tools: Vec<String> = match target {
            WatchTarget::Session(session) => self
                .storage
                .sessions()
                .get(&SessionId::new(session.as_str()))
                .ok()
                .and_then(|r| self.tool_of_profile(&r.profile))
                .map(|(_, tool)| tool)
                .into_iter()
                .collect(),
            WatchTarget::App(app) => self.tool_of_app(app).into_iter().collect(),
            WatchTarget::Account(account) => self
                .storage
                .accounts()
                .get(&juggler_core::model::AccountId::new(account.as_str()))
                .ok()
                .map(|a| a.harness.as_str().to_owned())
                .into_iter()
                .collect(),
            WatchTarget::Project(_) | WatchTarget::Pool(_) | WatchTarget::Any => self
                .storage
                .accounts()
                .list()
                .map(|accounts| {
                    accounts
                        .into_iter()
                        .map(|a| a.harness.as_str().to_owned())
                        .collect()
                })
                .unwrap_or_default(),
        };
        tools.iter().fold(Observes::AI_JUGGLER_OWN, |all, tool| {
            all | self.observes_of_tool(tool)
        })
    }

    /// The target still exists (`None` when the store can't say).
    #[must_use]
    pub fn exists(&self, target: &WatchTarget) -> Option<bool> {
        let found = |result: Result<(), juggler_core::error::CoreError>| match result {
            Ok(()) => Some(true),
            Err(juggler_core::error::CoreError::NotFound { .. }) => Some(false),
            Err(_) => None,
        };
        match target {
            WatchTarget::Session(id) => found(
                self.storage
                    .sessions()
                    .get(&SessionId::new(id.as_str()))
                    .map(drop),
            ),
            WatchTarget::App(id) => self.app_exists(id),
            WatchTarget::Project(id) => found(
                self.storage
                    .projects()
                    .get(&juggler_core::model::ProjectId::new(id.as_str()))
                    .map(drop),
            ),
            WatchTarget::Pool(id) => found(self.storage.pools().get(id).map(drop)),
            WatchTarget::Account(id) => found(
                self.storage
                    .accounts()
                    .get(&juggler_core::model::AccountId::new(id.as_str()))
                    .map(drop),
            ),
            WatchTarget::Any => Some(true),
        }
    }
}

/// Why a stored watch can't be heard for now: a target gone, or a
/// condition no longer offered (the closed set or an App's `observes`
/// narrowed). Never silent: the caller logs it as the watch's ending.
fn unheard(registry: &Registry, facts: &Facts<'_>, watch: &Watch) -> Option<EndReason> {
    if watch.targets.iter().any(|t| facts.exists(t) == Some(false)) {
        return Some(EndReason::TargetGone);
    }
    let observed: Vec<Observes> = watch.targets.iter().map(|t| facts.observes(t)).collect();
    if watch
        .conditions
        .iter()
        .any(|c| registry.get(&c.code).is_none())
        || catalogue::check_offered(registry, watch, &observed).is_err()
    {
        return Some(EndReason::NoLongerOffered);
    }
    None
}

/// Loads the engine from the store on first use: every active watch it
/// can hear for; the others end now, logged with why. With the store not
/// there yet (V54), the engine is empty.
fn load(state: &AppState, registry: &Arc<Registry>, now_ms: i64) -> Engine {
    let mut engine = Engine::new(registry.clone());
    let writer = state.watches.writer(state);
    let Ok(storage) = state.storage.lock() else {
        return engine;
    };
    let store = WatchStore::new(&storage, registry);
    if let Some(writer) = writer {
        migrate_app_targets(&store, &Facts::new(&storage, state), writer, now_ms);
    }
    let Ok(active) = store.active() else {
        return engine;
    };
    let facts = Facts::new(&storage, state);
    for Stored { watch, .. } in active {
        if let Some(reason) = unheard(registry, &facts, &watch) {
            let _ = store.log_end(&Expiry {
                watch: watch.id.clone(),
                reason,
                at_ms: now_ms,
            });
            continue;
        }
        engine.add(watch, now_ms);
    }
    engine
}

/// AJ-E3816: a watch on an App stored before Apps were targeted by their
/// App id named its launch settings; each such target moves to the App
/// those settings' sessions are in (a new version, the app's, once).
/// Every App id is `cli:` or `app:`; a launch setting's id never is.
fn migrate_app_targets(
    store: &WatchStore<'_>,
    facts: &Facts<'_>,
    writer: &KindWriter,
    now_ms: i64,
) {
    let legacy = |id: &str| !id.starts_with("cli:") && !id.starts_with("app:");
    for stored in store.every().unwrap_or_default() {
        if !stored
            .watch
            .targets
            .iter()
            .any(|t| matches!(t, WatchTarget::App(id) if legacy(id)))
        {
            continue;
        }
        let mut moved: Vec<WatchTarget> = Vec::with_capacity(stored.watch.targets.len());
        for target in stored.watch.targets {
            let next = match &target {
                WatchTarget::App(id) if legacy(id) => facts
                    .app_of_launch_settings(id)
                    .map_or(target.clone(), WatchTarget::App),
                _ => target.clone(),
            };
            if !moved.contains(&next) {
                moved.push(next);
            }
        }
        let _ = store.retarget(
            writer,
            &crate::features::ai::memory::RedactionSecrets,
            &stored.watch.id,
            moved,
            now_ms,
        );
    }
}

/// Queues what happened for the engine and wakes its thread.
pub fn hear(state: &Arc<AppState>, pending: Pending) {
    let watches = &state.watches;
    start_thread(state);
    if let Ok(mut inner) = watches.shared.lock() {
        if inner.queue.len() >= QUEUE_KEPT {
            inner.queue.pop_front();
        }
        inner.queue.push_back(pending);
    }
    watches.shared.ring();
}

/// Starts the watches at the app's start: their thread loads the stored
/// watches and sleeps until the first deadline, or until something
/// happens.
pub fn start(state: &Arc<AppState>) {
    start_thread(state);
}

/// The engine changed (a command): the thread recomputes its next wake.
pub fn changed(state: &Arc<AppState>) {
    start_thread(state);
    state.watches.shared.ring();
}

/// Runs `f` on the loaded engine.
pub fn with_engine<T>(
    state: &Arc<AppState>,
    now_ms: i64,
    f: impl FnOnce(&mut Engine) -> T,
) -> Option<T> {
    let registry = Registry::shipped();
    let loaded = {
        let inner = state.watches.shared.lock().ok()?;
        inner.engine.is_some()
    };
    let fresh = (!loaded).then(|| load(state, &registry, now_ms));
    let mut inner = state.watches.shared.lock().ok()?;
    if inner.engine.is_none() {
        inner.engine = fresh;
    }
    inner.engine.as_mut().map(f)
}

fn start_thread(state: &Arc<AppState>) {
    let watches = &state.watches;
    watches.thread.get_or_init(|| {
        let shared = watches.shared.clone();
        let weak = Arc::downgrade(state);
        let _ = std::thread::Builder::new()
            .name("watches".into())
            .spawn(move || run(&shared, &weak));
    });
}

/// The thread: waits until something is queued or the next deadline,
/// resolves what was queued (off every lock), runs the engine, and logs
/// what happened. With nothing queued and nothing armed, it sleeps until
/// woken.
fn run(shared: &Shared, state: &Weak<AppState>) {
    // The stored watches load here, off the window's thread, so their
    // deadlines and "until" times are armed from the start.
    if let Some(state) = state.upgrade() {
        with_engine(&state, crate::commands::now_millis(), |_| ());
        replay(&state);
    }
    loop {
        let batch: Vec<Pending> = {
            let Ok(mut inner) = shared.lock() else {
                return;
            };
            loop {
                if inner.closed {
                    return;
                }
                if !inner.queue.is_empty() {
                    break;
                }
                let now = crate::commands::now_millis();
                let wake = inner.engine.as_ref().and_then(Engine::next_wake);
                match wake {
                    Some(at) if at <= now => break,
                    wake => {
                        let Some(next) = shared.sleep(inner, wake, now) else {
                            return;
                        };
                        inner = next;
                    }
                }
            }
            inner.queue.drain(..).collect()
        };
        let Some(state) = state.upgrade() else {
            return;
        };
        step(&state, batch);
    }
}

/// One step: the queued observations resolved and heard, in order, then
/// what fell due by now.
fn step(state: &Arc<AppState>, batch: Vec<Pending>) {
    let observations: Vec<Observation> = match state.storage.lock() {
        Ok(storage) => {
            let facts = Facts::new(&storage, state);
            batch
                .into_iter()
                .flat_map(|pending| resolve(&facts, pending))
                .collect()
        }
        Err(_) => return,
    };
    // ADR-0036 §2.3: the bus's other subscriber, the digest's scheduler,
    // hears the same resolved observations.
    crate::features::digest::runtime::heard(state, &observations);
    let now = crate::commands::now_millis();
    let outcome = with_engine(state, now, |engine| {
        let mut out = engine.observe_all(&observations);
        out.merge(engine.tick(now));
        out
    })
    .unwrap_or_default();
    record(state, &outcome);
    heard_until(state, now);
}

/// The engine heard everything up to `now_ms`: a start replays only what
/// came after.
fn heard_until(state: &AppState, now_ms: i64) {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.app_settings().set(CURSOR_KEY, &now_ms.to_string());
    }
}

/// The journals' rows past the cursor, as observations in time order:
/// each session's ending (`ended: …`), what its status said then.
pub fn journal_since(
    storage: &Storage,
    cursor_ms: i64,
) -> Vec<(String, i64, Vec<crate::features::observe::Observed>)> {
    use crate::features::observe::from_status;
    const ENDINGS: [(&str, &str); 4] = [
        ("ended: finished", "finished"),
        ("ended: error", "failed"),
        ("ended: crashed", "failed"),
        ("ended: stopped by you", "stopped"),
    ];
    let prefixes: Vec<String> = ENDINGS.iter().map(|(p, _)| (*p).to_owned()).collect();
    let scope = juggler_core::storage::feed::FeedScope::default();
    let mut rows = Vec::new();
    let mut before = None;
    loop {
        let Ok(page) = storage.feed().lifecycle(before, &scope, &prefixes, 100) else {
            break;
        };
        let full = page.len() == 100;
        let mut past = false;
        for row in page {
            before = Some(row.key);
            if row.key.at_ms <= cursor_ms {
                past = true;
                break;
            }
            let status = ENDINGS
                .iter()
                .find(|(prefix, _)| row.event.starts_with(prefix))
                .map(|(_, status)| *status);
            if let Some(status) = status {
                rows.push((row.session.id.clone(), row.key.at_ms, from_status(status)));
            }
        }
        if past || !full || rows.len() >= REPLAY_MAX {
            break;
        }
    }
    rows.truncate(REPLAY_MAX);
    rows.reverse();
    rows
}

/// On start, once: the journals past the cursor replayed to the watches,
/// in order; what fires is told "while AI Juggler was closed". The first
/// start has no cursor: nothing to replay.
fn replay(state: &Arc<AppState>) {
    let now = crate::commands::now_millis();
    let observations: Vec<Observation> = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let cursor = storage
            .app_settings()
            .get(CURSOR_KEY)
            .ok()
            .flatten()
            .and_then(|raw| raw.parse::<i64>().ok());
        let _ = storage.app_settings().set(CURSOR_KEY, &now.to_string());
        let Some(cursor) = cursor else {
            return;
        };
        if state.watches.hearing() == 0 {
            return;
        }
        let facts = Facts::new(&storage, state);
        journal_since(&storage, cursor)
            .into_iter()
            .flat_map(|(session, at_ms, events)| {
                let subject = facts.session_subject(&session);
                events.into_iter().map(move |event| Observation {
                    at_ms,
                    subject: subject.clone(),
                    event,
                    replayed: true,
                })
            })
            .collect()
    };
    if observations.is_empty() {
        return;
    }
    let outcome =
        with_engine(state, now, |engine| engine.observe_all(&observations)).unwrap_or_default();
    record(state, &outcome);
}

fn resolve(facts: &Facts<'_>, pending: Pending) -> Vec<Observation> {
    match pending {
        Pending::Session {
            session,
            events,
            at_ms,
        } => {
            let subject = facts.session_subject(&session);
            events
                .into_iter()
                .map(|event| Observation {
                    at_ms,
                    subject: subject.clone(),
                    event,
                    replayed: false,
                })
                .collect()
        }
        Pending::Account {
            account,
            event,
            at_ms,
        } => vec![Observation {
            at_ms,
            subject: facts.account_subject(&account),
            event,
            replayed: false,
        }],
    }
}

/// Logs what fired and ended, and hands what fired to the notification
/// path (AJ-E3817).
pub fn record(state: &Arc<AppState>, outcome: &Outcome) {
    if outcome.is_empty() {
        return;
    }
    if let Ok(storage) = state.storage.lock() {
        let registry = Registry::shipped();
        let store = WatchStore::new(&storage, &registry);
        for fire in &outcome.fires {
            let _ = store.log_fire(fire);
        }
        for expiry in &outcome.ended {
            let _ = store.log_end(expiry);
        }
    }
    crate::features::notifications::watch_alerts::fired(state, &outcome.fires);
}
