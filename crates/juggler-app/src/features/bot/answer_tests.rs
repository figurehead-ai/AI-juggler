//! AJ-E3825: Bot through the model, every route over a fake engine that
//! counts its calls (D-144 in full, D-148): records grounded with their
//! citations, a general answer labelled and claiming no records, the
//! scope line, "which model" from the engine's own name, the fabrication
//! guard, every failure with its kind, the details line, the topic's
//! totals, history and memory in the context.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use crate::features::ai::run::{AiCount, AiFailure};
use crate::features::bot::commands::{
    bot_ask_with, bot_new_topic_impl, bot_topic_usage_impl, bot_wrong_impl,
};
use crate::features::bot::dto::{BotAskInput, BotContextPart, BotOutcome, BotTurnInfo, BotUnknown};
use crate::features::bot::fake::{Fake, default_rule, grounded};
use crate::features::bot::tests::{seed_session, state};
use crate::state::AppState;

fn ask_with(state: &Arc<AppState>, fake: &Fake, question: &str) -> BotTurnInfo {
    bot_ask_with(
        state,
        &BotAskInput {
            question: question.into(),
            scope: None,
            again: None,
        },
        fake,
    )
    .unwrap()
}

fn seeded() -> (tempfile::TempDir, Arc<AppState>) {
    let (dir, state) = state();
    seed_session(&state, dir.path(), "s-web", "web", "main");
    seed_session(&state, dir.path(), "s-api", "api", "main");
    (dir, state)
}

/// The maintainer's R1 (D-144 in full): with an AI ready, no Bot reply is
/// made without an engine call; nothing but the model answers in Bot's
/// voice (no records-only "nothing matches", no rows as an answer).
/// "Remember …" is the one reply made by code (ADR-0037 §2.3), an echo
/// card, never an answer.
#[test]
fn no_reply_is_made_without_the_model_while_ai_is_ready() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    for question in [
        "what is 3+5",
        "what LLM model are you",
        "Tell me about my session",
        "What needs me?",
        "Is web working?",
        "Hi!",
        "what can you do?",
        "Write me a function that parses dates",
        "How long did the tests take in api?",
        "Did the deploy to staging work?",
        "Which account has the most room?",
        "xq zzv blorp",
        "Welche Sitzungen laufen gerade?",
    ] {
        let before = fake.calls.get();
        let turn = ask_with(&state, &fake, question);
        assert!(fake.calls.get() > before, "{question}: no engine call");
        assert!(
            matches!(
                turn.outcome,
                BotOutcome::Answered
                    | BotOutcome::General
                    | BotOutcome::DontKnow
                    | BotOutcome::Failed
            ),
            "{question}: {:?}",
            turn.outcome
        );
        assert!(
            turn.written.is_some()
                || turn.general.is_some()
                || turn.failure.is_some()
                || turn.unknown == Some(BotUnknown::Unverified),
            "{question}: a reply not written by the model"
        );
        assert!(
            turn.details.is_some() || turn.failure.is_some(),
            "{question}: details"
        );
    }
    let before = fake.calls.get();
    let remembered = ask_with(&state, &fake, "Remember: call web the front agent");
    assert_eq!(remembered.outcome, BotOutcome::Remembered);
    assert_eq!(
        fake.calls.get(),
        before,
        "the echo card is code's (ADR-0037 §2.3)"
    );
}

/// R1: records found go to the model as grounding; its answer cites them
/// and keeps the citation check; the record chips come with it.
#[test]
fn records_found_are_grounded_and_cited() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let turn = ask_with(&state, &fake, "Is web working?");
    assert_eq!(turn.outcome, BotOutcome::Answered);
    let written = turn.written.as_ref().unwrap();
    assert_eq!(written.parts[0].cites, [1]);
    assert!(
        !turn.rows.is_empty(),
        "a grounded answer shows its record chips"
    );
    assert!(turn.general.is_none());
    assert!(grounded(&fake.requests.borrow()[0]));
    let details = turn.details.as_ref().unwrap();
    assert!(details.records >= 1);

    // A fabricated number is never shown; the failure says so, and no
    // row stands in as the answer.
    let lying = Fake::with(Box::new(|request| {
        if grounded(request) {
            Ok(
                r#"{"sentences":[{"text":"“web” finished and 212 tests passed.","records":[1]}]}"#
                    .to_owned(),
            )
        } else {
            default_rule(request)
        }
    }));
    let turn = ask_with(&state, &lying, "Is web working?");
    assert_eq!(
        (turn.outcome, turn.unknown),
        (BotOutcome::DontKnow, Some(BotUnknown::Unverified))
    );
    assert!(turn.written.is_none());
}

