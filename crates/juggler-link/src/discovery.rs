//! LAN discovery over mDNS / Bonjour (AJ-E704; ADR-0013 contract).
//!
//! Discovery is **unauthenticated** and never implies trust. What a node
//! advertises is deliberately minimal:
//!
//! - instance name = a random per-boot id (never the Mac's name);
//! - host name = `aij-<id>.local.` (never the real host name);
//! - TXT `v` = protocol version, TXT `h` = a keyed hint: the first 8
//!   bytes of SHA-256(identity key ‖ instance id). Only a machine that
//!   already pinned that key can recognise it, and it changes every boot
//!   — so the record can't be used to track a machine across networks.
//!
//! Labels, platform and fingerprints are exchanged only inside TLS.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sha2::{Digest, Sha256};

/// The service type.
pub const SERVICE_TYPE: &str = "_aijuggler._tcp.local.";

/// Discovery failures.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// The mDNS daemon failed.
    #[error("local-network discovery isn't available: {0}")]
    Daemon(String),
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// A fresh random per-boot instance id (8 hex digits).
///
/// # Errors
///
/// Returns [`DiscoveryError`] if the random source fails.
pub fn new_instance_id() -> Result<String, DiscoveryError> {
    let mut bytes = [0_u8; 4];
    getrandom::fill(&mut bytes).map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
    Ok(hex(&bytes))
}

/// The keyed hint for `key` under `instance_id`.
#[must_use]
pub fn hint(key: &[u8; 32], instance_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(instance_id.as_bytes());
    hex(&hasher.finalize()[..8])
}

/// A node seen on the LAN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredNode {
    /// Its per-boot instance id.
    pub instance_id: String,
    /// Addresses to try (port included).
    pub addresses: Vec<SocketAddr>,
    /// The advertised protocol version (`1.0`).
    pub version: String,
    /// The keyed hint (see [`hint`]).
    pub hint: String,
}

impl DiscoveredNode {
    /// Whether this node holds `pinned_key` (by its keyed hint).
    #[must_use]
    pub fn is(&self, pinned_key: &[u8; 32]) -> bool {
        hint(pinned_key, &self.instance_id) == self.hint
    }
}

/// Advertises this node while Link is on.
pub struct Advertiser {
    daemon: mdns_sd::ServiceDaemon,
    fullname: String,
}

impl std::fmt::Debug for Advertiser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Advertiser")
            .field("fullname", &self.fullname)
            .finish_non_exhaustive()
    }
}

impl Advertiser {
    /// Starts advertising `port` for the identity `key`.
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] when mDNS can't start.
    pub fn start(port: u16, key: &[u8; 32], instance_id: &str) -> Result<Self, DiscoveryError> {
        let daemon =
            mdns_sd::ServiceDaemon::new().map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
        let properties = [
            (
                "v",
                format!(
                    "{}.{}",
                    crate::protocol::PROTOCOL_MAJOR,
                    crate::protocol::PROTOCOL_MINOR
                ),
            ),
            ("h", hint(key, instance_id)),
        ];
        let info = mdns_sd::ServiceInfo::new(
            SERVICE_TYPE,
            instance_id,
            &format!("aij-{instance_id}.local."),
            "",
            port,
            &properties[..],
        )
        .map_err(|e| DiscoveryError::Daemon(e.to_string()))?
        .enable_addr_auto();
        let fullname = info.get_fullname().to_owned();
        daemon
            .register(info)
            .map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
        Ok(Self { daemon, fullname })
    }

    /// Withdraws the advertisement (goodbye packets) and stops mDNS.
    pub fn stop(self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            let _ = done.recv_timeout(std::time::Duration::from_secs(2));
        }
        let _ = self.daemon.shutdown();
    }
}

/// Browses for other nodes while Link is on.
pub struct Browser {
    daemon: mdns_sd::ServiceDaemon,
    seen: Arc<Mutex<HashMap<String, (DiscoveredNode, Instant)>>>,
    own_instance: String,
}

impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browser").finish_non_exhaustive()
    }
}

impl Browser {
    /// Starts browsing, ignoring this node's own `own_instance`.
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] when mDNS can't start.
    pub fn start(own_instance: &str) -> Result<Self, DiscoveryError> {
        let daemon =
            mdns_sd::ServiceDaemon::new().map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
        let events = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
        let seen: Arc<Mutex<HashMap<String, (DiscoveredNode, Instant)>>> = Arc::default();
        let map = seen.clone();
        std::thread::Builder::new()
            .name("aij-mdns-browse".to_owned())
            .spawn(move || {
                while let Ok(event) = events.recv() {
                    let Ok(mut map) = map.lock() else {
                        return;
                    };
                    match event {
                        mdns_sd::ServiceEvent::ServiceResolved(service) => {
                            let instance_id = service
                                .fullname
                                .strip_suffix(&format!(".{SERVICE_TYPE}"))
                                .unwrap_or(&service.fullname)
                                .to_owned();
                            let addresses = service
                                .addresses
                                .iter()
                                .map(|ip| SocketAddr::new(ip.to_ip_addr(), service.port))
                                .collect();
                            let node = DiscoveredNode {
                                instance_id: instance_id.clone(),
                                addresses,
                                version: service
                                    .txt_properties
                                    .get_property_val_str("v")
                                    .unwrap_or_default()
                                    .to_owned(),
                                hint: service
                                    .txt_properties
                                    .get_property_val_str("h")
                                    .unwrap_or_default()
                                    .to_owned(),
                            };
                            map.insert(instance_id, (node, Instant::now()));
                        }
                        mdns_sd::ServiceEvent::ServiceRemoved(_, fullname) => {
                            let instance_id = fullname
                                .strip_suffix(&format!(".{SERVICE_TYPE}"))
                                .unwrap_or(&fullname)
                                .to_owned();
                            map.remove(&instance_id);
                        }
                        _ => {}
                    }
                }
            })
            .map_err(|e| DiscoveryError::Daemon(e.to_string()))?;
        Ok(Self {
            daemon,
            seen,
            own_instance: own_instance.to_owned(),
        })
    }

    /// The nodes currently seen (never this node itself).
    #[must_use]
    pub fn nodes(&self) -> Vec<DiscoveredNode> {
        self.seen
            .lock()
            .map(|map| {
                map.values()
                    .filter(|(node, _)| node.instance_id != self.own_instance)
                    .map(|(node, _)| node.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Stops browsing.
    pub fn stop(self) {
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_match_only_the_pinned_key_and_change_per_boot() {
        let key = [7_u8; 32];
        let other = [8_u8; 32];
        let node = DiscoveredNode {
            instance_id: "a1b2c3d4".into(),
            addresses: Vec::new(),
            version: "1.0".into(),
            hint: hint(&key, "a1b2c3d4"),
        };
        assert!(node.is(&key));
        assert!(!node.is(&other));
        assert_ne!(
            hint(&key, "a1b2c3d4"),
            hint(&key, "ffffffff"),
            "a new boot, a new hint"
        );
        assert_eq!(hint(&key, "a1b2c3d4").len(), 16);
        let id = new_instance_id().unwrap();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
