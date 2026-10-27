//! Typed errors for the application shell.

/// Errors surfaced by app commands. `Display` is the program's own text
/// (logs and diagnostics); the UI gets plain words from
/// [`crate::error_words`], next to the failing action.
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
    /// AJ-E1001: the Account's state dir is signed in as someone else —
    /// nothing launched (no silent continuation exists).
    #[error("«{account}» is signed in as {detected} here, not {expected} — nothing was launched")]
    IdentityMismatch {
        /// The Account's label.
        account: String,
        /// Its remembered identity (masked).
        expected: String,
        /// The identity detected now (masked).
        detected: String,
    },
    /// AJ-E307 Req.2: the source is already imported under another
    /// Account — the duplicate-import hole, closed at the command.
    #[error("Already imported as «{label}» — open it from Accounts instead of importing again")]
    AlreadyImported {
        /// The existing account's label.
        label: String,
    },
    /// The user asked for something that can't be done now, said in the
    /// app's own words (a duplicate name, a session still running), shown
    /// as it is (AJ-E1501).
    #[error("{0}")]
    Refused(String),
    /// A switch stopped (§9): `words` are what the person reads (plain,
    /// the accounts named); the failure itself — its taxonomy code,
    /// stage and detail — goes to the error log. Built only with
    /// [`AppError::switch`].
    #[error("switch failed ({}, {:?}): {failure}", failure.code().unwrap_or("-"), failure.stage())]
    Switch {
        /// The failure, for the log and the usage-stats class.
        failure: Box<juggler_handover::pipeline::SwitchFailure>,
        /// Its words, from [`juggler_handover::pipeline::SwitchFailure::words`].
        words: String,
    },
}

impl AppError {
    /// A switch from `source` to `target` that stopped: the one way a
    /// switch failure reaches the interface.
    #[must_use]
    pub fn switch(
        failure: juggler_handover::pipeline::SwitchFailure,
        source: &str,
        target: &str,
    ) -> Self {
        let words = failure.words(source, target);
        Self::Switch {
            failure: Box::new(failure),
            words,
        }
    }

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
            Self::Refused(_) => ("refused".to_owned(), false),
            Self::IdentityMismatch { .. } => ("identity_mismatch".to_owned(), false),
            // Every switch failure leaves the original resumable; only a
            // conversation that's gone can't be tried again.
            Self::Switch { failure, .. } => (
                "switch_failed".to_owned(),
                !matches!(
                    **failure,
                    juggler_handover::pipeline::SwitchFailure::OriginalSessionGone
                ),
            ),
        };
        ErrorWire {
            code,
            retryable,
            // D-25: plain words, from one place; the program's own text
            // goes to the error log (`serialize`).
            user_message: crate::error_words::user_words(self),
        }
    }
}

/// Commands hand the coded wire to the webview (AJ-E301 Req.4).
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // AJ-E1103: an error crossing to the interface is one the user
        // met; usage stats count its class (never its text).
        crate::usage_stats::error_surfaced(self);
        let wire = self.wire();
        crate::error_log::record(&wire.code, &self.to_string());
        wire.serialize(serializer)
    }
}

/// Result alias used by all commands.
pub type AppResult<T> = Result<T, AppError>;
