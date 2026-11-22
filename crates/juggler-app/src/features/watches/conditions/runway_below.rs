//! `runway_below`: an account's room crosses below `percent` (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, ParamKind, ParamSpec,
    TargetKind, Verdict, words,
};

/// The rule.
pub struct RunwayBelow;

static ABOUT: About = About {
    code: "runway_below",
    targets: &[TargetKind::Account, TargetKind::App, TargetKind::Any],
    needs: Observes::RUNWAY,
    param: Some(ParamSpec {
        name: "percent",
        kind: ParamKind::Percent { min: 5, max: 95 },
        required: true,
    }),
    told_as: None,
    words: &words::RUNWAY_BELOW,
};

impl ConditionRule for RunwayBelow {
    /// Met when the share left crosses below the bound: a first reading
    /// already below it is told once too (the user asked to know), and a
    /// reading back above re-arms it.
    fn on(&self, param: Option<&Param>, memo: &mut Memo, obs: &Observation) -> Verdict {
        let (Observed::RunwayLeft { percent }, Some(Param::Percent(bound))) = (&obs.event, param)
        else {
            return Verdict::Ignore;
        };
        let was_below = memo.value.is_some_and(|before| before < i64::from(*bound));
        memo.value = Some(i64::from(*percent));
        if percent < bound && !was_below {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
