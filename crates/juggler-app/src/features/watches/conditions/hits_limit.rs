//! `hits_limit`: it reaches a usage limit (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, TargetKind, Verdict, words,
};
use crate::features::observe::SessionState;

/// The rule.
pub struct HitsLimit;

static ABOUT: About = About {
    code: "hits_limit",
    targets: &[
        TargetKind::Session,
        TargetKind::App,
        TargetKind::Account,
        TargetKind::Any,
    ],
    needs: Observes::LIMIT_HITS,
    param: None,
    told_as: Some("limit"),
    words: &words::HITS_LIMIT,
};

impl ConditionRule for HitsLimit {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(
            obs.event,
            Observed::StateEntered {
                state: SessionState::LimitHit
            }
        ) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
