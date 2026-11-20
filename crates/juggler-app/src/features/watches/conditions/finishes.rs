//! `finishes`: a turn ends cleanly and the agent waits idle (ADR-0036 §2.2).

use super::{
    About, ConditionRule, EVERY, Memo, Observation, Observed, Observes, Param, Verdict, words,
};
use crate::features::observe::{SessionState, TurnOutcome};

/// The rule.
pub struct Finishes;

static ABOUT: About = About {
    code: "finishes",
    targets: EVERY,
    needs: Observes::TURN_ENDS.union(Observes::STATES),
    param: None,
    told_as: Some("finished"),
    words: &words::FINISHES,
};

impl ConditionRule for Finishes {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::TurnEnded {
                outcome: TurnOutcome::Completed,
            }
            | Observed::StateEntered {
                state: SessionState::Finished | SessionState::Idle,
            } => Verdict::Met,
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
