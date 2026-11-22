//! `stuck`: it looks stuck and (with `for_min`) stays so that long (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, ParamKind, ParamSpec,
    Verdict, WORK, after, words,
};
use crate::features::observe::SessionState;

/// The rule.
pub struct Stuck;

static ABOUT: About = About {
    code: "stuck",
    targets: WORK,
    needs: Observes::STATES,
    param: Some(ParamSpec {
        name: "for_min",
        kind: ParamKind::Minutes { min: 0, max: 240 },
        required: false,
    }),
    told_as: Some("stuck"),
    words: &words::STUCK,
};

impl ConditionRule for Stuck {
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::StateEntered {
                state: SessionState::Stuck,
            } => after(param, obs.at_ms),
            Observed::StateEntered { .. } | Observed::TurnEnded { .. } | Observed::Ended { .. } => {
                Verdict::Disarm
            }
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
