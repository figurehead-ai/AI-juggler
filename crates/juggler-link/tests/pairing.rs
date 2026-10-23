//! AJ-E704 loopback pairing tests: SPAKE2 over the 6-digit code inside
//! an unpinned TLS 1.3 session, confirmation bound to that session, both
//! users confirming before anything is pinned — and the guard's
//! negatives (wrong code + rotation, not open, one session at a time,
//! lockout, cancel, pairing-link fingerprint mismatch).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    missing_docs
)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_link::cert::{NodeCertificate, node_certificate};
use juggler_link::node::{
    CommandRejection, LinkAuditEvent, LinkNode, MemoryReplayStore, NodeHandler, PairCandidate,
    PairingHost, PeerAccess, PeerDirectory,
};
use juggler_link::pairing::{MemoryGuardStore, PairingCode, PairingError, PairingGuard};
use juggler_link::pairing_rpc::{JoinError, PairingInput, pair_with};
use juggler_link::{MachineIdentity, Permission, proto};

struct NoPeers;
impl PeerDirectory for NoPeers {
    fn lookup(&self, _key: &[u8; 32]) -> Option<PeerAccess> {
        None
    }
}

#[derive(Default)]
struct Audit(Mutex<Vec<LinkAuditEvent>>);
impl NodeHandler for Audit {
    fn local_machine_id(&self) -> String {
        "host-mid".into()
    }
    fn telemetry_since(&self, _since: u64) -> Vec<proto::TelemetryDelta> {
        Vec::new()
    }
    fn telemetry_live(&self) -> tokio::sync::broadcast::Receiver<proto::TelemetryDelta> {
        tokio::sync::broadcast::channel(1).1
    }
    fn execute(
        &self,
        _p: &PeerAccess,
        _a: &proto::control_command::Action,
    ) -> Result<juggler_link::node::CommandDone, CommandRejection> {
        Err(CommandRejection {
            reason_code: "x".into(),
            message: "x".into(),
        })
    }
    fn audit(&self, event: LinkAuditEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Host {
    guard: Mutex<PairingGuard<MemoryGuardStore>>,
    key: [u8; 32],
    decision: Option<Permission>,
    pinned: Mutex<Vec<(PairCandidate, Permission)>>,
    /// Link went off on the host just before it pinned.
    refuse_pin: bool,
}

impl PairingHost for Host {
    fn begin(&self) -> Result<PairingCode, PairingError> {
        self.guard.lock().unwrap().begin_session(now())
    }
    fn failed(&self) {
        self.guard.lock().unwrap().fail(now()).unwrap();
    }
    fn released(&self) {
        self.guard.lock().unwrap().release();
    }
    fn local_label(&self) -> String {
        "Studio Mac".into()
    }
    fn local_key(&self) -> [u8; 32] {
        self.key
    }
    fn confirm(
        &self,
        _candidate: PairCandidate,
    ) -> tokio::sync::oneshot::Receiver<Option<Permission>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let decision = self.decision;
        tokio::spawn(async move {
            // A cancelling user takes a moment (holds the session slot).
            if decision.is_none() {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            let _ = tx.send(decision);
        });
        rx
    }
    fn succeeded(&self, candidate: &PairCandidate, permission: Permission) -> bool {
        if self.refuse_pin {
            return false;
        }
        self.guard.lock().unwrap().succeed();
        self.pinned
            .lock()
            .unwrap()
            .push((candidate.clone(), permission));
        true
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

struct World {
    _dir: tempfile::TempDir,
    node: LinkNode,
    host: Arc<Host>,
    audit: Arc<Audit>,
    host_identity: MachineIdentity,
    joiner_identity: MachineIdentity,
    joiner_cert: NodeCertificate,
}

async fn world(decision: Option<Permission>) -> World {
    world_with(decision, false).await
}

async fn world_with(decision: Option<Permission>, refuse_pin: bool) -> World {
    let dir = tempfile::tempdir().unwrap();
    let host_identity = MachineIdentity::load_or_create(&dir.path().join("host")).unwrap();
    let joiner_identity = MachineIdentity::load_or_create(&dir.path().join("joiner")).unwrap();
    let host = Arc::new(Host {
        guard: Mutex::new(PairingGuard::new(MemoryGuardStore::default())),
        key: host_identity.public().to_bytes(),
        decision,
        pinned: Mutex::default(),
        refuse_pin,
    });
    let audit = Arc::new(Audit::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let node = LinkNode::start(
        listener,
        &node_certificate(&host_identity).unwrap(),
        audit.clone(),
        Arc::new(NoPeers),
        Arc::new(MemoryReplayStore::default()),
        host.clone(),
    )
    .unwrap();
    World {
        joiner_cert: node_certificate(&joiner_identity).unwrap(),
        _dir: dir,
        node,
        host,
        audit,
        host_identity,
        joiner_identity,
    }
}

impl World {
    fn open(&self) -> PairingCode {
        self.host.guard.lock().unwrap().open(now()).unwrap()
    }
    async fn join(
        &self,
        typed: &str,
        decision: Option<Permission>,
    ) -> Result<juggler_link::pairing_rpc::JoinedHost, JoinError> {
        pair_with(
            self.node.addr(),
            juggler_link::pairing_rpc::Joiner {
                certificate: &self.joiner_cert,
                key: self.joiner_identity.public().to_bytes(),
                machine_id: "joiner-mid",
                label: "Laptop",
                port: 0,
            },
            &PairingInput::parse(typed).unwrap(),
            move |_| async move { decision },
        )
        .await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_right_code_and_both_confirmations_pin_both_keys() {
    let w = world(Some(Permission::View)).await;
    let code = w.open();
    let started = Instant::now();
    let joined = w
        .join(&code.display(), Some(Permission::Control))
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(60), "pair < 60 s");
    // The joiner pins exactly the host's real key, with what its user chose.
    assert_eq!(joined.candidate.key, w.host_identity.public().to_bytes());
    assert_eq!(
        joined.candidate.fingerprint,
        w.host_identity.public().fingerprint()
    );
    assert_eq!(joined.candidate.machine_id, "host-mid");
    assert_eq!(joined.candidate.label, "Studio Mac");
    assert_eq!(joined.permission, Permission::Control);
    // The host pins after it receives the joiner's commit (each side
    // pins on both commits) — give it a moment.
    for _ in 0..50 {
        if !w.host.pinned.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // The host pinned the joiner's real key at its own default (View).
    let pinned = w.host.pinned.lock().unwrap().clone();
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].0.key, w.joiner_identity.public().to_bytes());
    assert_eq!(pinned[0].0.machine_id, "joiner-mid");
    assert_eq!(pinned[0].1, Permission::View);
    // The window closed on success; the code is single-use.
    assert!(w.host.guard.lock().unwrap().current_code(now()).is_none());
    assert!(
        w.audit
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.kind == "pairing" && e.reason_code == "paired")
    );
}

/// The host refuses at the pin (its Link went off after both users said
/// yes): it tells the joiner, which pins nothing either, and its log says
/// so instead of "paired".
#[tokio::test(flavor = "multi_thread")]
async fn a_host_that_refuses_at_the_pin_leaves_the_joiner_unpinned() {
    let w = world_with(Some(Permission::View), true).await;
    let code = w.open();
    let joined = w.join(&code.display(), Some(Permission::View)).await;
    match joined {
        Err(JoinError::Host { reason_code, .. }) => assert_eq!(reason_code, "link_off"),
        other => panic!("the joiner pinned or failed otherwise: {other:?}"),
    }
    assert!(w.host.pinned.lock().unwrap().is_empty());
    for _ in 0..50 {
        if w.audit
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.kind == "pairing")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let reasons: Vec<String> = w
        .audit
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.kind == "pairing")
        .map(|e| e.reason_code.clone())
        .collect();
    assert_eq!(reasons, ["link_off"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_code_fails_rotates_and_never_logs_the_code() {
    let w = world(Some(Permission::View)).await;
    let code = w.open();
    let wrong = if code.display() == "000 000" {
        "111111"
    } else {
        "000000"
    };
    let error = w.join(wrong, Some(Permission::View)).await.unwrap_err();
    assert!(
        matches!(&error, JoinError::Host { message: m, .. } if m.contains("didn't match")),
        "{error}"
    );
    let rotated = w.host.guard.lock().unwrap().current_code(now()).unwrap();
    assert_ne!(rotated, code, "the code rotates after a failure");
    assert!(w.host.pinned.lock().unwrap().is_empty());
    let audit = w.audit.0.lock().unwrap();
    assert!(audit.iter().any(|e| e.reason_code == "mismatch"));
    let logged = format!("{audit:?}");
    assert!(!logged.contains(wrong) && !logged.contains(&code.display().replace(' ', "")));
}

#[tokio::test(flavor = "multi_thread")]
async fn pairing_is_refused_when_no_window_is_open() {
    let w = world(Some(Permission::View)).await;
    let error = w.join("123456", Some(Permission::View)).await.unwrap_err();
    assert!(
        matches!(&error, JoinError::Host { message: m, .. } if m.contains("isn't open")),
        "{error}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_one_session_runs_at_a_time() {
    // The host's user takes a while: the first session holds the slot.
    let w = Arc::new(world(None).await);
    let code = w.open();
    let first = {
        let w = w.clone();
        let typed = code.display();
        tokio::spawn(async move { w.join(&typed, Some(Permission::View)).await })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    let second = w
        .join(&code.display(), Some(Permission::View))
        .await
        .unwrap_err();
    assert!(
        matches!(&second, JoinError::Host { message: m, .. } if m.contains("Another Mac")),
        "{second}"
    );
    // The host's user cancelled → the first session ends, nothing pinned.
    assert!(matches!(first.await.unwrap(), Err(JoinError::Cancelled)));
    assert!(w.host.pinned.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn five_wrong_codes_lock_pairing() {
    let w = world(Some(Permission::View)).await;
    w.open();
    for _ in 0..5 {
        let current = w.host.guard.lock().unwrap().current_code(now()).unwrap();
        let wrong = if current.display() == "000 000" {
            "111111"
        } else {
            "000000"
        };
        let _ = w.join(wrong, Some(Permission::View)).await.unwrap_err();
    }
    let error = w.join("000000", Some(Permission::View)).await.unwrap_err();
    assert!(
        matches!(&error, JoinError::Host { message: m, .. } if m.contains("paused")),
        "{error}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn either_user_cancelling_pins_nothing() {
    let w = world(Some(Permission::View)).await;
    let code = w.open();
    // The joiner's user cancels after the codes matched.
    assert!(matches!(
        w.join(&code.display(), None).await,
        Err(JoinError::Cancelled)
    ));
    assert!(
        w.host.pinned.lock().unwrap().is_empty(),
        "the host pins only when both committed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pairing_link_with_another_fingerprint_is_refused_before_any_guess() {
    let w = world(Some(Permission::View)).await;
    let code = w.open();
    let link = PairingInput::link(&code, &w.joiner_identity.public().fingerprint()); // wrong Mac's fingerprint
    let error = w.join(&link, Some(Permission::View)).await.unwrap_err();
    assert!(matches!(error, JoinError::FingerprintMismatch), "{error}");
    // No guess was spent: the window still holds the same code.
    assert_eq!(
        w.host.guard.lock().unwrap().current_code(now()).unwrap(),
        code
    );
    // The right link pairs.
    let good = PairingInput::link(&code, &w.host_identity.public().fingerprint());
    assert!(w.join(&good, Some(Permission::View)).await.is_ok());
}

/// Live mDNS on this machine's network (multicast is often unavailable
/// on CI runners, so this runs explicitly):
/// `cargo test -p juggler-link --test pairing -- --ignored mdns`
#[test]
#[ignore = "needs a multicast-capable network interface"]
fn mdns_advertise_browse_and_withdraw() {
    use juggler_link::discovery::{Advertiser, Browser, new_instance_id};
    let key = [9_u8; 32];
    let instance = new_instance_id().unwrap();
    let browser = Browser::start("someone-else").unwrap();
    let advertiser = Advertiser::start(47_999, &key, &instance).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let found = loop {
        if let Some(node) = browser
            .nodes()
            .into_iter()
            .find(|n| n.instance_id == instance)
        {
            break node;
        }
        assert!(Instant::now() < deadline, "not discovered within 5 s");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(found.is(&key), "the keyed hint matches the pinned key");
    assert_eq!(found.version, "1.0");
    assert!(found.addresses.iter().all(|a| a.port() == 47_999));
    let withdrawn_at = Instant::now();
    advertiser.stop();
    loop {
        if !browser.nodes().iter().any(|n| n.instance_id == instance) {
            break;
        }
        assert!(
            withdrawn_at.elapsed() < Duration::from_secs(5),
            "withdrawn within 5 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("[perf] withdrawn after {:?}", withdrawn_at.elapsed());
    browser.stop();
}
