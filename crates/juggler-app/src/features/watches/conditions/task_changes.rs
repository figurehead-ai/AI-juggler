//! `task_changes`: a task moves to the state `to` (ADR-0036 §2.2).

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
        "task_changes"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Pool, TargetKind::Project]
    }
    fn needs(&self) -> Observes {
        Observes::TASK_EVENTS
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "to",
            kind: ParamKind::TaskState,
            required: true,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::TASK_CHANGES
    }
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match (&obs.event, param) {
            (Observed::TaskMoved { to }, Some(Param::TaskState(wanted))) if to == wanted => {
                Verdict::Met
            }
            _ => Verdict::Ignore,
        }
    }
}