/// R2 and D-148: a question with no records gets the model's short
/// answer, labelled general, with the scope line, never "nothing
/// matches"; it claims no records.
#[test]
fn a_general_question_gets_a_general_answer_with_the_scope_line() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let turn = ask_with(&state, &fake, "what is 3+5");
    assert_eq!(turn.outcome, BotOutcome::General);
    let general = turn.general.as_ref().unwrap();
    assert_eq!(general.text, "8.");
    assert!(general.off_topic);
    assert_eq!(general.model_name, "Apple's on-device model");
    assert!(
        turn.rows.is_empty() && turn.written.is_none(),
        "claims no records"
    );
    assert!(!grounded(&fake.requests.borrow()[0]));
    // Kept, it reads back the same.
    let page = crate::features::bot::commands::bot_page_impl(&state, None, 5).unwrap();
    assert_eq!(page.turns[0].general.as_ref().unwrap().text, "8.");
    assert_eq!(page.turns[0].outcome, BotOutcome::General);
}

/// R3: "which model are you" is answered from the engine's own name and
/// version (given as facts); a reply naming another model isn't shown.
#[test]
fn which_model_comes_from_the_engine() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let turn = ask_with(&state, &fake, "what LLM model are you");
    assert_eq!(turn.outcome, BotOutcome::General);
    assert!(
        turn.general
            .as_ref()
            .unwrap()
            .text
            .contains("Apple's on-device model")
    );
    assert!(!turn.general.as_ref().unwrap().off_topic);
    let sent = &fake.requests.borrow()[0];
    assert!(
        sent.instructions.contains("Apple's on-device model") && sent.instructions.contains("26.1")
    );

    let wrong = Fake::with(Box::new(|_| {
        Ok(r#"{"text":"I'm GPT-4, made by OpenAI.","about_their_work":false}"#.to_owned())
    }));
    let turn = ask_with(&state, &wrong, "Which model are you?");
    assert_eq!(
        turn.unknown,
        Some(BotUnknown::Unverified),
        "never a guessed name"
    );
    assert!(turn.general.is_none());
}

