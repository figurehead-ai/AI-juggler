//! Fetching the public file (§7) through an injected transport.
//!
//! - **At most once per [`FetchPolicy::min_interval`]** (a day by default),
//!   counting failed attempts too.
//! - **One GET per channel to a fixed URL** from a compiled endpoint list.
//!   A [`ConfigRequest`] holds only the URL (and, if the policy allows, an
//!   `If-None-Match`): there is no field for a cookie, an id, an account or
//!   a query string, so the transport can't be handed one. The production
//!   transport adds only `User-Agent: AIJuggler/<version>`.
//! - **The last good file is cached** and re-verified on every load, so a
//!   cache that was tampered with, or has expired, or was signed by a key
//!   since revoked, is never used.
//! - **Every failure falls back:** to the cached file while it's valid,
//!   else to the compiled defaults (no layer). Missing, invalid, unsigned
//!   or expired files, network errors, a full or broken store: none of
//!   them stops the app.
//! - **One refresh at a time** per fetcher (a lock around load, fetch and
//!   save), and the store's save only moves forward, so racing refreshes
//!   (two fetchers, or two calls) can't lower the floor or lose a
//!   revocation. The file returned is re-read from the store after the
//!   save, so a caller never gets a file a concurrent refresh revoked.
//! - **A failed save serves nothing new:** the fetched file's floor and
//!   revocations weren't kept, so the previous cached file (or the
//!   defaults) stays in use, and the attempt time is kept in memory so the
//!   daily limit still holds.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::envelope::{Channel, Verified, VerifyContext, check_file, record_accepted};
use crate::keys::KeyRing;
use crate::state::{Baseline, StateStore, StoreError, TrustState};
use crate::version::Version;
use crate::{MAX_FILE_BYTES, Rejection};

/// The per-request timeout the production transport uses (§7).
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// The default least time between two fetches (D-128: at most daily).
pub const DEFAULT_MIN_INTERVAL: Duration = Duration::from_hours(24);

/// The public files' base URLs, tried in order (D-129: served from the
/// Figurehead website, `https://config.figurehead.ai/v1/<channel>.json`).
/// Nothing verifies until keys are pinned ([`crate::keys::COMPILED_KEYS`]),
/// so until then a fetch changes nothing: the compiled defaults stay.
pub const COMPILED_ENDPOINTS: &[&str] = &["https://config.figurehead.ai"];

/// The only hosts the public file may come from. [`Endpoints::compiled`]
/// holds the compiled list to it, and the app's transport refuses any
/// other URL ([`allowed_url`]); it follows no redirect, so a 3xx to
/// another host (or this one) is a failed fetch.
pub const ALLOWED_HOSTS: &[&str] = &["config.figurehead.ai"];

/// The path under a base URL: `/v1/<channel>.json`.
const PATH: &str = "/v1/";

/// The host of an `https://` URL, if it has one with no port or user.
pub(crate) fn https_host(url: &str) -> Option<&str> {
    let host = url
        .strip_prefix("https://")?
        .split(['/', '?', '#'])
        .next()?;
    (!host.is_empty() && !host.contains([':', '@'])).then_some(host)
}

/// Whether `url` is an `https://` URL on an [`ALLOWED_HOSTS`] host.
#[must_use]
pub fn allowed_url(url: &str) -> bool {
    https_host(url).is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

/// Everything a request carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigRequest {
    /// The absolute URL, with no query.
    pub url: String,
    /// The cached file's entity tag, when conditional requests are on.
    pub if_none_match: Option<String>,
}

/// What came back.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConfigResponse {
    /// The HTTP status.
    pub status: u16,
    /// The `ETag` header, if any.
    pub etag: Option<String>,
    /// The body.
    pub body: Vec<u8>,
}

/// No response arrived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{0}")]
pub struct NetworkError(pub String);

/// Sends one GET. The app's implementation wraps the account client's
/// HTTPS transport (rustls, the platform trust store, no cookies, no
/// redirects, [`FETCH_TIMEOUT`]); tests use [`RecordingTransport`] or
/// [`ReplayTransport`].
pub trait Transport: Send + Sync {
    /// Sends `request`.
    ///
    /// # Errors
    ///
    /// [`NetworkError`] when no response arrived.
    fn get(&self, request: &ConfigRequest) -> Result<ConfigResponse, NetworkError>;
}

