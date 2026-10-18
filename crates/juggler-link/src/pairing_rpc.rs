//! The pairing exchange over the link (AJ-E704; ADR-0013 contract).
//!
//! Host (node) side: `serve` runs one session behind the `Pair` RPC.
//! Joiner (client) side: [`pair_with`] connects with an UNPINNED
//! verifier, records the key and TLS exporter it actually saw, runs the
//! SPAKE2 exchange, checks the host's confirmation, asks its own user,
//! and returns the candidate to pin only when both users committed.
//!
//! Step order: Start (joiner) → Challenge (host) → Confirm (joiner) →
//! Confirm (host) → Decision (host) ↔ Decision (joiner). A wrong code
//! or malformed step counts against the host's lockout; a cancel or a
//! timeout does not.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rustls::pki_types::ServerName;
use tokio_stream::StreamExt;
use tonic::transport::Endpoint;
use tonic::{Status, Streaming};

use crate::cert::{NodeCertificate, ed25519_key_of};
use crate::identity::fingerprint_of;
use crate::node::{LinkAuditEvent, LinkConnInfo, NodeHandler, PairCandidate, PairingHost};
use crate::pairing::{
    EXPORTER_LABEL, PAIRING_WINDOW_MS, PairingCode, PairingError, PairingSession, Role, Transcript,
};
use crate::proto::{self, pair_message::Step};
use crate::protocol::{PROTOCOL_MAJOR, PROTOCOL_MINOR};
use crate::registry::Permission;
use crate::verify::{Expected, ServerKeyVerifier, provider};

type Outbound = tokio::sync::mpsc::Sender<Result<proto::PairMessage, Status>>;

/// The server key and TLS exporter a pairing connection actually used.
type SeenSession = ([u8; 32], [u8; 32]);

fn window() -> Duration {
    Duration::from_millis(u64::try_from(PAIRING_WINDOW_MS).unwrap_or(120_000))
}

fn failure_code(error: &PairingError) -> &'static str {
    error.code()
}

fn message(step: Step) -> proto::PairMessage {
    proto::PairMessage { step: Some(step) }
}

/// How a host-side session ended.
enum HostOutcome {
    Paired,
    /// Counts against the lockout (wrong code, malformed).
    Failed(PairingError),
    /// Refused before any guess (not open, busy, locked, version).
    Refused(PairingError),
    /// A user cancelled, or the other side went away.
    Cancelled,
}

async fn next_step(inbound: &mut Streaming<proto::PairMessage>) -> Option<Step> {
    inbound
        .next()
        .await
        .and_then(Result::ok)
        .and_then(|m| m.step)
}

/// Runs one host-side session; returns the outbound stream receiver.
pub(crate) fn serve<P: PairingHost, H: NodeHandler>(
    conn: LinkConnInfo,
    mut inbound: Streaming<proto::PairMessage>,
    pairing: Arc<P>,
    handler: Arc<H>,
) -> tokio::sync::mpsc::Receiver<Result<proto::PairMessage, Status>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    tokio::spawn(async move {
        let local = handler.local_machine_id();
        let outcome = tokio::time::timeout(
            window(),
            host(&conn, &mut inbound, &sender, pairing.as_ref(), &local),
        )
        .await;
        let reason = match outcome {
            Ok(HostOutcome::Paired) => "paired".to_owned(),
            Ok(HostOutcome::Failed(error)) => {
                pairing.failed();
                let _ = sender.send(Ok(failure(&error))).await;
                failure_code(&error).to_owned()
            }
            Ok(HostOutcome::Refused(error)) => {
                let _ = sender.send(Ok(failure(&error))).await;
                failure_code(&error).to_owned()
            }
            Ok(HostOutcome::Cancelled) => {
                pairing.released();
                "cancelled".to_owned()
            }
            Err(_) => {
                pairing.released();
                let _ = sender.send(Ok(failure(&PairingError::Expired))).await;
                "expired".to_owned()
            }
        };
        // The attempt log: time, peer (by key fingerprint), outcome —
        // never the code that was entered.
        handler.audit(LinkAuditEvent {
            peer: fingerprint_of(&conn.peer_key),
            kind: "pairing".to_owned(),
            reason_code: reason,
            sequence: None,
            count: None,
        });
    });
    receiver
}

fn failure(error: &PairingError) -> proto::PairMessage {
    message(Step::Failure(proto::PairFailure {
        reason_code: failure_code(error).to_owned(),
        message: error.to_string(),
    }))
}

