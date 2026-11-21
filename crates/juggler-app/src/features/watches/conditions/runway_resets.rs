//! `runway_resets`: an account has room again (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, TargetKind, Verdict, words,
};

/// The rule.
pub struct RunwayResets;

static ABOUT: About = About {
    code: "runway_resets",
    targets: &[TargetKind::Account, TargetKind::Any],
    needs: Observes::RUNWAY,
    param: None,
    told_as: Some("reset"),
    words: &words::RUNWAY_RESETS,
};

impl ConditionRule for RunwayResets {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::RunwayReset) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
