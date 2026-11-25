//! D-144 in full (AJ-E3825): **Bot is the model, every time.** With an
//! AI ready, every question is answered by the model; nothing else
//! answers in Bot's voice. The record look-up is the model's tool: code
//! runs it first and passes the rows as grounding the model must cite
//! (the Apple helper's guided output has no tool calls Bot can check, so
//! code runs the look-up, ADR-0034 §6.3's "facts first").
//!
//! - **Grounded** (a question about the user's work, records found, or
//!   something AI Juggler doesn't record): the rows, framed as data; the
//!   answer is shown only when every fact in it is in the records it
//!   cites ([`super::written::decide`]).
//! - **General** (D-148: no records in the question): the model's short
//!   reply, labelled as Bot's own, claiming no records; a reply that
//!   claims work facts goes the grounded route instead ([`super::general`]).
//! - **Which model**: the general route with the engine's own name and
//!   version as facts; a reply that names another model isn't shown.
//! - **A failure** is said plainly with its fix ([`AiFailure`]); an
//!   answer that didn't check out is said so; never records alone.
//!
//! "Remember …" stays code (ADR-0037 §2.3: no model is involved), and is
//! the only reply made without the model.

use std::time::Instant;

use super::dto::{
    BotContext, BotContextPart, BotContextShare, BotDetails, BotGeneral, BotMemoryInfo,
    BotMemoryShare, BotOutcome, BotScope, BotUnknown, BotWritten,
};
use super::general;
use super::lookup::{Found, look_up};
use super::model::BotModel;
use super::question::Reading;
use super::world::World;
use super::written::{self, Decided, Extras, Parts};
use crate::features::ai::engine::{Request, Stop};
use crate::features::ai::run::{AiCount, AiFailure, Generated, estimate};

/// What a question comes with.
pub struct Asked<'a> {
    /// The question (secrets masked).
    pub question: &'a str,
    /// How code read it (aliases applied).
    pub reading: &'a Reading,
    /// The records now.
    pub world: &'a World,
    /// The chip it rode on.
    pub scope: Option<&'a BotScope>,
    /// Asked again after its last answer was marked wrong.
    pub again: bool,
    /// The user's memory used in this turn.
    pub memory: &'a [BotMemoryInfo],
    /// The topic's last exchanges, oldest first.
    pub history: &'a [(String, String)],
}

/// What Bot answers.
#[derive(Debug, Default)]
pub struct Answer {
    /// How it ended.
    pub outcome: Option<BotOutcome>,
    /// The look-up's result, when one ran (the rows, "Looked at").
    pub found: Option<Found>,
    /// The grounded answer.
    pub written: Option<BotWritten>,
    /// The general answer.
    pub general: Option<BotGeneral>,
    /// Why the engine failed.
    pub failure: Option<AiFailure>,
    /// Why there is no answer.
    pub unknown: Option<BotUnknown>,
    /// The details line (the last generation's).
    pub details: Option<BotDetails>,
    /// How many generations it took (a test reads it: never zero).
    pub calls: u32,
    /// Why a general reply went the grounded route, or why a grounded
    /// reply wasn't shown (a code, for the AI log).
    pub rerouted: Option<&'static str>,
}

/// D-144's line for what AI Juggler doesn't record: the model says it.
fn not_recorded(unknown: BotUnknown) -> &'static str {
    match unknown {
        BotUnknown::NothingFound => "No record matched the question.",
        BotUnknown::TestDurations => "AI Juggler doesn't record how long tests take.",
        BotUnknown::TestResults => "AI Juggler doesn't record test results for a session.",
        BotUnknown::Costs => "Bot doesn't look up costs; the Usage page has them.",
        BotUnknown::Authors => "AI Juggler doesn't record who wrote what.",
        BotUnknown::Unverified => "",
    }
}

fn memory_lines(memory: &[BotMemoryInfo]) -> Vec<String> {
    memory.iter().map(|m| m.text.clone()).collect()
}

fn share(part: BotContextPart, text: &str) -> BotContextShare {
    BotContextShare {
        part,
        tokens: AiCount {
            n: estimate(text),
            estimated: true,
        },
    }
}

