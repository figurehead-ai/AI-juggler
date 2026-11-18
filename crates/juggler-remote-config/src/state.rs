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
//! "Corrupt" means read in full and failing its checksum, format or parse.
//! A file that couldn't be read *now* (permissions, I/O errors, no file
//! handles) is not corrupt: the refresh fails closed. Nothing is fetched or
//! saved, nothing new is served, the last state this process knew is kept,
//! and the next refresh reads the file again. A save never overwrites a
//! file it couldn't read.
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
pub enum StoreError {
    /// The saved state was read in full and is not a state (bad checksum,
    /// format or JSON). The caller starts from the [`Baseline`], and a save
    /// may replace it.
    #[error("remote-config state is corrupt: {0}")]
    Corrupt(String),
    /// The state couldn't be read or written now (permissions, I/O, no
    /// file handles, a lock). Nothing may be overwritten on its account:
    /// the refresh fails closed and the next one reads again.
    #[error("remote-config state unavailable: {0}")]
    Unavailable(String),
}

/// Where [`TrustState`] lives.
pub trait StateStore: Send + Sync {
    /// The saved state; `Ok(None)` only when nothing was ever saved.
    ///
    /// # Errors
    ///
    /// [`StoreError::Corrupt`] when a saved state was read and isn't one
    /// (the caller then starts from the [`Baseline`]);
    /// [`StoreError::Unavailable`] when it couldn't be read now (the caller
    /// then fetches nothing and saves nothing).
    fn load(&self) -> Result<Option<TrustState>, StoreError>;

    /// Merges `state` into what's saved ([`TrustState::absorb`]): a save
    /// never lowers a floor or drops a revocation, whatever ran meanwhile.
    /// If what's saved can't be read now, nothing is written.
    ///
    /// # Errors
    ///
    /// [`StoreError::Unavailable`] when it can't be read or written.
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
            .map_err(|_| StoreError::Unavailable("poisoned".to_owned()))
    }

    fn save(&self, state: &TrustState) -> Result<(), StoreError> {
        let mut slot = self
            .state
            .lock()
            .map_err(|_| StoreError::Unavailable("poisoned".to_owned()))?;
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
///
/// Each read-modify-write holds an exclusive advisory lock on
/// `<path>.lock` (reads hold a shared one), so two stores on one path, in
/// one process or two, can't lose each other's updates.
#[derive(Debug)]
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    /// A store at `path` (its folder must exist).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn lock_path(&self) -> PathBuf {
        let mut name = self
            .path
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .unwrap_or_default();
        name.push(".lock");
        self.path.with_file_name(name)
    }

    /// Opens the lock file and takes the lock; it is released when the
    /// returned file is dropped.
    fn lock(&self, exclusive: bool) -> Result<std::fs::File, StoreError> {
        let unavailable = |e: std::io::Error| StoreError::Unavailable(format!("lock: {e}"));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.lock_path())
            .map_err(unavailable)?;
        if exclusive {
            file.lock().map_err(unavailable)?;
        } else {
            file.lock_shared().map_err(unavailable)?;
        }
        Ok(file)
    }

    /// Missing is `Ok(None)`; read in full but not a state is `Corrupt`;
    /// any other failure to read is `Unavailable`.
    fn read(&self) -> Result<Option<TrustState>, StoreError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StoreError::Unavailable(e.to_string())),
        };
        let corrupt = |why: &str| StoreError::Corrupt(why.to_owned());
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
        let mut nonce = [0u8; 8];
        getrandom::fill(&mut nonce).map_err(std::io::Error::other)?;
        let mut name = self
            .path
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .unwrap_or_default();
        name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            crate::hex(&nonce)
        ));
        let temp = self.path.with_file_name(name);
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
        let _lock = self.lock(false)?;
        self.read()
    }

    fn save(&self, state: &TrustState) -> Result<(), StoreError> {
        let _lock = self.lock(true)?;
        let mut merged = match self.read() {
            Ok(saved) => saved.unwrap_or_default(),
            // Read in full and not a state: the new one (already raised to
            // the baseline by its caller) replaces it.
            Err(StoreError::Corrupt(why)) => {
                tracing::warn!(%why, "remote config: replacing a corrupt state");
                TrustState::default()
            }
            // Couldn't read it now: never overwrite what may be good.
            Err(e) => return Err(e),
        };
        merged.absorb(state);
        self.write(&merged)
            .map_err(|e| StoreError::Unavailable(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(serial: u64, key: &str) -> AcceptedFile {
        AcceptedFile {
            serial,
            digest: format!("d{serial}"),
            key: key.to_owned(),
            file: String::new(),
            etag: None,
        }
    }

    #[test]
    fn absorb_takes_the_later_trusted_time() {
        let mut mine = TrustState {
            trusted_time: 50,
            ..TrustState::default()
        };
        mine.absorb(&TrustState {
            trusted_time: 20,
            ..TrustState::default()
        });
        assert_eq!(mine.trusted_time, 50);
        mine.absorb(&TrustState {
            trusted_time: 70,
            ..TrustState::default()
        });
        assert_eq!(mine.trusted_time, 70);
    }

    #[test]
    fn absorb_prefers_a_file_from_an_unrevoked_key() {
        // Mine: a burned serial from a key the other state revoked.
        let mut mine = TrustState::default();
        mine.channels.entry(Channel::Stable).or_default().accepted = Some(accepted(900, "stolen"));
        let mut theirs = TrustState::default();
        theirs.revoked.insert("stolen".to_owned());
        theirs.channels.entry(Channel::Stable).or_default().accepted = Some(accepted(6, "good"));
        mine.absorb(&theirs);
        let kept = mine.channels[&Channel::Stable]
            .accepted
            .as_ref()
            .map(|a| a.serial);
        assert_eq!(kept, Some(6));
        // Between unrevoked files, the higher serial.
        let mut lower = TrustState::default();
        lower.channels.entry(Channel::Stable).or_default().accepted = Some(accepted(5, "good"));
        mine.absorb(&lower);
        let kept = mine.channels[&Channel::Stable]
            .accepted
            .as_ref()
            .map(|a| a.serial);
        assert_eq!(kept, Some(6));
    }
}
