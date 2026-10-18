//! The serving side of the Fleet link (AJ-E703; ADR-0013 contract).
//!
//! A node listens with TLS 1.3 + mandatory Ed25519 client certificates,
//! then authorises EVERY call by looking the TLS-authenticated key up in
//! the peer directory (so an unpair or a permission downgrade applies to
//! the very next call, even on streams already open). Nothing but
//! `Hello` is served until `Hello` succeeds, and the caller's claimed
//! `MachineId` must equal the registry entry for its key.
//!
//! Control commands run through, in order: re-authorisation → console
//! id = TLS peer → Control permission → full service level → per-peer
//! rate limit → nonce shape → measured clock offset → 30 s expiry on the
//! node's clock → strictly increasing sequence + unseen nonce → the
//! node's own handler (which accepts only ids that already exist on the
//! node). Every rejection is audited.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::cert::{NodeCertificate, ed25519_key_of};
use crate::pairing::{EXPORTER_LABEL, PairingCode, PairingError};
use crate::proto;
use crate::protocol::{PROTOCOL_MAJOR, PROTOCOL_MINOR, ServiceLevel, negotiate};
use crate::registry::Permission;
use crate::verify::{ClientKeyVerifier, provider};

/// Command expiry on the node's clock.
pub const COMMAND_EXPIRY_MS: i64 = 30_000;
/// Clock offsets beyond this are flagged and refuse commands.
pub const MAX_CLOCK_SKEW_MS: i64 = 5 * 60_000;
/// Control commands per second per peer.
pub const COMMANDS_PER_SECOND: u32 = 20;
/// Concurrent TLS handshakes in flight.
pub const MAX_HANDSHAKES: usize = 8;
/// Handshake timeout.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Open connections.
pub const MAX_CONNECTIONS: usize = 32;
/// HTTP/2 concurrent streams per connection.
pub const MAX_STREAMS: u32 = 32;
/// Largest decoded message.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// What a pinned peer may do (looked up per call).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PeerAccess {
    /// The peer's `MachineId` (as pinned).
    pub machine_id: String,
    /// What it may do on this node.
    pub permission: Permission,
}

/// The node's view of its peer registry.
pub trait PeerDirectory: Send + Sync + 'static {
    /// The access for a TLS-authenticated key, or `None` when unpinned
    /// (or trust is broken).
    fn lookup(&self, key: &[u8; 32]) -> Option<PeerAccess>;
}

/// Why the node's handler refused a command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandRejection {
    /// Stable machine code (`unknown_record`, …).
    pub reason_code: String,
    /// Plain words.
    pub message: String,
}

/// An auditable link event (rejections and executed commands).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkAuditEvent {
    /// The peer's machine id (or `unpaired`).
    pub peer: String,
    /// `command_accepted` | `command_rejected` | `hello_rejected`.
    pub kind: String,
    /// The reason code for rejections.
    pub reason_code: String,
    /// The command sequence, when one was involved.
    pub sequence: Option<u64>,
}

/// What the node actually does — implemented by the app.
pub trait NodeHandler: Send + Sync + 'static {
    /// This node's `MachineId`.
    fn local_machine_id(&self) -> String;
    /// Telemetry deltas after `since` (the backlog).
    fn telemetry_since(&self, since: u64) -> Vec<proto::TelemetryDelta>;
    /// Live telemetry after the backlog.
    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta>;
    /// Executes an authorised command. Must accept only ids that already
    /// exist on the node.
    ///
    /// # Errors
    ///
    /// Returns a [`CommandRejection`] naming why.
    fn execute(
        &self,
        peer: &PeerAccess,
        action: &proto::control_command::Action,
    ) -> Result<String, CommandRejection>;
    /// Records an audit event.
    fn audit(&self, event: LinkAuditEvent);
}