/// The coordinator's R7: a plain answer that states AI Juggler facts (a
/// session's name, a count, a state) is caught and asked again on the
/// grounded route; one that still can't be checked is refused.
#[test]
fn a_plain_answer_claiming_work_facts_goes_grounded() {
    let (_dir, state) = seeded();
    for claim in [
        "You have 3 sessions running.",
        "Your session web is stuck.",
        "Two agents are idle right now.",
    ] {
        let fake = Fake::with(Box::new(move |request| {
            if grounded(request) {
                default_rule(request)
            } else {
                Ok(serde_json::json!({"text": claim, "about_their_work": false}).to_string())
            }
        }));
        let turn = ask_with(&state, &fake, "what is 3+5");
        assert_eq!(fake.calls.get(), 2, "{claim}: asked again, grounded");
        assert!(grounded(&fake.requests.borrow()[1]), "{claim}");
        assert_ne!(
            turn.outcome,
            BotOutcome::General,
            "{claim}: never shown as general"
        );
        assert!(turn.general.is_none(), "{claim}");
    }
    // The model itself says the message was about the user's work.
    let fake = Fake::with(Box::new(|request| {
        if grounded(request) {
            default_rule(request)
        } else {
            Ok(r#"{"text":"I'll look in your records.","about_their_work":true}"#.to_owned())
        }
    }));
    let turn = ask_with(&state, &fake, "how are things going with web?");
    assert_eq!(fake.calls.get(), 2);
    assert_ne!(turn.outcome, BotOutcome::General);
}

/// R6: every engine failure is a failed turn with its kind: no rows, no
/// written or general answer, no silent fallback.
#[test]
fn every_failure_is_said_with_its_kind() {
    let (_dir, state) = seeded();
    for kind in [
        AiFailure::Off,
        AiFailure::NotDownloaded,
        AiFailure::HelperDidntStart,
        AiFailure::TimedOut,
        AiFailure::Refused,
        AiFailure::TooLong,
    ] {
        for question in ["what is 3+5", "Is web working?"] {
            let fake = Fake::with(Box::new(move |_| Err(kind)));
            let turn = ask_with(&state, &fake, question);
            assert_eq!(
                (turn.outcome, turn.failure),
                (BotOutcome::Failed, Some(kind)),
                "{question}"
            );
            assert!(turn.rows.is_empty() && turn.written.is_none() && turn.general.is_none());
        }
    }
}

/// R8: each answer's details (the engine's count where it gave one, an
/// estimate labelled), and the topic's running totals, reset by a New
/// topic.
#[test]
fn details_and_topic_totals_come_from_the_engines_counts() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let first = ask_with(&state, &fake, "what is 3+5");
    let details = first.details.as_ref().unwrap();
    assert_eq!(
        details.tokens_in,
        AiCount {
            n: 300,
            estimated: false
        },
        "the engine's own"
    );
    assert!(
        details.tokens_out.estimated,
        "no count from the engine: labelled"
    );
    assert_eq!(details.model_name, "Apple's on-device model");
    assert_eq!(details.context.window, 4096);
    assert_eq!(details.context.used.n, 300);
    assert!(details.context.parts.iter().all(|p| p.tokens.estimated));
    ask_with(&state, &fake, "Is web working?");
    let usage = bot_topic_usage_impl(&state).unwrap();
    assert_eq!(usage.answers, 2);
    assert_eq!(usage.tokens_in.n, 600);
    assert!(usage.latest.as_ref().unwrap().records >= 1);
    let today = crate::features::ai::run::today(&state);
    assert_eq!((today.answers, today.tokens_in.n), (2, 600));
    bot_new_topic_impl(&state).unwrap();
    assert_eq!(bot_topic_usage_impl(&state).unwrap().answers, 0);
}

/// R9: the context breakdown names its parts; the topic's last exchanges
/// go with a follow-up (never one marked wrong), and the memory used is
/// counted by kind.
#[test]
fn history_and_memory_go_with_a_follow_up_and_are_counted() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let first = ask_with(&state, &fake, "what is 3+5");
    ask_with(&state, &fake, "Remember: call web the front agent");
    let follow = ask_with(&state, &fake, "Is the front agent working?");
    let sent = fake.requests.borrow().last().unwrap().clone();
    assert!(
        sent.prompt.contains("Earlier in this conversation"),
        "{}",
        sent.prompt
    );
    assert!(sent.prompt.contains("what is 3+5"));
    let details = follow.details.as_ref().unwrap();
    let parts: Vec<BotContextPart> = details.context.parts.iter().map(|p| p.part).collect();
    assert_eq!(
        parts,
        [
            BotContextPart::Instructions,
            BotContextPart::Memory,
            BotContextPart::Records,
            BotContextPart::History,
            BotContextPart::Question,
            BotContextPart::World
        ]
    );
    let preferences = details
        .memory
        .iter()
        .find(|m| m.kind == "preferences")
        .unwrap();
    assert_eq!(preferences.n, 1, "the alias used");
    let asked = details.memory.iter().find(|m| m.kind == "asked").unwrap();
    assert!(asked.n >= 1);

    // An answer marked wrong is never sent back.
    bot_wrong_impl(&state, &first.id, true).unwrap();
    ask_with(&state, &fake, "and what is 4+4");
    let sent = fake.requests.borrow().last().unwrap().clone();
    assert!(!sent.prompt.contains("what is 3+5"), "{}", sent.prompt);
}

