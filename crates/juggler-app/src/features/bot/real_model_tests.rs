//! AJ-E3829: Bot's five fixed questions through its real routing on this
//! Mac's real Apple engine, with fixture sessions (two in a temporary
//! data folder: "web" running, "api" waiting on the user), asserting real
//! answers. Run on the reference Mac with Apple Intelligence on, after
//! `scripts/build_apple_ai_helper.sh`:
//!
//! ```text
//! AIJ_APPLE_AI_HELPER=<built helper> cargo test --offline -p juggler-app --lib \
//!   features::bot::real_model_tests -- --ignored --nocapture
//! ```
//!
//! `scripts/qa/bot_real_model.sh` runs it and keeps the answers for the
//! walk to read (AJ-E3829 R2). Nothing of the user's is read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use crate::features::ai::engines::Registry;
use crate::features::ai::engines::apple::Apple;
use crate::features::ai::engines::local::Local;
use crate::features::ai::service::Service;
use crate::features::bot::dto::BotOutcome;
use crate::features::bot::model::ask_for_check;

fn words_of(turn: &crate::features::bot::dto::BotTurnInfo) -> String {
    turn.written
        .as_ref()
        .map(|w| {
            w.parts
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .or_else(|| turn.general.as_ref().map(|g| g.text.clone()))
        .unwrap_or_default()
}

/// Two fixture sessions ("web" running, "api" waiting on the user) in a
/// temporary data folder, Bot's Beta on, and a service on the built helper.
fn fixture() -> (tempfile::TempDir, Arc<crate::state::AppState>, Service) {
    let helper = std::env::var_os("AIJ_APPLE_AI_HELPER")
        .map(std::path::PathBuf::from)
        .expect("AIJ_APPLE_AI_HELPER names the built helper");
    let dir = tempfile::tempdir().unwrap();
    let state: Arc<crate::state::AppState> =
        crate::test_support::bare_state(dir.path().join("data"));
    super::tests::seed_session(&state, dir.path(), "s-web", "web", "main");
    {
        let cwd = dir.path().join("api");
        std::fs::create_dir_all(&cwd).unwrap();
        let storage = state.storage.lock().unwrap();
        storage
            .sessions()
            .insert(&crate::test_support::instance(
                "s-api",
                "rec-1",
                juggler_core::model::SessionStatus::WaitingUser,
                cwd,
            ))
            .unwrap();
        storage
            .app_settings()
            .set(
                &crate::features::ai::jobs::beta_key(crate::features::ai::models::Job::Bot),
                "1",
            )
            .unwrap();
    }
    let service = Service::new(Registry::with(
        Apple::with_program(Some(helper)),
        Local::default(),
    ));
    (dir, state, service)
}

/// One answer: its outcome, words (lower case) and cites, printed for the
/// walk to read.
struct Asked {
    outcome: BotOutcome,
    words: String,
    cites: Vec<u32>,
}

/// Asks `question` in a topic of its own (no answer leans on the last).
fn ask(state: &Arc<crate::state::AppState>, service: &Service, question: &str) -> Asked {
    crate::features::bot::commands::bot_new_topic_impl(state).unwrap();
    let checked = ask_for_check(state, service, question).unwrap();
    let turn = &checked.turn;
    let words = words_of(turn);
    let cites: Vec<u32> = turn
        .written
        .as_ref()
        .map(|w| w.parts.iter().flat_map(|p| p.cites.clone()).collect())
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::json!({
            "question": question, "outcome": checked.outcome, "model": checked.model_name,
            "words": words, "cites": cites, "failure": turn.failure,
            "ms": turn.details.as_ref().map(|d| d.ms),
        })
    );
    Asked {
        outcome: turn.outcome,
        words: words.to_lowercase(),
        cites,
    }
}

/// Asks each question and checks it; fails listing every answer that
/// wasn't a real one.
fn run(
    state: &Arc<crate::state::AppState>,
    service: &Service,
    questions: &[&str],
    ok: impl Fn(&str, &Asked) -> bool,
) {
    let mut fails = Vec::new();
    for question in questions {
        let asked = ask(state, service, question);
        if !ok(question, &asked) {
            fails.push(format!(
                "{question}: {:?} {:?} cites {:?}",
                asked.outcome, asked.words, asked.cites
            ));
        }
    }
    service.engines().release();
    assert!(fails.is_empty(), "not real answers:\n{}", fails.join("\n"));
}

#[test]
#[ignore = "needs the built helper and Apple Intelligence on; AJ-E3829 runs it"]
fn bot_answers_real_questions_through_the_real_model() {
    let (_dir, state, service) = fixture();
    let questions = [
        "what is 3+5",
        "which model are you",
        "what are my sessions",
        "what needs me",
        "flurble wibbit quantum toaster?",
    ];
    run(&state, &service, &questions, |question, a| {
        let grounded = a.outcome == BotOutcome::Answered && !a.cites.is_empty();
        match question {
            "what is 3+5" => a.outcome == BotOutcome::General && a.words.contains('8'),
            "which model are you" => a.outcome == BotOutcome::General && a.words.contains("apple"),
            "what are my sessions" => {
                grounded && a.words.contains("web") && a.words.contains("api")
            }
            "what needs me" => grounded && a.words.contains("api"),
            _ => {
                matches!(a.outcome, BotOutcome::General | BotOutcome::Answered)
                    && !a.words.trim().is_empty()
            }
        }
    });
}

/// AJ-E3832 R6 (ADR-0038 §5): the world model on the real engine. A name
/// the user taught Bot ("the payments agent") is found through the graph's
/// aliases, a session's App through its links, and what was remembered is
/// recalled as a record the answer cites (G9), all through Bot's real
/// routing on Apple's model, with the answers printed for the walk to read.
#[test]
#[ignore = "needs the built helper and Apple Intelligence on; AJ-E3832 R6 runs it"]
fn bot_reasons_over_the_world_model_on_the_real_model() {
    let (_dir, state, service) = fixture();
    let remembered =
        ask_for_check(&state, &service, "Remember: call api the payments agent").unwrap();
    println!(
        "{}",
        serde_json::json!({"question": "remember", "outcome": remembered.outcome})
    );
    assert_eq!(remembered.turn.outcome, BotOutcome::Remembered);
    let questions = [
        "what is the payments agent doing?",
        "which app is web running in?",
        "what did I ask you to remember about api?",
    ];
    run(&state, &service, &questions, |question, a| {
        let grounded = a.outcome == BotOutcome::Answered && !a.cites.is_empty();
        match question {
            "what is the payments agent doing?" => {
                grounded && (a.words.contains("api") || a.words.contains("payments"))
            }
            "which app is web running in?" => grounded,
            _ => grounded && a.words.contains("payments"),
        }
    });
}
