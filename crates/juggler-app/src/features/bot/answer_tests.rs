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

/// A model whose grounded drafts claim what no record holds.
fn lying() -> Fake {
    Fake::with(Box::new(|request| {
        if grounded(request) {
            Ok(
                r#"{"sentences":[{"text":"“web” finished and 212 tests passed.","records":[1]}]}"#
                    .to_owned(),
            )
        } else {
            default_rule(request)
        }
    }))
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
    let lying = lying();
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
    // 0.5.2 G53 (RW1 W4): naming nothing of the user's work, it carries
    // no open record into its prompt (an open session made the model
    // take it for a question about their work).
    assert!(
        fake.requests
            .borrow()
            .iter()
            .all(|r| !r.prompt.contains("web") && !r.prompt.contains("api"))
    );
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
    // A follow-up (G16): it refers back ("And …"), so the history goes.
    let follow = ask_with(&state, &fake, "And is the front agent working?");
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
        .find(|l| l.contains(". what the user told you"))
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

/// RW1's lab as Bot reads it (0.5.2 G15): sessions that ended, one with an
/// error, nothing open and nothing waiting on the user.
fn ended_lab() -> (tempfile::TempDir, Arc<AppState>) {
    use juggler_core::model::SessionStatus;
    let (dir, state) = state();
    let storage = state.storage.lock().unwrap();
    storage
        .accounts()
        .insert(&crate::test_support::account(
            "acc-1",
            "claude-code",
            "Work",
            dir.path().join("acc"),
        ))
        .unwrap();
    storage
        .profiles()
        .insert(&crate::test_support::launch_settings(
            "rec-1", "fixture", "acc-1",
        ))
        .unwrap();
    for (id, title, status) in [
        ("s-cart", "Split the cart total", SessionStatus::Finished),
        ("s-fail", "Rewrite the date parser", SessionStatus::Failed),
        (
            "cx-delete",
            "Remove the old handlers module",
            SessionStatus::Stopped,
        ),
    ] {
        let cwd = dir.path().join(id);
        std::fs::create_dir_all(&cwd).unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(id, "rec-1", status, cwd))
            .unwrap();
        storage
            .sessions()
            .set_label(&juggler_core::model::SessionId::new(id), Some(title))
            .unwrap();
    }
    drop(storage);
    (dir, state)
}

/// A fake whose grounded reply is `json`.
fn grounded_reply(json: &'static str) -> Fake {
    Fake::with(Box::new(
        move |request: &crate::features::ai::engine::Request| {
            Ok(if grounded(request) {
                json.to_owned()
            } else {
                default_rule(request)?
            })
        },
    ))
}

/// 0.5.2 G15: with nothing open, "what are my sessions" gets the most
/// recent records (the failure first) and a note saying nothing is open,
/// never "no record matched"; an answer saying so and naming them stands,
/// and one that invents a count is still refused.
#[test]
fn nothing_open_gives_the_nearest_records_and_a_true_note() {
    let (_dir, state) = ended_lab();
    let honest = grounded_reply(
        r#"{"sentences":[{"text":"Nothing is running or waiting on you right now.","records":[]},{"text":"“Rewrite the date parser” failed.","records":[1]}]}"#,
    );
    let turn = ask_with(&state, &honest, "what are my sessions");
    let sent = honest.requests.borrow().last().unwrap().clone();
    assert!(
        sent.prompt
            .contains("1. session “Rewrite the date parser”; state: failed"),
        "the failure first: {}",
        sent.prompt
    );
    assert!(
        !sent.prompt.contains("No record matched"),
        "{}",
        sent.prompt
    );
    assert!(sent.prompt.contains("most recent ones"), "{}", sent.prompt);
    assert!(
        sent.prompt
            .contains("“Nothing is running or waiting on you right now.”"),
        "the true sentence offered: {}",
        sent.prompt
    );
    assert_eq!(turn.outcome, BotOutcome::Answered);
    assert!(
        turn.rows.len() >= 2 && turn.rows.len() <= 3,
        "{:?}",
        turn.rows
    );
    let invents =
        grounded_reply(r#"{"sentences":[{"text":"You have two sessions","records":[1]}]}"#);
    let turn = ask_with(&state, &invents, "what are my sessions");
    assert_eq!(
        turn.outcome,
        BotOutcome::DontKnow,
        "a count of rows cited is 1"
    );
}

/// 0.5.2 G18: "what needs me" is Home's Needs you, a failure included:
/// the failed session is the record, the answer naming it (failed, or
/// needing you) stands, and "Nothing needs you right now." is refused and
/// never offered while it is there.
#[test]
fn what_needs_me_is_home_s_needs_you_a_failure_included() {
    let (_dir, state) = ended_lab();
    for (said, shown) in [
        ("“Rewrite the date parser” failed.", true),
        ("“Rewrite the date parser” stopped with an error.", true),
        ("“Rewrite the date parser” needs you.", true),
        ("“Remove the old handlers module” needs you.", false),
        ("Nothing needs you right now.", false),
    ] {
        let reply = serde_json::json!({"sentences":[{"text": said, "records": [1]}]}).to_string();
        let fake = Fake::with(Box::new(
            move |request: &crate::features::ai::engine::Request| {
                Ok(if grounded(request) {
                    reply.clone()
                } else {
                    default_rule(request)?
                })
            },
        ));
        let turn = ask_with(&state, &fake, "what needs me");
        let titles: Vec<&str> = turn.rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["Rewrite the date parser"], "{said}");
        let sent = fake.requests.borrow().last().unwrap().clone();
        assert!(
            !sent.prompt.contains("Nothing needs you"),
            "{}",
            sent.prompt
        );
        assert_eq!(
            turn.outcome == BotOutcome::Answered,
            shown,
            "{said}: {:?}",
            turn.outcome
        );
    }
}

