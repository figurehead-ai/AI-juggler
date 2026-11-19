#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test: a failed step fails the test"
)]

use std::sync::Arc;

use super::commands::{
    bot_ask_impl, bot_clear_impl, bot_count_impl, bot_find_impl, bot_new_topic_impl, bot_page_impl,
};
use super::dto::{
    BotAskInput, BotOutcome, BotRowInfo, BotRowKind, BotScope, BotScopeKind, BotState, BotUnknown,
};
use super::lookup::look_up;
use super::question::read;
use super::world::World;
use super::world::tests_support::row;
use crate::state::AppState;

/// 2026-11-19 12:00 UTC.
const NOW: i64 = 1_795_521_600_000;
const MIN: i64 = 60_000;

fn session(id: &str, title: &str, state: BotState, project: &str, at: i64) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        project: Some(project.into()),
        project_id: Some(format!("p-{project}")),
        app_label: Some("Work".into()),
        at_ms: Some(at),
        ..row(BotRowKind::Session, title, Some(state))
    }
}

fn task(id: &str, title: &str, state: BotState, pool: &str, at: i64) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        pool: Some(pool.into()),
        pool_id: Some(format!("pool-{pool}")),
        project: Some("shop".into()),
        project_id: Some("p-shop".into()),
        at_ms: Some(at),
        ..row(BotRowKind::Task, title, Some(state))
    }
}

/// The mock's fixture: generic names only.
fn world() -> World {
    World {
        now_ms: NOW,
        offset_hours: 0,
        sessions: vec![
            session("s-web", "web", BotState::AsksYou, "shop", NOW - 12 * MIN),
            session(
                "s-login",
                "Fix the login redirect",
                BotState::Stuck,
                "shop",
                NOW - 18 * MIN,
            ),
            session("s-api", "api", BotState::Working, "shop", NOW - 34 * MIN),
            session(
                "s-docs",
                "Draft the settings page",
                BotState::Finished,
                "docs-site",
                NOW - 60 * MIN,
            ),
            session(
                "s-old",
                "Old work",
                BotState::Finished,
                "shop",
                NOW - 30 * 60 * MIN,
            ),
        ],
        tasks: vec![
            task(
                "t-export",
                "Wire the export button",
                BotState::Blocked,
                "Release pool",
                NOW - 31 * MIN,
            ),
            task(
                "t-done",
                "Tidy the changelog",
                BotState::Done,
                "Release pool",
                NOW - 90 * MIN,
            ),
        ],
        accounts: vec![
            BotRowInfo {
                used_percent: Some(54),
                ..row(BotRowKind::Account, "Work", None)
            },
            BotRowInfo {
                used_percent: Some(36),
                ..row(BotRowKind::Account, "Personal", None)
            },
        ],
        removed: Vec::new(),
    }
}

fn titles(rows: &[BotRowInfo]) -> Vec<&str> {
    rows.iter().map(|r| r.title.as_str()).collect()
}

/// The lead's rule rests on this: each row keeps its own state, never one
/// word for several ("What is blocked?": asks you, looks stuck, blocked).
#[test]
fn each_row_keeps_its_own_state_in_attention_order() {
    let found = look_up(&read("What is blocked?"), &world(), None);
    assert_eq!(
        titles(&found.rows),
        ["web", "Fix the login redirect", "Wire the export button"]
    );
    assert_eq!(
        found.rows.iter().map(|r| r.state).collect::<Vec<_>>(),
        [
            Some(BotState::AsksYou),
            Some(BotState::Stuck),
            Some(BotState::Blocked)
        ]
    );
    assert_eq!(found.looked.reach, "everything");
}

#[test]
fn time_and_kind_words_narrow_the_rows() {
    let found = look_up(&read("What finished today?"), &world(), None);
    assert_eq!(
        titles(&found.rows),
        ["Draft the settings page", "Tidy the changelog"]
    );
    let found = look_up(&read("Welche Sitzungen sind heute fertig?"), &world(), None);
    assert_eq!(titles(&found.rows), ["Draft the settings page"]);
    let found = look_up(&read("Which account has the most room?"), &world(), None);
    assert_eq!(titles(&found.rows), ["Personal", "Work"]);
    assert_eq!(found.looked.reach, "accounts");
}

/// D-136 point 3: with the chip, only the scope is looked at; without
/// it (× pressed), everything.
#[test]
fn the_scope_chip_limits_the_look_up_and_widening_drops_it() {
    let api = BotScope {
        kind: BotScopeKind::Session,
        id: "s-api".into(),
        name: "api".into(),
        sessions: Vec::new(),
    };
    let found = look_up(&read("Anything stuck?"), &world(), Some(&api));
    assert!(
        found.rows.is_empty(),
        "api isn't stuck; nothing else is looked at"
    );
    assert_eq!(found.looked.reach, "scope");
    assert_eq!(
        found.looked.scope.as_ref().map(|s| s.name.as_str()),
        Some("api")
    );
    let found = look_up(&read("Anything stuck?"), &world(), None);
    assert_eq!(titles(&found.rows), ["Fix the login redirect"]);
    assert_eq!(found.looked.reach, "everything");
    // An App's chip carries its sessions by id.
    let app = BotScope {
        kind: BotScopeKind::App,
        id: "app:one".into(),
        name: "Agent app one".into(),
        sessions: vec!["s-web".into(), "s-login".into()],
    };
    let found = look_up(&read("What needs me?"), &world(), Some(&app));
    assert_eq!(titles(&found.rows), ["web", "Fix the login redirect"]);
    let project = BotScope {
        kind: BotScopeKind::Project,
        id: "p-docs-site".into(),
        name: "docs-site".into(),
        sessions: Vec::new(),
    };
    let found = look_up(&read("What finished?"), &world(), Some(&project));
    assert_eq!(titles(&found.rows), ["Draft the settings page"]);
}

