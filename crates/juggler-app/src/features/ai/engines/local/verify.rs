//! Hash before load (ADR-0035 §11.8, ADR-0033 §5's stamp): a model file
//! is handed to the helper only after its full SHA-256 matched the one it
//! is named by, and only while it is unchanged since.
//!
//! - **The first load after AI Juggler starts** hashes the whole file
//!   (1 to 2 seconds a gigabyte, on the job's own thread, stopped with the
//!   job). The file must be a plain file (no link) of the exact size, and
//!   the file read must be the one looked at (same device and inode).
//! - **A verified file gets a stamp** (size, device, inode and modified
//!   time). Any later load compares the stamp first; any
//!   difference hashes it again before the helper sees it.
//! - **A mismatch is never loaded.** It stays on disk (it is the user's
//!   download, removed in Settings › AI › Models); the job falls back.
//!
//! What this guards: a cut, corrupt or swapped file reaching llama.cpp's
//! parser, which then only ever reads bytes we measured. A process
//! running as this user can rewrite AI Juggler's own data folder anyway,
//! so it is not a boundary against one.

use std::collections::BTreeMap;
use std::fs::Metadata;
use std::io::Read as _;
use std::path::Path;
use std::sync::Mutex;

use sha2::{Digest as _, Sha256};

use crate::features::ai::engine::{EngineError, Reason, Stop};

/// The bytes read at a time while hashing.
const CHUNK: usize = 1024 * 1024;

/// What a file looked like when it was verified: its size, its identity
/// on disk (the platform seam, [`juggler_platform::transfer::file_id`])
/// and its modified time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    len: u64,
    id: juggler_platform::transfer::FileId,
    modified: Option<std::time::SystemTime>,
}

impl Stamp {
    /// `path`'s stamp from `meta` (its own, a link not followed); `None`
    /// where the system can't tell a file's identity (fails closed).
    fn of(path: &Path, meta: &Metadata) -> Option<Self> {
        Some(Self {
            len: meta.len(),
            id: juggler_platform::transfer::file_id(path).ok()?,
            modified: meta.modified().ok(),
        })
    }
}

/// The stamps of the files verified since AI Juggler started, by hash.
#[derive(Debug, Default)]
pub struct Verified {
    stamps: Mutex<BTreeMap<String, Stamp>>,
}

/// Why a file can't be loaded: it isn't fit (missing, a link, the wrong
/// size, the wrong hash), or the job stopped while it was hashed.
fn not_fit() -> EngineError {
    EngineError::NotHere(Reason::Unknown)
}

