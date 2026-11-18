//! What the app remembers between fetches (§3, §7): the rollback floor per
//! channel and key, the revoked key fingerprints, the trusted time, and
//! the last good file.
//!
//! # Where it lives
//!
//! This is security state, not a cache. It lives in its own file in the
//! app's durable data folder (Application Support on this platform's
//! terms; never Caches), written by [`FileStore`]: atomically (a temp file,
//! fsync, rename, fsync the folder) and with a checksum. Clearing the
//! cache, resetting the install id or "reset" in Settings never touches
//! it.
//!
//! # When it's missing or corrupt
//!
//! The app fails **safe**, not open: it starts from the [`Baseline`]
//! compiled into the release (the serial of the newest file known at
//! build time per channel, and the compiled revocations), never from "no
//! floor". The baseline is also applied over every loaded state, so the
//! floor can never sit below it.
//!
//! # Writes only move forward
//!
//! [`StateStore::save`] merges into what's stored ([`TrustState::absorb`]):
//! floors and the trusted time take the maximum, revocations the union. Two
//! refreshes racing can't lower the floor or lose a revocation.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::envelope::Channel;

/// Per-channel compiled floors: the serial of the newest file known when
/// this release was built. Filled by the publish workflow with the release.
pub const COMPILED_FLOORS: &[(Channel, u64)] = &[];

/// Compiled revocations: fingerprints of keys revoked when this release
/// was built, so a revocation reaches fresh installs through the signed
/// update channel too.
pub const COMPILED_REVOKED: &[&str] = &[];

/// What a release knows at build time; the state never goes below it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Baseline {
    /// The lowest acceptable serial per channel.
    pub floors: BTreeMap<Channel, u64>,
    /// Revoked key fingerprints.
    pub revoked: BTreeSet<String>,
}

impl Baseline {
    /// [`COMPILED_FLOORS`] and [`COMPILED_REVOKED`].
    #[must_use]
    pub fn compiled() -> Self {
        Self {
            floors: COMPILED_FLOORS.iter().copied().collect(),
            revoked: COMPILED_REVOKED.iter().map(|p| (*p).to_owned()).collect(),
        }
    }
}

/// Everything the trust checks remember.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustState {
    /// Revoked key fingerprints. Only ever grows.
    #[serde(default)]
    pub revoked: BTreeSet<String>,
    /// The highest `issued_at` ever accepted: expiry is judged against
    /// `max(now, this)`.
    #[serde(default)]
    pub trusted_time: u64,
    /// Floors from the compiled baseline, per channel.
    #[serde(default)]
    pub pinned_floor: BTreeMap<Channel, u64>,
    /// Per channel.
    #[serde(default)]
    pub channels: BTreeMap<Channel, ChannelState>,
}

/// One channel's memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelState {
    /// The highest serial accepted, per signing key fingerprint.
    #[serde(default)]
    pub serials: BTreeMap<String, u64>,
    /// The last accepted file, verbatim.
    #[serde(default)]
    pub accepted: Option<AcceptedFile>,
    /// When a fetch was last attempted, Unix seconds (any outcome).
    #[serde(default)]
    pub last_attempt: Option<u64>,
}

/// The last accepted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedFile {
    /// Its serial.
    pub serial: u64,
    /// The SHA-256 of its signed payload, hex.
    pub digest: String,
    /// Its signing key's fingerprint.
    pub key: String,
    /// The whole file as received (it is re-verified on every load).
    pub file: String,
    /// Its entity tag (sent back only when conditional requests are on).
    #[serde(default)]
    pub etag: Option<String>,
}

impl TrustState {
    /// The highest serial accepted on `channel` from a key that isn't
    /// revoked. A key revoked after it burned a huge serial no longer holds
    /// the floor up, so publishing can resume with another key.
    #[must_use]
    pub fn accepted_floor(&self, channel: Channel) -> Option<u64> {
        self.channels.get(&channel).and_then(|state| {
            state
                .serials
                .iter()
                .filter(|(key, _)| !self.revoked.contains(*key))
                .map(|(_, serial)| *serial)
                .max()
        })
    }