/// Per-console replay state (the app persists it).
pub trait ReplayStore: Send + Sync + 'static {
    /// The highest accepted sequence for a console (0 = none).
    fn high_water(&self, console: &str) -> u64;
    /// Records an accepted sequence.
    fn record(&self, console: &str, sequence: u64);
}

/// An in-memory [`ReplayStore`] (tests; the app persists instead).
#[derive(Default, Debug)]
pub struct MemoryReplayStore(Mutex<HashMap<String, u64>>);

impl ReplayStore for MemoryReplayStore {
    fn high_water(&self, console: &str) -> u64 {
        // A poisoned lock reads as "everything seen" — fail closed.
        self.0
            .lock()
            .map_or(u64::MAX, |map| map.get(console).copied().unwrap_or(0))
    }

    fn record(&self, console: &str, sequence: u64) {
        if let Ok(mut map) = self.0.lock() {
            map.insert(console.to_owned(), sequence);
        }
    }
}

/// What tonic hands each call about its connection.
#[derive(Clone, Debug)]
pub struct LinkConnInfo {
    /// A process-unique connection id.
    pub conn_id: u64,
    /// The TLS-authenticated Ed25519 key of the peer.
    pub peer_key: [u8; 32],
    /// The peer's address.
    pub addr: SocketAddr,
    /// This TLS session's exporter value ([`EXPORTER_LABEL`]) — what
    /// pairing binds its confirmation to.
    pub exporter: [u8; 32],
}

/// A machine that finished the code exchange and awaits the users'
/// confirmation (shown with its label and fingerprint).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairCandidate {
    /// Its `MachineId`.
    pub machine_id: String,
    /// The label it reported.
    pub label: String,
    /// The key seen in THIS TLS session (what gets pinned).
    pub key: [u8; 32],
    /// That key's fingerprint.
    pub fingerprint: String,
}

/// The host side of pairing — implemented by the app (the window, the
/// guard, the user's confirmation, and the pin).
pub trait PairingHost: Send + Sync + 'static {
    /// Admits one session (window open, not busy, not locked out).
    ///
    /// # Errors
    ///
    /// Returns why pairing can't start.
    fn begin(&self) -> Result<PairingCode, PairingError>;
    /// A session failed (wrong code, malformed): count it and rotate.
    fn failed(&self);
    /// A session ended without a failure (cancel, timeout).
    fn released(&self);
    /// This Mac's label for the other screen.
    fn local_label(&self) -> String;
    /// This Mac's identity key.
    fn local_key(&self) -> [u8; 32];
    /// Asks this Mac's user to confirm `candidate`; resolves to the
    /// permission to grant, or `None` for Cancel.
    fn confirm(
        &self,
        candidate: PairCandidate,
    ) -> tokio::sync::oneshot::Receiver<Option<Permission>>;
    /// Both users confirmed: pin `candidate` with `permission`.
    fn succeeded(&self, candidate: &PairCandidate, permission: Permission);
}

/// The server-side TLS stream, carrying its connection info.
pub struct LinkStream {
    inner: tokio_rustls::server::TlsStream<TcpStream>,
    info: LinkConnInfo,
    sessions: Arc<Mutex<HashMap<u64, Session>>>,
    open: Arc<AtomicUsize>,
}

impl Drop for LinkStream {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&self.info.conn_id);
        }
        self.open.fetch_sub(1, Ordering::SeqCst);
    }
}

impl tonic::transport::server::Connected for LinkStream {
    type ConnectInfo = LinkConnInfo;

    fn connect_info(&self) -> Self::ConnectInfo {
        self.info.clone()
    }
}

impl AsyncRead for LinkStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for LinkStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A connection's state after `Hello`.
#[derive(Clone, Debug)]
struct Session {
    machine_id: String,
    level: ServiceLevel,
    /// node clock − peer clock, ms (from heartbeats).
    offset_ms: Option<i64>,
}

/// Token bucket per console.
#[derive(Debug)]
struct Bucket {
    tokens: f64,
    at: Instant,
}

