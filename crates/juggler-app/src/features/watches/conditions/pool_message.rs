//! `pool_message`: a pool gets a message, from one agent with `from` (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, ParamKind, ParamSpec,
    TargetKind, Verdict, words,
};

/// The rule.
pub struct PoolMessage;

static ABOUT: About = About {
    code: "pool_message",
    targets: &[TargetKind::Pool],
    needs: Observes::POOL_MESSAGES,
    param: Some(ParamSpec {
        name: "from",
        kind: ParamKind::Agent,
        required: false,
    }),
    told_as: None,
    words: &words::POOL_MESSAGE,
};

impl ConditionRule for PoolMessage {
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        let Observed::PoolMessage { from } = &obs.event else {
            return Verdict::Ignore;
        };
        match param {
            Some(Param::Agent(wanted)) if from.as_deref() != Some(wanted.as_str()) => {
                Verdict::Ignore
            }
            _ => Verdict::Met,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