    /// The floor for `channel`: the compiled one or the accepted one,
    /// whichever is higher.
    #[must_use]
    pub fn floor(&self, channel: Channel) -> Option<u64> {
        let pinned = self.pinned_floor.get(&channel).copied();
        match (pinned, self.accepted_floor(channel)) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
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

    /// Raises this state to `baseline`.
    pub fn apply_baseline(&mut self, baseline: &Baseline) {
        self.revoked.extend(baseline.revoked.iter().cloned());
        for (channel, floor) in &baseline.floors {
            let held = self.pinned_floor.entry(*channel).or_insert(*floor);
            *held = (*held).max(*floor);
        }
    }

    /// Merges `other` in, only ever forward: revocations by union, floors,
    /// the trusted time and attempt times by maximum. The last good file is
    /// the one from an unrevoked key with the higher serial.
    pub fn absorb(&mut self, other: &Self) {
        self.revoked.extend(other.revoked.iter().cloned());
        self.trusted_time = self.trusted_time.max(other.trusted_time);
        for (channel, floor) in &other.pinned_floor {
            let held = self.pinned_floor.entry(*channel).or_insert(*floor);
            *held = (*held).max(*floor);
        }
        for (channel, theirs) in &other.channels {
            let mine = self.channels.entry(*channel).or_default();
            for (key, serial) in &theirs.serials {
                let held = mine.serials.entry(key.clone()).or_insert(*serial);
                *held = (*held).max(*serial);
            }
            mine.last_attempt = mine.last_attempt.max(theirs.last_attempt);
            let pick = |file: &Option<AcceptedFile>| {
                file.as_ref()
                    .map(|f| (!self.revoked.contains(&f.key), f.serial))
            };
            if pick(&theirs.accepted) >= pick(&mine.accepted) {
                mine.accepted.clone_from(&theirs.accepted);
            }
        }
    }
}

/// The store couldn't be read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("remote-config state: {0}")]
pub struct StoreError(pub String);

/// Where [`TrustState`] lives.
pub trait StateStore: Send + Sync {
    /// The saved state; `Ok(None)` only when nothing was ever saved.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when a saved state exists but can't be read, or fails
    /// its checksum. The caller then starts from the [`Baseline`].
    fn load(&self) -> Result<Option<TrustState>, StoreError>;

    /// Merges `state` into what's saved ([`TrustState::absorb`]): a save
    /// never lowers a floor or drops a revocation, whatever ran meanwhile.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when it can't be written.
    fn save(&self, state: &TrustState) -> Result<(), StoreError>;
}

/// A store in memory (tests, and a session with no data folder).
#[derive(Debug, Default)]
pub struct MemoryStore {
    state: Mutex<Option<TrustState>>,
}

impl MemoryStore {
    /// A store holding `state`.
    #[must_use]
    pub fn with(state: TrustState) -> Self {
        Self {
            state: Mutex::new(Some(state)),
        }
    }
}

impl StateStore for MemoryStore {
    fn load(&self) -> Result<Option<TrustState>, StoreError> {
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
        slot.get_or_insert_with(TrustState::default).absorb(state);
        Ok(())
    }
}

/// The on-disk form: the state's JSON and its SHA-256.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: u32,
    sha256: String,
    state: String,
}

/// The durable store: one checksummed file, written atomically. The path
/// is the app's to choose, in its durable data folder, never a cache.
#[derive(Debug)]
pub struct FileStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FileStore {
    /// A store at `path` (its folder must exist).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    fn read(&self) -> Result<Option<TrustState>, StoreError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError(e.to_string())),
        };
        let corrupt = |why: &str| StoreError(format!("corrupt: {why}"));
        let sealed: Sealed = serde_json::from_slice(&bytes).map_err(|_| corrupt("not JSON"))?;
        if sealed.format != 1 {
            return Err(corrupt("format"));
        }
        if crate::hex(&Sha256::digest(sealed.state.as_bytes())) != sealed.sha256 {
            return Err(corrupt("checksum"));
        }
        serde_json::from_str(&sealed.state)
            .map(Some)
            .map_err(|_| corrupt("state"))
    }

    fn write(&self, state: &TrustState) -> std::io::Result<()> {
        let text = serde_json::to_string(state).map_err(std::io::Error::other)?;
        let sealed = serde_json::to_vec(&Sealed {
            format: 1,
            sha256: crate::hex(&Sha256::digest(text.as_bytes())),
            state: text,
        })
        .map_err(std::io::Error::other)?;
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let temp = self
            .path
            .with_file_name(format!("{name}.{}.tmp", std::process::id()));
        let result = (|| {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&sealed)?;
            file.sync_all()?;
            std::fs::rename(&temp, &self.path)?;
            sync_folder(&self.path);
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}

/// Makes the rename durable. Folders can be opened and synced on Unix
/// systems; elsewhere the rename's own guarantees are what there is.
fn sync_folder(path: &Path) {
    if cfg!(unix)
        && let Some(folder) = path.parent()
        && let Err(e) = std::fs::File::open(folder).and_then(|f| f.sync_all())
    {
        tracing::warn!(error = %e, "remote config: folder not synced");
    }
}

impl StateStore for FileStore {
    fn load(&self) -> Result<Option<TrustState>, StoreError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| StoreError("poisoned".to_owned()))?;
        self.read()
    }

    fn save(&self, state: &TrustState) -> Result<(), StoreError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| StoreError("poisoned".to_owned()))?;
        // A corrupt file can't be merged; the new state (already raised to
        // the baseline by its caller) replaces it.
        let mut merged = self.read().ok().flatten().unwrap_or_default();
        merged.absorb(state);
        self.write(&merged).map_err(|e| StoreError(e.to_string()))
    }
}
