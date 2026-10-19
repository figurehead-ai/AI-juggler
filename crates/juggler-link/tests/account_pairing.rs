//! AJ-E1206 loopback: Macs paired through the account directory.
//!
//! A key from the directory is a candidate; the user's confirmation pins
//! it under a provisional id (`acct-…`). The first Hello over mutual TLS,
//! which proves the caller holds that key, binds the peer's real
//! `MachineId`, once. Nothing else can use the bind: a code-paired peer,
//! an already-bound peer, or a key nobody pinned is refused, and the
//! registry is unchanged.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::sync::{Arc, Mutex};

use juggler_link::cert::node_certificate;
use juggler_link::client;
use juggler_link::node::{
    CommandRejection, LinkAuditEvent, LinkNode, MemoryReplayStore, NodeHandler, PairCandidate,
    PairingHost, PeerAccess, PeerDirectory,
};
use juggler_link::pairing::{PairingCode, PairingError};
use juggler_link::{
    MachineIdentity, PROVISIONAL_PREFIX, PairedVia, Peer, PeerRegistry, Permission, PublicIdentity,
    proto,
};

/// A directory backed by a real signed registry.
struct RegistryDirectory {
    identity: MachineIdentity,
    registry: Mutex<PeerRegistry>,
}

impl PeerDirectory for RegistryDirectory {
    fn lookup(&self, key: &[u8; 32]) -> Option<PeerAccess> {
        let key = PublicIdentity::from_bytes(key).ok()?;
        self.registry
            .lock()
            .unwrap()
            .pinned(&key)
            .map(|peer| PeerAccess {
                machine_id: peer.machine_id.clone(),
                permission: peer.permission,
            })
    }
    fn bind_account_peer(&self, key: &[u8; 32], machine_id: &str) -> Option<PeerAccess> {
        let key = PublicIdentity::from_bytes(key).ok()?;
        self.registry
            .lock()
            .unwrap()
            .bind_account_peer(&self.identity, &key, machine_id)
            .ok()
            .flatten()
            .map(|peer| PeerAccess {
                machine_id: peer.machine_id,
                permission: peer.permission,
            })
    }
}

#[derive(Default)]
struct Handler {
    audit: Mutex<Vec<LinkAuditEvent>>,
}

impl NodeHandler for Handler {
    fn local_machine_id(&self) -> String {
        "node-real".into()
    }
    fn telemetry_since(&self, _since: u64) -> Vec<proto::TelemetryDelta> {
        Vec::new()
    }
    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta> {
        tokio::sync::broadcast::channel(1).1
    }
    fn execute(
        &self,
        _peer: &PeerAccess,
        _action: &proto::control_command::Action,
    ) -> Result<String, CommandRejection> {
        Err(CommandRejection {
            reason_code: "unknown_record".into(),
            message: "none".into(),
        })
    }
    fn audit(&self, event: LinkAuditEvent) {
        self.audit.lock().unwrap().push(event);
    }
}

struct NoPairing;

