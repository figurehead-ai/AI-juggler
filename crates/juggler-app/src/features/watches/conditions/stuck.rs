//! `stuck`: it looks stuck and (with `for_min`) stays so that long (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, ParamKind, ParamSpec, Verdict, after};
use crate::features::observe::{Observation, Observed, SessionState};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "stuck"
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
        Observes::STATES
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "for_min",
            kind: ParamKind::Minutes { min: 0, max: 240 },
            required: false,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::STUCK
    }
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::StateEntered {
                state: SessionState::Stuck,
            } => after(param, obs.at_ms),
            Observed::StateEntered { .. } | Observed::TurnEnded { .. } | Observed::Ended { .. } => {
                Verdict::Disarm
            }
            _ => Verdict::Ignore,
        }
    }
}
