//! `asks_you`: it asks a question or for an approval, and (with `after_min`) still waits that long (ADR-0036 §2.2).

use super::{
    About, ConditionRule, EVERY, Memo, Observation, Observed, Observes, Param, ParamKind,
    ParamSpec, Verdict, after, words,
};
use crate::features::observe::SessionState;

/// The rule.
pub struct AsksYou;

static ABOUT: About = About {
    code: "asks_you",
    targets: EVERY,
    needs: Observes::ASKS,
    param: Some(ParamSpec {
        name: "after_min",
        kind: ParamKind::Minutes { min: 0, max: 120 },
        required: false,
    }),
    told_as: Some("asks"),
    words: &words::ASKS_YOU,
};

impl ConditionRule for AsksYou {
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::StateEntered {
                state: SessionState::AsksYou,
            }
            | Observed::AskOpened => after(param, obs.at_ms),
            Observed::StateEntered { .. } | Observed::AskResolved | Observed::Ended { .. } => {
                Verdict::Disarm
            }
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
