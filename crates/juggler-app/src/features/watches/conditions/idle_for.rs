//! `idle_for`: no activity for `min` minutes while it's open (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, ParamKind, ParamSpec,
    Verdict, WORK, after, words,
};

/// The rule.
pub struct IdleFor;

static ABOUT: About = About {
    code: "idle_for",
    targets: WORK,
    needs: Observes::STATES.union(Observes::TURN_ENDS),
    param: Some(ParamSpec {
        name: "min",
        kind: ParamKind::Minutes { min: 5, max: 480 },
        required: true,
    }),
    told_as: None,
    words: &words::IDLE_FOR,
};

impl ConditionRule for IdleFor {
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            // Ended: nothing is open to be idle.
            Observed::Ended { .. } => Verdict::Disarm,
            // Anything the session did restarts the count.
            _ if obs.subject.session.is_some() => after(param, obs.at_ms),
            _ => Verdict::Ignore,
        }
    }
    fn on_start(&self, param: Option<&Param>, now_ms: i64) -> Verdict {
        after(param, now_ms)
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