/// G15, G18: Search (⌘K) shows only what matched: the nearest records are
/// Bot's, for its answer to cite; "what needs me" finds Home's Needs you.
#[test]
fn search_keeps_showing_only_what_matched() {
    let (_dir, state) = ended_lab();
    let find = |text: &str| {
        crate::features::bot::lookup::find_records(&state, text, None)
            .unwrap()
            .rows
    };
    assert!(find("what are my sessions").is_empty());
    let needs = find("what needs me");
    assert_eq!(needs.len(), 1, "{needs:?}");
    assert_eq!(needs[0].title, "Rewrite the date parser");
}

/// 0.5.2 G16: a question that doesn't refer back is read on its own: "3+5"
/// after a refused grounded question gets no history and none of the
/// topic's earlier questions, so it stays Bot's own answer; a follow-up
/// still gets them.
#[test]
fn a_question_that_doesn_t_refer_back_inherits_nothing_from_its_topic() {
    let (_dir, state) = ended_lab();
    let fake = Fake::new();
    ask_with(&state, &fake, "what is 4+4");
    ask_with(&state, &fake, "what needs me");
    let sum = ask_with(&state, &fake, "3+5");
    assert_eq!(sum.outcome, BotOutcome::General);
    let sent = fake.requests.borrow().last().unwrap().clone();
    assert!(
        !sent.prompt.contains("Earlier in this conversation"),
        "{}",
        sent.prompt
    );
    assert!(
        !sent.prompt.contains("earlier you asked"),
        "{}",
        sent.prompt
    );
    ask_with(&state, &fake, "and 2+2?");
    let sent = fake.requests.borrow().last().unwrap().clone();
    assert!(
        sent.prompt.contains("Earlier in this conversation"),
        "{}",
        sent.prompt
    );
}

/// A fake whose grounded reply is `text`, citing nothing.
fn uncited_reply(text: &'static str) -> Fake {
    Fake::with(Box::new(
        move |request: &crate::features::ai::engine::Request| {
            if grounded(request) {
                Ok(serde_json::json!({"sentences":[{"text": text, "records": []}]}).to_string())
            } else {
                default_rule(request)
            }
        },
    ))
}

/// 0.5.2 G17 (D-130): when a time filter or an unmatched name leaves no
/// record, an uncited "You have no sessions." is refused while sessions
/// exist; what code verified (nothing matched these words, nothing in
/// this time) stands, and the note says what was read.
#[test]
fn an_empty_look_up_never_lets_an_unread_none_through() {
    let (_dir, state) = seeded();
    for (question, honest) in [
        (
            "what is the payments agent doing?",
            "Nothing matches “payments agent”.",
        ),
        ("what finished today?", "Nothing finished today."),
    ] {
        let fake = uncited_reply(honest);
        let turn = ask_with(&state, &fake, question);
        assert_eq!(turn.outcome, BotOutcome::Answered, "{question}: {turn:?}");
        assert!(turn.rows.is_empty(), "{question}");
        for invented in ["You have no sessions.", "There are no sessions yet."] {
            let turn = ask_with(&state, &uncited_reply(invented), question);
            assert_eq!(
                (turn.outcome, turn.unknown),
                (BotOutcome::DontKnow, Some(BotUnknown::Unverified)),
                "{question}: {invented}"
            );
        }
    }
    let fake = uncited_reply("Nothing finished today.");
    ask_with(&state, &fake, "what finished today?");
    let sent = fake.requests.borrow().last().unwrap().clone();
    assert!(sent.prompt.contains("falls in the time"), "{}", sent.prompt);
    assert!(
        !sent.prompt.contains("No record matched"),
        "{}",
        sent.prompt
    );
}