/// "I don't know" never answers a nearby question.
#[test]
fn what_is_not_recorded_or_not_found_is_i_dont_know() {
    let found = look_up(&read("How long did the tests take in api?"), &world(), None);
    assert_eq!(found.unknown, Some(BotUnknown::TestDurations));
    assert_eq!(
        titles(&found.rows),
        ["api"],
        "the nearest record, labelled as such"
    );
    assert_eq!(found.looked.reach, "subject");
    let found = look_up(&read("Did the deploy to staging work?"), &world(), None);
    assert_eq!(found.unknown, Some(BotUnknown::NothingFound));
    assert!(found.rows.is_empty());
    assert!(found.shortcut.is_some());
}

#[test]
fn a_named_record_answers_with_its_own_state() {
    let found = look_up(&read("Is Fix the login redirect finished?"), &world(), None);
    assert_eq!(titles(&found.rows), ["Fix the login redirect"]);
    assert_eq!(
        found.rows[0].state,
        Some(BotState::Stuck),
        "its real state, not the one asked about"
    );
    let found = look_up(&read("What changed in shop yesterday?"), &world(), None);
    assert_eq!(titles(&found.rows), ["Old work"]);
    assert_eq!(
        found
            .looked
            .scope
            .as_ref()
            .map(|s| (s.kind, s.name.as_str())),
        Some((BotScopeKind::Project, "shop"))
    );
}

fn state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(
            dir.path().to_path_buf(),
            juggler_adapter::AdapterRegistry::new(),
        )
        .unwrap(),
    );
    (dir, state)
}

fn ask(state: &Arc<AppState>, question: &str) -> super::dto::BotTurnInfo {
    bot_ask_impl(
        state,
        &BotAskInput {
            question: question.into(),
            scope: None,
        },
    )
    .unwrap()
}

/// AJ-E3807: with no model, look-ups still answer; AJ-E3808: the turns
/// stay on this Mac, are paged, found and cleared whole.
#[test]
fn asking_keeps_turns_that_page_find_and_clear() {
    let (_dir, state) = state();
    let hi = ask(&state, "hello");
    assert_eq!(hi.outcome, BotOutcome::Help);
    let none = ask(&state, "Did the deploy to staging work?");
    assert_eq!(none.outcome, BotOutcome::DontKnow);
    assert!(none.written.is_none(), "no model: never a written answer");
    let code = ask(&state, "Write me a function that sorts dates");
    assert_eq!(code.outcome, BotOutcome::OutOfScope);
    bot_new_topic_impl(&state).unwrap();
    ask(&state, "What needs me?");
    let page = bot_page_impl(&state, None, 2).unwrap();
    assert_eq!(page.turns.len(), 2);
    assert!(page.more);
    assert_eq!(
        page.topics.iter().filter(|t| !t.first).count(),
        1,
        "the New topic divider"
    );
    let oldest = page.turns.last().unwrap();
    let rest = bot_page_impl(&state, Some((oldest.at_ms, oldest.id.as_str())), 20).unwrap();
    assert_eq!(rest.turns.len(), 2);
    assert_eq!(bot_find_impl(&state, "staging").unwrap().len(), 1);
    let count = bot_count_impl(&state).unwrap();
    assert_eq!(count.questions, 4);
    assert_eq!(bot_clear_impl(&state).unwrap(), 4);
    assert!(bot_page_impl(&state, None, 20).unwrap().turns.is_empty());
}

/// AJ-E3815 Req 1 and 2: asking never writes a watch (or anything but the
/// turn); a watch exists only after the confirm card's click.
#[test]
fn asking_never_creates_a_watch() {
    let (_dir, state) = state();
    ask(&state, "Watch web and tell me when it finishes");
    ask(&state, "Tell me when api gets stuck");
    let storage = state.storage.lock().unwrap();
    let watches = storage
        .memory()
        .current_of_kind(
            juggler_core::storage::memory::Kind::StandingRequest,
            None,
            100,
        )
        .unwrap();
    assert!(watches.is_empty());
}

/// A secret typed into a question is masked before it is kept.
#[test]
fn a_secret_in_a_question_is_kept_masked() {
    let (_dir, state) = state();
    let key = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let turn = ask(&state, &format!("is {key} still used by api?"));
    assert!(!turn.question.contains(key));
    let page = bot_page_impl(&state, None, 5).unwrap();
    assert!(!page.turns[0].question.contains(key));
}
