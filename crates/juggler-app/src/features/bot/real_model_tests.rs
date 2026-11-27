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
//!
//! 0.5.2 G15 and G16: the RW1 lab's records (nothing open, one failure)
//! too, each turn printed with every draft and the exact verdict code
//! gives it (the rule and its words), so a refusal is read, not guessed.

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

/// A service on the built helper (`AIJ_APPLE_AI_HELPER`).
fn service() -> Service {
    let helper = std::env::var_os("AIJ_APPLE_AI_HELPER")
        .map(std::path::PathBuf::from)
        .expect("AIJ_APPLE_AI_HELPER names the built helper");
    Service::new(Registry::with(
        Apple::with_program(Some(helper)),
        Local::default(),
    ))
}

/// Bot's Beta on.
fn beta_on(storage: &juggler_core::storage::Storage) {
    storage
        .app_settings()
        .set(
            &crate::features::ai::jobs::beta_key(crate::features::ai::models::Job::Bot),
            "1",
        )
        .unwrap();
}

/// Two fixture sessions ("web" running, "api" waiting on the user) in a
/// temporary data folder, Bot's Beta on, and a service on the built helper.
fn fixture() -> (tempfile::TempDir, Arc<crate::state::AppState>, Service) {
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
        beta_on(&storage);
    }
    (dir, state, service())
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
    ask_recorded(state, service, question, false)
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

/// A Bot model that hands each generation to the real one and keeps what
/// was sent and what came back, so a refused draft can be read with the
/// exact rule that refused it (0.5.2 G15, G16).
struct Recording<'a> {
    live: crate::features::bot::model::Live<'a>,
    seen: std::cell::RefCell<Vec<(crate::features::ai::engine::Request, String)>>,
}

impl crate::features::bot::model::BotModel for Recording<'_> {
    fn status(&self) -> crate::features::ai::run::AiEngineStatusInfo {
        self.live.status()
    }

    fn generate(
        &self,
        request: &crate::features::ai::engine::Request,
        stop: &crate::features::ai::engine::Stop,
    ) -> Result<crate::features::ai::run::Generated, crate::features::ai::run::AiFailure> {
        let generated = self.live.generate(request, stop)?;
        self.seen
            .borrow_mut()
            .push((request.clone(), generated.draft.text.clone()));
        Ok(generated)
    }
}

/// 0.5.2 RW1's lab as Bot reads it (G15): four Claude Code sessions that
/// ended, one of them stopped with an error (exit 1: Failed), and two
/// Codex CLI conversations, named as the lab's stand-in and Codex fixture
/// name their asks; nothing open, nothing waiting on the user.
fn lab_fixture() -> (tempfile::TempDir, Arc<crate::state::AppState>, Service) {
    use juggler_core::model::SessionStatus;
    let dir = tempfile::tempdir().unwrap();
    let state: Arc<crate::state::AppState> =
        crate::test_support::bare_state(dir.path().join("data"));
    {
        let storage = state.storage.lock().unwrap();
        for (account, harness, label, profile) in [
            ("acc-work", "claude-code", "Work", "rec-work"),
            ("acc-chatgpt", "codex", "ChatGPT", "rec-chatgpt"),
        ] {
            storage
                .accounts()
                .insert(&crate::test_support::account(
                    account,
                    harness,
                    label,
                    dir.path().join(account),
                ))
                .unwrap();
            storage
                .profiles()
                .insert(&crate::test_support::launch_settings(
                    profile, label, account,
                ))
                .unwrap();
        }
        for (id, profile, project, title, status) in [
            (
                "s-edits",
                "rec-work",
                "web",
                "Split the cart total into its own module and test it",
                SessionStatus::Finished,
            ),
            (
                "s-revert",
                "rec-work",
                "api",
                "Change the date format in the API responses to ISO 8601",
                SessionStatus::Finished,
            ),
            (
                "s-question",
                "rec-work",
                "api",
                "Add retries to the sync client",
                SessionStatus::Finished,
            ),
            (
                "s-fail",
                "rec-work",
                "web",
                "Rewrite the date parser to accept two-digit years",
                SessionStatus::Failed,
            ),
            (
                "cx-patch",
                "rec-chatgpt",
                "web",
                "Add a retry helper to the API client",
                SessionStatus::Stopped,
            ),
            (
                "cx-delete",
                "rec-chatgpt",
                "api",
                "Remove the old handlers module",
                SessionStatus::Stopped,
            ),
        ] {
            let cwd = dir.path().join(project);
            std::fs::create_dir_all(&cwd).unwrap();
            storage
                .sessions()
                .insert(&crate::test_support::instance(id, profile, status, cwd))
                .unwrap();
            storage
                .sessions()
                .set_label(&juggler_core::model::SessionId::new(id), Some(title))
                .unwrap();
        }
        beta_on(&storage);
    }
    (dir, state, service())
}

