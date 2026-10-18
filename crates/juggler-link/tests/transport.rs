//! AJ-E703 loopback two-node transport tests: mutual TLS against pinned
//! keys, and every security negative of the ADR-0013 architecture
//! contract that the transport owns.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_link::cert::{NodeCertificate, node_certificate};
use juggler_link::client::{self, ConnectError, Liveness};
use juggler_link::node::{
    CommandRejection, LinkAuditEvent, LinkNode, MemoryReplayStore, NodeHandler, PairCandidate,
    PairingHost, PeerAccess, PeerDirectory,
};
use juggler_link::pairing::{PairingCode, PairingError};
use juggler_link::protocol::{PROTOCOL_MAJOR, PROTOCOL_MINOR};
use juggler_link::{MachineIdentity, Permission, proto};

#[derive(Default)]
struct Directory(Mutex<HashMap<[u8; 32], PeerAccess>>);

impl Directory {
    fn pin(&self, key: [u8; 32], machine_id: &str, permission: Permission) {
        self.0.lock().unwrap().insert(
            key,
            PeerAccess {
                machine_id: machine_id.to_owned(),
                permission,
            },
        );
    }
    fn unpin(&self, key: &[u8; 32]) {
        self.0.lock().unwrap().remove(key);
    }
}

impl PeerDirectory for Directory {
    fn lookup(&self, key: &[u8; 32]) -> Option<PeerAccess> {
        self.0.lock().unwrap().get(key).cloned()
    }
}

struct Handler {
    live: tokio::sync::broadcast::Sender<proto::TelemetryDelta>,
    audit: Mutex<Vec<LinkAuditEvent>>,
    executed: Mutex<Vec<String>>,
}

impl Handler {
    fn new() -> Self {
        Self {
            live: tokio::sync::broadcast::channel(16).0,
            audit: Mutex::default(),
            executed: Mutex::default(),
        }
    }
}

fn delta(sequence: u64) -> proto::TelemetryDelta {
    proto::TelemetryDelta {
        origin_machine_id: "node-a".into(),
        sequence,
        hlc_ms: 1,
        hlc_counter: 0,
        payload: Some(proto::telemetry_delta::Payload::Resources(
            proto::MachineResources {
                cpu_percent: 3.5,
                memory_used_bytes: 1,
                memory_total_bytes: 2,
            },
        )),
    }
}

impl NodeHandler for Handler {
    fn local_machine_id(&self) -> String {
        "node-a".into()
    }
    fn telemetry_since(&self, since: u64) -> Vec<proto::TelemetryDelta> {
        (since + 1..=since + 2).map(delta).collect()
    }
    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta> {
        self.live.subscribe()
    }
    fn execute(
        &self,
        _peer: &PeerAccess,
        action: &proto::control_command::Action,
    ) -> Result<String, CommandRejection> {
        match action {
            proto::control_command::Action::StopInstance(stop) if stop.instance_id == "inst-1" => {
                self.executed.lock().unwrap().push(stop.instance_id.clone());
                Ok("Stopping".into())
            }
            _ => Err(CommandRejection {
                reason_code: "unknown_record".into(),
                message: "No such Instance on this Mac.".into(),
            }),
        }
    }
    fn audit(&self, event: LinkAuditEvent) {
        self.audit.lock().unwrap().push(event);
    }
}

/// Transport tests never open a pairing window.
struct NoPairing;

impl PairingHost for NoPairing {
    fn begin(&self) -> Result<PairingCode, PairingError> {
        Err(PairingError::NotOpen)
    }
    fn failed(&self) {}
    fn released(&self) {}
    fn local_label(&self) -> String {
        "Node A".into()
    }
    fn local_key(&self) -> [u8; 32] {
        [0; 32]
    }
    fn confirm(
        &self,
        _candidate: PairCandidate,
    ) -> tokio::sync::oneshot::Receiver<Option<Permission>> {
        tokio::sync::oneshot::channel().1
    }
    fn succeeded(&self, _candidate: &PairCandidate, _permission: Permission) {}
}

struct Fixture {
    _dir: tempfile::TempDir,
    node: LinkNode,
    node_key: [u8; 32],
    handler: Arc<Handler>,
    directory: Arc<Directory>,
    console_cert: NodeCertificate,
    console_key: [u8; 32],
}

