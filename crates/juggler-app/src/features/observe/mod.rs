//! ADR-0036 §2.3: the observation bus's vocabulary. An [`Observation`] is
//! a typed fact AI Juggler just wrote (a session's state entered, a turn
//! ended, an ask, an ending, the account's room, processes left, a
//! restart, a pool message, a task's move, a commit, a removal), with
//! the session's App, project, account and pools resolved by whoever
//! publishes it, so its subscribers (the watch engine now; the digest
//! scheduler, AJ-E3712; the 0.6 bot) stay pure.
//!
//! The journals stay the durable record; the bus only fans out what was
//! just written. [`from_status`] maps the supervisor's session statuses,
//! the one stream every session already reports through
//! (`notifications::session_status`), and [`publish_for_session`] hands an
//! observation to the subscribers. Sites that publish today: a session's
//! status, processes left, a crash's restart and an account's reset
//! ([`crate::features::notifications`]). The rest wait for the feed's
//! sources (AJ-E3702, AJ-E3705, AJ-E3706): see the watches module's docs.

use std::sync::Arc;

use juggler_adapter::Observes;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// What an observation is about, resolved when it is published.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    /// The session, for a session's fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The Apps it belongs to: the session's own, or every App of an
    /// account for the account's room.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<String>,
    /// Its project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Its account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The pools it is in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pools: Vec<String>,
}

/// A session's state, as the watch conditions read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Working.
    Working,
    /// Waiting idle after a clean turn (another app's conversation).
    Idle,
    /// Asking the user something, or for an approval.
    AsksYou,
    /// Looks stuck.
    Stuck,
    /// At a usage limit.
    LimitHit,
    /// Finished on its own.
    Finished,
    /// Failed.
    Failed,
    /// Stopped by the user.
    Stopped,
    /// Handed over to another account.
    HandedOver,
}

/// How a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnOutcome {
    /// Cleanly.
    Completed,
    /// Interrupted.
    Interrupted,
    /// It failed.
    Failed,
}

/// How a session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndKind {
    /// It finished on its own.
    Clean,
    /// The user stopped it, or it was handed over.
    Stopped,
    /// It failed or crashed.
    Error,
}

/// A task's state, as a watch names it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum TaskState {
    /// Done.
    Done,
    /// Blocked.
    Blocked,
    /// Waiting for review.
    InReview,
}

/// What kind of thing was removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Removed {
    /// A session no longer exists.
    Session,
    /// An App or profile was removed.
    App,
    /// A project was removed.
    Project,
    /// A pool was removed.
    Pool,
    /// An account was removed.
    Account,
}

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Observed {
    /// A session started.
    Started,
    /// A session entered a state.
    StateEntered {
        /// The state.
        state: SessionState,
    },
    /// A turn ended.
    TurnEnded {
        /// How.
        outcome: TurnOutcome,
    },
    /// An ask opened.
    AskOpened,
    /// An ask was answered or withdrawn.
    AskResolved,
    /// A session ended.
    Ended {
        /// How.
        how: EndKind,
    },
    /// The account's room now, as a share left.
    RunwayLeft {
        /// Percent left, 0–100.
        percent: u8,
    },
    /// The account has room again (its window reset).
    RunwayReset,
    /// A session ended and left processes running.
    ProcessesLeft {
        /// How many.
        count: u32,
    },
    /// A crashed session or App was started again.
    Restarted,
    /// A pool got a message.
    PoolMessage {
        /// The sending agent, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
    },
    /// A task moved to a state.
    TaskMoved {
        /// The state.
        to: TaskState,
    },
    /// A commit landed.
    Commit,
    /// Something a watch may target is gone.
    Removed {
        /// What kind.
        what: Removed,
        /// Its id.
        id: String,
    },
}

impl Observed {
    /// The kind of observation it is ([`Observes`]); `None` for a removal,
    /// which every watch hears.
    #[must_use]
    pub const fn signal(&self) -> Option<Observes> {
        Some(match self {
            Self::Started | Self::Ended { .. } => Observes::LIFECYCLE,
            Self::StateEntered { state } => match state {
                SessionState::AsksYou => Observes::ASKS,
                SessionState::LimitHit => Observes::LIMIT_HITS,
                _ => Observes::STATES,
            },
            Self::TurnEnded { .. } => Observes::TURN_ENDS,
            Self::AskOpened | Self::AskResolved => Observes::ASKS,
            Self::RunwayLeft { .. } | Self::RunwayReset => Observes::RUNWAY,
            Self::ProcessesLeft { .. } => Observes::PROCESSES,
            Self::Restarted => Observes::RESTARTS,
            Self::PoolMessage { .. } => Observes::POOL_MESSAGES,
            Self::TaskMoved { .. } => Observes::TASK_EVENTS,
            Self::Commit => Observes::COMMITS,
            Self::Removed { .. } => return None,
        })
    }

