use super::*;
use crate::features::ai::scripted::Scripted;
use crate::features::bot::dto::{BotAskInput, BotOutcome, BotRowInfo, BotState};
use crate::features::bot::world::tests_support::row;

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

/// AJ-E3830 (0.5.2): the request pre-fills the form with what code read;
/// nothing is added (the store is written only by the user's click).
#[test]
fn a_watch_request_pre_fills_the_form_and_adds_nothing() {
    let (_dir, state) = crate::features::bot::tests::state();
    let fake = Scripted::new(&["FAIL"]);
    let got = drafted_with(
        &fake,
        &state,
        &world(),
        "Let me know when api finishes.",
        false,
    )
    .expect("a pre-fill");
    let WatchDrafted::Prefilled { prefill } = &got else {
        unreachable!("{got:?}")
    };
    assert_eq!(
        prefill.draft.targets,
        vec![WatchTarget::Session("s-api".into())]
    );
    // A store without the session's App offers nothing for it: the
    // condition is left rather than filled with one it can't report.
    match prefill.draft.conditions.first() {
        Some(c) => assert_eq!(c.code, "finishes"),
        None => assert_eq!(prefill.left, vec![PrefillField::Condition]),
    }
    assert!(!prefill.card, "the card needs the user's Beta");
    assert_eq!(fake.calls(), 0, "code read every word");
    let storage = state.storage.lock().unwrap();
    assert_eq!(
        crate::features::bot::commands::kept_watches(&storage),
        0,
        "nothing added without the user's click"
    );
}

/// The card only with the Beta on, and only for a draft the form's own
/// check passes.
#[test]
fn the_card_needs_the_beta_and_the_form_s_check() {
    let (_dir, state) = crate::features::bot::tests::state();
    let fake = Scripted::new(&["FAIL"]);
    let on = drafted_with(
        &fake,
        &state,
        &world(),
        "Let me know when api finishes.",
        true,
    );
    let Some(WatchDrafted::Prefilled { prefill }) = on else {
        unreachable!("{on:?}")
    };
    let check = crate::features::watches::commands::watches_preview_impl(
        &state,
        &prefill.draft,
        crate::features::watches::words::Lang::En,
    )
    .ok()
    .and_then(|p| p.problem);
    assert_eq!(prefill.card, check.is_none(), "{prefill:?} {check:?}");
}

/// With no model served (the Watches verdict stays `not_yet`), a watch
/// request still opens the pre-filled form: code alone fills what it reads.
#[test]
fn with_no_model_a_watch_request_opens_the_pre_filled_form() {
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
    assert_eq!(turn.outcome, BotOutcome::WatchDraft);
    assert!(
        matches!(turn.watch_draft, Some(WatchDrafted::Prefilled { .. })),
        "{:?}",
        turn.watch_draft
    );
    let storage = state.storage.lock().unwrap();
    assert_eq!(crate::features::bot::commands::kept_watches(&storage), 0);
}

/// "When did api finish?" is a question, never a watch.
#[test]
fn a_question_about_the_past_is_no_watch() {
    for q in [
        "Tell me when did api finish?",
        "Let me know when was the web session started",
        "Sag mir, wann hat api geendet?",
    ] {
        assert!(asks_when_it_happened(q), "{q}");
    }
    for q in ["Tell me when api finishes", "Sag mir, wenn api fertig ist"] {
        assert!(!asks_when_it_happened(q), "{q}");
    }
}

fn prefill_with(left: Vec<PrefillField>, why: PrefillWhy, asked: Option<&str>) -> WatchPrefill {
    WatchPrefill {
        draft: crate::features::watches::commands::WatchDraftInfo {
            targets: Vec::new(),
            matching: crate::features::watches::model::Matching::Each,
            conditions: Vec::new(),
            lifetime: crate::features::watches::model::LifetimeChoice::Unsaid,
            even_in_quiet_hours: false,
            chat_turn: None,
        },
        left,
        why,
        asked: asked.map(str::to_owned),
        nearest: None,
        card: false,
    }
}

/// Bot's one line names what it left, in plain words.
#[test]
fn the_line_says_what_was_left() {
    let line = prefill_line(&prefill_with(
        vec![PrefillField::Lifetime],
        PrefillWhy::Unread,
        Some("bis ich es entferne"),
    ));
    assert_eq!(
        line,
        "I couldn't read “bis ich es entferne”. Left for you to pick: how long. The Watches form is open."
    );
    let line = prefill_line(&prefill_with(
        vec![
            PrefillField::Condition,
            PrefillField::Lifetime,
            PrefillField::QuietHours,
        ],
        PrefillWhy::Unread,
        Some("yo"),
    ));
    assert!(
        line.contains("what to watch for, how long and quiet hours"),
        "{line}"
    );
    let none = prefill_line(&prefill_with(
        vec![
            PrefillField::Target,
            PrefillField::Condition,
            PrefillField::Lifetime,
            PrefillField::QuietHours,
        ],
        PrefillWhy::Unread,
        Some("x"),
    ));
    assert_eq!(
        none,
        "I couldn't fill the form from your words. The Watches form is open."
    );
    let mut card = prefill_with(Vec::new(), PrefillWhy::Verified, None);
    card.card = true;
    assert_eq!(
        prefill_line(&card),
        "Here is the watch. Check it, then click Watch."
    );
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
        WatchDrafted::Prefilled {
            prefill: prefill_with(vec![PrefillField::Target], PrefillWhy::Several, None),
        },
    );
    match info.watch_draft {
        Some(WatchDrafted::Prefilled { prefill }) => assert_eq!(prefill.draft.chat_turn, Some(id)),
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
