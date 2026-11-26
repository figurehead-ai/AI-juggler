//! AJ-E3831 R2 (D-149): the digest, map and reduce. **Map:** each calm
//! agent's newest turn gets the feed line's pick ([`crate::features::feed::pick`]),
//! one small call per agent. **Reduce:** code, the rollup's own: the
//! order (needs and failures first), the cap, the counts, every agent's
//! facts. The model never writes the digest; an agent that needs you, has
//! no words, or whose pick fails keeps its rollup line, and so does a
//! turn that reverted or abandoned its change (G7, D-149's amendment).
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
        // G7: a reverted or abandoned turn keeps the plain words (what was
        // tried, and that it was undone), never a pick.
        if line.reverted || pick::undone(words) {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::features::ai::scripted::Scripted;
    use crate::features::digest::rollup::{Tests, Turn, Window, rollup};

    fn agent(session: &str, words: &str) -> AgentFacts {
        AgentFacts {
            session: session.into(),
            name: session.into(),
            turns: vec![Turn {
                at_ms: 10,
                outcome: None,
                tests: Tests::Ran {
                    passed: 6,
                    failed: 0,
                },
                caveat: None,
                words: Some(words.into()),
                question: None,
                failure: None,
            }],
            ..AgentFacts::default()
        }
    }

    /// G7: the reverted turn keeps its plain words; the other is picked.
    #[test]
    fn a_reverted_turn_keeps_its_plain_words() {
        let agents = vec![
            agent(
                "yew",
                "I tried a bigger cache for the yew reader but rolled it back because memory doubled. No code changes remain.",
            ),
            agent(
                "elm",
                "I added a retry to the elm export, so the nightly run now finishes.",
            ),
        ];
        let mut digest = rollup(
            Window {
                start_ms: 0,
                end_ms: 20,
            },
            &agents,
            0,
        );
        let model = Scripted::new(&[r#"{"fact":"1","verb":"none"}"#]);
        let picked = map(
            &model,
            &agents,
            &mut digest,
            &Stop::at(Instant::now() + Duration::from_secs(5)),
        );
        assert_eq!(picked, 1);
        assert_eq!(model.calls(), 1, "no pick for the reverted turn");
        let yew = digest
            .lines
            .iter()
            .find(|l| l.session_id == "yew")
            .map(|l| l.picked.clone());
        assert_eq!(yew, Some(None));
        assert!(digest.ai);
    }
}
