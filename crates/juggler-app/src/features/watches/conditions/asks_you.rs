//! `asks_you`: it asks a question or for an approval, and (with `after_min`) still waits that long (ADR-0036 §2.2).

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
        "asks_you"
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
        Observes::ASKS
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "after_min",
            kind: ParamKind::Minutes { min: 0, max: 120 },
            required: false,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::ASKS_YOU
    }
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::StateEntered {
                state: SessionState::AsksYou,
            }
            | Observed::AskOpened => after(param, obs.at_ms),
            Observed::StateEntered { .. } | Observed::AskResolved | Observed::Ended { .. } => {
                Verdict::Disarm
            }
            _ => Verdict::Ignore,
        }
    }
}
