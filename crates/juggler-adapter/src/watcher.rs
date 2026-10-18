//! Session watching: how the core observes harness-native sessions
//! (read-only, by contract — spec §4 design rules).

use std::path::PathBuf;

use crate::error::AdapterError;

/// An observable change in a harness's native session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// A new native session appeared.
    Discovered {
        /// The harness's own session identifier.
        native_id: String,
        /// Where its transcript lives.
        transcript_path: PathBuf,
    },
    /// The transcript grew (new messages/turns appended).
    MessageAppended {
        /// The harness's own session identifier.
        native_id: String,
        /// Where its transcript lives.
        transcript_path: PathBuf,
    },
    /// The session ended (harness wrote a terminal marker).
    Ended {
        /// The harness's own session identifier.
        native_id: String,
    },
}

/// Polls harness-native session state and yields events.
///
/// Implementations must be read-only toward harness state and must never
/// panic on malformed input — watching degrades, not crashes
/// (`docs/plan/QUALITY.md` §1).
pub trait SessionWatcher: Send {
    /// Waits up to `timeout` for the next event; `Ok(None)` means nothing
    /// changed within the window.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError::Watch`] when the watched state cannot be
    /// read (permissions, removed directories, …).
    fn poll(&mut self, timeout: std::time::Duration) -> Result<Option<SessionEvent>, AdapterError>;
}