    /// Its code, for the log (`state_entered`, `ended`, …).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::StateEntered { .. } => "state_entered",
            Self::TurnEnded { .. } => "turn_ended",
            Self::AskOpened => "ask_opened",
            Self::AskResolved => "ask_resolved",
            Self::Ended { .. } => "ended",
            Self::RunwayLeft { .. } => "runway_left",
            Self::RunwayReset => "runway_reset",
            Self::ProcessesLeft { .. } => "processes_left",
            Self::Restarted => "restarted",
            Self::PoolMessage { .. } => "pool_message",
            Self::TaskMoved { .. } => "task_moved",
            Self::Commit => "commit",
            Self::Removed { .. } => "removed",
        }
    }
}

/// One observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// When it happened, epoch ms.
    pub at_ms: i64,
    /// What it is about.
    #[serde(default)]
    pub subject: Subject,
    /// What happened.
    pub event: Observed,
    /// Replayed from the journals on start ("while AI Juggler was closed").
    #[serde(default)]
    pub replayed: bool,
}

/// What a session's status (`SessionStatus::as_str`) says happened: the
/// state it entered and, for a status that ends the process, its ending.
#[must_use]
pub fn from_status(status: &str) -> Vec<Observed> {
    let state = |state| Observed::StateEntered { state };
    let ended = |how| Observed::Ended { how };
    match status {
        "starting" => vec![Observed::Started],
        "running" => vec![state(SessionState::Working)],
        "waiting_user" => vec![state(SessionState::AsksYou)],
        "stuck" => vec![state(SessionState::Stuck)],
        "quota_blocked" => vec![state(SessionState::LimitHit), ended(EndKind::Error)],
        "finished" => vec![state(SessionState::Finished), ended(EndKind::Clean)],
        "failed" => vec![state(SessionState::Failed), ended(EndKind::Error)],
        "stopped" => vec![state(SessionState::Stopped), ended(EndKind::Stopped)],
        "handed_over" => vec![state(SessionState::HandedOver), ended(EndKind::Stopped)],
        _ => Vec::new(),
    }
}

/// Hands what happened to a session to the subscribers. The subject is
/// resolved off the caller's thread (it reads the store, and a caller may
/// hold the store's lock), so publishing never blocks or deadlocks.
pub fn publish_for_session(
    state: &Arc<AppState>,
    session: &str,
    events: Vec<Observed>,
    at_ms: i64,
) {
    if events.is_empty() {
        return;
    }
    crate::features::watches::runtime::hear(
        state,
        Pending::Session {
            session: session.to_owned(),
            events,
            at_ms,
        },
    );
}

/// Hands what happened to an account (its room) to the subscribers.
pub fn publish_for_account(state: &Arc<AppState>, account: &str, event: Observed, at_ms: i64) {
    crate::features::watches::runtime::hear(
        state,
        Pending::Account {
            account: account.to_owned(),
            event,
            at_ms,
        },
    );
}

/// An observation waiting for its subject to be resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pending {
    /// About one session.
    Session {
        /// The session's id.
        session: String,
        /// What happened, in order.
        events: Vec<Observed>,
        /// When.
        at_ms: i64,
    },
    /// About one account.
    Account {
        /// The account's id.
        account: String,
        /// What happened.
        event: Observed,
        /// When.
        at_ms: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_that_ends_a_process_ends_its_session() {
        for status in [
            "quota_blocked",
            "finished",
            "failed",
            "stopped",
            "handed_over",
        ] {
            assert!(
                from_status(status)
                    .iter()
                    .any(|event| matches!(event, Observed::Ended { .. })),
                "{status}"
            );
        }
        for status in ["running", "waiting_user", "stuck"] {
            assert!(
                !from_status(status)
                    .iter()
                    .any(|event| matches!(event, Observed::Ended { .. })),
                "{status}"
            );
        }
        assert!(from_status("unknown").is_empty());
    }

    #[test]
    fn a_removal_is_heard_by_every_watch() {
        let removed = Observed::Removed {
            what: Removed::App,
            id: "a".into(),
        };
        assert_eq!(removed.signal(), None);
        assert_eq!(Observed::Commit.signal(), Some(Observes::COMMITS));
    }
}
