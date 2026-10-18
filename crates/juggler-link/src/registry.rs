//! The paired-peer trust store (AJ-E702; ADR-0013 architecture contract).
//!
//! `fleet/peers.json` records every pinned peer — its `MachineId`,
//! local label, pinned public key, fingerprint, permission, how it was
//! paired, and when — and is **signed with this machine's identity
//! key**. Any bad signature, unknown schema version, or missing field
//! fails closed: the registry reports `Trust broken — re-pair required`
//! and pins nothing, so no connection is attempted until the user
//! resets trust and pairs again. Writes are atomic and owner-only.

use std::path::{Path, PathBuf};

use base64::Engine as _;

use crate::identity::{IdentityError, MachineIdentity, PublicIdentity, write_owner_only};

/// The registry file name inside the fleet folder.
const REGISTRY_FILE: &str = "peers.json";

/// The schema this code writes and accepts.
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

/// What a paired peer may do on THIS machine (enforced here, per
/// command — never trusted from the peer's UI).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Paired, but nothing is served.
    NoAccess,
    /// Telemetry and Inspector reads (the pairing default).
    View,
    /// View plus launch/stop/switch/clean/focus commands.
    Control,
}

impl Permission {
    /// The plain-words label the UI shows.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::NoAccess => "No access",
            Self::View => "View only",
            Self::Control => "View + control",
        }
    }
}

/// How the peer's key was pinned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairedVia {
    /// The 6-digit code (SPAKE2) or QR.
    Code,
    /// The user's own Figurehead account directory (ADR-0014).
    Account,
}

/// One pinned peer.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    /// The peer's stable `MachineId`.
    pub machine_id: String,
    /// The local, editable display label.
    pub label: String,
    /// The pinned Ed25519 public key (standard base64).
    pub public_key: String,
    /// The display fingerprint of that key.
    pub fingerprint: String,
    /// What the peer may do here.
    pub permission: Permission,
    /// Control commands also need a local confirmation on this machine.
    pub ask_each_time: bool,
    /// How the key was pinned.
    pub paired_via: PairedVia,
    /// When it was paired, epoch ms.
    pub paired_at_ms: i64,
    /// When it was last heard from, epoch ms.
    pub last_seen_ms: Option<i64>,
}

/// The signed body (what the signature covers).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    schema_version: u32,
    /// Bumped on every write; the app mirrors it in its database so an
    /// older (validly signed) file is detected as a rollback.
    generation: u64,
    peers: Vec<Peer>,
}

/// The on-disk envelope.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    body: Body,
    signature: String,
}

/// Registry failures.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// The registry failed verification; nothing is trusted until the
    /// user resets trust and re-pairs.
    #[error("Trust broken — re-pair required ({0})")]
    TrustBroken(String),
    /// No peer with that machine id.
    #[error("no paired machine {0}")]
    UnknownPeer(String),
    /// Storage failure.
    #[error(transparent)]
    Storage(#[from] IdentityError),
}

/// The loaded trust state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TrustState {
    /// Verified; these peers are pinned.
    Trusted,
    /// Verification failed — nothing is pinned.
    Broken {
        /// Why (for the Details disclosure).
        reason: String,
    },
}

/// The paired-peer registry.
#[derive(Debug)]
pub struct PeerRegistry {
    path: PathBuf,
    generation: u64,
    peers: Vec<Peer>,
    trust: TrustState,
}

