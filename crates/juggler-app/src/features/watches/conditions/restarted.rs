//! `restarted`: a crashed session or App is started again by keep alive (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, TargetKind, Verdict, words,
};

/// The rule.
pub struct Restarted;

static ABOUT: About = About {
    code: "restarted",
    targets: &[TargetKind::App, TargetKind::Any],
    needs: Observes::RESTARTS,
    param: None,
    told_as: Some("restarted"),
    words: &words::RESTARTED,
};

impl ConditionRule for Restarted {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Restarted) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
