//! Typed errors for the supervisor layer.

use std::path::PathBuf;

use juggler_core::model::{MachineId, SessionId};

/// Errors surfaced while supervising harness sessions.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SupervisorError {
    /// Spawning the harness process failed.
    #[error("spawn failed for {program}: {message}")]
    Spawn {
        /// The program that was supposed to start.
        program: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// The session id is not currently supervised.
    #[error("session {0} is not supervised")]
    UnknownSession(SessionId),
    /// The candidate's ledger row belongs to a DIFFERENT machine
    /// (AJ-E511, D-13): PID/PGID are only unique per machine+boot, so
    /// a cross-machine signal would be the false-kill bug at fleet
    /// scale. NOTHING was signalled; no cross-machine signal path
    /// exists in this epic (fleet transport is `juggler-link`).
    #[error("recorded on {recorded}, this node is {local} — Juggler will not touch it")]
    WrongMachine {
        /// The machine that owns the record.
        recorded: MachineId,
        /// The executing node's machine id.
        local: MachineId,
    },
    /// The process failed fail-closed identity verification
    /// (AJ-E503 §6.2): the PID may name a stranger now — NOTHING was
    /// signalled. Callers surface this as a typed refusal, never as
    /// [`SupervisorError::UnknownSession`].
    #[error("process {pid} failed identity verification — refusing to touch it")]
    Unverifiable {
        /// The pid that no longer matches its recorded identity.
        pid: u32,
    },
    /// The spawn ledger is degraded (AJ-E501): tracking is read-only
    /// until restart; spawns continue, ledger-backed cleanup does not.
    #[error("process ledger degraded — tracking is read-only until restart")]
    LedgerDegraded,
    /// The operation needs a PTY session but this one runs piped.
    #[error("session {session} has no pty (piped/headless session)")]
    NotATty {
        /// The session without a terminal.
        session: SessionId,
    },
    /// The supervision journal could not be read or written.
    #[error("journal error at {path}: {message}")]
    Journal {
        /// Journal file path.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// An I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