/// One request and what came back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exchange {
    /// The request.
    pub request: ConfigRequest,
    /// The response, or the network failure.
    pub response: Result<ConfigResponse, NetworkError>,
}

/// A scripted transport that records every request: it proves what was
/// sent, and that nothing else was.
#[derive(Debug, Default)]
pub struct RecordingTransport {
    exchanges: Mutex<Vec<Exchange>>,
    script: Mutex<VecDeque<Result<ConfigResponse, NetworkError>>>,
}

impl RecordingTransport {
    /// Queues the next response.
    pub fn respond(&self, status: u16, body: &[u8]) {
        self.push(Ok(ConfigResponse {
            status,
            etag: None,
            body: body.to_vec(),
        }));
    }

    /// Queues a full response or a network failure.
    pub fn push(&self, response: Result<ConfigResponse, NetworkError>) {
        if let Ok(mut script) = self.script.lock() {
            script.push_back(response);
        }
    }

    /// Every request sent so far.
    #[must_use]
    pub fn requests(&self) -> Vec<ConfigRequest> {
        self.exchanges()
            .into_iter()
            .map(|exchange| exchange.request)
            .collect()
    }

    /// Every exchange so far: a recording [`ReplayTransport`] can play back.
    #[must_use]
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.exchanges.lock().map(|e| e.clone()).unwrap_or_default()
    }
}

impl Transport for RecordingTransport {
    fn get(&self, request: &ConfigRequest) -> Result<ConfigResponse, NetworkError> {
        let response = self
            .script
            .lock()
            .ok()
            .and_then(|mut script| script.pop_front())
            .unwrap_or_else(|| Err(NetworkError("no scripted response".to_owned())));
        if let Ok(mut exchanges) = self.exchanges.lock() {
            exchanges.push(Exchange {
                request: request.clone(),
                response: response.clone(),
            });
        }
        response
    }
}

/// Wraps a transport and records each exchange (to save a real session as
/// a fixture).
pub struct Recorder<T> {
    inner: T,
    exchanges: Mutex<Vec<Exchange>>,
}

impl<T> std::fmt::Debug for Recorder<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder").finish_non_exhaustive()
    }
}

impl<T: Transport> Recorder<T> {
    /// Records what `inner` sends and receives.
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            exchanges: Mutex::new(Vec::new()),
        }
    }

    /// The recording so far.
    #[must_use]
    pub fn recording(&self) -> Vec<Exchange> {
        self.exchanges.lock().map(|e| e.clone()).unwrap_or_default()
    }
}

impl<T: Transport> Transport for Recorder<T> {
    fn get(&self, request: &ConfigRequest) -> Result<ConfigResponse, NetworkError> {
        let response = self.inner.get(request);
        if let Ok(mut exchanges) = self.exchanges.lock() {
            exchanges.push(Exchange {
                request: request.clone(),
                response: response.clone(),
            });
        }
        response
    }
}

/// Plays a recording back, refusing any request that differs from the
/// recorded one.
#[derive(Debug)]
pub struct ReplayTransport {
    exchanges: Mutex<VecDeque<Exchange>>,
}

impl ReplayTransport {
    /// Plays `exchanges` back in order.
    #[must_use]
    pub fn new(exchanges: Vec<Exchange>) -> Self {
        Self {
            exchanges: Mutex::new(exchanges.into()),
        }
    }

    /// How many recorded exchanges are left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.exchanges.lock().map_or(0, |e| e.len())
    }
}

impl Transport for ReplayTransport {
    fn get(&self, request: &ConfigRequest) -> Result<ConfigResponse, NetworkError> {
        let mut exchanges = self
            .exchanges
            .lock()
            .map_err(|_| NetworkError("replay poisoned".to_owned()))?;
        match exchanges.front() {
            Some(next) if next.request == *request => exchanges.pop_front().map_or_else(
                || Err(NetworkError("replay finished".to_owned())),
                |exchange| exchange.response,
            ),
            Some(_) => Err(NetworkError(
                "replay: request differs from the recording".to_owned(),
            )),
            None => Err(NetworkError("replay finished".to_owned())),
        }
    }
}

