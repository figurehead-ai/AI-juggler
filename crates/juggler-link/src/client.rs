//! The connecting side of the Fleet link (AJ-E703; ADR-0013 contract).
//!
//! A console connects over TLS 1.3 only, presents its identity
//! certificate, and accepts the node only if the node's certificate
//! carries exactly the pinned key. Resumption is off, so every
//! connection proves possession of both keys afresh. A liveness loop
//! heartbeats every 2 s and reports `Offline` after two misses (≤ 5 s);
//! it never retries with weaker settings.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use tonic::transport::{Channel, Endpoint};

use crate::cert::NodeCertificate;
use crate::proto;
use crate::protocol::{PROTOCOL_MAJOR, PROTOCOL_MINOR};
use crate::verify::{Expected, ServerKeyVerifier, provider};

/// Heartbeat interval.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
/// Heartbeat timeout per attempt.
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(1);
/// Missed heartbeats before `Offline`.
pub const MISSES_BEFORE_OFFLINE: u32 = 2;

/// The cosmetic TLS server name (identity is the pinned key).
const SERVER_NAME: &str = "aijuggler.node";

/// Connection failures, in plain words.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// The TLS configuration could not be built.
    #[error("could not set up a secure connection: {0}")]
    Config(String),
    /// The node could not be reached or the handshake failed (including
    /// a certificate that isn't the pinned key).
    #[error("couldn't connect securely to that Mac: {0}")]
    Connect(String),
    /// The node refused the Hello.
    #[error("{0}")]
    Refused(String),
}

/// An authenticated link to one node.
#[derive(Clone, Debug)]
pub struct LinkPeer {
    /// The gRPC client.
    pub client: proto::link_client::LinkClient<Channel>,
    /// The node's Hello answer.
    pub hello: proto::HelloResponse,
}

/// Connects to `addr`, expecting `expected` (the pinned key), and says
/// Hello as `machine_id`.
///
/// # Errors
///
/// Returns [`ConnectError`] naming the failure.
pub async fn connect(
    addr: SocketAddr,
    certificate: &NodeCertificate,
    expected: [u8; 32],
    machine_id: &str,
) -> Result<LinkPeer, ConnectError> {
    connect_with_version(
        addr,
        certificate,
        expected,
        machine_id,
        PROTOCOL_MAJOR,
        PROTOCOL_MINOR,
    )
    .await
}

/// [`connect`] advertising an explicit protocol version (tests and the
/// mixed-version matrix).
///
/// # Errors
///
/// Returns [`ConnectError`] naming the failure.
pub async fn connect_with_version(
    addr: SocketAddr,
    certificate: &NodeCertificate,
    expected: [u8; 32],
    machine_id: &str,
    major: u32,
    minor: u32,
) -> Result<LinkPeer, ConnectError> {
    let mut client = open_channel(addr, certificate, expected).await?;
    let hello = client
        .hello(proto::HelloRequest {
            version: Some(proto::ProtocolVersion { major, minor }),
            machine_id: machine_id.to_owned(),
            sent_at_ms: now_ms(),
        })
        .await
        .map_err(|status| match status.code() {
            tonic::Code::Unavailable | tonic::Code::Unknown => {
                ConnectError::Connect(status.message().to_owned())
            }
            _ => ConnectError::Refused(status.message().to_owned()),
        })?
        .into_inner();
    Ok(LinkPeer { client, hello })
}

/// The mutually-authenticated TLS 1.3 channel to a node pinned to
/// `expected`, without the Hello. The node authorizes every RPC on its
/// own (an unpaired caller gets nothing), so this is safe to expose; it
/// exists for diagnostics and the negative tests.
///
/// # Errors
///
/// Returns [`ConnectError`] when the node can't be reached or its key
/// isn't the pinned one.
pub async fn open_channel(
    addr: SocketAddr,
    certificate: &NodeCertificate,
    expected: [u8; 32],
) -> Result<proto::link_client::LinkClient<tonic::transport::Channel>, ConnectError> {
    let mut config = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| ConnectError::Config(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(ServerKeyVerifier::new(Expected::Pinned(
            expected,
        ))))
        .with_client_auth_cert(vec![certificate.cert.clone()], certificate.key.clone_key())
        .map_err(|e| ConnectError::Config(e.to_string()))?;
    config.alpn_protocols = vec![b"h2".to_vec()];
    config.resumption = rustls::client::Resumption::disabled();
    config.enable_early_data = false;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let channel = Endpoint::from_static("http://aijuggler.node")
        .connect_timeout(Duration::from_secs(10))
        .connect_with_connector(tower::service_fn(move |_uri| {
            let connector = connector.clone();
            async move {
                let tcp = tokio::net::TcpStream::connect(addr).await?;
                let name = ServerName::try_from(SERVER_NAME)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
                let tls = connector.connect(name, tcp).await?;
                Ok::<_, std::io::Error>(TokioIo::new(tls))
            }
        }))
        .await
        .map_err(|e| ConnectError::Connect(describe(&e)))?;
    Ok(proto::link_client::LinkClient::new(channel)
        .max_decoding_message_size(crate::node::MAX_MESSAGE_BYTES)
        .max_encoding_message_size(crate::node::MAX_MESSAGE_BYTES))
}