/// AJ-E3829 (Apple's model, a store with no sessions): asked "what are my
/// sessions" with nothing recorded, the model said nothing matched but
/// cited a record 1 that doesn't exist, and the turn read "I don't know".
/// With no rows, impossible cites are dropped and the honest words stand;
/// a work claim is still refused.
#[test]
fn with_no_records_an_honest_none_stands_and_a_claim_does_not() {
    let (_dir, state) = state();
    let honest = Fake::with(Box::new(|request| {
        if grounded(request) {
            Ok(r#"{"sentences":[{"records":[1],"text":"There are no sessions yet."}]}"#.to_owned())
        } else {
            default_rule(request)
        }
    }));
    let turn = ask_with(&state, &honest, "what are my sessions");
    assert_eq!(turn.outcome, BotOutcome::Answered);
    let written = turn.written.as_ref().unwrap();
    assert!(
        written.parts.iter().all(|p| p.cites.is_empty()),
        "no cite to a record that doesn't exist"
    );

    let claiming = Fake::with(Box::new(|request| {
        if grounded(request) {
            Ok(
                r#"{"sentences":[{"records":[1],"text":"Your two sessions are stuck."}]}"#
                    .to_owned(),
            )
        } else {
            default_rule(request)
        }
    }));
    let turn = ask_with(&state, &claiming, "what are my sessions");
    assert_eq!(turn.outcome, BotOutcome::DontKnow);
    assert!(turn.written.is_none());
}

/// The number of the first memory record in a grounded request's frame.
fn memory_number(prompt: &str) -> Option<u32> {
    prompt
        .lines()
        .find(|l| l.contains(". memory:"))
        .and_then(|l| l.split('.').next())
        .and_then(|n| n.trim().parse().ok())
}

/// AJ-E3832 G9: a question about what the user told Bot gets the memory as
/// a numbered record; an answer resting on it is shown, cites it, and its
/// row opens What Bot remembers (its key); an invented memory isn't shown.
#[test]
fn a_memory_is_a_record_an_answer_can_cite() {
    let (_dir, state) = seeded();
    let remembered = ask_with(
        &state,
        &Fake::new(),
        "Remember: call api the payments agent",
    );
    assert_eq!(remembered.outcome, BotOutcome::Remembered);
    let honest = Fake::with(Box::new(|request| {
        let n = memory_number(&request.prompt).unwrap_or(0);
        Ok(serde_json::json!({"sentences":[{
            "text": "You asked me to call \u{201c}api\u{201d} \u{201c}the payments agent\u{201d}.",
            "records": [n]}]})
        .to_string())
    }));
    let turn = ask_with(&state, &honest, "what did I ask you to remember about api?");
    assert_eq!(turn.outcome, BotOutcome::Answered, "{turn:?}");
    let written = turn.written.as_ref().unwrap();
    let cited = written.parts[0].cites[0] as usize;
    let row = &turn.rows[cited - 1];
    assert_eq!(row.kind, crate::features::bot::dto::BotRowKind::Memory);
    assert_eq!(row.title, "the payments agent");
    assert_eq!(row.about.as_deref(), Some("api"));
    assert_eq!(row.memory_key.as_deref(), Some("alias:session:s-api"));
    // The turn keeps only the memory's random id: read again on the page.
    let storage = state.storage.lock().unwrap();
    let kept = storage.chat().turn(&turn.id).unwrap().unwrap();
    assert!(!kept.cites.contains("payments"), "{}", kept.cites);
    drop(storage);
    let page = crate::features::bot::commands::bot_page_impl(&state, None, 20).unwrap();
    let shown = page.turns.iter().find(|t| t.id == turn.id).unwrap();
    assert!(
        shown
            .rows
            .iter()
            .any(|r| r.title == "the payments agent" && !r.gone)
    );

    // An invented memory claim cites the memory but says what it doesn't.
    let invented = Fake::with(Box::new(|request| {
        let n = memory_number(&request.prompt).unwrap_or(0);
        Ok(serde_json::json!({"sentences":[{
            "text": "You asked me to deploy the billing service on Mondays.",
            "records": [n]}]})
        .to_string())
    }));
    let refused = ask_with(
        &state,
        &invented,
        "what did I ask you to remember about api?",
    );
    assert_eq!(refused.outcome, BotOutcome::DontKnow, "{refused:?}");
    assert!(refused.written.is_none());
    // "I will" stays an act even beside a memory.
    let acting = Fake::with(Box::new(|request| {
        let n = memory_number(&request.prompt).unwrap_or(0);
        Ok(serde_json::json!({"sentences":[{
            "text": "I will call \u{201c}api\u{201d} \u{201c}the payments agent\u{201d}.",
            "records": [n]}]})
        .to_string())
    }));
    let acted = ask_with(&state, &acting, "what did I ask you to remember about api?");
    assert!(acted.written.is_none(), "{acted:?}");
}
