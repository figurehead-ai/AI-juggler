use std::cell::RefCell;

use super::*;
use crate::features::ai::engine::{Draft, Finish, Request, Usage};
use crate::features::ai::run::{AiFailure, JobModel};
use crate::features::bot::dto::{BotAskInput, BotOutcome, BotRowInfo, BotState};
use crate::features::bot::world::tests_support::row;

struct Fake(Vec<&'static str>, RefCell<usize>);
impl JobModel for Fake {
    fn generate(&self, _request: &Request, _stop: &Stop) -> Result<Draft, AiFailure> {
        let n = *self.1.borrow();
        *self.1.borrow_mut() += 1;
        Ok(Draft {
            text: self.0[n.min(self.0.len() - 1)].to_owned(),
            finish: Finish::Stop,
            usage: Usage::default(),
        })
    }
}

fn world() -> World {
    let session = |id: &str, title: &str, project: &str| BotRowInfo {
        id: id.into(),
        project: Some(project.into()),
        project_id: Some(format!("p-{project}")),
        app_label: Some("Work".into()),
        app_id: Some("work".into()),
        at_ms: Some(1_000),
        ..row(BotRowKind::Session, title, Some(BotState::Working))
    };
    World {
        now_ms: 10_000,
        offset_hours: 0,
        sessions: vec![
            session("s-web", "web", "shop"),
            session("s-api", "api", "shop"),
        ],
        tasks: Vec::new(),
        accounts: Vec::new(),
        removed: Vec::new(),
    }
}

/// AJ-E3830: the request fills the form; nothing is added (the store is
/// written only by the user's click in the form).
#[test]
fn a_watch_request_fills_the_form_and_adds_nothing() {
    let (_dir, state) = crate::features::bot::tests::state();
    let fake = Fake(
        vec![
            r#"{"intent":"create","target":"s2"}"#,
            r#"{"condition":"finishes","param":"none","condition2":"none","param2":"none","lifetime":"unsaid","until":"none","unsupported":"none"}"#,
        ],
        RefCell::new(0),
    );
    let got =
        drafted_with(&fake, &state, &world(), "Let me know when api finishes.").expect("a draft");
    match &got {
        WatchDrafted::Filled { draft } => {
            assert_eq!(draft.targets, vec![WatchTarget::Session("s-api".into())])
        }
        WatchDrafted::Form => {} // a store without the target's App: the form's own check refuses, the empty form opens
        other => unreachable!("{other:?}"),
    }
    let storage = state.storage.lock().unwrap();
    assert_eq!(
        crate::features::bot::commands::kept_watches(&storage),
        0,
        "nothing added without the user's click"
    );
}

/// Until a round records a passing verdict, the Watches job isn't served
/// and Bot answers a watch request as it did.
#[test]
fn without_the_jobs_verdict_bot_answers_as_before() {
    let (_dir, state) = crate::features::bot::tests::state();
    let turn = crate::features::bot::commands::bot_ask_with(
        &state,
        &BotAskInput {
            question: "Let me know when api finishes.".into(),
            scope: None,
            again: None,
        },
        &crate::features::bot::fake::Fake::new(),
    )
    .unwrap();
    assert_ne!(turn.outcome, BotOutcome::WatchDraft);
    assert!(turn.watch_draft.is_none());
}

#[test]
fn the_draft_carries_its_turn_and_one_plain_line() {
    let (_dir, state) = crate::features::bot::tests::state();
    let mut info = crate::features::bot::commands::bot_ask_with(
        &state,
        &BotAskInput {
            question: "What needs me?".into(),
            scope: None,
            again: None,
        },
        &crate::features::bot::fake::Fake::new(),
    )
    .unwrap();
    let id = info.id.clone();
    attach(
        &mut info,
        WatchDrafted::Pick {
            draft: crate::features::watches::commands::WatchDraftInfo {
                targets: Vec::new(),
                matching: crate::features::watches::model::Matching::Each,
                conditions: Vec::new(),
                lifetime: crate::features::watches::model::LifetimeChoice::Unsaid,
                even_in_quiet_hours: false,
                chat_turn: None,
            },
        },
    );
    match info.watch_draft {
        Some(WatchDrafted::Pick { draft }) => assert_eq!(draft.chat_turn, Some(id)),
        other => unreachable!("{other:?}"),
    }
    assert!(
        shown(&WatchDrafted::CantWatch {
            asked: "tests pass".into(),
            draft: None
        })
        .contains("“tests pass”")
    );
}