/// Sends one control command over `peer`'s connection and waits for
/// the node's verdict. The node re-checks everything (permission,
/// replay, expiry, that the record exists) and answers in its own words.
///
/// # Errors
///
/// Returns [`ConnectError`] when the command couldn't be delivered.
pub async fn send_command(
    peer: &LinkPeer,
    console_machine_id: &str,
    sequence: u64,
    action: proto::control_command::Action,
) -> Result<proto::CommandResult, ConnectError> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| ConnectError::Config(e.to_string()))?;
    let command = proto::ControlCommand {
        console_machine_id: console_machine_id.to_owned(),
        nonce: nonce.to_vec(),
        sequence,
        issued_at_ms: now_ms(),
        action: Some(action),
    };
    let mut client = peer.client.clone();
    let mut results = client
        .command(tokio_stream::iter(vec![command]))
        .await
        .map_err(|status| ConnectError::Connect(status.message().to_owned()))?
        .into_inner();
    match results.message().await {
        Ok(Some(result)) => Ok(result),
        Ok(None) => Err(ConnectError::Connect(
            "the node closed the stream".to_owned(),
        )),
        Err(status) => Err(ConnectError::Connect(status.message().to_owned())),
    }
}

/// The size of one handover chunk.
const HANDOVER_CHUNK: usize = 1024 * 1024;

/// AJ-E706: pushes a handover over `peer`'s connection — the command
/// (with every command check on the node), the bundle in 1 MiB chunks,
/// then its sha256 — and returns the node's final verdict (`done` or
/// `rejected`). `on_progress` sees each update; setting `cancel` to
/// true stops the transfer within one chunk (the node discards what it
/// received and reports nothing — the result is `phase = "cancelled"`).
///
/// # Errors
///
/// Returns [`ConnectError`] when the handover couldn't be delivered.
pub async fn push_handover(
    peer: &LinkPeer,
    console_machine_id: &str,
    sequence: u64,
    action: proto::control_command::Action,
    bundle: Vec<u8>,
    mut on_progress: impl FnMut(&proto::HandoverProgress) + Send,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) -> Result<proto::HandoverProgress, ConnectError> {
    use sha2::{Digest, Sha256};
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| ConnectError::Config(e.to_string()))?;
    let command = proto::ControlCommand {
        console_machine_id: console_machine_id.to_owned(),
        nonce: nonce.to_vec(),
        sequence,
        issued_at_ms: now_ms(),
        action: Some(action),
    };
    let digest = Sha256::digest(&bundle)
        .iter()
        .fold(String::new(), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        });
    let (parts, outbound) = tokio::sync::mpsc::channel::<proto::HandoverPart>(4);
    let mut feeder_cancel = cancel.clone();
    let feeder = tokio::spawn(async move {
        let part = |part| proto::HandoverPart { part: Some(part) };
        if parts
            .send(part(proto::handover_part::Part::Command(command)))
            .await
            .is_err()
        {
            return;
        }
        for chunk in bundle.chunks(HANDOVER_CHUNK) {
            if *feeder_cancel.borrow_and_update() {
                return; // dropping the sender ends the stream: cancelled
            }
            if parts
                .send(part(proto::handover_part::Part::Chunk(chunk.to_vec())))
                .await
                .is_err()
            {
                return;
            }
        }
        let _ = parts
            .send(part(proto::handover_part::Part::End(proto::HandoverEnd {
                sha256: digest,
            })))
            .await;
        // Keep the stream open until the node answers.
        let _ = feeder_cancel.changed().await;
    });
    let mut client = peer.client.clone();
    let response = client
        .handover(tokio_stream::wrappers::ReceiverStream::new(outbound))
        .await;
    let mut updates = match response {
        Ok(response) => response.into_inner(),
        Err(status) => {
            feeder.abort();
            return Err(ConnectError::Connect(status.message().to_owned()));
        }
    };
    let cancelled = proto::HandoverProgress {
        phase: "cancelled".to_owned(),
        ..proto::HandoverProgress::default()
    };
    loop {
        tokio::select! {
            next = updates.message() => match next {
                Ok(Some(update)) => {
                    on_progress(&update);
                    if update.phase == "done" || update.phase == "rejected" {
                        feeder.abort();
                        return Ok(update);
                    }
                }
                Ok(None) => {
                    feeder.abort();
                    if *cancel.borrow() {
                        return Ok(cancelled);
                    }
                    return Err(ConnectError::Connect("the node closed the handover".to_owned()));
                }
                Err(status) => {
                    feeder.abort();
                    return Err(ConnectError::Connect(status.message().to_owned()));
                }
            },
            changed = cancel.changed() => {
                if changed.is_err() || *cancel.borrow() {
                    feeder.abort();
                    return Ok(cancelled);
                }
            }
        }
    }
}

fn describe(error: &tonic::transport::Error) -> String {
    use std::error::Error as _;
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text = format!("{text}: {inner}");
        source = inner.source();
    }
    text
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

/// A peer's connection state, as the UI chip shows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Liveness {
    /// Heartbeats answered.
    Live,
    /// Two consecutive heartbeats missed.
    Offline,
}

/// Runs the heartbeat loop against `peer`, publishing state changes on
/// the returned watch channel. Dropping the returned handle stops it.
#[must_use]
pub fn watch_liveness(
    mut client: proto::link_client::LinkClient<Channel>,
) -> (
    tokio::sync::watch::Receiver<Liveness>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = tokio::sync::watch::channel(Liveness::Live);
    let task = tokio::spawn(async move {
        let mut misses = 0_u32;
        loop {
            let ping = proto::Ping {
                sent_at_ms: now_ms(),
            };
            let answered = tokio::time::timeout(HEARTBEAT_TIMEOUT, client.heartbeat(ping))
                .await
                .is_ok_and(|result| result.is_ok());
            misses = if answered { 0 } else { misses + 1 };
            let state = if misses >= MISSES_BEFORE_OFFLINE {
                Liveness::Offline
            } else {
                Liveness::Live
            };
            if *tx.borrow() != state && tx.send(state).is_err() {
                return;
            }
            tokio::time::sleep(HEARTBEAT_INTERVAL).await;
        }
    });
    (rx, task)
}