async fn host<P: PairingHost>(
    conn: &LinkConnInfo,
    inbound: &mut Streaming<proto::PairMessage>,
    sender: &Outbound,
    pairing: &P,
    local_machine_id: &str,
) -> HostOutcome {
    let Some(Step::Start(start)) = next_step(inbound).await else {
        return HostOutcome::Cancelled;
    };
    let version = start.version.unwrap_or_default();
    if (version.major, version.minor) != (PROTOCOL_MAJOR, PROTOCOL_MINOR)
        || start.machine_id.is_empty()
        || start.machine_id.len() > 128
        || start.label.chars().count() > 64
    {
        return HostOutcome::Refused(PairingError::Malformed);
    }
    let code = match pairing.begin() {
        Ok(code) => code,
        Err(error) => return HostOutcome::Refused(error),
    };
    let (mut session, host_message) = PairingSession::start(Role::Host, &code);
    let transcript = Transcript {
        host_key: pairing.local_key(),
        joiner_key: conn.peer_key,
        host_machine_id: local_machine_id.to_owned(),
        joiner_machine_id: start.machine_id.clone(),
        protocol: (PROTOCOL_MAJOR, PROTOCOL_MINOR),
        tls_exporter: conn.exporter,
    };
    let Ok(host_confirm) = session.finish(&start.spake_message, &transcript) else {
        return HostOutcome::Failed(PairingError::Malformed);
    };
    let challenge = Step::Challenge(proto::PairChallenge {
        machine_id: local_machine_id.to_owned(),
        label: pairing.local_label(),
        spake_message: host_message,
    });
    if sender.send(Ok(message(challenge))).await.is_err() {
        return HostOutcome::Cancelled;
    }
    let Some(Step::Confirm(joiner_confirm)) = next_step(inbound).await else {
        return HostOutcome::Failed(PairingError::Malformed);
    };
    let Ok(mac) = <[u8; 32]>::try_from(joiner_confirm.mac.as_slice()) else {
        return HostOutcome::Failed(PairingError::Malformed);
    };
    if let Err(error) = session.verify(&mac, &transcript) {
        return HostOutcome::Failed(error);
    }
    let confirm = Step::Confirm(proto::PairConfirm {
        mac: host_confirm.to_vec(),
    });
    if sender.send(Ok(message(confirm))).await.is_err() {
        return HostOutcome::Cancelled;
    }
    let reach = u16::try_from(start.listen_port)
        .ok()
        .filter(|port| *port != 0)
        .map(|port| std::net::SocketAddr::new(conn.addr.ip(), port));
    let candidate = PairCandidate {
        machine_id: start.machine_id,
        label: start.label,
        key: conn.peer_key,
        fingerprint: fingerprint_of(&conn.peer_key),
        reach,
    };
    let ours = pairing.confirm(candidate.clone()).await.ok().flatten();
    let decision = Step::Decision(proto::PairDecision {
        commit: ours.is_some(),
    });
    if sender.send(Ok(message(decision))).await.is_err() {
        return HostOutcome::Cancelled;
    }
    let Some(Step::Decision(theirs)) = next_step(inbound).await else {
        return HostOutcome::Cancelled;
    };
    match ours {
        Some(permission) if theirs.commit => {
            pairing.succeeded(&candidate, permission);
            HostOutcome::Paired
        }
        _ => HostOutcome::Cancelled,
    }
}

async fn joiner_next(inbound: &mut Streaming<proto::PairMessage>) -> Result<Step, JoinError> {
    match next_step(inbound).await {
        Some(Step::Failure(failed)) => Err(JoinError::Host {
            reason_code: failed.reason_code,
            message: failed.message,
        }),
        Some(step) => Ok(step),
        None => Err(JoinError::Cancelled),
    }
}

/// What the joiner typed or pasted: the code, optionally with the host's
/// fingerprint (the pairing link the host's QR / Copy carries).
#[derive(Clone, Debug)]
pub struct PairingInput {
    code: PairingCode,
    expected_fingerprint: Option<String>,
}

impl PairingInput {
    /// The link the host shares: `aijuggler-pair:v1:<code>:<fingerprint>`.
    #[must_use]
    pub fn link(code: &PairingCode, host_fingerprint: &str) -> String {
        format!(
            "aijuggler-pair:v1:{}:{}",
            code.display().replace(' ', ""),
            host_fingerprint.replace(' ', "")
        )
    }