/// Seen nonces per console, kept for the expiry window.
type NonceCache = HashMap<String, VecDeque<(Instant, [u8; 16])>>;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

/// The gRPC service implementation.
struct LinkService<H, D, R, P> {
    handler: Arc<H>,
    directory: Arc<D>,
    replay: Arc<R>,
    pairing: Arc<P>,
    sessions: Arc<Mutex<HashMap<u64, Session>>>,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
    nonces: Arc<Mutex<NonceCache>>,
}

impl<H: NodeHandler, D: PeerDirectory, R: ReplayStore, P: PairingHost> LinkService<H, D, R, P> {
    fn conn<T>(request: &Request<T>) -> Result<LinkConnInfo, Status> {
        request
            .extensions()
            .get::<LinkConnInfo>()
            .cloned()
            .ok_or_else(|| Status::unauthenticated("no link identity"))
    }

    /// Re-authorises a call: pinned key, finished Hello, same machine.
    fn authorize(&self, conn: &LinkConnInfo) -> Result<(PeerAccess, Session), Status> {
        let access = self
            .directory
            .lookup(&conn.peer_key)
            .ok_or_else(|| Status::permission_denied("This Mac isn't paired with that one."))?;
        let session = self
            .sessions
            .lock()
            .map_err(|_| Status::internal("link state"))?
            .get(&conn.conn_id)
            .cloned()
            .ok_or_else(|| Status::failed_precondition("Hello first"))?;
        if session.machine_id != access.machine_id {
            return Err(Status::unauthenticated("machine identity mismatch"));
        }
        Ok((access, session))
    }

    fn reject(&self, peer: &str, kind: &str, reason: &str, sequence: Option<u64>) {
        self.handler.audit(LinkAuditEvent {
            peer: peer.to_owned(),
            kind: kind.to_owned(),
            reason_code: reason.to_owned(),
            sequence,
        });
    }

