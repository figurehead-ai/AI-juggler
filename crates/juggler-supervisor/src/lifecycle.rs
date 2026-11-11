//! Lifecycle events observed by the supervisor (spec §5.1).

use juggler_core::model::SessionId;

/// How a supervised process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// Exited with a status code.
    Code(i32),
    /// Terminated by a signal; carries the signal number when known.
    Signaled(Option<i32>),
}

impl ExitKind {
    /// True only for a clean zero exit.
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, ExitKind::Code(0))
    }

    /// As the process ledger keeps it: `code:1`, `signal:9`
    /// (`signal:unknown`).
    #[must_use]
    pub fn ledger_words(self) -> String {
        match self {
            ExitKind::Code(code) => format!("code:{code}"),
            ExitKind::Signaled(signal) => format!("signal:{}", signal_words(signal)),
        }
    }

    /// AJ-E2912: tells debug mode a launch ended (target `aij::flow`): its
    /// code, or the signal that ended it; never what it printed.
    pub fn log_exit(self) {
        match self {
            ExitKind::Code(code) => {
                tracing::info!(target: "aij::flow", flow = "launch", step = "exited", code);
            }
            ExitKind::Signaled(signal) => tracing::info!(
                target: "aij::flow",
                flow = "launch",
                step = "exited",
                signal = signal_words(signal),
            ),
        }
    }
}

fn signal_words(signal: Option<i32>) -> String {
    signal.map_or_else(|| "unknown".to_owned(), |s| s.to_string())
}

/// Events emitted by the supervisor for every supervised session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// The process was spawned and is running under this supervisor.
    /// Sent before the thread that waits for the process (and sends its
    /// [`LifecycleEvent::Exited`]) exists, so a start is never received
    /// after its exit.
    Started {
        /// Which session.
        session: SessionId,
        /// OS process id.
        pid: u32,
    },
    /// The process ended (normal exit, crash, or stop ladder).
    Exited {
        /// Which session.
        session: SessionId,
        /// OS process id.
        pid: u32,
        /// How it ended.
        kind: ExitKind,
    },
    /// A helper row was confirmed `Orphaned` by the reconciler
    /// (AJ-E504): its Instance's root exited, the grace window
    /// passed, and two consecutive ticks agreed it was left behind.
    HelperOrphaned {
        /// The Instance the helper was left behind by.
        session: SessionId,
        /// The orphaned helper's process id.
        pid: u32,
    },
}

impl LifecycleEvent {
    /// A [`LifecycleEvent::Started`] for `session`'s process `pid`.
    #[must_use]
    pub fn started(session: &SessionId, pid: u32) -> Self {
        LifecycleEvent::Started {
            session: session.clone(),
            pid,
        }
    }

    /// The session this event belongs to.
    #[must_use]
    pub fn session(&self) -> &SessionId {
        match self {
            LifecycleEvent::Started { session, .. }
            | LifecycleEvent::Exited { session, .. }
            | LifecycleEvent::HelperOrphaned { session, .. } => session,
        }
    }
}