/// Each generation of one turn, with the verdict code would give it: the
/// general route's guard, or the grounded check's rule and words.
fn verdicts(
    state: &Arc<crate::state::AppState>,
    question: &str,
    turn: &crate::features::bot::dto::BotTurnInfo,
    seen: &[(crate::features::ai::engine::Request, String)],
) -> Vec<serde_json::Value> {
    use crate::features::ai::engine::{Draft, Finish, Usage};
    use crate::features::bot::{check, general, written};
    let world = crate::features::bot::world::gather(state).unwrap();
    seen.iter()
        .map(|(request, text)| {
            let draft = Draft {
                text: text.clone(),
                finish: Finish::Stop,
                usage: Usage::default(),
            };
            let route = request
                .shape
                .as_ref()
                .and_then(|s| s.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let verdict = if route == "Reply" {
                general::parse(&draft).map_or_else(
                    || "shape".to_owned(),
                    |(text, about)| {
                        format!(
                            "about_their_work {about}, guard {:?}",
                            general::guard(&text, &general::record_names(&world, question))
                        )
                    },
                )
            } else {
                written::parse(&draft).map_or_else(
                    || "shape".to_owned(),
                    |parts| {
                        format!(
                            "{:?}",
                            check::check_with(
                                &parts,
                                &turn.rows,
                                question,
                                &world.removed,
                                &crate::features::bot::answer::absent(&world),
                            )
                        )
                    },
                )
            };
            serde_json::json!({
                "route": route, "prompt": request.prompt, "draft": text, "verdict": verdict,
            })
        })
        .collect()
}

/// One question through Bot's real routing, recorded: a fresh topic first
/// unless `same_topic`.
fn ask_recorded(
    state: &Arc<crate::state::AppState>,
    service: &Service,
    question: &str,
    same_topic: bool,
) -> Asked {
    if !same_topic {
        crate::features::bot::commands::bot_new_topic_impl(state).unwrap();
    }
    let model = Recording {
        live: crate::features::bot::model::Live { service, state },
        seen: std::cell::RefCell::default(),
    };
    let turn = crate::features::bot::commands::bot_ask_with(
        state,
        &crate::features::bot::dto::BotAskInput {
            question: question.to_owned(),
            scope: None,
            again: None,
        },
        &model,
    )
    .unwrap();
    let words = words_of(&turn);
    let cites: Vec<u32> = turn
        .written
        .as_ref()
        .map(|w| w.parts.iter().flat_map(|p| p.cites.clone()).collect())
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::json!({
            "question": question, "same_topic": same_topic, "outcome": turn.outcome.code(),
            "model": turn.details.as_ref().map(|d| d.model_name.clone()),
            "words": words, "cites": cites, "failure": turn.failure,
            "ms": turn.details.as_ref().map(|d| d.ms),
            "rows": turn.rows.iter().map(|r| (r.title.clone(), r.state.map(|s| s.code()))).collect::<Vec<_>>(),
            "generations": verdicts(state, question, &turn, &model.seen.borrow()),
        })
    );
    Asked {
        outcome: turn.outcome,
        words: words.to_lowercase(),
        cites,
    }
}

/// 0.5.2 G15 and G16, found validating the RW1 lab on the real Dev: with
/// every session ended (one with an error) and two Codex conversations,
/// "what are my sessions" and "what needs me" were refused as not
/// checking out, and "3+5" asked next in the same topic was refused too
/// (in a fresh topic it answered 8). Each question three times (the model
/// samples): every grounded answer cites records, none is refused, 3+5 is
/// Bot's own 8 in the same topic.
#[test]
#[ignore = "needs the built helper and Apple Intelligence on; 0.5.2 G15 and G16"]
fn bot_answers_the_rw1_labs_questions_on_the_real_model() {
    let (_dir, state, service) = lab_fixture();
    let mut fails = Vec::new();
    for _ in 0..3 {
        for question in ["what are my sessions", "what needs me"] {
            let a = ask_recorded(&state, &service, question, false);
            if a.outcome != BotOutcome::Answered || a.cites.is_empty() {
                fails.push(format!("{question}: {:?} {:?}", a.outcome, a.words));
            }
            let sum = ask_recorded(&state, &service, "3+5", true);
            if sum.outcome != BotOutcome::General || !sum.words.contains('8') {
                fails.push(format!(
                    "3+5 after {question}: {:?} {:?}",
                    sum.outcome, sum.words
                ));
            }
        }
    }
    service.engines().release();
    assert!(fails.is_empty(), "not real answers:\n{}", fails.join("\n"));
}
