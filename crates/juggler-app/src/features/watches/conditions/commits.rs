//! `commits`: a commit lands in the project (offered once the git journal exists, AJ-E3706) (ADR-0036 §2.2).

use super::{
    About, ConditionRule, Memo, Observation, Observed, Observes, Param, TargetKind, Verdict, words,
};

/// The rule.
pub struct Commits;

static ABOUT: About = About {
    code: "commits",
    targets: &[TargetKind::Session, TargetKind::Project],
    needs: Observes::COMMITS,
    param: None,
    told_as: None,
    words: &words::COMMITS,
};

impl ConditionRule for Commits {
    fn on(&self, _param: Option<&Param>, _memo: &mut Memo, obs: &Observation) -> Verdict {
        if matches!(obs.event, Observed::Commit) {
            Verdict::Met
        } else {
            Verdict::Ignore
        }
    }
    fn about(&self) -> &'static About {
        &ABOUT
    }
}
