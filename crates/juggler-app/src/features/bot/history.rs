//! A turn kept and read back (AJ-E3808, ADR-0037 §1.6): what is stored
//! is the question (secrets masked), the outcome, the written answer
//! shown, each row's id with its state *then* (never an agent's words),
//! and the scope. Read back, each row is looked up again by id: how it
//! stands now beside how it stood then, or "no longer here".

use serde::{Deserialize, Serialize};

use super::dto::{
    BotLang, BotLooked, BotMemoryInfo, BotOutcome, BotRowInfo, BotScope, BotShortcut, BotTurnInfo,
    BotUnknown, BotWritten, BotWrittenPart,
};
use super::world::World;
use juggler_core::storage::memory::Turn;

/// What a turn keeps beside its question (the `entities` column).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    /// The chip.
    pub scope: Option<BotScope>,
    /// The question's language.
    #[serde(default)]
    pub lang: BotLang,
    /// Why Bot didn't know.
    pub unknown: Option<BotUnknown>,
    /// The nearest screen.
    pub shortcut: Option<BotShortcut>,
    /// What it looked at.
    pub looked: Option<BotLooked>,
    /// How many matched.
    #[serde(default)]
    pub total: u32,
    /// The written answer's model and each sentence's cites.
    pub written_model: Option<String>,
    /// Each sentence's cites.
    #[serde(default)]
    pub written_cites: Vec<Vec<u32>>,
    /// A memory kept.
    pub memory: Option<BotMemoryInfo>,
    /// Memories used.
    #[serde(default)]
    pub used: Vec<BotMemoryInfo>,
    /// "This is wrong" was said about it.
    #[serde(default)]
    pub flagged: bool,
}

/// A row as kept: no agent words, no then/now marks.
#[must_use]
pub fn kept_row(row: &BotRowInfo) -> BotRowInfo {
    BotRowInfo {
        quote: None,
        then_state: None,
        gone: false,
        ..row.clone()
    }
}

/// The `cites` column for `rows`.
#[must_use]
pub fn cites_json(rows: &[BotRowInfo]) -> String {
    serde_json::to_string(&rows.iter().map(kept_row).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".to_owned())
}

/// The `shown` column: a written answer's sentences, one a line.
#[must_use]
pub fn shown_text(written: Option<&BotWritten>) -> Option<String> {
    written.map(|w| {
        w.parts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// A kept turn as the window shows it, its rows looked up again in
/// `world` (`None`: show them as kept, the turn just asked).
#[must_use]
pub fn turn_info(turn: &Turn, world: Option<&World>) -> BotTurnInfo {
    let meta: Meta = serde_json::from_str(&turn.entities).unwrap_or_default();
    let kept: Vec<BotRowInfo> = serde_json::from_str(&turn.cites).unwrap_or_default();
    let rows = kept
        .into_iter()
        .map(|then| match world {
            None => then,
            Some(world) => match world.find(then.kind, &then.id) {
                Some(now) => BotRowInfo {
                    then_state: then.state.filter(|was| Some(*was) != now.state),
                    quote: None,
                    ..now.clone()
                },
                None => BotRowInfo { gone: true, ..then },
            },
        })
        .collect::<Vec<_>>();
    let written = match (&turn.shown, &meta.written_model) {
        (Some(shown), Some(model)) if !turn.redacted => Some(BotWritten {
            parts: shown
                .split('\n')
                .enumerate()
                .map(|(i, text)| BotWrittenPart {
                    text: text.to_owned(),
                    cites: meta.written_cites.get(i).cloned().unwrap_or_default(),
                })
                .collect(),
            model_name: model.clone(),
            on_this_mac: true,
        }),
        _ => None,
    };
    BotTurnInfo {
        id: turn.id.clone(),
        at_ms: turn.at_ms,
        question: turn.question.clone(),
        lang: meta.lang,
        outcome: BotOutcome::parse(&turn.outcome),
        scope: meta.scope,
        total: meta.total.max(u32::try_from(rows.len()).unwrap_or(0)),
        rows,
        unknown: meta.unknown,
        shortcut: meta.shortcut,
        looked: meta.looked.unwrap_or_else(|| BotLooked {
            reach: "everything".to_owned(),
            scope: None,
            memories: 0,
        }),
        written,
        memory: meta.memory,
        used: meta.used,
        live: world.is_some(),
        redacted: turn.redacted,
        flagged: meta.flagged,
    }
}
