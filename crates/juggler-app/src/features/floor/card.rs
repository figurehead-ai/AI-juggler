//! AJ-E3517: what a hover card says a person asks you or did last. Read
//! when the card opens, through the session page's own last-messages
//! reader (its setting and its redaction, AJ-E2108): off in Settings,
//! nothing is read; nothing is kept or written.

use std::sync::Arc;

use juggler_floor::overlay::Said;
use juggler_floor::state::FloorState;

use crate::features::conversations::feed::dto::TurnInfo;
use crate::features::conversations::last_messages::{Ask, last_messages_impl};
use crate::state::AppState;

/// The most a question shows.
pub const QUESTION_MAX: usize = 120;
/// The most a latest step shows.
pub const STEP_MAX: usize = 160;

/// What the session with the rail's `key` asks (in `state`) or did last.
#[must_use]
pub fn said(state: &Arc<AppState>, key: &str, floor_state: FloorState) -> Said {
    let Some((account, id)) = conversation_of(state, key) else {
        return Said::Nothing;
    };
    let ask = Ask {
        max: Some(3),
        ..Ask::default()
    };
    last_messages_impl(state, &account, &id, &ask)
        .map_or(Said::Nothing, |info| from_turns(&info.turns, floor_state))
}

/// The conversation behind a rail key: another app's (`c:account/id`), or
/// the one AI Juggler's own session `s:id` runs.
fn conversation_of(state: &Arc<AppState>, key: &str) -> Option<(String, String)> {
    if let Some(rest) = key.strip_prefix("c:") {
        let (account, id) = rest.split_once('/')?;
        return Some((account.to_owned(), id.to_owned()));
    }
    let own = key.strip_prefix("s:")?;
    crate::features::conversations::commands::list_indexed(state)
        .ok()?
        .into_iter()
        .find(|c| c.session_id.as_deref() == Some(own))
        .map(|c| (c.account_id, c.id))
}

/// From the last turns (oldest first): what the last assistant turn asks,
/// when the session asks you; else its latest step, else its last words.
#[must_use]
pub fn from_turns(turns: &[TurnInfo], floor_state: FloorState) -> Said {
    let Some(last) = turns.iter().rev().find(|t| t.speaker == "assistant") else {
        return Said::Nothing;
    };
    let words = last
        .blocks
        .iter()
        .rev()
        .find(|b| b.kind == "text" && !b.text.trim().is_empty())
        .map(|b| plain(&b.text));
    let step = last.blocks.iter().rev().find_map(|b| {
        (b.kind == "steps")
            .then(|| b.steps.last().cloned())
            .flatten()
    });
    match (floor_state, words, step) {
        (FloorState::Asks, Some(q), _) => Said::Asks(cut(&q, QUESTION_MAX)),
        (_, _, Some(s)) => Said::Step(cut(&plain(&s), STEP_MAX)),
        (_, Some(w), None) => Said::Step(cut(&w, STEP_MAX)),
        (_, None, None) => Said::Nothing,
    }
}

/// Markdown's marks out (emphasis and code anywhere, headings and quotes
/// at a word's start), whitespace folded.
fn plain(text: &str) -> String {
    text.split_whitespace()
        .map(|w| {
            w.trim_start_matches(['#', '>'])
                .chars()
                .filter(|c| !matches!(c, '*' | '`'))
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// At most `max` characters, ending in ".." when cut.
fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let keep: String = text.chars().take(max.saturating_sub(2)).collect();
    format!("{}..", keep.trim_end())
}

#[cfg(test)]
mod tests {
    use super::{QUESTION_MAX, STEP_MAX, from_turns};
    use crate::features::conversations::feed::dto::{TurnBlockInfo, TurnInfo};
    use juggler_floor::overlay::Said;
    use juggler_floor::state::FloorState;

    fn block(kind: &str, text: &str, steps: &[&str]) -> TurnBlockInfo {
        TurnBlockInfo {
            kind: kind.into(),
            text: text.into(),
            steps: steps.iter().map(|s| (*s).to_owned()).collect(),
            hidden: 0,
            calls: Vec::new(),
            thoughts: Vec::new(),
            title: None,
            title_running: None,
            after_thinking: false,
            message_id: None,
            at_ms: None,
            final_answer: false,
        }
    }

    fn turn(speaker: &str, blocks: Vec<TurnBlockInfo>) -> TurnInfo {
        TurnInfo {
            id: "0:0".into(),
            revision: 0,
            offset: 0,
            open: false,
            speaker: speaker.into(),
            blocks,
            blocks_hidden: 0,
            at_ms: None,
            model: None,
            redacted: 0,
        }
    }

    #[test]
    fn a_card_says_the_question_or_the_latest_step_within_its_length() {
        let turns = vec![
            turn("you", vec![block("text", "fix the tests", &[])]),
            turn(
                "assistant",
                vec![
                    block("steps", "", &["ran \"pnpm test\"", "Edit a.ts"]),
                    block("text", "**Should I** also update the `snapshot`?", &[]),
                ],
            ),
        ];
        assert_eq!(
            from_turns(&turns, FloorState::Asks),
            Said::Asks("Should I also update the snapshot?".into())
        );
        assert_eq!(
            from_turns(&turns, FloorState::Working),
            Said::Step("Edit a.ts".into())
        );
        assert_eq!(from_turns(&turns[..1], FloorState::Asks), Said::Nothing);
        let long = "word ".repeat(80);
        let Said::Asks(q) = from_turns(
            &[turn("assistant", vec![block("text", &long, &[])])],
            FloorState::Asks,
        ) else {
            unreachable!("a question")
        };
        assert!(q.chars().count() <= QUESTION_MAX && q.ends_with(".."));
        let Said::Step(s) = from_turns(
            &[turn("assistant", vec![block("text", &long, &[])])],
            FloorState::Idle,
        ) else {
            unreachable!("a step")
        };
        assert!(s.chars().count() <= STEP_MAX);
    }
}
