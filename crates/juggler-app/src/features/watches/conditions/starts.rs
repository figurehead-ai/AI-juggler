//! `starts`: a new session starts (ADR-0036 §2.2).

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
        "starts"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::App, TargetKind::Project, TargetKind::Any]
    }
    fn needs(&self) -> Observes {
        Observes::LIFECYCLE
    }
    fn words(&self) -> &'static CondWords {
        &words::STARTS
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Started) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
}
