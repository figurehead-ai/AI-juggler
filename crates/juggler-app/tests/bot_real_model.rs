//! AJ-E3829's entry (built by AJ-E3825): Bot's five fixed questions
//! through its real routing on this Mac's real Apple engine. Run by hand
//! (or by AJ-E3829's script) on the reference Mac with Apple Intelligence
//! on, after `scripts/build_apple_ai_helper.sh`:
//!
//! ```text
//! AIJ_APPLE_AI_HELPER=<built helper> cargo test --offline -p juggler-app \
//!   --test bot_real_model -- --ignored --nocapture
//! ```
//!
//! Each answer is printed as one JSON line (question, outcome, model,
//! words, cites, failure, details) for the check to assert and a walk to
//! read; nothing is asserted here but that every reply came from the
//! model. A temporary data folder only; nothing of the user's is read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use juggler_app::features::ai::engines::Registry;
use juggler_app::features::ai::engines::apple::Apple;
use juggler_app::features::ai::engines::local::Local;
use juggler_app::features::ai::service::Service;
use juggler_app::features::bot::model::ask_for_check;
use juggler_app::state::AppState;

const QUESTIONS: [&str; 5] = [
    "what is 3+5",
    "which model are you",
    "what are my sessions",
    "what needs me",
    "flurble wibbit quantum toaster?",
];

#[test]
#[ignore = "needs the built helper and Apple Intelligence on; AJ-E3829 runs it"]
fn bot_answers_through_the_real_model() {
    let helper = std::env::var_os("AIJ_APPLE_AI_HELPER")
        .map(std::path::PathBuf::from)
        .expect("AIJ_APPLE_AI_HELPER names the built helper");
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().join("app-data"),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(
            &juggler_app::features::ai::jobs::beta_key(juggler_app::features::ai::models::Job::Bot),
            "1",
        )
        .unwrap();
    let service = Service::new(Registry::with(
        Apple::with_program(Some(helper)),
        Local::default(),
    ));
    for question in QUESTIONS {
        let checked = ask_for_check(&state, &service, question).unwrap();
        let turn = &checked.turn;
        let words = turn
            .written
            .as_ref()
            .map(|w| {
                w.parts
                    .iter()
                    .map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .or_else(|| turn.general.as_ref().map(|g| g.text.clone()));
        let line = serde_json::json!({
            "question": question,
            "outcome": checked.outcome,
            "model": checked.model_name,
            "words": words,
            "cites": turn.written.as_ref().map(|w| w.parts.iter().map(|p| p.cites.clone()).collect::<Vec<_>>()),
            "failure": turn.failure,
            "details": turn.details,
        });
        println!("{line}");
        assert!(
            turn.details.is_some() || turn.failure.is_some(),
            "{question}: a reply not from the model"
        );
    }
    service.engines().release();
}
