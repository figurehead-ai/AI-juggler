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
    #[error("“{account}” is signed in as {detected} here, not {expected} — nothing was launched")]
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
    #[error("Already imported as “{label}” — open it from Accounts instead of importing again")]
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
    /// A move to account that stopped (ADR-0031, 0.4.9; AJ-E2319: typed):
    /// its class, what holds the files, the move's id, and its words.
    #[error("move stopped ({:?}): {} ({})", .0.class, .0.words, .0.detail)]
    Move(Box<MoveStop>),
}

/// AJ-E2319: what kind of stop a move's refusal is, which decides what
/// the window offers next.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[serde(rename_all = "snake_case")]
pub enum StopClass {
    /// It can clear by itself (a writer still finishing, another move
    /// running): tried once more by itself, then Retry.
    Transient,
    /// Only the person can clear it (an app asking something, a session
    /// working, a terminal on the conversation): no Retry, Check again
    /// reads the offer without quitting anything, and the holder's door.
    NeedsYou,
    /// This setup can't move (the tool's data in a shape the move doesn't
    /// know, the disks): Move hidden, Copy offered.
    Setup,
    /// Something that should never happen: Copy details and Report.
    Bug,
}

/// What holds the conversation's files: an app or profile (with its
/// place's id, for "Go to") or a program outside them.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveHolder {
    /// In words ("Claude app · Work", "claude").
    pub name: String,
    /// The app or profile AI Juggler knows it as, for its door.
    pub place_id: Option<String>,
}

/// A move that stopped (AJ-E2319): `words` say why in plain words, never
/// a path; `detail` is the program's own text, paths and all, for Copy
/// details.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MoveStop {
    /// What it offers next.
    pub class: StopClass,
    /// What holds the files (empty: nothing named).
    pub holders: Vec<MoveHolder>,
    /// The move it stopped (none: refused before one began): Undo targets
    /// it by this id.
    pub move_id: Option<String>,
    /// Both accounts are exactly as they were. `false`: the move was left
    /// part way, for Undo of `move_id`.
    pub unchanged: bool,
    /// What the person reads.
    pub words: String,
    /// What Copy details copies (the home folder written `~`).
    pub detail: String,
}

impl MoveStop {
    /// A stop of `class` that changed nothing.
    #[must_use]
    pub fn new(class: StopClass, words: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            class,
            holders: Vec::new(),
            move_id: None,
            unchanged: true,
            words: words.into(),
            detail: detail.into(),
        }
    }

    /// Held by `holders`.
    #[must_use]
    pub fn held_by(mut self, holders: Vec<MoveHolder>) -> Self {
        self.holders = holders;
        self
    }

    /// Whether trying again can help: only a transient stop that changed
    /// nothing (a Retry quits and reopens both apps again).
    #[must_use]
    pub fn retryable(&self) -> bool {
        self.class == StopClass::Transient && self.unchanged
    }
}

impl From<MoveStop> for AppError {
    fn from(stop: MoveStop) -> Self {
        Self::Move(Box::new(stop))
    }
}

/// A move's stop as the window reads it (AJ-E2319), beside the error's
/// words and detail.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveStopInfo {
    /// What it offers next.
    pub class: StopClass,
    /// What holds the files.
    pub holders: Vec<MoveHolder>,
    /// The move it stopped, for Undo by id.
    pub move_id: Option<String>,
    /// Both accounts are exactly as they were.
    pub unchanged: bool,
}

/// A backup archive refused or not staged (juggler-core reads it): the
/// same errors the restore's check always gave.
impl From<juggler_core::backup_archive::ArchiveError> for AppError {
    fn from(error: juggler_core::backup_archive::ArchiveError) -> Self {
        use juggler_core::backup_archive::ArchiveError;
        match error {
            ArchiveError::Refused(message) => Self::Validation {
                subject: "restore",
                message,
            },
            ArchiveError::Io { context, message } => Self::Io { context, message },
            // Non-exhaustive: a reason added later is still a refusal.
            other => Self::Validation {
                subject: "restore",
                message: other.to_string(),
            },
        }
    }
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
    /// 0.3.9: what happened in the program's own words, for "Copy details"
    /// (a switch that stopped); never shown on its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// AJ-E2319: a move's stop, typed (its class, holders and id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<MoveStopInfo>,
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
            // Every switch failure leaves the original resumable; Retry
            // only where trying again can work (0.3.9).
            Self::Switch { failure, .. } => ("switch_failed".to_owned(), failure.retryable()),
            // The switch screen says a refusal that changed nothing calmly;
            // Retry only where it can help (AJ-E2319).
            Self::Move(stop) => (
                if stop.unchanged {
                    "move_refused"
                } else {
                    "move_stopped"
                }
                .to_owned(),
                stop.retryable(),
            ),
        };
        ErrorWire {
            code,
            retryable,
            // D-25: plain words, from one place; the program's own text
            // goes to the error log (`serialize`).
            user_message: crate::error_words::user_words(self),
            detail: match self {
                Self::Switch { failure, .. } => Some(without_home(&format!(
                    "{} (at {:?})",
                    failure,
                    failure.stage()
                ))),
                Self::Move(stop) => Some(without_home(&stop.detail)),
                _ => None,
            },
            stop: match self {
                Self::Move(stop) => Some(MoveStopInfo {
                    class: stop.class,
                    holders: stop.holders.clone(),
                    move_id: stop.move_id.clone(),
                    unchanged: stop.unchanged,
                }),
                _ => None,
            },
        }
    }
}

/// `text` with the home folder written `~`, as a path and as Claude's
/// folder name for it (`-Users-you-…`): what Copy details copies may go
/// into a public bug report, and the user's name shouldn't.
fn without_home(text: &str) -> String {
    let Some(home) = juggler_platform::user::home().map(|home| home.display().to_string()) else {
        return text.to_owned();
    };
    if home.len() < 2 {
        return text.to_owned();
    }
    text.replace(&home, "~")
        .replace(&home.replace('/', "-"), "~")
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
