//! `hits_limit`: it reaches a usage limit (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, Verdict};
use crate::features::observe::{Observation, Observed, SessionState};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "hits_limit"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[
            TargetKind::Session,
            TargetKind::App,
            TargetKind::Account,
            TargetKind::Any,
        ]
    }
    fn needs(&self) -> Observes {
        Observes::LIMIT_HITS
    }
    fn words(&self) -> &'static CondWords {
        &words::HITS_LIMIT
    }
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
}