impl Verified {
    /// Whether `path` is the file named by `sha256`, of `size_bytes`:
    /// from its stamp when it is unchanged since it was hashed, otherwise
    /// by hashing it now (stopped with `stop`).
    ///
    /// # Errors
    ///
    /// [`EngineError::Stopped`] or [`EngineError::TimedOut`] by `stop`;
    /// [`EngineError::NotHere`] (`Unknown`) for a file that isn't fit.
    pub fn check(
        &self,
        path: &Path,
        sha256: &str,
        size_bytes: u64,
        stop: &Stop,
    ) -> Result<(), EngineError> {
        let looked = std::fs::symlink_metadata(path).map_err(|_| not_fit())?;
        if !looked.file_type().is_file() || looked.len() != size_bytes {
            return Err(not_fit());
        }
        let stamp = Stamp::of(path, &looked).ok_or_else(not_fit)?;
        if self.stamp(sha256) == Some(stamp) {
            return Ok(());
        }
        self.forget(sha256);
        let mut file = std::fs::File::open(path).map_err(|_| not_fit())?;
        let opened = file.metadata().map_err(|_| not_fit())?;
        if !opened.file_type().is_file() || opened.len() != stamp.len {
            return Err(not_fit());
        }
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; CHUNK];
        let mut read_bytes = 0_u64;
        loop {
            if stop.stopped() {
                return Err(EngineError::Stopped);
            }
            if std::time::Instant::now() >= stop.deadline() {
                return Err(EngineError::TimedOut);
            }
            let n = file.read(&mut buffer).map_err(|_| not_fit())?;
            if n == 0 {
                break;
            }
            read_bytes += n as u64;
            if read_bytes > size_bytes {
                return Err(not_fit());
            }
            hasher.update(&buffer[..n]);
        }
        drop(file);
        // The same file, unchanged, after the read: a link or another
        // file put in its place meanwhile changes its identity or time.
        let after = std::fs::symlink_metadata(path)
            .ok()
            .and_then(|meta| Stamp::of(path, &meta));
        let digest = hex(&hasher.finalize());
        if read_bytes != size_bytes || after != Some(stamp) || digest != sha256 {
            tracing::warn!(
                target: "ai",
                "a downloaded model's file doesn't match its hash; it isn't loaded"
            );
            return Err(not_fit());
        }
        if let Ok(mut stamps) = self.stamps.lock() {
            stamps.insert(sha256.to_owned(), stamp);
        }
        Ok(())
    }

    fn stamp(&self, sha256: &str) -> Option<Stamp> {
        self.stamps.lock().ok()?.get(sha256).copied()
    }

    fn forget(&self, sha256: &str) {
        if let Ok(mut stamps) = self.stamps.lock() {
            stamps.remove(sha256);
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn sha(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    fn soon() -> Stop {
        Stop::at(Instant::now() + Duration::from_secs(10))
    }

    #[test]
    fn a_file_is_hashed_once_then_known_by_its_stamp_until_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"GGUF model bytes".to_vec();
        let sum = sha(&bytes);
        let path = dir.path().join(format!("{sum}.gguf"));
        std::fs::write(&path, &bytes).unwrap();
        let verified = Verified::default();
        verified.check(&path, &sum, 16, &soon()).unwrap();
        assert!(verified.stamp(&sum).is_some());
        verified.check(&path, &sum, 16, &soon()).unwrap();
        // The same size, other bytes: the stamp differs, the hash fails.
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&path, b"GGUF model bytez").unwrap();
        assert_eq!(
            verified.check(&path, &sum, 16, &soon()),
            Err(EngineError::NotHere(Reason::Unknown))
        );
        assert!(
            verified.stamp(&sum).is_none(),
            "a changed file is forgotten"
        );
    }

    #[test]
    fn a_wrong_size_a_wrong_hash_a_link_or_no_file_is_never_fit() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"abc".to_vec();
        let sum = sha(&bytes);
        let path = dir.path().join(format!("{sum}.gguf"));
        std::fs::write(&path, &bytes).unwrap();
        let verified = Verified::default();
        let unfit = Err(EngineError::NotHere(Reason::Unknown));
        assert_eq!(verified.check(&path, &sum, 4, &soon()), unfit);
        assert_eq!(verified.check(&path, &sha(b"abd"), 3, &soon()), unfit);
        assert_eq!(
            verified.check(&dir.path().join("missing.gguf"), &sum, 3, &soon()),
            unfit
        );
        #[cfg(unix)]
        {
            let link = dir.path().join("link.gguf");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert_eq!(verified.check(&link, &sum, 3, &soon()), unfit);
        }
        assert_eq!(verified.check(&path, &sum, 3, &soon()), Ok(()));
    }

    #[test]
    fn hashing_stops_with_the_job() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = vec![7_u8; 3 * CHUNK];
        let sum = sha(&bytes);
        let path = dir.path().join(format!("{sum}.gguf"));
        std::fs::write(&path, &bytes).unwrap();
        let verified = Verified::default();
        let stop = soon();
        stop.stop();
        assert_eq!(
            verified.check(&path, &sum, bytes.len() as u64, &stop),
            Err(EngineError::Stopped)
        );
        assert_eq!(
            verified.check(&path, &sum, bytes.len() as u64, &Stop::at(Instant::now())),
            Err(EngineError::TimedOut)
        );
        assert!(verified.stamp(&sum).is_none());
    }
}