/// G18: a session's need is Home's, read by the one rule
/// (`notifications::needs::session_kind`): may be stuck while its status
/// reads running, and a limit parked till its reset is calm.
#[test]
fn a_session_s_need_is_read_by_home_s_rule() {
    let (_dir, state) = ended_lab();
    let infos = {
        let storage = state.storage.lock().unwrap();
        crate::features::sessions::read_model::session_infos_snapshot(&storage).unwrap()
    };
    let mut info = infos.first().unwrap().clone();
    let row = |info: &crate::features::sessions::dto::InstanceInfo| {
        crate::features::bot::world::session_row(
            info,
            &std::collections::HashMap::new(),
            &mut Vec::new(),
        )
    };
    info.status = "running".into();
    info.needs_attention = Some(crate::features::sessions::dto::NeedsAttention::Stuck);
    let stuck = row(&info);
    assert_eq!(
        (stuck.state, stuck.need),
        (
            Some(crate::features::bot::dto::BotState::Stuck),
            Some("stuck")
        )
    );
    info.status = "quota_blocked".into();
    info.needs_attention = None;
    info.parked = Some("acc-1".into());
    let parked = row(&info);
    assert_eq!(
        (parked.state, parked.need),
        (Some(crate::features::bot::dto::BotState::Limit), None)
    );
    info.status = "failed".into();
    info.parked = None;
    assert_eq!(
        row(&info).need,
        Some("failed"),
        "a failure is in Needs you now"
    );
}

/// 0.5.2 G37 (left from G18): tasks used Bot's own waiting states
/// (blocked, in review), a second rule beside Home's. A task needs the user
/// by Home's one rule (`tasks::needs::needs`): in review it does; blocked
/// (it waits on another task, not on the user) it doesn't.
#[test]
fn a_task_s_need_is_read_by_home_s_rule() {
    use juggler_core::tasks::TaskStatus;
    let (_dir, state) = crate::features::bot::tests::state();
    crate::features::bot::tests::seed_tasks(
        &state,
        &[
            ("aj-0001", "Ship the orders API", TaskStatus::InReview),
            ("aj-0002", "Write the totals docs", TaskStatus::Blocked),
        ],
    );
    let world = crate::features::bot::world::gather(&state).unwrap();
    let need = |title: &str| {
        world
            .tasks
            .iter()
            .find(|row| row.title == title)
            .unwrap()
            .need
    };
    assert_eq!(need("Ship the orders API"), Some("review"));
    assert_eq!(
        need("Write the totals docs"),
        None,
        "blocked: not on the user"
    );
    assert!(!crate::features::bot::answer::absent(&world).nothing_needs);
    let found = crate::features::bot::lookup::find_records(&state, "which tasks need me", None)
        .unwrap()
        .rows;
    let titles: Vec<&str> = found.iter().map(|r| r.title.as_str()).collect();
    assert_eq!(titles, ["Ship the orders API"], "{titles:?}");
}

/// 0.5.2 G29 (RW1 F6): "call api the payments agent", said plainly (no
/// "Remember:"), is the user naming a record: code's own echo card that
/// keeps it (ADR-0037 §2.3, D-144: "Remember …" stays code), never the
/// model's reply, and the alias is found afterwards.
#[test]
fn a_plain_alias_is_remembered_by_code() {
    let (_dir, state) = seeded();
    let fake = Fake::new();
    let turn = ask_with(&state, &fake, "call api the payments agent");
    assert_eq!(turn.outcome, BotOutcome::Remembered, "{turn:?}");
    let memory = turn.memory.as_ref().unwrap();
    assert_eq!(memory.slot, "alias");
    assert_eq!(memory.about.as_deref(), Some("api"));
    assert_eq!(memory.text, "the payments agent");
    assert!(
        fake.requests.borrow().is_empty(),
        "code's own reply, no model call"
    );
    // A name that isn't a record's is a question, as before.
    let other = ask_with(&state, &Fake::new(), "call me when it's done");
    assert_ne!(other.outcome, BotOutcome::Remembered);
}

/// 0.5.2 G53 (RW1 W4): a grounded draft the check refuses is asked for
/// once more, sampled, with a note that it didn't check out; the second
/// is shown only if it passes, and a lie twice is still never shown.
#[test]
fn a_refused_draft_is_asked_once_more_and_shown_only_if_it_checks_out() {
    let (_dir, state) = seeded();
    let mended = Fake::with(Box::new(|request| {
        if !grounded(request) {
            return default_rule(request);
        }
        if request.prompt.contains("didn't check out") {
            assert!(
                matches!(
                    request.decoding,
                    crate::features::ai::engine::Decoding::Sampled { .. }
                ),
                "the second draft samples"
            );
            return default_rule(request);
        }
        Ok(r#"{"sentences":[{"text":"“web” crashed.","records":[1]}]}"#.to_owned())
    }));
    let turn = ask_with(&state, &mended, "Is web working?");
    assert_eq!(turn.outcome, BotOutcome::Answered);
    assert_eq!(mended.calls.get(), 2, "one more draft, no more");
    let lying = lying();
    let turn = ask_with(&state, &lying, "Is web working?");
    assert_eq!(turn.outcome, BotOutcome::DontKnow);
    assert_eq!(lying.calls.get(), 2);
    assert!(turn.written.is_none());
}
