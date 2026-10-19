//! Typed errors for `juggler-core`.

use std::path::PathBuf;

/// Errors surfaced by the domain and storage layers.
///
/// Library callers match on these; they are part of the crate's public API
/// and semver-protected (`docs/plan/QUALITY.md` §3).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A SQLite failure (I/O, constraint, or SQL error).
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    /// Serialized domain state (JSON columns) failed to (de)serialize.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The database was written by a newer AI Juggler and cannot be opened
    /// safely. The user should update the application.
    #[error(
        "database schema version {found} is newer than supported version {supported}; update AI Juggler"
    )]
    SchemaTooNew {
        /// Schema version found in the database.
        found: i64,
        /// Highest schema version this build understands.
        supported: i64,
    },
    /// The file exists but is not a valid AI Juggler database.
    #[error("not an AI Juggler database: {path}")]
    NotADatabase {
        /// The path that was opened.
        path: PathBuf,
        /// The underlying storage error.
        source: rusqlite::Error,
    },
    /// An entity referenced by id does not exist.
    #[error("{entity} {id} not found")]
    NotFound {
        /// The entity kind, e.g. `"account"`.
        entity: &'static str,
        /// The missing identifier.
        id: String,
    },
    /// Filesystem failure with context (the funnel store's JSONL).
    #[error("filesystem: {0}")]
    Io(String),
    /// Caller-supplied input failed validation (honest rejection).
    #[error("invalid {subject}: {message}")]
    Validation {
        /// What was being validated.
        subject: &'static str,
        /// Why it was rejected.
        message: String,
    },
}

impl CoreError {
    /// The error's class as usage statistics name it (AJ-E1103), from the
    /// variant and SQLite's typed result code only — never the message.
    /// `None` for refusals that aren't failures (not found, validation).
    #[must_use]
    pub fn stats_code(&self) -> Option<&'static str> {
        use rusqlite::ErrorCode;
        Some(match self {
            Self::Storage(rusqlite::Error::SqliteFailure(failure, _)) => match failure.code {
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => "db_locked",
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => "db_corrupt",
                ErrorCode::PermissionDenied | ErrorCode::ReadOnly => "permission_denied",
                _ => "io_error",
            },
            Self::Storage(_) | Self::Io(_) => "io_error",
            Self::Serialization(_) => "parse_failed",
            Self::SchemaTooNew { .. } => "version_unsupported",
            Self::NotADatabase { .. } => "db_corrupt",
            Self::NotFound { .. } | Self::Validation { .. } => return None,
        })
    }
}
