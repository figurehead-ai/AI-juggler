//! `leaves_processes`: a session ends and leaves processes running (ADR-0036 §2.2).

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
        "leaves_processes"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[
            TargetKind::Session,
            TargetKind::App,
            TargetKind::Project,
            TargetKind::Any,
        ]
    }
    fn needs(&self) -> Observes {
        Observes::PROCESSES
    }
    fn told_as(&self) -> Option<&'static str> {
        Some("leftovers")
    }
    fn words(&self) -> &'static CondWords {
        &words::LEAVES_PROCESSES
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::ProcessesLeft { count } if count > 0 => Verdict::Met,
            _ => Verdict::Ignore,
        }
    }
}
