//! `commits`: a commit lands in the project (offered once the git journal exists, AJ-E3706) (ADR-0036 §2.2).

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
        "commits"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Session, TargetKind::Project]
    }
    fn needs(&self) -> Observes {
        Observes::COMMITS
    }
    fn words(&self) -> &'static CondWords {
        &words::COMMITS
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Commit) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
}
