use super::*;
use crate::features::ai::scripted::Scripted;
use crate::features::bot::dto::{
    BotAskInput, BotOutcome, BotRowInfo, BotScope, BotScopeKind, BotState,
};
use crate::features::bot::world::tests_support::row;

/// A session row of the App Work in `project` (its id `p-<project>`).
fn session(id: &str, title: &str, project: &str, state: BotState) -> BotRowInfo {
    BotRowInfo {
        id: id.into(),
        project: Some(project.into()),
        project_id: Some(format!("p-{project}")),
        app_label: Some("Work".into()),
        app_id: Some("work".into()),
        at_ms: Some(1_000),
        ..row(BotRowKind::Session, title, Some(state))
    }
}

/// A world of these sessions, nothing else.
fn world_of(sessions: Vec<BotRowInfo>) -> World {
    World {
        now_ms: 10_000,
        offset_hours: 0,
        sessions,
        tasks: Vec::new(),
        accounts: Vec::new(),
        removed: Vec::new(),
    }
}

fn world() -> World {
    world_of(vec![
        session("s-web", "web", "shop", BotState::Working),
        session("s-api", "api", "shop", BotState::Working),
    ])
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
        None,
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
        None,
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
        choices: Vec::new(),
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

/// 0.5.2 G60 (RW1 R31), Bot's own path: the request goes through the
/// candidates Bot's world makes (its rows, their projects and Apps), as
/// the window asks it. "web" names the project and the open session in it
/// (the session not titled yet, named by its folder or by its facts): the
/// form opens with the target left and the picker on both, never a
/// guess (the lifetime unsaid: the form's suggestion, labelled so, once a
/// target is picked). This test store holds no App, so nothing is on
/// offer and the condition is left too; the window's pre-fill (the
/// condition kept) is the prefill tests' and the window's vitest.
#[test]
fn an_ambiguous_name_opens_the_picker_on_its_matches() {
    let (_dir, state) = crate::features::bot::tests::state();
    for open in ["web", "web · started 3:23 AM"] {
        let world = world_of(vec![
            session("s-open", open, "web", BotState::Idle),
            session(
                "s-failed",
                "Rewrite the date parser to accept two-digit years",
                "web",
                BotState::Failed,
            ),
            session(
                "s-api",
                "Add retries to the sync client",
                "api",
                BotState::Idle,
            ),
        ]);
        let fake = Scripted::new(&["FAIL", "FAIL", "FAIL", "FAIL"]);
        let got = drafted_with(
            &fake,
            &state,
            &world,
            "tell me when web finishes",
            false,
            None,
        )
        .expect("a pre-fill");
        let WatchDrafted::Prefilled { prefill } = &got else {
            unreachable!("{got:?}")
        };
        assert!(prefill.draft.targets.is_empty(), "{open}: {prefill:?}");
        assert!(
            prefill.left.contains(&PrefillField::Target),
            "{open}: {prefill:?}"
        );
        assert!(
            prefill
                .choices
                .contains(&WatchTarget::Session("s-open".into())),
            "{open}: {prefill:?}"
        );
        assert!(
            prefill
                .choices
                .contains(&WatchTarget::Project("p-web".into())),
            "{open}: {prefill:?}"
        );
        assert_eq!(
            prefill.draft.lifetime,
            crate::features::watches::model::LifetimeChoice::Unsaid
        );
        // What the window receives: the choices ride the turn over IPC.
        let wire = serde_json::to_value(&got).unwrap();
        assert!(
            wire["prefill"]["choices"]
                .as_array()
                .is_some_and(|c| !c.is_empty()),
            "{wire}"
        );
    }
}

/// The RW1 lab's sessions: an open untitled one in web, a failed one in
/// web, and api's titled one.
fn rw1_world() -> World {
    world_of(vec![
        session("s-open", "web · started 3:23 AM", "web", BotState::Idle),
        session(
            "s-failed",
            "Rewrite the date parser to accept two-digit years",
            "web",
            BotState::Failed,
        ),
        session(
            "s-api",
            "Add retries to the sync client",
            "api",
            BotState::Idle,
        ),
    ])
}

fn chip(kind: BotScopeKind, id: &str, name: &str) -> BotScope {
    BotScope {
        kind,
        id: id.into(),
        name: name.into(),
        sessions: Vec::new(),
    }
}

/// 0.5.2 G65: a watch asked from a post reads only the user's own words;
/// the chip's session, by its key, is the target. The post's sentence
/// names another session and project ("Add retries to the sync client" in
/// api): none of it fills anything or is offered.
#[test]
fn a_post_scoped_watch_takes_its_target_from_the_chip_never_the_post() {
    let (_dir, state) = crate::features::bot::tests::state();
    let asked = "About this post: \u{201c}Claude Code CLI finished \u{201c}Add retries to the sync client\u{201d} in api, in under a minute.\u{201d} tell me when it finishes";
    let scope = chip(BotScopeKind::Session, "s-open", "web · started 3:23 AM");
    let got = drafted(&state, &rw1_world(), asked, Some(&scope)).expect("a pre-fill");
    let WatchDrafted::Prefilled { prefill } = &got else {
        unreachable!("{got:?}")
    };
    assert_eq!(
        prefill.draft.targets,
        vec![WatchTarget::Session("s-open".into())],
        "{prefill:?}"
    );
    assert!(!prefill.left.contains(&PrefillField::Target), "{prefill:?}");
    assert!(prefill.choices.is_empty(), "{prefill:?}");
    for word in ["retries", "api", "Claude"] {
        assert!(
            !prefill.asked.as_deref().unwrap_or("").contains(word),
            "the post's words were read: {prefill:?}"
        );
    }
    // A reminder from the post is about the chip's session too.
    assert!(
        state
            .schedules
            .install(crate::features::schedule::handlers())
    );
    let remind = drafted(
        &state,
        &rw1_world(),
        "About this post: \u{201c}Claude Code CLI finished in api.\u{201d} remind me in an hour",
        Some(&scope),
    );
    let Some(WatchDrafted::Remind { reminder }) = remind else {
        unreachable!("{remind:?}")
    };
    assert_eq!(reminder.about, Some(WatchTarget::Session("s-open".into())));
}

/// 0.5.2 G65: a post's sentence naming a project ("in web") never fills
/// the target nor is offered, when the chip names no one record (a
/// project's chip) and the user's words name nothing.
#[test]
fn a_post_s_sentence_never_fills_a_field() {
    let (_dir, state) = crate::features::bot::tests::state();
    let asked = "About this post: \u{201c}Claude Code CLI finished \u{201c}Rewrite the date parser to accept two-digit years\u{201d} in web.\u{201d} tell me when it finishes";
    for scope in [None, Some(chip(BotScopeKind::Project, "p-api", "api"))] {
        let got = drafted(&state, &rw1_world(), asked, scope.as_ref()).expect("a pre-fill");
        let WatchDrafted::Prefilled { prefill } = &got else {
            unreachable!("{got:?}")
        };
        assert!(prefill.draft.targets.is_empty(), "{prefill:?}");
        assert!(prefill.choices.is_empty(), "{prefill:?}");
        assert!(!prefill.card);
    }
}

/// 0.5.2 G65: the user's own words naming another target than the chip:
/// neither is guessed; the picker offers both, the chip's first.
#[test]
fn words_naming_another_target_than_the_chip_leave_the_target_with_both() {
    let mut p = WatchPrefill {
        draft: crate::features::watches::commands::WatchDraftInfo {
            targets: vec![WatchTarget::Session("s-api".into())],
            matching: crate::features::watches::model::Matching::Each,
            conditions: vec![crate::features::watches::model::Condition {
                code: "finishes".into(),
                param: None,
            }],
            lifetime: crate::features::watches::model::LifetimeChoice::Unsaid,
            even_in_quiet_hours: false,
            chat_turn: None,
        },
        left: Vec::new(),
        why: PrefillWhy::Verified,
        asked: None,
        nearest: None,
        card: true,
        choices: Vec::new(),
    };
    let open = WatchTarget::Session("s-open".into());
    pinned(&mut p, &open, &["finishes".to_owned()], &|_| None);
    assert!(p.draft.targets.is_empty());
    assert_eq!(p.choices, vec![open, WatchTarget::Session("s-api".into())]);
    assert_eq!(p.left, vec![PrefillField::Target]);
    assert!(!p.card);
}
