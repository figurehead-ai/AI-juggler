//! `fails`: a turn or the session fails (ADR-0036 §2.2).

use super::{
    About, ConditionRule, EVERY, Memo, Observation, Observed, Observes, Param, Verdict, words,
};
use crate::features::observe::{EndKind, SessionState, TurnOutcome};

/// The rule.
pub struct Fails;

static ABOUT: About = About {
    code: "fails",
    targets: EVERY,
    needs: Observes::TURN_ENDS.union(Observes::STATES),
    param: None,
    told_as: Some("failed"),
    words: &words::FAILS,
};

impl ConditionRule for Fails {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::TurnEnded {
                outcome: TurnOutcome::Failed,
            }
            | Observed::StateEntered {
                state: SessionState::Failed,
            }
            | Observed::Ended {
                how: EndKind::Error,
            } => Verdict::Met,
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
