//! Typed errors for the adapter layer.

use std::path::PathBuf;

/// Errors surfaced by harness adapters.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AdapterError {
    /// The harness binary could not be found on this machine.
    #[error("harness not found: {0}")]
    NotFound(String),
    /// The installed harness version is outside the versions this adapter
    /// was verified against.
    #[error("version drift: installed {version}, verified against {verified:?}")]
    VersionDrift {
        /// The version found on this machine.
        version: String,
        /// Versions this adapter was validated against.
        verified: Vec<String>,
    },
    /// Preparing the isolated account directory failed.
    #[error("account directory error at {path}: {message}")]
    AccountDir {
        /// The directory being prepared.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// Launch specification could not be built for the given profile/mode.
    #[error("invalid launch request: {0}")]
    InvalidLaunch(String),
    /// The adapter does not implement this operation.
    #[error("unsupported by this adapter: {0}")]
    Unsupported(String),
    /// An I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Watching harness state failed.
    #[error("watch error: {0}")]
    Watch(String),
}