impl PeerRegistry {
    /// Loads and verifies the registry under `fleet_dir`.
    /// `recorded_generation` is the generation the app last stored in its
    /// own database (`None` on first run). A missing file is an empty,
    /// trusted registry only when nothing was ever recorded; anything
    /// unverifiable — including a file older than the recorded
    /// generation (rollback) or a deleted file — is
    /// [`TrustState::Broken`] (fail closed).
    #[must_use]
    pub fn load(
        fleet_dir: &Path,
        identity: &MachineIdentity,
        recorded_generation: Option<u64>,
    ) -> Self {
        let path = fleet_dir.join(REGISTRY_FILE);
        let recorded = recorded_generation.unwrap_or(0);
        let broken = |reason: &str| Self {
            path: path.clone(),
            generation: recorded,
            peers: Vec::new(),
            trust: TrustState::Broken {
                reason: reason.to_owned(),
            },
        };
        let raw = match std::fs::read(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if recorded > 0 {
                    return broken("the registry file is missing");
                }
                return Self {
                    path,
                    generation: 0,
                    peers: Vec::new(),
                    trust: TrustState::Trusted,
                };
            }
            Err(_) => return broken("the registry could not be read"),
        };
        let Ok(envelope) = serde_json::from_slice::<Envelope>(&raw) else {
            return broken("the registry is malformed or has missing fields");
        };
        if envelope.body.schema_version != REGISTRY_SCHEMA_VERSION {
            return broken("the registry was written by an unknown version");
        }
        let Ok(signature) = base64::engine::general_purpose::STANDARD.decode(&envelope.signature)
        else {
            return broken("the registry signature is malformed");
        };
        let Ok(signature) = <[u8; 64]>::try_from(signature.as_slice()) else {
            return broken("the registry signature is malformed");
        };
        let Ok(signed) = serde_json::to_vec(&envelope.body) else {
            return broken("the registry could not be checked");
        };
        if !identity.public().verify(&signed, &signature) {
            return broken("the registry signature does not match this Mac's identity");
        }
        if envelope.body.generation < recorded {
            return broken("an older copy of the registry was restored (rollback)");
        }
        for peer in &envelope.body.peers {
            let Ok(key) = PublicIdentity::from_base64(&peer.public_key) else {
                return broken("a pinned key is damaged");
            };
            if key.fingerprint() != peer.fingerprint {
                return broken("a pinned fingerprint does not match its key");
            }
        }
        Self {
            path,
            generation: envelope.body.generation,
            peers: envelope.body.peers,
            trust: TrustState::Trusted,
        }
    }

    /// The current generation. Callers SHALL record it in the app
    /// database after every successful mutation (it is what makes a
    /// rolled-back file detectable on the next load).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The trust state (the UI's `Trust broken` banner).
    #[must_use]
    pub fn trust(&self) -> &TrustState {
        &self.trust
    }

    /// Every pinned peer (empty when trust is broken).
    #[must_use]
    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }

    /// The pinned peer holding `key`, if any. Connections call this per
    /// request, so an unpair revokes on the next call.
    #[must_use]
    pub fn pinned(&self, key: &PublicIdentity) -> Option<&Peer> {
        if self.trust != TrustState::Trusted {
            return None;
        }
        let wanted = key.to_base64();
        self.peers.iter().find(|peer| peer.public_key == wanted)
    }

    /// Pins (or re-pins) a peer, replacing any entry with the same
    /// machine id or the same key.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::TrustBroken`] while trust is broken
    /// (reset first), or a storage error.
    pub fn pin(&mut self, identity: &MachineIdentity, peer: Peer) -> Result<(), RegistryError> {
        self.require_trusted()?;
        self.peers.retain(|existing| {
            existing.machine_id != peer.machine_id && existing.public_key != peer.public_key
        });
        self.peers.push(peer);
        self.peers.sort_by(|a, b| a.machine_id.cmp(&b.machine_id));
        self.save(identity)
    }

    /// Removes a peer's pin ("Unpair"). Returns whether one was removed.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] per [`Self::pin`].
    pub fn unpair(
        &mut self,
        identity: &MachineIdentity,
        machine_id: &str,
    ) -> Result<bool, RegistryError> {
        self.require_trusted()?;
        let before = self.peers.len();
        self.peers.retain(|peer| peer.machine_id != machine_id);
        let removed = self.peers.len() != before;
        if removed {
            self.save(identity)?;
        }
        Ok(removed)
    }

    /// Changes what a peer may do here (the node-side confirmation is the
    /// caller's UI step; this only records the confirmed choice).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownPeer`] or per [`Self::pin`].
    pub fn set_permission(
        &mut self,
        identity: &MachineIdentity,
        machine_id: &str,
        permission: Permission,
        ask_each_time: bool,
    ) -> Result<(), RegistryError> {
        self.update(identity, machine_id, |peer| {
            peer.permission = permission;
            peer.ask_each_time = ask_each_time;
        })
    }

    /// Renames a peer locally (never changes its id or key).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownPeer`] or per [`Self::pin`].
    pub fn relabel(
        &mut self,
        identity: &MachineIdentity,
        machine_id: &str,
        label: &str,
    ) -> Result<(), RegistryError> {
        let label = label.trim().to_owned();
        self.update(identity, machine_id, |peer| peer.label = label)
    }

    /// Records that a peer was heard from.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::UnknownPeer`] or per [`Self::pin`].
    pub fn touch(
        &mut self,
        identity: &MachineIdentity,
        machine_id: &str,
        at_ms: i64,
    ) -> Result<(), RegistryError> {
        self.update(identity, machine_id, |peer| peer.last_seen_ms = Some(at_ms))
    }

    /// Clears a broken (or any) registry so the user can pair again —
    /// only ever from an explicit "Reset trust" click.
    ///
    /// # Errors
    ///
    /// Returns a storage error.
    pub fn reset_trust(&mut self, identity: &MachineIdentity) -> Result<(), RegistryError> {
        self.peers.clear();
        self.trust = TrustState::Trusted;
        self.save(identity)
    }

    fn update(
        &mut self,
        identity: &MachineIdentity,
        machine_id: &str,
        change: impl FnOnce(&mut Peer),
    ) -> Result<(), RegistryError> {
        self.require_trusted()?;
        let peer = self
            .peers
            .iter_mut()
            .find(|peer| peer.machine_id == machine_id)
            .ok_or_else(|| RegistryError::UnknownPeer(machine_id.to_owned()))?;
        change(peer);
        self.save(identity)
    }

    fn require_trusted(&self) -> Result<(), RegistryError> {
        match &self.trust {
            TrustState::Trusted => Ok(()),
            TrustState::Broken { reason } => Err(RegistryError::TrustBroken(reason.clone())),
        }
    }

    fn save(&mut self, identity: &MachineIdentity) -> Result<(), RegistryError> {
        let body = Body {
            schema_version: REGISTRY_SCHEMA_VERSION,
            generation: self.generation + 1,
            peers: self.peers.clone(),
        };
        let signed = serde_json::to_vec(&body)
            .map_err(|e| RegistryError::Storage(IdentityError::Io(e.to_string())))?;
        let envelope = Envelope {
            signature: base64::engine::general_purpose::STANDARD.encode(identity.sign(&signed)),
            body,
        };
        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|e| RegistryError::Storage(IdentityError::Io(e.to_string())))?;
        write_owner_only(&self.path, &bytes)?;
        self.generation += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf, MachineIdentity) {
        let dir = tempfile::tempdir().unwrap();
        let fleet = dir.path().join("fleet");
        let identity = MachineIdentity::load_or_create(&fleet).unwrap();
        (dir, fleet, identity)
    }

    fn peer(dir: &Path, id: &str) -> Peer {
        let other = MachineIdentity::load_or_create(&dir.join(id)).unwrap();
        Peer {
            machine_id: id.to_owned(),
            label: format!("{id} label"),
            public_key: other.public().to_base64(),
            fingerprint: other.public().fingerprint(),
            permission: Permission::View,
            ask_each_time: false,
            paired_via: PairedVia::Code,
            paired_at_ms: 1,
            last_seen_ms: None,
        }
    }

    #[test]
    fn pins_survive_a_reload_and_resolve_by_key() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        assert_eq!(registry.trust(), &TrustState::Trusted);
        let mini = peer(dir.path(), "mac-mini");
        registry.pin(&identity, mini.clone()).unwrap();
        let reloaded = PeerRegistry::load(&fleet, &identity, None);
        assert_eq!(reloaded.peers(), std::slice::from_ref(&mini));
        let key = PublicIdentity::from_base64(&mini.public_key).unwrap();
        assert_eq!(reloaded.pinned(&key).unwrap().machine_id, "mac-mini");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(fleet.join(REGISTRY_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn any_tamper_fails_closed() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        let mini = peer(dir.path(), "mac-mini");
        registry.pin(&identity, mini.clone()).unwrap();
        let path = fleet.join(REGISTRY_FILE);
        let original = std::fs::read_to_string(&path).unwrap();
        let attacker = peer(dir.path(), "attacker");
        let cases: Vec<(&str, String)> = vec![
            // An injected pin (the classic attack): signature no longer matches.
            (
                "injected key",
                original.replace(&mini.public_key, &attacker.public_key),
            ),
            // A raised permission.
            (
                "raised permission",
                original.replace("\"view\"", "\"control\""),
            ),
            // A missing field.
            (
                "missing field",
                original.replace("\"ask_each_time\": false,", ""),
            ),
            // An unknown schema.
            (
                "future schema",
                original.replace("\"schema_version\": 1", "\"schema_version\": 9"),
            ),
            // Garbage.
            ("garbage", "{not json".to_owned()),
        ];
        for (name, tampered) in cases {
            std::fs::write(&path, &tampered).unwrap();
            let loaded = PeerRegistry::load(&fleet, &identity, None);
            assert!(
                matches!(loaded.trust(), TrustState::Broken { .. }),
                "{name}"
            );
            assert!(loaded.peers().is_empty(), "{name}: nothing pinned");
            let key = PublicIdentity::from_base64(&mini.public_key).unwrap();
            assert!(
                loaded.pinned(&key).is_none(),
                "{name}: no connection allowed"
            );
        }
        // A registry signed by ANOTHER machine's key is refused too.
        std::fs::write(&path, &original).unwrap();
        let other = MachineIdentity::load_or_create(&dir.path().join("other")).unwrap();
        assert!(matches!(
            PeerRegistry::load(&fleet, &other, None).trust(),
            TrustState::Broken { .. }
        ));
    }

    #[test]
    fn a_rolled_back_or_deleted_registry_fails_closed() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        registry.pin(&identity, peer(dir.path(), "old")).unwrap();
        let path = fleet.join(REGISTRY_FILE);
        let old_copy = std::fs::read(&path).unwrap();
        registry.pin(&identity, peer(dir.path(), "new")).unwrap();
        let recorded = registry.generation();
        assert_eq!(recorded, 2);
        // The current file loads against the recorded generation.
        assert_eq!(
            PeerRegistry::load(&fleet, &identity, Some(recorded))
                .peers()
                .len(),
            2
        );
        // A validly SIGNED older copy is a rollback.
        std::fs::write(&path, &old_copy).unwrap();
        let rolled_back = PeerRegistry::load(&fleet, &identity, Some(recorded));
        assert!(
            matches!(rolled_back.trust(), TrustState::Broken { reason } if reason.contains("rollback"))
        );
        // Deleting the file once something was recorded is not "first run".
        std::fs::remove_file(&path).unwrap();
        let deleted = PeerRegistry::load(&fleet, &identity, Some(recorded));
        assert!(matches!(deleted.trust(), TrustState::Broken { .. }));
        assert!(matches!(
            PeerRegistry::load(&fleet, &identity, None).trust(),
            TrustState::Trusted
        ));
    }

    #[test]
    fn writes_are_refused_while_trust_is_broken_until_reset() {
        let (dir, fleet, identity) = setup();
        std::fs::write(fleet.join(REGISTRY_FILE), "{}").unwrap();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        let mini = peer(dir.path(), "mac-mini");
        assert!(matches!(
            registry.pin(&identity, mini.clone()),
            Err(RegistryError::TrustBroken(_))
        ));
        registry.reset_trust(&identity).unwrap();
        registry.pin(&identity, mini).unwrap();
        assert_eq!(PeerRegistry::load(&fleet, &identity, None).peers().len(), 1);
    }

    #[test]
    fn unpair_revokes_the_pin_immediately() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        let mini = peer(dir.path(), "mac-mini");
        let key = PublicIdentity::from_base64(&mini.public_key).unwrap();
        registry.pin(&identity, mini).unwrap();
        assert!(registry.pinned(&key).is_some());
        let started = std::time::Instant::now();
        assert!(registry.unpair(&identity, "mac-mini").unwrap());
        assert!(
            registry.pinned(&key).is_none(),
            "the next request is refused"
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert!(
            PeerRegistry::load(&fleet, &identity, None)
                .pinned(&key)
                .is_none()
        );
        assert!(!registry.unpair(&identity, "mac-mini").unwrap());
    }

    #[test]
    fn relabel_and_permission_never_touch_id_or_key() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        let mini = peer(dir.path(), "mac-mini");
        registry.pin(&identity, mini.clone()).unwrap();
        registry
            .relabel(&identity, "mac-mini", "  Studio  ")
            .unwrap();
        registry
            .set_permission(&identity, "mac-mini", Permission::Control, true)
            .unwrap();
        registry.touch(&identity, "mac-mini", 42).unwrap();
        let reloaded = PeerRegistry::load(&fleet, &identity, None);
        let stored = &reloaded.peers()[0];
        assert_eq!(stored.label, "Studio");
        assert_eq!(stored.permission, Permission::Control);
        assert!(stored.ask_each_time);
        assert_eq!(stored.last_seen_ms, Some(42));
        assert_eq!(stored.machine_id, mini.machine_id);
        assert_eq!(stored.public_key, mini.public_key);
        assert!(matches!(
            registry.relabel(&identity, "nope", "x"),
            Err(RegistryError::UnknownPeer(_))
        ));
        assert_eq!(Permission::View.label(), "View only");
    }

    #[test]
    fn fifty_peers_load_fast_and_stay_small() {
        let (dir, fleet, identity) = setup();
        let mut registry = PeerRegistry::load(&fleet, &identity, None);
        for index in 0..50 {
            registry
                .pin(&identity, peer(dir.path(), &format!("m{index:02}")))
                .unwrap();
        }
        let size = std::fs::metadata(fleet.join(REGISTRY_FILE)).unwrap().len();
        assert!(size / 50 < 5 * 1024, "< 5 KB per peer ({size} bytes total)");
        let started = std::time::Instant::now();
        let loaded = PeerRegistry::load(&fleet, &identity, None);
        assert_eq!(loaded.peers().len(), 50);
        assert!(
            started.elapsed() < std::time::Duration::from_millis(50),
            "{:?}",
            started.elapsed()
        );
    }
}
