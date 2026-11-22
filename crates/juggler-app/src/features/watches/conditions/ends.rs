//! `ends`: the session closes, however it ended (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, Verdict, WORK, words,
};

/// The rule.
pub struct Ends;

static ABOUT: About = About {
    code: "ends",
    targets: WORK,
    needs: Observes::LIFECYCLE,
    param: None,
    told_as: None,
    words: &words::ENDS,
};

impl ConditionRule for Ends {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Ended { .. }) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
