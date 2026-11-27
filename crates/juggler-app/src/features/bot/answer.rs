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

use std::fmt::Write as _;
use std::time::Instant;

use super::dto::{
    BotContext, BotContextPart, BotContextShare, BotDetails, BotGeneral, BotLang, BotMemoryInfo,
    BotMemoryShare, BotOutcome, BotScope, BotState, BotUnknown, BotWritten,
};
use super::general;
use super::lookup::{Found, look_up, nearest};
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

/// Every state, for [`absent`].
const STATES: [BotState; 13] = [
    BotState::AsksYou,
    BotState::Stuck,
    BotState::Limit,
    BotState::Failed,
    BotState::Blocked,
    BotState::InReview,
    BotState::Working,
    BotState::Finished,
    BotState::Ended,
    BotState::Done,
    BotState::Ready,
    BotState::Todo,
    BotState::Cancelled,
];

/// 0.5.2 G15: the states no session or task holds now, read by code from
/// every record (none when a record's state is unknown: it could be any),
/// and (G18) whether nothing needs the user by Home's rule.
#[must_use]
pub fn absent(world: &World) -> super::check::Absent {
    let rows = || world.sessions.iter().chain(&world.tasks);
    if rows().any(|row| row.state.is_none()) {
        return super::check::Absent::default();
    }
    let task_waits = [
        BotState::AsksYou,
        BotState::Stuck,
        BotState::Limit,
        BotState::Blocked,
        BotState::InReview,
    ];
    super::check::Absent {
        states: STATES
            .into_iter()
            .filter(|state| !rows().any(|row| row.state == Some(*state)))
            .collect(),
        nothing_needs: !world.sessions.iter().any(|row| row.need.is_some())
            && !world
                .tasks
                .iter()
                .any(|row| row.state.is_some_and(|s| task_waits.contains(&s))),
    }
}

/// G15: a sentence saying nothing is in the states asked about, in the
/// question's language, offered to the model only when the check passes
/// it (it is true of every record now).
fn none_sentence(
    asked: &[BotState],
    lang: BotLang,
    absent: &super::check::Absent,
) -> Option<&'static str> {
    let named = |states: &[BotState]| {
        asked.len() == states.len() && states.iter().all(|s| asked.contains(s))
    };
    let (en, de) = if asked.is_empty() || named(&super::check::OPEN) {
        (
            "Nothing is running or waiting on you right now.",
            "Gerade läuft oder wartet nichts.",
        )
    } else if asked.contains(&BotState::AsksYou) {
        (
            "Nothing needs you right now.",
            "Gerade braucht dich nichts.",
        )
    } else if named(&[BotState::Stuck]) {
        ("Nothing is stuck right now.", "Gerade hängt nichts.")
    } else if named(&[BotState::Failed]) {
        ("Nothing failed.", "Nichts ist fehlgeschlagen.")
    } else if named(&[BotState::Working]) {
        ("Nothing is running right now.", "Gerade läuft nichts.")
    } else if named(&[BotState::Finished, BotState::Done]) {
        ("Nothing has finished.", "Nichts ist fertig.")
    } else {
        return None;
    };
    let sentence = match lang {
        BotLang::En => en,
        BotLang::De => de,
    };
    super::check::says_none_in(sentence, absent, &[]).then_some(sentence)
}

/// G15: a sentence about the first nearest record in its own state
/// ("“api” failed."), for the note's example; only one the check passes.
fn nearest_example(rows: &[super::dto::BotRowInfo], lang: BotLang) -> Option<String> {
    let row = rows.first()?;
    let said = match (row.state?, lang) {
        (BotState::Failed, BotLang::En) => "failed",
        (BotState::Ended, BotLang::En) => "ended",
        (BotState::Finished, BotLang::En) => "finished",
        (BotState::Failed, BotLang::De) => "ist fehlgeschlagen",
        (BotState::Ended, BotLang::De) => "ist beendet",
        (BotState::Finished, BotLang::De) => "ist fertig",
        _ => return None,
    };
    let text = format!(
        "\u{201c}{}\u{201d} {said}.",
        super::written::neutral(&row.title)
    );
    let part = super::dto::BotWrittenPart {
        text: text.clone(),
        cites: vec![1],
    };
    super::check::check(&[part], rows, "", &[])
        .is_ok()
        .then_some(text)
}