    fn take_token(&self, console: &str) -> bool {
        let Ok(mut buckets) = self.buckets.lock() else {
            return false;
        };
        let now = Instant::now();
        let bucket = buckets.entry(console.to_owned()).or_insert(Bucket {
            tokens: f64::from(COMMANDS_PER_SECOND),
            at: now,
        });
        let refill = now.duration_since(bucket.at).as_secs_f64() * f64::from(COMMANDS_PER_SECOND);
        bucket.tokens = (bucket.tokens + refill).min(f64::from(COMMANDS_PER_SECOND));
        bucket.at = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Runs every check on one command; returns the result to stream.
    fn handle_command(
        &self,
        conn: &LinkConnInfo,
        command: &proto::ControlCommand,
    ) -> proto::CommandResult {
        let sequence = command.sequence;
        let rejected = |code: &str, message: &str| proto::CommandResult {
            sequence,
            outcome: "rejected".to_owned(),
            reason_code: code.to_owned(),
            message: message.to_owned(),
        };
        let (access, session) = match self.authorize(conn) {
            Ok(pair) => pair,
            Err(status) => {
                self.reject(
                    "unpaired",
                    "command_rejected",
                    "not_authorised",
                    Some(sequence),
                );
                return rejected("not_authorised", status.message());
            }
        };
        let fail = |code: &str, message: &str| {
            self.reject(&access.machine_id, "command_rejected", code, Some(sequence));
            rejected(code, message)
        };
        if command.console_machine_id != access.machine_id {
            return fail(
                "wrong_console",
                "The command names a different machine than the connection.",
            );
        }
        if access.permission != Permission::Control {
            return fail(
                "permission_denied",
                "View-only pairing — change it in Fleet on this Mac.",
            );
        }
        if session.level != ServiceLevel::Full {
            return fail(
                "degraded",
                "Commands need both Macs on the same AI Juggler release.",
            );
        }
        if !self.take_token(&access.machine_id) {
            return fail("rate_limited", "Too many commands — slow down.");
        }
        let Ok(nonce) = <[u8; 16]>::try_from(command.nonce.as_slice()) else {
            return fail("malformed", "The command is malformed.");
        };
        let Some(offset) = session.offset_ms else {
            return fail("clock_unmeasured", "Waiting for the first heartbeat.");
        };
        if offset.abs() > MAX_CLOCK_SKEW_MS {
            return fail(
                "clock_skew",
                "The two Macs' clocks are more than 5 minutes apart.",
            );
        }
        let age = now_ms() - command.issued_at_ms.saturating_add(offset);
        if !(-COMMAND_EXPIRY_MS..=COMMAND_EXPIRY_MS).contains(&age) {
            return fail("expired", "The command expired before it arrived.");
        }
        if sequence <= self.replay.high_water(&access.machine_id) {
            return fail("replayed", "That command was already received.");
        }
        {
            let Ok(mut nonces) = self.nonces.lock() else {
                return fail("internal", "Link state unavailable.");
            };
            let seen = nonces.entry(access.machine_id.clone()).or_default();
            let window =
                Duration::from_millis(u64::try_from(COMMAND_EXPIRY_MS * 2).unwrap_or(60_000));
            while seen.front().is_some_and(|(at, _)| at.elapsed() > window) {
                seen.pop_front();
            }
            if seen.iter().any(|(_, used)| *used == nonce) {
                return fail("replayed", "That command was already received.");
            }
            seen.push_back((Instant::now(), nonce));
        }
        self.replay.record(&access.machine_id, sequence);
        let Some(action) = &command.action else {
            return fail("malformed", "The command has no action.");
        };
        match self.handler.execute(&access, action) {
            Ok(message) => {
                self.handler.audit(LinkAuditEvent {
                    peer: access.machine_id.clone(),
                    kind: "command_accepted".to_owned(),
                    reason_code: String::new(),
                    sequence: Some(sequence),
                });
                proto::CommandResult {
                    sequence,
                    outcome: "accepted".to_owned(),
                    reason_code: String::new(),
                    message,
                }
            }
            Err(rejection) => fail(&rejection.reason_code, &rejection.message),
        }
    }
}

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl<H: NodeHandler, D: PeerDirectory, R: ReplayStore, P: PairingHost> proto::link_server::Link
    for LinkService<H, D, R, P>
{
    async fn hello(
        &self,
        request: Request<proto::HelloRequest>,
    ) -> Result<Response<proto::HelloResponse>, Status> {
        let conn = Self::conn(&request)?;
        let hello = request.into_inner();
        let Some(access) = self.directory.lookup(&conn.peer_key) else {
            self.reject("unpaired", "hello_rejected", "not_paired", None);
            return Err(Status::permission_denied(
                "This Mac isn't paired with that one.",
            ));
        };
        if hello.machine_id != access.machine_id {
            self.reject(
                &access.machine_id,
                "hello_rejected",
                "machine_mismatch",
                None,
            );
            return Err(Status::unauthenticated("machine identity mismatch"));
        }
        let version = hello.version.unwrap_or_default();
        let level = negotiate(version.major, version.minor);
        if let ServiceLevel::Refused { reason } = &level {
            self.reject(&access.machine_id, "hello_rejected", "version", None);
            return Err(Status::failed_precondition(reason.clone()));
        }
        if access.permission == Permission::NoAccess {
            self.reject(&access.machine_id, "hello_rejected", "no_access", None);
            return Err(Status::permission_denied(
                "This Mac doesn't share anything with that one.",
            ));
        }
        let degraded_reason = match &level {
            ServiceLevel::Degraded { reason } => reason.clone(),
            _ => String::new(),
        };
        self.sessions
            .lock()
            .map_err(|_| Status::internal("link state"))?
            .insert(
                conn.conn_id,
                Session {
                    machine_id: access.machine_id.clone(),
                    level: level.clone(),
                    offset_ms: None,
                },
            );
        Ok(Response::new(proto::HelloResponse {
            version: Some(proto::ProtocolVersion {
                major: PROTOCOL_MAJOR,
                minor: PROTOCOL_MINOR,
            }),
            machine_id: self.handler.local_machine_id(),
            service_level: level.wire().to_owned(),
            degraded_reason,
            permission: match access.permission {
                Permission::NoAccess => "no_access",
                Permission::View => "view",
                Permission::Control => "control",
            }
            .to_owned(),
            sent_at_ms: now_ms(),
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<proto::Ping>,
    ) -> Result<Response<proto::Pong>, Status> {
        let conn = Self::conn(&request)?;
        self.authorize(&conn)?;
        let ping = request.into_inner();
        let now = now_ms();
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(session) = sessions.get_mut(&conn.conn_id)
        {
            session.offset_ms = Some(now - ping.sent_at_ms);
        }
        Ok(Response::new(proto::Pong {
            ping_sent_at_ms: ping.sent_at_ms,
            node_time_ms: now,
        }))
    }

    type TelemetryStream = BoxStream<proto::TelemetryDelta>;

    async fn telemetry(
        &self,
        request: Request<proto::TelemetryRequest>,
    ) -> Result<Response<Self::TelemetryStream>, Status> {
        let conn = Self::conn(&request)?;
        self.authorize(&conn)?;
        let since = request.into_inner().since_sequence;
        let backlog = self.handler.telemetry_since(since);
        let live = tokio_stream::wrappers::BroadcastStream::new(self.handler.telemetry_live())
            .filter_map(Result::ok);
        let directory = self.directory.clone();
        let key = conn.peer_key;
        // Per-item permission re-check: an unpair or NoAccess ends the
        // stream at the next item.
        let stream = tokio_stream::iter(backlog)
            .chain(live)
            .take_while(move |_| {
                directory
                    .lookup(&key)
                    .is_some_and(|access| access.permission != Permission::NoAccess)
            })
            .map(Ok);
        Ok(Response::new(Box::pin(stream)))
    }

    type CommandStream = BoxStream<proto::CommandResult>;

    async fn command(
        &self,
        request: Request<Streaming<proto::ControlCommand>>,
    ) -> Result<Response<Self::CommandStream>, Status> {
        let conn = Self::conn(&request)?;
        self.authorize(&conn)?;
        let mut inbound = request.into_inner();
        let (sender, receiver) = tokio::sync::mpsc::channel(16);
        let service = Self {
            handler: self.handler.clone(),
            directory: self.directory.clone(),
            replay: self.replay.clone(),
            pairing: self.pairing.clone(),
            sessions: self.sessions.clone(),
            buckets: self.buckets.clone(),
            nonces: self.nonces.clone(),
        };
        tokio::spawn(async move {
            while let Some(next) = inbound.next().await {
                let command = match next {
                    Ok(command) => command,
                    Err(status) => {
                        // Oversize or malformed frames end the stream
                        // with the decode error — never silently.
                        let _ = sender.send(Err(status)).await;
                        break;
                    }
                };
                let result = service.handle_command(&conn, &command);
                let stop = result.reason_code == "not_authorised";
                if sender.send(Ok(result)).await.is_err() || stop {
                    break;
                }
            }
        });
        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
        )))
    }

    type PairStream = BoxStream<proto::PairMessage>;

    async fn pair(
        &self,
        request: Request<Streaming<proto::PairMessage>>,
    ) -> Result<Response<Self::PairStream>, Status> {
        // The one call an unpinned key may make: it grants nothing until
        // both codes match AND both users confirm.
        let conn = Self::conn(&request)?;
        let receiver = crate::pairing_rpc::serve(
            conn,
            request.into_inner(),
            self.pairing.clone(),
            self.handler.clone(),
        );
        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
        )))
    }
}