/// A base URL that can't be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("endpoint {0:?} is not an https base URL without query, fragment or credentials")]
pub struct EndpointError(pub String);

/// The base URLs the public file is fetched from, host-agnostic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Endpoints {
    bases: Vec<String>,
}

impl Endpoints {
    /// `https://` base URLs with no query, fragment or user info.
    ///
    /// # Errors
    ///
    /// [`EndpointError`] for any other.
    pub fn new<I, S>(bases: I) -> Result<Self, EndpointError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let bases = bases
            .into_iter()
            .map(|base| {
                let base: String = base.into();
                let rest = base.strip_prefix("https://").unwrap_or_default();
                let host = rest.split('/').next().unwrap_or_default();
                if rest.is_empty()
                    || host.is_empty()
                    || base.contains(['?', '#', '@', ' '])
                    || base.chars().any(char::is_control)
                {
                    Err(EndpointError(base))
                } else {
                    Ok(base.trim_end_matches('/').to_owned())
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { bases })
    }

    /// [`COMPILED_ENDPOINTS`], each on an [`ALLOWED_HOSTS`] host.
    ///
    /// # Errors
    ///
    /// As [`Endpoints::new`], and for a base on any other host; a test
    /// holds the compiled list to it.
    pub fn compiled() -> Result<Self, EndpointError> {
        if let Some(base) = COMPILED_ENDPOINTS.iter().find(|base| !allowed_url(base)) {
            return Err(EndpointError((*base).to_owned()));
        }
        Self::new(COMPILED_ENDPOINTS.iter().copied())
    }

    /// The file's URL on each base, in order.
    #[must_use]
    pub fn urls(&self, channel: Channel) -> Vec<String> {
        self.bases
            .iter()
            .map(|base| format!("{base}{PATH}{}.json", channel.as_str()))
            .collect()
    }
}

/// How fetching behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchPolicy {
    /// Settings › Privacy "Check for configuration updates". Off: no
    /// request, and no public layer (compiled defaults).
    pub enabled: bool,
    /// The least time between two attempts.
    pub min_interval: Duration,
    /// Send `If-None-Match`. Off by default: a hostile host could hand
    /// each install a unique entity tag and so recognise it on the next
    /// fetch. The file is small and fetched daily, so the saving is not
    /// worth an echoed server token.
    pub conditional: bool,
}

impl Default for FetchPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            min_interval: DEFAULT_MIN_INTERVAL,
            conditional: false,
        }
    }
}

/// What one refresh did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    /// The off switch is off.
    Disabled,
    /// No endpoint is compiled in.
    NoEndpoint,
    /// The last attempt was too recent.
    NotDue,
    /// The host said the cached file is current.
    NotModified,
    /// A new file was verified and accepted.
    Accepted {
        /// Its serial.
        serial: u64,
    },
    /// Nothing usable came back; the reason from the last endpoint tried.
    Failed(Failure),
}

/// Why a fetch gave nothing new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No response.
    Network(String),
    /// A status other than 200 or 304 (404 is "no file").
    Status(u16),
    /// The file failed a check.
    Rejected(Rejection),
    /// The state store can't be read or written now: nothing was fetched
    /// or nothing new is used.
    Store(String),
}

/// The result of a refresh: the file in use (if any) and what happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refresh {
    /// The verified public file in use; `None` is the compiled defaults.
    pub file: Option<Verified>,
    /// What this refresh did.
    pub attempt: Attempt,
    /// When a fetch was last attempted for this channel, Unix seconds (any
    /// outcome; this refresh's if it fetched), when known: "last checked".
    pub checked_at: Option<u64>,
}

/// Fetches, verifies and caches the public file for one channel.
pub struct Fetcher {
    transport: Arc<dyn Transport>,
    endpoints: Endpoints,
    keys: KeyRing,
    store: Arc<dyn StateStore>,
    policy: FetchPolicy,
    baseline: Baseline,
    /// Held across a whole refresh: what this process remembers.
    memory: Mutex<Memory>,
}