/// G15's note: nothing is in the state asked about (or nothing is open),
/// so the rows are the most recent records, each in its own state; with a
/// sentence saying so and an example, both true now and passed by the
/// check, never a sentence to copy that the records don't back.
fn nearest_note(
    asked: &Reading,
    scoped: bool,
    rows: &[super::dto::BotRowInfo],
    absent: &super::check::Absent,
) -> String {
    let mut note = String::from(match (asked.states.is_empty(), scoped) {
        (true, false) => {
            "Nothing is open right now: no session or task is working, waiting on the user, stuck or blocked (AI Juggler checked every one)."
        }
        (false, false) => {
            "No session or task is in the state the question asks about right now (AI Juggler checked every one)."
        }
        (true, true) => {
            "Nothing in what the question is about is open right now (AI Juggler checked every one)."
        }
        (false, true) => {
            "Nothing in what the question is about is in the state it asks about right now (AI Juggler checked every one)."
        }
    });
    note.push_str(
        " The records below are the user's most recent ones instead, each in its own state.",
    );
    if let Some(none) = none_sentence(&asked.states, asked.lang, absent) {
        let _ = write!(note, " Say so first: \u{201c}{none}\u{201d}");
    }
    note.push_str(
        " Then name the records by their names in quotation marks, each with its own state and its number",
    );
    match nearest_example(rows, asked.lang) {
        Some(example) => {
            let _ = write!(
                note,
                ", for example: \u{201c}{example}\u{201d} with the record 1."
            );
        }
        None => note.push('.'),
    }
    note
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

/// A grounded reply that cites no record, when every sentence of it says
/// only what code verified (0.5.2 G17, [`super::check::says_only_verified`]):
/// nothing matched the question's words or fell in its time, the store
/// holds none of a kind, or what AI Juggler doesn't record.
///
/// With no rows at all, no cite can be real: a small model given only
/// "nothing matched" still cites a record 1 (AJ-E3829 on Apple's model,
/// "what are my sessions" with none). Those cites are dropped, then the
/// words alone are read.
fn uncited(
    draft: &crate::features::ai::engine::Draft,
    asked: &Asked<'_>,
    no_rows: bool,
    verified: &super::check::Verified<'_>,
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
    let clean = parts.iter().all(|p| {
        p.cites.is_empty() && super::check::says_only_verified(&p.text, asked.question, verified)
    }) && !juggler_core::storage::memory::SecretScan::finds_secret(
        &crate::features::ai::memory::RedactionSecrets,
        &text,
    );
    clean.then_some(parts)
}

/// G17: the question's own time words ("today", "this week"), for a
/// sentence saying nothing fell in that time.
fn time_phrase(question: &str) -> Option<String> {
    let words: Vec<String> = question
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let at = words.iter().position(|w| {
        matches!(
            w.as_str(),
            "today"
                | "yesterday"
                | "week"
                | "morning"
                | "afternoon"
                | "evening"
                | "heute"
                | "gestern"
        )
    })?;
    let word = words.get(at)?;
    Some(match at.checked_sub(1).and_then(|i| words.get(i)) {
        Some(this) if this == "this" => format!("this {word}"),
        _ => word.clone(),
    })
}

/// G17: a sentence code verified for an empty look-up ("Nothing matches
/// “billing”.", "Nothing finished today."), offered only when the check
/// passes it; none otherwise.
fn empty_sentence(asked: &Asked<'_>, verified: &super::check::Verified<'_>) -> Option<String> {
    let de = asked.reading.lang == BotLang::De;
    let sentence = if verified.matched_nothing {
        let subject = asked.reading.subject.join(" ");
        if subject.is_empty() {
            return None;
        }
        if de {
            format!("Nichts passt zu \u{201e}{subject}\u{201c}.")
        } else {
            format!("Nothing matches \u{201c}{subject}\u{201d}.")
        }
    } else {
        let states = verified.window?;
        let time = time_phrase(asked.question)?;
        let finished = [BotState::Finished, BotState::Done];
        let named = |these: &[BotState]| {
            states.len() == these.len() && these.iter().all(|s| states.contains(s))
        };
        match (de, states.is_empty()) {
            (false, true) => format!("There is nothing {time}."),
            (false, false) if named(&finished) => format!("Nothing finished {time}."),
            (false, false) if named(&[BotState::Failed]) => format!("Nothing failed {time}."),
            (false, false) if named(&[BotState::Working]) => format!("Nothing was running {time}."),
            (true, false) if named(&finished) => format!("Nichts ist {time} fertig."),
            _ => return None,
        }
    };
    super::check::says_only_verified(&sentence, asked.question, verified).then_some(sentence)
}

/// G17: the note when a look-up under a time, or over an empty store,
/// found nothing: what code read, never "no record matched these words".
fn empty_note(asked: &Asked<'_>) -> Option<String> {
    if asked.reading.when != super::question::When::Any {
        let state = if asked.reading.states.is_empty() {
            ""
        } else {
            ", in the state it asks about"
        };
        return Some(format!(
            "No session or task falls in the time the question asks about{state} (AI Juggler checked every one). \
If that answers it, say so in one sentence of your own with the question's time word, citing no record."
        ));
    }
    let kinds = match (
        asked.world.sessions.is_empty(),
        asked.world.tasks.is_empty(),
    ) {
        (true, true) => "sessions or tasks",
        (true, false) if asked.reading.kinds.sessions => "sessions",
        (false, true) if asked.reading.kinds.tasks => "tasks",
        _ => return None,
    };
    Some(format!(
        "The user has no {kinds} in AI Juggler yet (AI Juggler checked). Say so in one sentence of your own, citing no record."
    ))
}

/// What the grounded route gives the model (G15, G17): the rows (the
/// nearest records when states matched nothing), the note saying what
/// code read, and what a reply citing nothing may say.
fn grounding<'a>(
    asked: &Asked<'a>,
    mut found: Found,
    absent: &super::check::Absent,
) -> (Found, Option<String>, super::check::Verified<'a>) {
    // G15: states that matched nothing ("what needs me" with nothing
    // waiting) get the most recent records to cite, never none.
    let mut note = None;
    if found.rows.is_empty()
        && found.unknown.is_none()
        && let Some(near) = nearest(asked.reading, asked.world, asked.scope)
    {
        note = Some(nearest_note(
            asked.reading,
            asked.scope.is_some(),
            &near.rows,
            absent,
        ));
        found = near;
    } else if found.rows.is_empty() && found.unknown.is_none() {
        note = empty_note(asked);
    }
    let not_recorded_line = found
        .unknown
        .map(not_recorded)
        .filter(|line| !line.is_empty());
    let verified = super::check::Verified {
        matched_nothing: found.rows.is_empty() && found.unknown == Some(BotUnknown::NothingFound),
        window: (found.rows.is_empty()
            && found.unknown.is_none()
            && asked.reading.when != super::question::When::Any)
            .then_some(asked.reading.states.as_slice()),
        no_sessions: asked.world.sessions.is_empty(),
        no_tasks: asked.world.tasks.is_empty(),
        note: not_recorded_line.filter(|_| found.unknown != Some(BotUnknown::NothingFound)),
    };
    if let Some(sentence) = empty_sentence(asked, &verified) {
        let line = note
            .take()
            .or_else(|| not_recorded_line.map(str::to_owned))
            .unwrap_or_default();
        note = Some(format!(
            "{line} Say only that, in a sentence citing no record: \u{201c}{sentence}\u{201d}"
        ));
    }
    (found, note, verified)
}

/// The grounded route: the look-up's rows as data the answer must cite.
fn grounded(model: &dyn BotModel, asked: &Asked<'_>, found: Found, answer: &mut Answer) {
    let absent = absent(asked.world);
    let (found, note, verified) = grounding(asked, found, &absent);
    let extras = Extras {
        note: note.or_else(|| {
            found
                .unknown
                .map(not_recorded)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    found
                        .rows
                        .is_empty()
                        .then(|| not_recorded(BotUnknown::NothingFound).to_owned())
                })
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
            match written::decide_with(
                &generated.draft,
                &found.rows,
                asked.question,
                &asked.world.removed,
                &absent,
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
                    if let Some(parts) =
                        uncited(&generated.draft, asked, found.rows.is_empty(), &verified)
                    {
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