    /// Parses six digits, or a pairing link.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Malformed`] for anything else.
    pub fn parse(typed: &str) -> Result<Self, PairingError> {
        let trimmed = typed.trim();
        if let Some(rest) = trimmed.strip_prefix("aijuggler-pair:v1:") {
            let (code, fingerprint) = rest.split_once(':').ok_or(PairingError::Malformed)?;
            let fingerprint = fingerprint.to_ascii_uppercase();
            if fingerprint.len() != 32 || !fingerprint.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(PairingError::Malformed);
            }
            return Ok(Self {
                code: PairingCode::parse(code)?,
                expected_fingerprint: Some(fingerprint),
            });
        }
        Ok(Self {
            code: PairingCode::parse(trimmed)?,
            expected_fingerprint: None,
        })
    }
}

/// Why joining failed, in plain words.
#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    /// Couldn't connect.
    #[error("couldn't reach that Mac: {0}")]
    Connect(String),
    /// The host's key doesn't match the pairing link's fingerprint.
    #[error(
        "That Mac's fingerprint doesn't match the pairing link — don't pair; something may be intercepting."
    )]
    FingerprintMismatch,
    /// The host refused or the exchange failed.
    #[error("{0}")]
    Pairing(PairingError),
    /// The host reported a failure.
    #[error("{message}")]
    Host {
        /// The host's reason code (see [`PairingError::code`]).
        reason_code: String,
        /// The host's plain-words message.
        message: String,
    },
    /// A user cancelled on either side.
    #[error("Pairing was cancelled.")]
    Cancelled,
}

impl JoinError {
    /// A stable reason code for the UI: `unreachable`, `fingerprint`,
    /// `cancelled`, or the host's [`PairingError::code`].
    #[must_use]
    pub fn reason_code(&self) -> &str {
        match self {
            Self::Connect(_) => "unreachable",
            Self::FingerprintMismatch => "fingerprint",
            Self::Pairing(error) => error.code(),
            Self::Host { reason_code, .. } => reason_code,
            Self::Cancelled => "cancelled",
        }
    }
}

/// A host the joiner may pin (after both users committed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinedHost {
    /// The host as a candidate (key = the one seen in TLS).
    pub candidate: PairCandidate,
    /// What THIS machine's user chose to grant the host.
    pub permission: Permission,
}

/// Opens the pairing connection: TLS 1.3 with an UNPINNED verifier,
/// returning the channel plus the server key and exporter this session
/// actually used (what the transcript must bind).
async fn connect_unpinned(
    addr: SocketAddr,
    certificate: &NodeCertificate,
) -> Result<(tonic::transport::Channel, [u8; 32], [u8; 32]), JoinError> {
    let seen: Arc<Mutex<Option<SeenSession>>> = Arc::default();
    let mut config = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| JoinError::Connect(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(ServerKeyVerifier::new(Expected::AnyForPairing)))
        .with_client_auth_cert(vec![certificate.cert.clone()], certificate.key.clone_key())
        .map_err(|e| JoinError::Connect(e.to_string()))?;
    config.alpn_protocols = vec![b"h2".to_vec()];
    config.resumption = rustls::client::Resumption::disabled();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let slot = seen.clone();
    let channel = Endpoint::from_static("http://aijuggler.node")
        .connect_timeout(Duration::from_secs(10))
        .connect_with_connector(tower::service_fn(move |_uri| {
            let connector = connector.clone();
            let slot = slot.clone();
            async move {
                let tcp = tokio::net::TcpStream::connect(addr).await?;
                let name = ServerName::try_from("aijuggler.node")
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
                let tls = connector.connect(name, tcp).await?;
                let connection = tls.get_ref().1;
                let key = connection
                    .peer_certificates()
                    .and_then(|certs| certs.first())
                    .and_then(|cert| ed25519_key_of(cert).ok())
                    .ok_or_else(|| std::io::Error::other("no Ed25519 certificate"))?;
                let exporter = connection
                    .export_keying_material([0_u8; 32], EXPORTER_LABEL, None)
                    .map_err(std::io::Error::other)?;
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some((key, exporter));
                }
                Ok::<_, std::io::Error>(TokioIo::new(tls))
            }
        }))
        .await
        .map_err(|e| JoinError::Connect(e.to_string()))?;
    let (host_key, exporter) = seen
        .lock()
        .ok()
        .and_then(|slot| *slot)
        .ok_or_else(|| JoinError::Connect("no TLS session".to_owned()))?;
    Ok((channel, host_key, exporter))
}

/// Who is joining: this node's certificate, key, id, label, and the
/// port its own node listens on (0 = not listening).
#[derive(Clone, Copy, Debug)]
pub struct Joiner<'a> {
    /// This node's TLS certificate.
    pub certificate: &'a NodeCertificate,
    /// This node's identity public key.
    pub key: [u8; 32],
    /// This node's `MachineId`.
    pub machine_id: &'a str,
    /// This node's label (shown on the host).
    pub label: &'a str,
    /// This node's listening port.
    pub port: u16,
}

