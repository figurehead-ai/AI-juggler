//! What the app remembers between fetches (§3, §7): the highest serial
//! accepted per channel and key, the revoked key ids, and the last good
//! file. Kept behind [`StateStore`] so the app can put it in the
//! registered cache (ADR-0023) and tests can use [`MemoryStore`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::envelope::Channel;

/// Everything the trust checks remember.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustState {
    /// Key ids revoked by a signed revocation list. Only ever grows.
    #[serde(default)]
    pub revoked: BTreeSet<String>,
    /// Per channel.
    #[serde(default)]
    pub channels: BTreeMap<Channel, ChannelState>,
}

/// One channel's memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelState {
    /// The highest serial accepted, per signing key id.
    #[serde(default)]
    pub serials: BTreeMap<String, u64>,
    /// The last accepted file, verbatim.
    #[serde(default)]
    pub accepted: Option<AcceptedFile>,
    /// When a fetch was last attempted, Unix seconds (any outcome).
    #[serde(default)]
    pub last_attempt: Option<u64>,
    /// The accepted file's entity tag (sent back only when conditional
    /// requests are on).
    #[serde(default)]
    pub etag: Option<String>,
}

/// The last accepted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedFile {
    /// Its serial.
    pub serial: u64,
    /// The SHA-256 of its signed payload, hex.
    pub digest: String,
    /// The whole file as received (it is re-verified on every load).
    pub file: String,
}

impl TrustState {
    /// The serial floor for `channel`: the highest serial accepted from a
    /// key that isn't revoked. A key revoked after it burned a huge serial
    /// no longer holds the floor up, so publishing can resume with another
    /// key; a replay of an older file from a live key is still refused.
    #[must_use]
    pub fn floor(&self, channel: Channel) -> Option<u64> {
        self.channels.get(&channel).and_then(|state| {
            state
                .serials
                .iter()
                .filter(|(key, _)| !self.revoked.contains(*key))
                .map(|(_, serial)| *serial)
                .max()
        })
    }

    /// The digest of the accepted file with `serial` on `channel`, if that
    /// is the one accepted.
    #[must_use]
    pub fn accepted_digest(&self, channel: Channel, serial: u64) -> Option<&str> {
        self.channels
            .get(&channel)
            .and_then(|state| state.accepted.as_ref())
            .filter(|accepted| accepted.serial == serial)
            .map(|accepted| accepted.digest.as_str())
    }
}

/// The store couldn't be read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("remote-config state: {0}")]
pub struct StoreError(pub String);

/// Where [`TrustState`] lives.
///
/// A store that can't load makes the fetch fail closed: no remote file is
/// accepted (the rollback floor would be unknown), and the app runs on its
/// compiled defaults until the store is readable or cleared.
pub trait StateStore: Send + Sync {
    /// The saved state; [`TrustState::default`] when nothing was saved.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the saved state can't be read.
    fn load(&self) -> Result<TrustState, StoreError>;

    /// Saves `state`, replacing what was saved.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when it can't be written.
    fn save(&self, state: &TrustState) -> Result<(), StoreError>;
}

/// A store in memory (tests, and a session with no cache).
#[derive(Debug, Default)]
pub struct MemoryStore {
    state: Mutex<TrustState>,
}

impl MemoryStore {
    /// A store holding `state`.
    #[must_use]
    pub fn with(state: TrustState) -> Self {
        Self {
            state: Mutex::new(state),
        }
    }
}

impl StateStore for MemoryStore {
    fn load(&self) -> Result<TrustState, StoreError> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| StoreError("poisoned".to_owned()))
    }

    fn save(&self, state: &TrustState) -> Result<(), StoreError> {
        let mut slot = self
            .state
            .lock()
            .map_err(|_| StoreError("poisoned".to_owned()))?;
        *slot = state.clone();
        Ok(())
    }
}
