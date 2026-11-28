//! A turn kept and read back (AJ-E3808, ADR-0037 §1.6): what is stored
//! is the question (secrets masked), the outcome, the written answer
//! shown, each row's id with its state *then* (never an agent's words),
//! and the scope. Read back, each row is looked up again by id: how it
//! stands now beside how it stood then, or "no longer here".

use serde::{Deserialize, Serialize};

use super::dto::{
    BotDetails, BotGeneral, BotLang, BotLooked, BotMemoryInfo, BotOutcome, BotPastReminder,
    BotRowInfo, BotRowKind, BotScope, BotShortcut, BotTally, BotTurnInfo, BotUnknown, BotWritten,
    BotWrittenPart,
};
use super::world::World;
use crate::features::ai::run::AiFailure;
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
    /// Every match counted by kind and state when asked.
    #[serde(default)]
    pub tallies: Vec<BotTally>,
    /// The written answer's model and each sentence's cites.
    pub written_model: Option<String>,
    /// Each sentence's cites.
    #[serde(default)]
    pub written_cites: Vec<Vec<u32>>,
    /// The memory kept, by its subject key only: its words live in the
    /// memory store, so a forget leaves none here (the Bot review's H1).
    pub memory: Option<String>,
    /// The memories used, by subject key only.
    #[serde(default)]
    pub used: Vec<String>,
    /// "This is wrong" was said about it.
    #[serde(default)]
    pub flagged: bool,
    /// D-148: a general answer's model (its words are the `shown` column).
    #[serde(default)]
    pub general_model: Option<String>,
    /// The general answer's question was outside AI Juggler.
    #[serde(default)]
    pub off_topic: bool,
    /// AJ-E3825: why the engine failed.
    #[serde(default)]
    pub failure: Option<AiFailure>,
    /// AJ-E3825: the answer's details (counts and sizes, no text).
    #[serde(default)]
    pub details: Option<BotDetails>,
    /// 0.5.2 G30: the turn drafted a reminder; the id of the one set from
    /// it once the user sets it (no words: the reminder keeps its own).
    #[serde(default)]
    pub remind: Option<BotPastReminder>,
}

/// A row as kept: no agent words, no then/now marks.
#[must_use]
pub fn kept_row(row: &BotRowInfo) -> BotRowInfo {
    if row.kind == BotRowKind::Memory {
        // A memory keeps only its random id with the turn: its words are
        // read again when shown, so a forget leaves none behind.
        return super::world::empty_row(BotRowKind::Memory, row.id.clone(), String::new());
    }
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
/// `world` (`None`: show them as kept, the turn just asked), its memories
/// read now through `memory` (a forgotten one reads as none).
#[must_use]
pub fn turn_info(
    turn: &Turn,
    world: Option<&World>,
    memory: &dyn Fn(&str) -> Option<BotMemoryInfo>,
) -> BotTurnInfo {
    let meta: Meta = serde_json::from_str(&turn.entities).unwrap_or_default();
    let kept: Vec<BotRowInfo> = serde_json::from_str(&turn.cites).unwrap_or_default();
    let rows = kept
        .into_iter()
        .map(|then| match world {
            None => then,
            Some(_) if then.kind == BotRowKind::Memory => {
                match memory(&format!("subject:{}", then.id)) {
                    Some(info) => BotRowInfo {
                        title: info.text,
                        about: info.about,
                        memory_key: Some(info.key),
                        at_ms: Some(info.at_ms),
                        ..then
                    },
                    None => BotRowInfo { gone: true, ..then },
                }
            }
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
    let general = match (&turn.shown, &meta.general_model) {
        (Some(shown), Some(model)) if !turn.redacted => Some(BotGeneral {
            text: shown.clone(),
            model_name: model.clone(),
            off_topic: meta.off_topic,
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
        tallies: meta.tallies,
        rows,
        unknown: meta.unknown,
        shortcut: meta.shortcut,
        looked: meta.looked.unwrap_or_else(|| BotLooked {
            reach: "everything".to_owned(),
            scope: None,
            memories: 0,
        }),
        written,
        memory: if turn.redacted {
            None
        } else {
            meta.memory.as_deref().and_then(memory)
        },
        used: if turn.redacted {
            Vec::new()
        } else {
            meta.used.iter().filter_map(|key| memory(key)).collect()
        },
        live: world.is_some(),
        redacted: turn.redacted,
        flagged: meta.flagged,
        general,
        failure: meta.failure,
        details: meta.details,
        watch_draft: None,
        past_reminder: meta.remind,
    }
}
