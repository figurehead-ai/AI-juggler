//! A session's state as the rail says it, and as the Floor draws it
//! (SPEC §9.7, AJ-E3505 req. 4). The same rules as the rail's
//! `sessionItem` and `conversationItem` (`ui/src/lib/rail-items.ts`): one
//! fixture of cases, `ui/src/lib/fixtures/session-states.json`, is read
//! by this module's tests and by the rail's, so the two never drift.
//!
//! Only what the backend knows: "probably" where the state is worked out
//! rather than recorded (D-37).

use serde::{Deserialize, Serialize};

/// The rail's state (E21's words).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailState {
    /// Asks you.
    AsksYou,
    /// Working.
    Working,
    /// Idle.
    Idle,
    /// Done, review it.
    DoneReview,
    /// Ended.
    Ended,
}

impl RailState {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "asks_you" => Self::AsksYou,
            "working" => Self::Working,
            "idle" => Self::Idle,
            "done_review" => Self::DoneReview,
            "ended" => Self::Ended,
            _ => return None,
        })
    }

    const fn word(self) -> &'static str {
        match self {
            Self::AsksYou => "Asks you",
            Self::Working => "Working",
            Self::Idle => "Idle",
            Self::DoneReview => "Done, review it",
            Self::Ended => "Ended",
        }
    }
}

/// Why a session is in Needs you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    /// It asks a question or a permission.
    Asks,
    /// Its Account is out.
    Limit,
    /// It looks stuck.
    Stuck,
    /// It stopped with an error.
    Failed,
    /// It's done: review it.
    Review,
}

/// How the Floor draws a person at a desk (SPEC §9.7's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum FloorState {
    /// Typing.
    Working,
    /// Leaning back with a mug.
    Idle,
    /// Stands and waves: a question or a permission.
    Asks,
    /// Scratches their head.
    Stuck,
    /// Asleep at the desk: the Account is out.
    Limit,
    /// Hands on head, smoke: stopped with an error.
    Crashed,
    /// Stretches, a green tick.
    Done,
    /// Gone: the desk is kept a while, then freed.
    Ended,
}

/// A session's state, worked out once for the rail's words and the
/// Floor's drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    /// The rail's state.
    pub rail: RailState,
    /// Why it needs you, when it does.
    pub need: Option<Need>,
    /// Worked out, not recorded.
    pub probably: bool,
}

impl Seen {
    /// The words every row uses: the reason it needs you, else its state
    /// ("Probably asks you").
    #[must_use]
    pub fn word(&self) -> String {
        match self.need {
            Some(Need::Limit) => "Limit reached".to_owned(),
            Some(Need::Failed) => "Stopped with an error".to_owned(),
            Some(Need::Stuck) => "Probably stuck".to_owned(),
            Some(Need::Review) => RailState::DoneReview.word().to_owned(),
            Some(Need::Asks) | None if self.probably => {
                format!("Probably {}", self.rail.word().to_lowercase())
            }
            Some(Need::Asks) | None => self.rail.word().to_owned(),
        }
    }

    /// How the Floor draws it: the need first, then the state.
    #[must_use]
    pub const fn floor(&self) -> FloorState {
        match (self.need, self.rail) {
            (Some(Need::Asks), _) | (None, RailState::AsksYou) => FloorState::Asks,
            (Some(Need::Stuck), _) => FloorState::Stuck,
            (Some(Need::Limit), _) => FloorState::Limit,
            (Some(Need::Failed), _) => FloorState::Crashed,
            (Some(Need::Review), _) | (None, RailState::DoneReview) => FloorState::Done,
            (None, RailState::Working) => FloorState::Working,
            (None, RailState::Idle) => FloorState::Idle,
            (None, RailState::Ended) => FloorState::Ended,
        }
    }
}