/// What a fetcher keeps in memory, for when the store can't be read or
/// written.
#[derive(Debug, Default)]
struct Memory {
    /// The last attempt per channel.
    attempts: BTreeMap<Channel, u64>,
    /// The last state read or saved.
    state: Option<TrustState>,
}

impl std::fmt::Debug for Fetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fetcher")
            .field("endpoints", &self.endpoints)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

/// Who is asking: this app's channel and version, and the time.
#[derive(Debug, Clone, Copy)]
pub struct AppContext<'a> {
    /// The app's channel.
    pub channel: Channel,
    /// The app's version.
    pub app_version: &'a Version,
    /// Now, Unix seconds.
    pub now: u64,
}

impl Fetcher {
    /// A fetcher over the given seams, with the compiled [`Baseline`].
    #[must_use]
    pub fn new(
        transport: Arc<dyn Transport>,
        endpoints: Endpoints,
        keys: KeyRing,
        store: Arc<dyn StateStore>,
        policy: FetchPolicy,
    ) -> Self {
        Self {
            transport,
            endpoints,
            keys,
            store,
            policy,
            baseline: Baseline::compiled(),
            memory: Mutex::new(Memory::default()),
        }
    }

    /// The same, starting from another baseline (tests).
    #[must_use]
    pub fn with_baseline(mut self, baseline: Baseline) -> Self {
        self.baseline = baseline;
        self
    }

    fn memory(&self) -> std::sync::MutexGuard<'_, Memory> {
        match self.memory.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// The saved state raised to the baseline. A missing or corrupt state
    /// is the baseline alone (fail safe, never "no floor"); a state that
    /// can't be read now is an error (fail closed: the caller fetches and
    /// saves nothing).
    fn load(&self, memory: &mut Memory) -> Result<TrustState, StoreError> {
        let mut state = match self.store.load() {
            Ok(state) => state.unwrap_or_default(),
            Err(StoreError::Corrupt(why)) => {
                tracing::warn!(%why, "remote config: state corrupt; the compiled baseline");
                TrustState::default()
            }
            Err(e) => {
                tracing::warn!(error = %e, "remote config: state unreadable now; nothing fetched");
                return Err(e);
            }
        };
        state.apply_baseline(&self.baseline);
        memory.state = Some(state.clone());
        Ok(state)
    }

    /// The cached file, re-verified, with no network: what the app uses at
    /// launch before the first fetch.
    #[must_use]
    pub fn cached(&self, app: &AppContext<'_>) -> Option<Verified> {
        if !self.policy.enabled {
            return None;
        }
        let mut memory = self.memory();
        match self.load(&mut memory) {
            Ok(state) => self.verify_cached(&state, app),
            Err(_) => memory
                .state
                .as_ref()
                .and_then(|state| self.verify_cached(state, app)),
        }
    }

    fn verify_cached(&self, state: &TrustState, app: &AppContext<'_>) -> Option<Verified> {
        let accepted = state.channels.get(&app.channel)?.accepted.as_ref()?;
        let ctx = self.context(app);
        match check_file(accepted.file.as_bytes(), &ctx, state).result {
            Ok(verified) => Some(verified),
            Err(rejection) => {
                tracing::info!(%rejection, "remote config: the cached file is no longer used");
                None
            }
        }
    }

    const fn context<'a>(&'a self, app: &AppContext<'a>) -> VerifyContext<'a> {
        VerifyContext {
            keys: &self.keys,
            channel: app.channel,
            app_version: app.app_version,
            now: app.now,
        }
    }

