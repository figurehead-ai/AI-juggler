//! `restarted`: a crashed session or App is started again by keep alive (ADR-0036 §2.2).

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
        "restarted"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::App, TargetKind::Any]
    }
    fn needs(&self) -> Observes {
        Observes::RESTARTS
    }
    fn words(&self) -> &'static CondWords {
        &words::RESTARTED
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Restarted) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
}