async fn fixture(permission: Permission) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let node_identity = MachineIdentity::load_or_create(&dir.path().join("a")).unwrap();
    let console_identity = MachineIdentity::load_or_create(&dir.path().join("b")).unwrap();
    let handler = Arc::new(Handler::new());
    let directory = Arc::new(Directory::default());
    directory.pin(
        console_identity.public().to_bytes(),
        "console-b",
        permission,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let node = LinkNode::start(
        listener,
        &node_certificate(&node_identity).unwrap(),
        handler.clone(),
        directory.clone(),
        Arc::new(MemoryReplayStore::default()),
        Arc::new(NoPairing),
    )
    .unwrap();
    Fixture {
        node_key: node_identity.public().to_bytes(),
        console_cert: node_certificate(&console_identity).unwrap(),
        console_key: console_identity.public().to_bytes(),
        _dir: dir,
        node,
        handler,
        directory,
    }
}

impl Fixture {
    async fn connect(&self) -> client::LinkPeer {
        client::connect(
            self.node.addr(),
            &self.console_cert,
            self.node_key,
            "console-b",
        )
        .await
        .unwrap()
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn stop(sequence: u64, nonce: u8) -> proto::ControlCommand {
    proto::ControlCommand {
        console_machine_id: "console-b".into(),
        nonce: vec![nonce; 16],
        sequence,
        issued_at_ms: now_ms(),
        action: Some(proto::control_command::Action::StopInstance(
            proto::StopInstance {
                instance_id: "inst-1".into(),
            },
        )),
    }
}

async fn run(
    peer: &mut client::LinkPeer,
    commands: Vec<proto::ControlCommand>,
) -> Vec<proto::CommandResult> {
    let mut stream = peer
        .client
        .command(tokio_stream::iter(commands))
        .await
        .unwrap()
        .into_inner();
    let mut results = Vec::new();
    while let Some(result) = stream.message().await.unwrap() {
        results.push(result);
    }
    results
}

async fn heartbeat(peer: &mut client::LinkPeer) {
    peer.client
        .heartbeat(proto::Ping {
            sent_at_ms: now_ms(),
        })
        .await
        .unwrap();
}

fn codes(results: &[proto::CommandResult]) -> Vec<String> {
    results
        .iter()
        .map(|r| {
            if r.outcome == "accepted" {
                "accepted".to_owned()
            } else {
                r.reason_code.clone()
            }
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn mutual_tls_with_pinned_keys_connects_and_serves() {
    let f = fixture(Permission::Control).await;
    let started = Instant::now();
    let mut peer = f.connect().await;
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "LAN handshake budget"
    );
    assert_eq!(peer.hello.machine_id, "node-a");
    assert_eq!(peer.hello.service_level, "full");
    assert_eq!(peer.hello.permission, "control");
    // Heartbeat RTT budget: p95 < 50 ms on loopback (LAN budget).
    let mut rtts: Vec<Duration> = Vec::new();
    for _ in 0..40 {
        let sent = Instant::now();
        heartbeat(&mut peer).await;
        rtts.push(sent.elapsed());
    }
    rtts.sort();
    let p95 = rtts[(rtts.len() * 95) / 100 - 1];
    println!("[perf] heartbeat RTT p95 = {p95:?}");
    assert!(p95 < Duration::from_millis(50), "p95 {p95:?}");
    assert_eq!(codes(&run(&mut peer, vec![stop(1, 1)]).await), ["accepted"]);
    assert_eq!(*f.handler.executed.lock().unwrap(), ["inst-1"]);
    // Telemetry: the backlog, then a live item.
    let mut telemetry = peer
        .client
        .telemetry(proto::TelemetryRequest { since_sequence: 0 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(telemetry.message().await.unwrap().unwrap().sequence, 1);
    assert_eq!(telemetry.message().await.unwrap().unwrap().sequence, 2);
    f.handler.live.send(delta(3)).unwrap();
    assert_eq!(telemetry.message().await.unwrap().unwrap().sequence, 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_presenting_an_unpinned_key_is_refused_at_tls() {
    let f = fixture(Permission::Control).await;
    let wrong = [7_u8; 32];
    let error = client::connect(f.node.addr(), &f.console_cert, wrong, "console-b")
        .await
        .unwrap_err();
    assert!(matches!(error, ConnectError::Connect(_)), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unpaired_console_and_a_machine_id_mismatch_are_refused() {
    let f = fixture(Permission::Control).await;
    let dir = tempfile::tempdir().unwrap();
    let stranger = node_certificate(&MachineIdentity::load_or_create(dir.path()).unwrap()).unwrap();
    let error = client::connect(f.node.addr(), &stranger, f.node_key, "console-b")
        .await
        .unwrap_err();
    assert!(
        matches!(&error, ConnectError::Refused(m) if m.contains("isn't paired")),
        "{error}"
    );
    // The pinned key, claiming another machine's id.
    let error = client::connect(f.node.addr(), &f.console_cert, f.node_key, "someone-else")
        .await
        .unwrap_err();
    assert!(
        matches!(&error, ConnectError::Refused(m) if m.contains("mismatch")),
        "{error}"
    );
    let audit = f.handler.audit.lock().unwrap();
    assert!(audit.iter().any(|e| e.reason_code == "not_paired"));
    assert!(audit.iter().any(|e| e.reason_code == "machine_mismatch"));
}

#[tokio::test(flavor = "multi_thread")]
async fn tls_1_2_is_refused() {
    use rustls::pki_types::ServerName;
    let f = fixture(Permission::Control).await;
    let config = rustls::ClientConfig::builder_with_provider(juggler_link::verify::provider())
        .with_protocol_versions(&[&rustls::version::TLS12])
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(juggler_link::verify::ServerKeyVerifier::new(
            juggler_link::verify::Expected::Pinned(f.node_key),
        )))
        .with_client_auth_cert(
            vec![f.console_cert.cert.clone()],
            f.console_cert.key.clone_key(),
        )
        .unwrap();
    let tcp = tokio::net::TcpStream::connect(f.node.addr()).await.unwrap();
    let result = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("aijuggler.node").unwrap(), tcp)
        .await;
    assert!(result.is_err(), "a TLS 1.2-only client must not connect");
}

#[tokio::test(flavor = "multi_thread")]
async fn replay_reorder_expiry_wrong_console_and_unmeasured_clock_are_rejected() {
    let f = fixture(Permission::Control).await;
    let mut peer = f.connect().await;
    // No heartbeat yet: the node can't convert the console's clock.
    assert_eq!(
        codes(&run(&mut peer, vec![stop(1, 1)]).await),
        ["clock_unmeasured"]
    );
    heartbeat(&mut peer).await;
    let mut expired = stop(3, 3);
    expired.issued_at_ms -= 60_000;
    let mut impostor = stop(4, 4);
    impostor.console_machine_id = "node-z".into();
    let results = run(
        &mut peer,
        vec![
            stop(2, 2), // accepted
            stop(2, 9), // same sequence → replayed
            stop(1, 8), // lower sequence → replayed
            expired,    // expired on the node's clock
            impostor,   // names a different console than the TLS peer
            stop(5, 2), // fresh sequence, reused nonce → replayed
        ],
    )
    .await;
    assert_eq!(
        codes(&results),
        [
            "accepted",
            "replayed",
            "replayed",
            "expired",
            "wrong_console",
            "replayed"
        ]
    );
    assert!(
        f.handler
            .audit
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.kind == "command_rejected")
            .count()
            >= 6
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn view_only_peers_get_telemetry_but_no_commands() {
    let f = fixture(Permission::View).await;
    let mut peer = f.connect().await;
    assert_eq!(peer.hello.permission, "view");
    heartbeat(&mut peer).await;
    assert_eq!(
        codes(&run(&mut peer, vec![stop(1, 1)]).await),
        ["permission_denied"]
    );
    assert!(f.handler.executed.lock().unwrap().is_empty());
    let mut telemetry = peer
        .client
        .telemetry(proto::TelemetryRequest { since_sequence: 0 })
        .await
        .unwrap()
        .into_inner();
    assert!(telemetry.message().await.unwrap().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn unpairing_revokes_an_open_command_stream_at_the_next_command() {
    let f = fixture(Permission::Control).await;
    let mut peer = f.connect().await;
    heartbeat(&mut peer).await;
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let mut results = peer
        .client
        .command(tokio_stream::wrappers::ReceiverStream::new(rx))
        .await
        .unwrap()
        .into_inner();
    tx.send(stop(1, 1)).await.unwrap();
    assert_eq!(
        results.message().await.unwrap().unwrap().outcome,
        "accepted"
    );
    let revoked_at = Instant::now();
    f.directory.unpin(&f.console_key);
    tx.send(stop(2, 2)).await.unwrap();
    let next = results.message().await.unwrap().unwrap();
    assert_eq!(next.reason_code, "not_authorised");
    assert!(revoked_at.elapsed() < Duration::from_secs(2));
    // A permission downgrade applies the same way to new calls.
    assert!(
        peer.client
            .heartbeat(proto::Ping {
                sent_at_ms: now_ms()
            })
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_mixed_version_matrix() {
    let f = fixture(Permission::Control).await;
    // One minor apart → telemetry only, labelled; commands refused.
    let mut degraded = client::connect_with_version(
        f.node.addr(),
        &f.console_cert,
        f.node_key,
        "console-b",
        PROTOCOL_MAJOR,
        PROTOCOL_MINOR + 1,
    )
    .await
    .unwrap();
    assert_eq!(degraded.hello.service_level, "degraded");
    assert!(degraded.hello.degraded_reason.contains("telemetry only"));
    heartbeat(&mut degraded).await;
    assert_eq!(
        codes(&run(&mut degraded, vec![stop(1, 1)]).await),
        ["degraded"]
    );
    let mut telemetry = degraded
        .client
        .telemetry(proto::TelemetryRequest { since_sequence: 0 })
        .await
        .unwrap()
        .into_inner();
    assert!(telemetry.message().await.unwrap().is_some());
    // Two minors apart, or another major → refused with a reason.
    for (major, minor) in [
        (PROTOCOL_MAJOR, PROTOCOL_MINOR + 2),
        (PROTOCOL_MAJOR + 1, 0),
    ] {
        let error = client::connect_with_version(
            f.node.addr(),
            &f.console_cert,
            f.node_key,
            "console-b",
            major,
            minor,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&error, ConnectError::Refused(m) if m.contains("Update")),
            "{error}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn command_floods_are_rate_limited_and_oversize_messages_refused() {
    let f = fixture(Permission::Control).await;
    let mut peer = f.connect().await;
    heartbeat(&mut peer).await;
    let burst: Vec<_> = (1..=30).map(|n| stop(n, n as u8)).collect();
    let results = codes(&run(&mut peer, burst).await);
    assert!(results.iter().any(|c| c == "rate_limited"), "{results:?}");
    assert!(results.iter().filter(|c| *c == "accepted").count() <= 21);
    // A 5 MiB message exceeds the NODE's 4 MiB decode limit (the
    // client's own limit is lifted so the node is what refuses).
    let mut huge = stop(100, 100);
    huge.nonce = vec![0; 5 * 1024 * 1024];
    let mut unlimited = peer.client.clone().max_encoding_message_size(usize::MAX);
    let outcome = match unlimited.command(tokio_stream::iter(vec![huge])).await {
        Err(status) => Err(status),
        Ok(response) => response.into_inner().message().await,
    };
    let status = outcome.expect_err("the node refuses an oversize message");
    assert!(
        matches!(
            status.code(),
            tonic::Code::ResourceExhausted | tonic::Code::OutOfRange | tonic::Code::Internal
        ),
        "{status:?}"
    );
    assert!(
        f.handler
            .executed
            .lock()
            .unwrap()
            .iter()
            .all(|id| id == "inst-1")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stalled_handshakes_are_capped_before_any_tls_work() {
    let f = fixture(Permission::Control).await;
    // Eight raw sockets that never speak TLS hold every handshake slot.
    let mut stalled = Vec::new();
    for _ in 0..8 {
        stalled.push(tokio::net::TcpStream::connect(f.node.addr()).await.unwrap());
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let blocked = tokio::time::timeout(
        Duration::from_secs(3),
        client::connect(f.node.addr(), &f.console_cert, f.node_key, "console-b"),
    )
    .await;
    assert!(
        !matches!(blocked, Ok(Ok(_))),
        "a ninth handshake is dropped while eight are in flight"
    );
    drop(stalled);
    tokio::time::sleep(Duration::from_millis(200)).await;
    f.connect().await; // the slots free up
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dead_node_is_reported_offline_within_five_seconds() {
    let f = fixture(Permission::View).await;
    let peer = f.connect().await;
    let (mut state, _task) = client::watch_liveness(peer.client.clone());
    assert_eq!(*state.borrow(), Liveness::Live);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let stopped_at = Instant::now();
    f.node.stop().await;
    tokio::time::timeout(Duration::from_secs(6), async {
        while *state.borrow() != Liveness::Offline {
            state.changed().await.unwrap();
        }
    })
    .await
    .expect("offline within the budget");
    assert!(
        stopped_at.elapsed() <= Duration::from_secs(6),
        "{:?}",
        stopped_at.elapsed()
    );
}