/// The details line and breakdowns for one generation.
fn details(
    generated: &Generated,
    request: &Request,
    parts: &Parts,
    asked: &Asked<'_>,
    records: u32,
    ms: u64,
) -> BotDetails {
    let sent = format!("{}\n{}", request.instructions, request.prompt);
    let tokens_in = AiCount::counted_or_estimated(generated.draft.usage.input, &sent);
    let tokens_out =
        AiCount::counted_or_estimated(generated.draft.usage.output, &generated.draft.text);
    let window = generated.context_tokens;
    let of_slot = |slots: &[&str]| -> Vec<&BotMemoryInfo> {
        asked
            .memory
            .iter()
            .filter(|m| slots.contains(&m.slot.as_str()))
            .collect()
    };
    let memory_share = |kind: &str, items: &[&BotMemoryInfo]| BotMemoryShare {
        kind: kind.to_owned(),
        n: u32::try_from(items.len()).unwrap_or(u32::MAX),
        tokens: AiCount {
            n: items.iter().map(|m| estimate(&m.text)).sum(),
            estimated: true,
        },
    };
    let history_text: String = asked
        .history
        .iter()
        .map(|(q, a)| format!("{q}\n{a}"))
        .collect::<Vec<_>>()
        .join("\n");
    BotDetails {
        model_name: generated.model_name.clone(),
        tokens_in,
        tokens_out,
        ms,
        records,
        context: BotContext {
            window,
            used: tokens_in,
            answer_room: request.max_tokens.min(window.saturating_sub(tokens_in.n)),
            parts: vec![
                share(BotContextPart::Instructions, &request.instructions),
                share(BotContextPart::Memory, &parts.memory),
                share(BotContextPart::Records, &parts.records),
                share(BotContextPart::History, &parts.history),
                share(BotContextPart::Question, &parts.question),
            ],
        },
        memory: vec![
            memory_share("preferences", &of_slot(&["alias", "focus"])),
            memory_share("notes", &of_slot(&["note"])),
            BotMemoryShare {
                kind: "asked".to_owned(),
                n: u32::try_from(asked.history.len()).unwrap_or(u32::MAX),
                tokens: AiCount {
                    n: estimate(&history_text),
                    estimated: true,
                },
            },
            // Watches aren't sent with Bot's questions (ADR-0036's own job).
            BotMemoryShare {
                kind: "watches".to_owned(),
                n: 0,
                tokens: AiCount::default(),
            },
        ],
    }
}

