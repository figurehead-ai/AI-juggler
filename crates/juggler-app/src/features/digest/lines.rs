//! AJ-E3831 R2 (D-149): the digest, map and reduce. **Map:** each calm
//! agent's newest turn gets the feed line's pick ([`crate::features::feed::pick`]),
//! one small call per agent. **Reduce:** code, the rollup's own: the
//! order (needs and failures first), the cap, the counts, every agent's
//! facts. The model never writes the digest; an agent that needs you, has
//! no words, or whose pick fails keeps its rollup line.
//!
//! It runs only when the router serves the digest job (its verdict), so
//! until a harness v6 round passes every digest is the rollup.

use crate::features::ai::engine::Stop;
use crate::features::ai::run::JobModel;
use crate::features::feed::pick;
use crate::features::observe::TurnOutcome;

use super::rollup::{AgentFacts, DigestInfo};

/// Fills each calm line's picked words; the lines picked.
pub fn map(
    model: &dyn JobModel,
    agents: &[AgentFacts],
    digest: &mut DigestInfo,
    stop: &Stop,
) -> u32 {
    let mut picked = 0;
    for line in &mut digest.lines {
        if line.level >= 2 {
            continue;
        }
        let Some(turn) = agents
            .iter()
            .find(|a| a.session == line.session_id)
            .and_then(|a| a.turns.iter().max_by_key(|t| t.at_ms))
        else {
            continue;
        };
        let Some(words) = turn.words.as_deref().filter(|w| !w.trim().is_empty()) else {
            continue;
        };
        let read = pick::Turn {
            words,
            german: pick::german(words),
            tests: turn.tests,
            failed: turn.outcome == Some(TurnOutcome::Failed),
        };
        if let Some(text) = pick::line(model, &read, stop) {
            line.picked = Some(text);
            if line.quote_kind.as_deref() == Some("last") {
                line.quote = None;
                line.quote_kind = None;
            }
            picked += 1;
        }
    }
    digest.ai = picked > 0;
    picked
}
