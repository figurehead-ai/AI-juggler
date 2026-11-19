//! `pool_message`: a pool gets a message, from one agent with `from` (ADR-0036 §2.2).

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
        "pool_message"
    }
    fn targets(&self) -> &'static [TargetKind] {
        &[TargetKind::Pool]
    }
    fn needs(&self) -> Observes {
        Observes::POOL_MESSAGES
    }
    fn param(&self) -> Option<ParamSpec> {
        Some(ParamSpec {
            name: "from",
            kind: ParamKind::Agent,
            required: false,
        })
    }
    fn words(&self) -> &'static CondWords {
        &words::POOL_MESSAGE
    }
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        let Observed::PoolMessage { from } = &obs.event else {
            return Verdict::Ignore;
        };
        match param {
            Some(Param::Agent(wanted)) if from.as_deref() != Some(wanted.as_str()) => {
                Verdict::Ignore
            }
            _ => Verdict::Met,
        }
    }
}
