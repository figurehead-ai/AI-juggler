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
    let mut client = proto::link_client::LinkClient::new(channel)
        .max_decoding_message_size(crate::node::MAX_MESSAGE_BYTES)
        .max_encoding_message_size(crate::node::MAX_MESSAGE_BYTES);
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
