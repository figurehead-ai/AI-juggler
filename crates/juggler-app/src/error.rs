//! Typed errors for the application shell.

/// Errors surfaced by app commands. Serialized to the UI as their
/// human-readable message; the UI shows them verbatim next to the failing
/// action.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppError {
    /// Domain/storage failure.
    #[error("{0}")]
    Core(#[from] juggler_core::CoreError),
    /// Supervision failure.
    #[error("{0}")]
    Supervisor(#[from] juggler_supervisor::SupervisorError),
    /// Adapter failure.
    #[error("{0}")]
    Adapter(#[from] juggler_adapter::AdapterError),
    /// A referenced entity does not exist.
    #[error("{entity} {id} not found")]
    NotFound {
        /// Entity kind.
        entity: &'static str,
        /// Missing identifier.
        id: String,
    },
    /// The requested harness has no registered adapter.
    #[error("no adapter for harness {0}")]
    UnknownHarness(String),
    /// The cleanup audit log is unavailable (AJ-E509 Req.3): NO
    /// AUDIT, NO KILL — the refused action is surfaced with this
    /// typed error, never swallowed.
    #[error(
        "cleanup audit unavailable — kill refused (no audit, no kill); retry once the audit log recovers"
    )]
    AuditUnavailable,
    /// Caller-supplied input failed validation (honest rejection).
    #[error("invalid {subject}: {message}")]
    Validation {
        /// What was being validated.
        subject: &'static str,
        /// Why it was rejected.
        message: String,
    },
    /// Filesystem/OS failure with context.
    #[error("{context}: {message}")]
    Io {
        /// What was happening.
        context: &'static str,
        /// The failure.
        message: String,
    },
    /// State lock poisoned (a previous panic while holding it).
    #[error("internal state unavailable")]
    StatePoisoned,
    /// AJ-E307 Req.2: the source is already imported under another
    /// Account — the duplicate-import hole, closed at the command.
    #[error("Already imported as «{label}» — open it from Accounts instead of importing again")]
    AlreadyImported {
        /// The existing account's label.
        label: String,
    },
}

impl AppError {
    /// Convenience constructor for IO errors with context.
    #[must_use]
    pub fn io(context: &'static str, error: &std::io::Error) -> Self {
        Self::Io {
            context,
            message: error.to_string(),
        }
    }
}

/// The coded error wire (AJ-E301 Req.4): every command rejection
/// carries a stable machine `code`, whether a retry can succeed, and
/// a curated `user_message` — the UI renders `user_message` (with a
/// retry affordance when `retryable`), never a raw OS string.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ErrorWire {
    /// Stable `snake_case` machine code (`account_import_failed`, …).
    pub code: String,
    /// True when re-running the same call can succeed (I/O, probes,
    /// audit availability); false for validation/not-found classes.
    pub retryable: bool,
    /// The human-facing message (curated context, not a raw OS error).
    pub user_message: String,
}

impl AppError {
    /// The coded wire form of this error (AJ-E301 Req.4).
    #[must_use]
    pub fn wire(&self) -> ErrorWire {
        let (code, retryable) = match self {
            Self::Core(_) => ("core_storage".to_owned(), true),
            Self::Supervisor(_) => ("supervisor".to_owned(), true),
            Self::Adapter(_) => ("adapter".to_owned(), true),
            Self::NotFound { .. } => ("not_found".to_owned(), false),
            Self::UnknownHarness(_) => ("unknown_harness".to_owned(), false),
            Self::AuditUnavailable => ("audit_unavailable".to_owned(), true),
            Self::Validation { .. } => ("validation".to_owned(), false),
            Self::Io { .. } => ("io".to_owned(), true),
            Self::StatePoisoned => ("state_poisoned".to_owned(), true),
            Self::AlreadyImported { .. } => ("already_imported".to_owned(), false),
        };
        ErrorWire {
            code,
            retryable,
            user_message: self.to_string(),
        }
    }
}

/// Commands hand the coded wire to the webview (AJ-E301 Req.4).
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.wire().serialize(serializer)
    }
}

/// Result alias used by all commands.
pub type AppResult<T> = Result<T, AppError>;
