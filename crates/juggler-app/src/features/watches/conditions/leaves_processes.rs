//! `leaves_processes`: a session ends and leaves processes running (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, Verdict, WORK, words,
};

/// The rule.
pub struct LeavesProcesses;

static ABOUT: About = About {
    code: "leaves_processes",
    targets: WORK,
    needs: Observes::PROCESSES,
    param: None,
    told_as: Some("leftovers"),
    words: &words::LEAVES_PROCESSES,
};

impl ConditionRule for LeavesProcesses {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        match obs.event {
            Observed::ProcessesLeft { count } if count > 0 => Verdict::Met,
            _ => Verdict::Ignore,
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