impl PairingHost for NoPairing {
    fn begin(&self) -> Result<PairingCode, PairingError> {
        Err(PairingError::NotOpen)
    }
    fn failed(&self) {}
    fn released(&self) {}
    fn local_label(&self) -> String {
        "Node".into()
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

fn peer(machine_id: &str, key: &PublicIdentity, via: PairedVia) -> Peer {
    Peer {
        machine_id: machine_id.to_owned(),
        label: "Mac mini".to_owned(),
        public_key: key.to_base64(),
        fingerprint: key.fingerprint(),
        permission: Permission::View,
        ask_each_time: false,
        paired_via: via,
        paired_at_ms: 1,
        last_seen_ms: None,
    }
}

struct Node {
    _dir: tempfile::TempDir,
    node: LinkNode,
    key: [u8; 32],
    directory: Arc<RegistryDirectory>,
    handler: Arc<Handler>,
}

async fn node_trusting(pins: Vec<Peer>) -> Node {
    let dir = tempfile::tempdir().unwrap();
    let identity = MachineIdentity::load_or_create(dir.path()).unwrap();
    let mut registry = PeerRegistry::load(dir.path(), &identity, None);
    for pin in pins {
        registry.pin(&identity, pin).unwrap();
    }
    let key = identity.public().to_bytes();
    let certificate = node_certificate(&identity).unwrap();
    let directory = Arc::new(RegistryDirectory {
        identity,
        registry: Mutex::new(registry),
    });
    let handler = Arc::new(Handler::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let node = LinkNode::start(
        listener,
        &certificate,
        handler.clone(),
        directory.clone(),
        Arc::new(MemoryReplayStore::default()),
        Arc::new(NoPairing),
    )
    .unwrap();
    Node {
        _dir: dir,
        node,
        key,
        directory,
        handler,
    }
}

fn ids(node: &Node) -> Vec<String> {
    node.directory
        .registry
        .lock()
        .unwrap()
        .peers()
        .iter()
        .map(|peer| peer.machine_id.clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_hello_binds_an_account_peer_once() {
    let console_dir = tempfile::tempdir().unwrap();
    let console = MachineIdentity::load_or_create(console_dir.path()).unwrap();
    let provisional = format!("{PROVISIONAL_PREFIX}dev_laptop");
    let node = node_trusting(vec![peer(
        &provisional,
        &console.public(),
        PairedVia::Account,
    )])
    .await;
    let certificate = node_certificate(&console).unwrap();

    let linked = client::connect(node.node.addr(), &certificate, node.key, "laptop-real")
        .await
        .unwrap();
    assert_eq!(
        linked.hello.machine_id, "node-real",
        "the console learns the node's real id too"
    );
    assert_eq!(
        ids(&node),
        ["laptop-real"],
        "bound to the id the key-holder gave"
    );
    assert!(
        node.handler
            .audit
            .lock()
            .unwrap()
            .iter()
            .any(|event| event.kind == "account_peer_bound" && event.peer == "laptop-real")
    );
    // Bound once: the same key cannot claim another id later.
    let refused = client::connect(node.node.addr(), &certificate, node.key, "someone-else")
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("mismatch"), "{refused}");
    assert_eq!(ids(&node), ["laptop-real"]);
    // The registry file round-trips signed with the bound id.
    let reloaded = PeerRegistry::load(node._dir.path(), &node.directory.identity, None);
    assert_eq!(reloaded.peers()[0].machine_id, "laptop-real");
    assert_eq!(reloaded.peers()[0].paired_via, PairedVia::Account);
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_else_can_use_the_bind() {
    let dir = tempfile::tempdir().unwrap();
    let console = MachineIdentity::load_or_create(&dir.path().join("console")).unwrap();
    let stranger = MachineIdentity::load_or_create(&dir.path().join("stranger")).unwrap();
    // Code-paired peers keep the id they paired with.
    let node = node_trusting(vec![peer(
        "laptop-paired",
        &console.public(),
        PairedVia::Code,
    )])
    .await;
    let certificate = node_certificate(&console).unwrap();
    assert!(
        client::connect(node.node.addr(), &certificate, node.key, "laptop-new")
            .await
            .is_err()
    );
    assert_eq!(ids(&node), ["laptop-paired"]);

    // A key nobody confirmed (an attacker's entry in the directory) is
    // simply not pinned: refused before any bind.
    let provisional = format!("{PROVISIONAL_PREFIX}dev_laptop");
    let node = node_trusting(vec![peer(
        &provisional,
        &console.public(),
        PairedVia::Account,
    )])
    .await;
    let stranger_cert = node_certificate(&stranger).unwrap();
    let refused = client::connect(node.node.addr(), &stranger_cert, node.key, "laptop-real")
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("isn't paired"), "{refused}");
    assert_eq!(
        ids(&node),
        std::slice::from_ref(&provisional),
        "nothing bound"
    );

    // A provisional id can't be claimed as a real one.
    assert!(
        client::connect(
            node.node.addr(),
            &certificate,
            node.key,
            &format!("{PROVISIONAL_PREFIX}x")
        )
        .await
        .is_err()
    );
    assert_eq!(ids(&node), [provisional]);

    // And the console refuses a node whose TLS key differs from the
    // confirmed pin, at the handshake.
    let wrong = stranger.public().to_bytes();
    assert!(
        client::connect(node.node.addr(), &certificate, wrong, "laptop-real")
            .await
            .is_err()
    );
}
