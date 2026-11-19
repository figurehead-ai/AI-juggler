//! `runway_resets`: an account has room again (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, Verdict};
use crate::features::observe::{Observation, Observed};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "runway_resets"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Account, TargetKind::Any]
    }
    fn needs(&self) -> Observes {
        Observes::RUNWAY
    }
    fn told_as(&self) -> Option<&'static str> {
        Some("reset")
    }
    fn words(&self) -> &'static CondWords {
        &words::RUNWAY_RESETS
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::RunwayReset) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
}
