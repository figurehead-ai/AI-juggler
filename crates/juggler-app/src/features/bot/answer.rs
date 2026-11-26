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
    /// What the world model connected to the question (ADR-0038 §5).
    pub connected: &'a crate::features::ai::world::assemble::WorldContext,
    /// What the user told Bot to remember, as records, when the question
    /// asks about it (AJ-E3832 G9): numbered with the look-up's rows, so
    /// an answer from a memory cites it.
    pub memories: &'a [super::dto::BotRowInfo],
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
        BotUnknown::NothingFound => {
            "No record matched these words. If the memory or the conversation above answers the question, answer from it; otherwise say plainly, in a few words of your own, that you found nothing about it. Never claim how many sessions or tasks there are."
        }
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
                share(BotContextPart::World, &parts.world),
            ],
        },
        memory: with_world_memory(
            vec![
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
                // Watches go only as the world model's lines (a watch the
                // user set, connected to the question).
                BotMemoryShare {
                    kind: "watches".to_owned(),
                    n: 0,
                    tokens: AiCount::default(),
                },
            ],
            // Only what was sent: the general route sends no world section.
            if parts.world.is_empty() {
                &[]
            } else {
                &asked.connected.memory
            },
            asked.memory,
        ),
        world: if parts.world.is_empty() {
            Vec::new()
        } else {
            asked.connected.items.clone()
        },
    }
}

/// The memory breakdown with the world model's memories added by their
/// record's kind (a preference, a note, an earlier question, a watch).
fn with_world_memory(
    mut shares: Vec<BotMemoryShare>,
    world: &[(juggler_core::storage::memory::Kind, String, u32)],
    already: &[BotMemoryInfo],
) -> Vec<BotMemoryShare> {
    use juggler_core::storage::memory::Kind;
    for (kind, key, tokens) in world {
        // An alias the question used is counted once, already.
        if already.iter().any(|m| &m.key == key) {
            continue;
        }
        let slot = match kind {
            Kind::Preference => "preferences",
            Kind::Note => "notes",
            Kind::StandingRequest => "watches",
            _ => "asked",
        };
        if let Some(share) = shares.iter_mut().find(|s| s.kind == slot) {
            share.n = share.n.saturating_add(1);
            share.tokens.n = share.tokens.n.saturating_add(*tokens);
        }
    }
    shares
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
///
/// With no rows at all, no cite can be real: a small model given only
/// "nothing matched" still cites a record 1 (AJ-E3829 on Apple's model,
/// "what are my sessions" with none). Those cites are dropped, then the
/// same guard runs on the words alone.
fn uncited(
    draft: &crate::features::ai::engine::Draft,
    asked: &Asked<'_>,
    no_rows: bool,
) -> Option<Vec<super::dto::BotWrittenPart>> {
    let mut parts = written::parse(draft)?;
    if no_rows {
        for part in &mut parts {
            part.cites.clear();
        }
    }
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
        world: asked.connected.text.clone(),
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
                    if let Some(parts) = uncited(&generated.draft, asked, found.rows.is_empty()) {
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
    let mut found = look_up(asked.reading, asked.world, asked.scope);
    if !asked.memories.is_empty() {
        // The memories are records of this question: the grounded route,
        // and "nothing found" no longer holds.
        if found.unknown == Some(BotUnknown::NothingFound) {
            found.unknown = None;
        }
        // The memories first: the question is about them.
        let rows = std::mem::take(&mut found.rows);
        found.rows = asked.memories.iter().cloned().chain(rows).collect();
        grounded(model, asked, found, &mut answer);
        return answer;
    }
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
    // The world section goes with a general question too (ADR-0038 §5:
    // every turn): what the user said shapes a plain answer; a reply that
    // states a work fact from it is asked again grounded (D-148's guard).
    let world = asked.connected.text.clone();
    let request = general::request(
        asked.question,
        asked.reading.lang,
        &name,
        status.version.as_deref(),
        &history,
        &[memory.as_str(), world.as_str()]
            .iter()
            .filter(|p| !p.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let parts = Parts {
        records: String::new(),
        memory,
        history,
        world,
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