/// A running node.
pub struct LinkNode {
    addr: SocketAddr,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl LinkNode {
    /// The address it listens on.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stops listening and drops every connection.
    pub async fn stop(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let _ = (&mut self.task).await;
    }

    /// Starts serving on `listener`.
    ///
    /// # Errors
    ///
    /// Returns an error string when the TLS configuration cannot be
    /// built.
    pub fn start<H: NodeHandler, D: PeerDirectory, R: ReplayStore, P: PairingHost>(
        listener: TcpListener,
        certificate: &NodeCertificate,
        handler: Arc<H>,
        directory: Arc<D>,
        replay: Arc<R>,
        pairing: Arc<P>,
    ) -> Result<Self, String> {
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let mut config = rustls::ServerConfig::builder_with_provider(provider())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .with_client_cert_verifier(Arc::new(ClientKeyVerifier::new()))
            .with_single_cert(vec![certificate.cert.clone()], certificate.key.clone_key())
            .map_err(|e| e.to_string())?;
        config.alpn_protocols = vec![b"h2".to_vec()];
        // No resumption: every connection proves its key afresh.
        config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
        config.send_tls13_tickets = 0;
        config.max_early_data_size = 0;
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

        let sessions: Arc<Mutex<HashMap<u64, Session>>> = Arc::default();
        let service = LinkService {
            handler,
            directory,
            replay,
            pairing,
            sessions: sessions.clone(),
            buckets: Arc::default(),
            nonces: Arc::default(),
        };
        let (shutdown, on_shutdown) = tokio::sync::oneshot::channel::<()>();
        let (incoming_tx, incoming_rx) =
            tokio::sync::mpsc::channel::<Result<LinkStream, std::io::Error>>(8);
        let handshakes = Arc::new(tokio::sync::Semaphore::new(MAX_HANDSHAKES));
        let open = Arc::new(AtomicUsize::new(0));
        let next_id = Arc::new(AtomicU64::new(1));

        let accept_loop = tokio::spawn(async move {
            loop {
                let Ok((tcp, peer_addr)) = listener.accept().await else {
                    continue;
                };
                if open.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    drop(tcp);
                    continue;
                }
                let Ok(permit) = handshakes.clone().try_acquire_owned() else {
                    // Too many handshakes in flight: drop before TLS work.
                    drop(tcp);
                    continue;
                };
                let acceptor = acceptor.clone();
                let tx = incoming_tx.clone();
                let sessions = sessions.clone();
                let open = open.clone();
                let conn_id = next_id.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let handshake =
                        tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(tcp)).await;
                    drop(permit);
                    let Ok(Ok(tls)) = handshake else {
                        return;
                    };
                    let connection = tls.get_ref().1;
                    let Some(peer_key) = connection
                        .peer_certificates()
                        .and_then(|certs| certs.first())
                        .and_then(|cert| ed25519_key_of(cert).ok())
                    else {
                        return;
                    };
                    let Ok(exporter) =
                        connection.export_keying_material([0_u8; 32], EXPORTER_LABEL, None)
                    else {
                        return;
                    };
                    open.fetch_add(1, Ordering::SeqCst);
                    let stream = LinkStream {
                        inner: tls,
                        info: LinkConnInfo {
                            conn_id,
                            peer_key,
                            addr: peer_addr,
                            exporter,
                        },
                        sessions,
                        open,
                    };
                    let _ = tx.send(Ok(stream)).await;
                });
            }
        });

        let server = tonic::transport::Server::builder()
            .max_concurrent_streams(MAX_STREAMS)
            .http2_max_pending_accept_reset_streams(Some(16))
            .add_service(
                proto::link_server::LinkServer::new(service)
                    .max_decoding_message_size(MAX_MESSAGE_BYTES)
                    .max_encoding_message_size(MAX_MESSAGE_BYTES),
            )
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::ReceiverStream::new(incoming_rx),
                async move {
                    let _ = on_shutdown.await;
                },
            );
        let task = tokio::spawn(async move {
            let _ = server.await;
            accept_loop.abort();
        });
        Ok(Self {
            addr,
            shutdown: Some(shutdown),
            task,
        })
    }
}
