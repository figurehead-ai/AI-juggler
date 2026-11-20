//! `starts`: a new session starts (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, TargetKind, Verdict, words,
};

/// The rule.
pub struct Starts;

static ABOUT: About = About {
    code: "starts",
    targets: &[TargetKind::App, TargetKind::Project, TargetKind::Any],
    needs: Observes::LIFECYCLE,
    param: None,
    told_as: None,
    words: &words::STARTS,
};

impl ConditionRule for Starts {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Started) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
