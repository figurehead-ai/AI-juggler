//! `task_changes`: a task moves to the state `to` (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, ParamKind, ParamSpec,
    TargetKind, Verdict, words,
};

/// The rule.
pub struct TaskChanges;

static ABOUT: About = About {
    code: "task_changes",
    targets: &[TargetKind::Pool, TargetKind::Project],
    needs: Observes::TASK_EVENTS,
    param: Some(ParamSpec {
        name: "to",
        kind: ParamKind::TaskState,
        required: true,
    }),
    told_as: None,
    words: &words::TASK_CHANGES,
};

impl ConditionRule for TaskChanges {
    fn on(&self, param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match (&obs.event, param) {
            (Observed::TaskMoved { to }, Some(Param::TaskState(wanted))) if to == wanted => {
                Verdict::Met
            }
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
