//! `idle_for`: no activity for `min` minutes while it's open (ADR-0036 §2.2).

use juggler_adapter::Observes;

use super::{ConditionRule, Memo, ParamKind, ParamSpec, Verdict, after};
use crate::features::observe::{Observation, Observed};
use crate::features::watches::model::{Param, TargetKind};
use crate::features::watches::words::{self, CondWords};

/// The rule.
pub struct Rule;

/// The registered rule.
pub const RULE: Rule = Rule;

impl ConditionRule for Rule {
    fn code(&self) -> &'static str {
        "idle_for"
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
        Observes::STATES.union(Observes::TURN_ENDS)
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "min",
            kind: ParamKind::Minutes { min: 5, max: 480 },
            required: true,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::IDLE_FOR
    }
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            // Ended: nothing is open to be idle.
            Observed::Ended { .. } => Verdict::Disarm,
            // Anything the session did restarts the count.
            _ if obs.subject.session.is_some() => after(param, obs.at_ms),
            _ => Verdict::Ignore,
        }
    }
    fn on_start(&self, param: Option<&Param>, now_ms: i64) -> Verdict {
        after(param, now_ms)
    }
}