    /// Fetches if due, verifies, and caches: see the module docs.
    #[must_use]
    pub fn refresh(&self, app: &AppContext<'_>) -> Refresh {
        if !self.policy.enabled {
            return Refresh {
                file: None,
                attempt: Attempt::Disabled,
                checked_at: None,
            };
        }
        // One refresh at a time: the lock is held to the end.
        let mut memory = self.memory();
        let mut state = match self.load(&mut memory) {
            Ok(state) => state,
            Err(e) => {
                return Refresh {
                    file: memory
                        .state
                        .as_ref()
                        .and_then(|state| self.verify_cached(state, app)),
                    attempt: Attempt::Failed(Failure::Store(e.to_string())),
                    checked_at: memory.attempts.get(&app.channel).copied(),
                };
            }
        };
        let cached = self.verify_cached(&state, app);
        let urls = self.endpoints.urls(app.channel);
        if urls.is_empty() {
            return Refresh {
                file: cached,
                attempt: Attempt::NoEndpoint,
                checked_at: None,
            };
        }
        let slot = state.channels.entry(app.channel).or_default();
        let last = slot
            .last_attempt
            .max(memory.attempts.get(&app.channel).copied());
        let interval = self.policy.min_interval.as_secs();
        // A clock set back makes the last attempt look future: then it's due.
        if last.is_some_and(|last| last <= app.now && app.now - last < interval) {
            return Refresh {
                file: cached,
                attempt: Attempt::NotDue,
                checked_at: last,
            };
        }
        slot.last_attempt = Some(app.now);
        memory.attempts.insert(app.channel, app.now);
        let before = cached;
        let attempt = self.fetch(&urls, &mut state, app, before.is_some());
        if let Err(e) = self.store.save(&state) {
            tracing::warn!(error = %e, "remote config: state not saved; nothing new is used");
            // A revocation seen this time still retires the old file.
            return Refresh {
                file: before.filter(|file| !state.revoked.contains(&file.key)),
                attempt: Attempt::Failed(Failure::Store(e.to_string())),
                checked_at: Some(app.now),
            };
        }
        // What's stored now may be newer than what this refresh saw.
        let stored = self.load(&mut memory).unwrap_or(state);
        let file = self.verify_cached(&stored, app);
        Refresh {
            file,
            attempt,
            checked_at: Some(app.now),
        }
    }

    fn fetch(
        &self,
        urls: &[String],
        state: &mut TrustState,
        app: &AppContext<'_>,
        has_cache: bool,
    ) -> Attempt {
        let etag = if self.policy.conditional && has_cache {
            state
                .channels
                .get(&app.channel)
                .and_then(|slot| slot.accepted.as_ref())
                .and_then(|accepted| accepted.etag.clone())
        } else {
            None
        };
        let mut failure = Failure::Status(0);
        for url in urls {
            let request = ConfigRequest {
                url: url.clone(),
                if_none_match: etag.clone(),
            };
            let response = match self.transport.get(&request) {
                Ok(response) => response,
                Err(e) => {
                    failure = Failure::Network(e.0);
                    continue;
                }
            };
            match response.status {
                304 if has_cache => return Attempt::NotModified,
                200 => {}
                status => {
                    failure = Failure::Status(status);
                    continue;
                }
            }
            match self.accept(&response, state, app) {
                Ok(verified) => {
                    return Attempt::Accepted {
                        serial: verified.envelope.serial,
                    };
                }
                Err(rejection) => {
                    tracing::warn!(%rejection, "remote config: file rejected");
                    failure = Failure::Rejected(rejection);
                }
            }
        }
        Attempt::Failed(failure)
    }

    fn accept(
        &self,
        response: &ConfigResponse,
        state: &mut TrustState,
        app: &AppContext<'_>,
    ) -> Result<Verified, Rejection> {
        if response.body.len() > MAX_FILE_BYTES {
            return Err(Rejection::TooLarge(response.body.len()));
        }
        let checked = check_file(&response.body, &self.context(app), state);
        // A verified revocation stands even if the rest of the file fails.
        state.revoked.extend(checked.newly_revoked);
        let verified = checked.result?;
        record_accepted(state, app.channel, &verified, &response.body)?;
        if let Some(accepted) = state
            .channels
            .get_mut(&app.channel)
            .and_then(|slot| slot.accepted.as_mut())
        {
            accepted.etag = response
                .etag
                .clone()
                .filter(|tag| tag.len() <= MAX_ETAG_LEN);
        }
        Ok(verified)
    }
}

/// The longest entity tag kept.
pub const MAX_ETAG_LEN: usize = 128;