const ENDED: [&str; 4] = ["finished", "stopped", "failed", "handed_over"];
const NEED_STATUSES: [&str; 4] = ["waiting_user", "stuck", "quota_blocked", "failed"];
const ACTIVE: [&str; 2] = ["running", "starting"];

/// One of AI Juggler's own sessions: its status, its attention record,
/// the tool's own word for its state (its protocol or its hooks) and
/// whether the user snoozed it.
#[must_use]
pub fn own_session(
    status: &str,
    needs_attention: Option<&str>,
    runtime: Option<&str>,
    snoozed: bool,
) -> Seen {
    let raw = needs_attention.or_else(|| NEED_STATUSES.contains(&status).then_some(status));
    let attention = if snoozed { None } else { raw };
    let need = match attention {
        Some("waiting_user") => Some(Need::Asks),
        Some("quota_blocked") => Some(Need::Limit),
        Some("stuck") => Some(Need::Stuck),
        Some("failed") => Some(Need::Failed),
        _ => None,
    };
    let guessed = if matches!(need, Some(Need::Asks | Need::Stuck | Need::Limit)) {
        RailState::AsksYou
    } else if ENDED.contains(&status) {
        RailState::Ended
    } else if ACTIVE.contains(&status) {
        RailState::Working
    } else {
        RailState::Idle
    };
    let exact = if snoozed {
        None
    } else {
        runtime.and_then(RailState::parse)
    };
    Seen {
        rail: exact.unwrap_or(guessed),
        need,
        probably: need == Some(Need::Stuck),
    }
}

/// A session another app runs: whether it's live, the state read from
/// the app, whether that state is worked out, and whether the user has
/// looked since its last activity.
#[must_use]
pub fn other_session(live: bool, state: Option<&str>, probably: bool, looked: bool) -> Seen {
    let rail = match state {
        _ if !live => RailState::Ended,
        Some("asks_you") => RailState::AsksYou,
        Some("working") => RailState::Working,
        Some("done_review") if !looked => RailState::DoneReview,
        _ => RailState::Idle,
    };
    let need = match rail {
        RailState::AsksYou => Some(Need::Asks),
        RailState::DoneReview => Some(Need::Review),
        _ => None,
    };
    Seen {
        rail,
        need,
        probably: rail == RailState::AsksYou && probably,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use serde::Deserialize;

    use super::{FloorState, RailState, other_session, own_session};

    #[derive(Deserialize)]
    struct Own {
        status: String,
        needs_attention: Option<String>,
        runtime: Option<String>,
        snoozed: bool,
        rail: RailState,
        word: String,
        floor: FloorState,
    }

    #[derive(Deserialize)]
    struct Other {
        live: bool,
        state: Option<String>,
        probably: bool,
        looked: bool,
        rail: RailState,
        word: String,
        floor: FloorState,
    }

    #[derive(Deserialize)]
    struct Cases {
        own: Vec<Own>,
        other: Vec<Other>,
    }

    #[test]
    fn the_floor_and_the_rail_read_one_session_the_same_way() {
        let cases: Cases = serde_json::from_str(include_str!(
            "../../../ui/src/lib/fixtures/session-states.json"
        ))
        .unwrap();
        assert!(cases.own.len() >= 15 && cases.other.len() >= 6);
        for case in &cases.own {
            let seen = own_session(
                &case.status,
                case.needs_attention.as_deref(),
                case.runtime.as_deref(),
                case.snoozed,
            );
            let got = (seen.rail, seen.word(), seen.floor());
            assert_eq!(
                got,
                (case.rail, case.word.clone(), case.floor),
                "{} {:?} {:?}",
                case.status,
                case.needs_attention,
                case.runtime
            );
        }
        for case in &cases.other {
            let seen = other_session(case.live, case.state.as_deref(), case.probably, case.looked);
            let got = (seen.rail, seen.word(), seen.floor());
            assert_eq!(
                got,
                (case.rail, case.word.clone(), case.floor),
                "{:?} {:?}",
                case.live,
                case.state
            );
        }
    }
}