/// Joins the host at `addr` as `joiner` with `input`. `confirm` shows the host's
/// label + fingerprint to this Mac's user and resolves to the permission
/// to grant, or `None` for Cancel.
///
/// # Errors
///
/// Returns [`JoinError`] naming the failure.
pub async fn pair_with<F, Fut>(
    addr: SocketAddr,
    joiner: Joiner<'_>,
    input: &PairingInput,
    confirm: F,
) -> Result<JoinedHost, JoinError>
where
    F: FnOnce(PairCandidate) -> Fut,
    Fut: std::future::Future<Output = Option<Permission>>,
{
    let Joiner {
        certificate,
        key: own_key,
        machine_id: own_machine_id,
        label: own_label,
        port: own_port,
    } = joiner;
    let (channel, host_key, exporter) = connect_unpinned(addr, certificate).await?;
    if let Some(expected) = &input.expected_fingerprint
        && fingerprint_of(&host_key).replace(' ', "") != *expected
    {
        return Err(JoinError::FingerprintMismatch);
    }

    tokio::time::timeout(window(), async {
        let (tx, rx) = tokio::sync::mpsc::channel::<proto::PairMessage>(4);
        let mut client = proto::link_client::LinkClient::new(channel);
        let (mut session, joiner_message) = PairingSession::start(Role::Joiner, &input.code);
        tx.send(message(Step::Start(proto::PairStart {
            machine_id: own_machine_id.to_owned(),
            label: own_label.to_owned(),
            spake_message: joiner_message,
            version: Some(proto::ProtocolVersion {
                major: PROTOCOL_MAJOR,
                minor: PROTOCOL_MINOR,
            }),
            listen_port: u32::from(own_port),
        })))
        .await
        .map_err(|_| JoinError::Cancelled)?;
        let mut inbound = client
            .pair(tokio_stream::wrappers::ReceiverStream::new(rx))
            .await
            .map_err(|status| JoinError::Connect(status.message().to_owned()))?
            .into_inner();
        let Step::Challenge(challenge) = joiner_next(&mut inbound).await? else {
            return Err(JoinError::Pairing(PairingError::Malformed));
        };
        let transcript = Transcript {
            host_key,
            joiner_key: own_key,
            host_machine_id: challenge.machine_id.clone(),
            joiner_machine_id: own_machine_id.to_owned(),
            protocol: (PROTOCOL_MAJOR, PROTOCOL_MINOR),
            tls_exporter: exporter,
        };
        let joiner_confirm = session
            .finish(&challenge.spake_message, &transcript)
            .map_err(JoinError::Pairing)?;
        tx.send(message(Step::Confirm(proto::PairConfirm {
            mac: joiner_confirm.to_vec(),
        })))
        .await
        .map_err(|_| JoinError::Cancelled)?;
        let Step::Confirm(host_confirm) = joiner_next(&mut inbound).await? else {
            return Err(JoinError::Pairing(PairingError::Malformed));
        };
        let mac = <[u8; 32]>::try_from(host_confirm.mac.as_slice())
            .map_err(|_| JoinError::Pairing(PairingError::Malformed))?;
        session
            .verify(&mac, &transcript)
            .map_err(JoinError::Pairing)?;
        let candidate = PairCandidate {
            machine_id: challenge.machine_id,
            label: challenge.label,
            key: host_key,
            fingerprint: fingerprint_of(&host_key),
            reach: Some(addr),
        };
        let ours = confirm(candidate.clone()).await;
        tx.send(message(Step::Decision(proto::PairDecision {
            commit: ours.is_some(),
        })))
        .await
        .map_err(|_| JoinError::Cancelled)?;
        let Step::Decision(theirs) = joiner_next(&mut inbound).await? else {
            return Err(JoinError::Pairing(PairingError::Malformed));
        };
        // Keep our side open until the host ends the stream: that is the
        // proof it received our decision (and pinned, when both
        // committed). Dropping early could lose our last message.
        let drained = tokio::time::timeout(Duration::from_secs(5), async {
            while next_step(&mut inbound).await.is_some() {}
        })
        .await;
        drop(tx);
        if drained.is_err() {
            return Err(JoinError::Cancelled);
        }
        match ours {
            Some(permission) if theirs.commit => Ok(JoinedHost {
                candidate,
                permission,
            }),
            _ => Err(JoinError::Cancelled),
        }
    })
    .await
    .map_err(|_| JoinError::Pairing(PairingError::Expired))?
}