fn generate(
    model: &dyn BotModel,
    request: &Request,
    answer: &mut Answer,
) -> Result<(Generated, u64), AiFailure> {
    let stop = Stop::at(Instant::now() + written::DEADLINE);
    answer.calls += 1;
    let began = Instant::now();
    let generated = model.generate(request, &stop)?;
    let ms = u64::try_from(began.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(generated.ms);
    Ok((generated, ms))
}

fn failed(answer: &mut Answer, failure: AiFailure) {
    answer.outcome = Some(BotOutcome::Failed);
    answer.failure = Some(failure);
}

/// D-144: an answer that couldn't be checked: said so, never shown.
fn unchecked(answer: &mut Answer) {
    answer.outcome = Some(BotOutcome::DontKnow);
    answer.unknown = Some(BotUnknown::Unverified);
}

/// A grounded reply that cites no record and claims nothing about the
/// user's work (D-148's guard), when that is all it is.
fn uncited(
    draft: &crate::features::ai::engine::Draft,
    asked: &Asked<'_>,
) -> Option<Vec<super::dto::BotWrittenPart>> {
    let parts = written::parse(draft)?;
    let text = parts
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let names = general::record_names(asked.world, asked.question);
    let clean = parts.iter().all(|p| p.cites.is_empty())
        && !text.contains('\u{201c}')
        && general::guard(&text, &names).is_ok()
        && !juggler_core::storage::memory::SecretScan::finds_secret(
            &crate::features::ai::memory::RedactionSecrets,
            &text,
        );
    clean.then_some(parts)
}

/// The grounded route: the look-up's rows as data the answer must cite.
fn grounded(model: &dyn BotModel, asked: &Asked<'_>, found: Found, answer: &mut Answer) {
    let extras = Extras {
        note: found
            .unknown
            .map(not_recorded)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .or_else(|| {
                found
                    .rows
                    .is_empty()
                    .then(|| not_recorded(BotUnknown::NothingFound).to_owned())
            }),
        memory: memory_lines(asked.memory),
        history: asked.history.to_vec(),
    };
    let (request, parts) = written::request_with(
        asked.question,
        asked.reading.lang,
        &found.rows,
        asked.again,
        &extras,
    );
    let records = u32::try_from(found.rows.len()).unwrap_or(u32::MAX);
    match generate(model, &request, answer) {
        Err(failure) => failed(answer, failure),
        Ok((generated, ms)) => {
            answer.details = Some(details(&generated, &request, &parts, asked, records, ms));
            match written::decide(
                &generated.draft,
                &found.rows,
                asked.question,
                &asked.world.removed,
            ) {
                Decided::Shown(parts) => {
                    answer.outcome = Some(BotOutcome::Answered);
                    answer.written = Some(BotWritten {
                        parts,
                        model_name: generated.model_name,
                        on_this_mac: true,
                    });
                }
                // A reply that cites nothing and claims no work fact ("AI
                // Juggler doesn't record how long tests take", "nothing
                // matches that") stands: it rests on no row.
                Decided::Records(why) => {
                    if let Some(parts) = uncited(&generated.draft, asked) {
                        answer.outcome = Some(BotOutcome::Answered);
                        answer.written = Some(BotWritten {
                            parts,
                            model_name: generated.model_name,
                            on_this_mac: true,
                        });
                    } else {
                        answer.rerouted = Some(why);
                        unchecked(answer);
                    }
                }
            }
        }
    }
    answer.found = Some(found);
}

#[cfg(test)]
#[path = "answer_tests.rs"]
mod tests;

/// Answers `asked` through `model` (see the module).
#[must_use]
pub fn answer(model: &dyn BotModel, asked: &Asked<'_>) -> Answer {
    let mut answer = Answer::default();
    let found = look_up(asked.reading, asked.world, asked.scope);
    // Rows its own words matched, or something AI Juggler knows it
    // doesn't record; a question whose words matched nothing isn't about
    // the records (the 0.5.1 Nightly's "nothing matches" for 3+5).
    // A question that names nothing gets every record from the look-up:
    // those aren't matches, so they don't make it about the records.
    let has_records = (!asked.reading.subject.is_empty() && !found.rows.is_empty())
        || found
            .unknown
            .is_some_and(|unknown| unknown != BotUnknown::NothingFound);
    if asked.reading.work || asked.scope.is_some() || (has_records && !asked.reading.which_model) {
        grounded(model, asked, found, &mut answer);
        return answer;
    }
    let status = model.status();
    let name = status.model_name.clone().unwrap_or_default();
    let history = written::history_section(asked.history);
    let memory = written::memory_section(&memory_lines(asked.memory));
    let request = general::request(
        asked.question,
        asked.reading.lang,
        &name,
        status.version.as_deref(),
        &history,
        &memory,
    );
    let parts = Parts {
        records: String::new(),
        memory,
        history,
        question: asked.question.to_owned(),
    };
    let (generated, ms) = match generate(model, &request, &mut answer) {
        Ok(done) => done,
        Err(failure) => {
            failed(&mut answer, failure);
            return answer;
        }
    };
    answer.details = Some(details(&generated, &request, &parts, asked, 0, ms));
    let Some((text, about_work)) = general::parse(&generated.draft) else {
        unchecked(&mut answer);
        return answer;
    };
    if asked.reading.which_model && !general::names_itself(&text, &generated.model_name) {
        unchecked(&mut answer);
        return answer;
    }
    let claim = general::guard(&text, &general::record_names(asked.world, asked.question));
    if about_work || claim.is_err() {
        // D-148's guard: a work fact never stands as general; the
        // grounded route answers it from the records it cites.
        answer.rerouted = Some(match claim {
            Err(general::Claim::Name) => "name",
            Err(general::Claim::Count) => "count",
            Err(general::Claim::State) => "state",
            Ok(()) => "about_work",
        });
        let wide = Reading {
            work: true,
            ..asked.reading.clone()
        };
        let found = look_up(&wide, asked.world, asked.scope);
        grounded(model, asked, found, &mut answer);
        return answer;
    }
    answer.outcome = Some(BotOutcome::General);
    answer.general = Some(BotGeneral {
        text,
        model_name: generated.model_name,
        // D-148's scope line: the question was outside AI Juggler.
        off_topic: !asked.reading.which_model && !asked.reading.help,
    });
    answer.found = Some(found);
    answer
}
