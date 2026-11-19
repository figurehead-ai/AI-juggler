//! `finishes`: a turn ends cleanly and the agent waits idle (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, Verdict};
use crate::features::observe::{Observation, Observed, SessionState, TurnOutcome};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "finishes"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[
            TargetKind::Session,
            TargetKind::App,
            TargetKind::Project,
            TargetKind::Pool,
            TargetKind::Any,
        ]
    }
    fn needs(&self) -> Observes {
        Observes::TURN_ENDS.union(Observes::STATES)
    }
    fn words(&self) -> &'static CondWords {
        &words::FINISHES
    }
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::TurnEnded {
                outcome: TurnOutcome::Completed,
            }
            | Observed::StateEntered {
                state: SessionState::Finished | SessionState::Idle,
            } => Verdict::Met,
            _ => Verdict::Ignore,
        }
    }
}
