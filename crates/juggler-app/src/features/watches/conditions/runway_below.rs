//! `runway_below`: an account's room crosses below `percent` (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, ParamKind, ParamSpec, Verdict};
use crate::features::observe::{Observation, Observed};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "runway_below"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Account, TargetKind::App, TargetKind::Any]
    }
    fn needs(&self) -> Observes {
        Observes::RUNWAY
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "percent",
            kind: ParamKind::Percent { min: 5, max: 95 },
            required: true,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::RUNWAY_BELOW
    }
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
}
