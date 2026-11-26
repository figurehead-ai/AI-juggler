#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]
//! AJ-E2027 R3: a reminder set through the command is stored, comes due,
//! and is told once through the bell's model; the user's list, its
//! counts, Pause all and Remove; a time out of bounds is refused.

use std::collections::HashSet;

use super::*;
use crate::features::schedule::registry::{Outcome, RunCtx};
use crate::features::schedule::service::take_due;

fn state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::seven_sessions(dir.path());
    assert!(
        state
            .schedules
            .install(crate::features::schedule::handlers())
    );
    (dir, state)
}

fn remind(state: &Arc<AppState>, at_ms: i64, about: Option<WatchTarget>) -> ScheduleInfo {
    schedule_remind_impl(
        state,
        &RemindRequest {
            at_ms,
            title: "Look at “api”".into(),
            about,
            chat_turn: None,
        },
    )
    .unwrap()
}

#[test]
fn a_reminder_is_set_listed_and_told_once_when_due() {
    let (_dir, state) = state();
    let now = crate::commands::now_millis();
    let made = remind(
        &state,
        now + 3_600_000,
        Some(WatchTarget::Session("s-1".into())),
    );
    assert_eq!(made.next_due_ms, Some(now + 3_600_000));
    assert_eq!(
        (made.created_by.as_str(), made.handler.as_str()),
        ("user", "reminder")
    );
    let page =
        schedule_list_impl(&state, &PageRequest::first(10), ScheduleListWhich::Yours).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].about,
        Some(WatchTarget::Session("s-1".into()))
    );
    assert!(
        schedule_list_impl(&state, &PageRequest::first(10), ScheduleListWhich::App)
            .unwrap()
            .items
            .is_empty()
    );
    // Due: the handler tells it through the one model, once.
    let registry = state.schedules.registry().cloned().unwrap();
    let due = {
        let storage = state.storage.lock().unwrap();
        let ctx = context(&storage);
        take_due(
            &storage,
            &registry,
            &ctx,
            now + 3_600_000,
            Some(true),
            &HashSet::<String>::new(),
            2,
        )
    };
    assert_eq!(due.len(), 1);
    let busy = || false;
    let ctx = RunCtx {
        state: &state,
        id: &made.id,
        title: &made.title,
        due_ms: now + 3_600_000,
        now_ms: now + 3_600_000,
        late: true,
        missed: 0,
        busy: &busy,
    };
    assert_eq!(
        due[0].handler.run(&due[0].schedule.payload, &ctx),
        Outcome::Done
    );
    let told_again = crate::features::notifications::watch_alerts::reminded(
        &state,
        &made.id,
        now + 3_600_000,
        &made.title,
        None,
        true,
    );
    assert!(!told_again, "told once per reminder and due time");
    let storage = state.storage.lock().unwrap();
    let kinds = storage.notices().earlier_kinds_since(0).unwrap();
    assert!(
        kinds.iter().any(|(kind, n)| kind == "watch" && *n == 1),
        "{kinds:?}"
    );
}

#[test]
fn the_list_pauses_all_and_removes_by_id() {
    let (_dir, state) = state();
    let now = crate::commands::now_millis();
    let a = remind(&state, now + 60_000, None);
    let b = remind(&state, now + 120_000, None);
    assert_eq!(
        schedule_summary_impl(&state).unwrap(),
        SchedulesSummary {
            active: 2,
            paused: 0
        }
    );
    assert_eq!(
        schedule_pause_all_impl(&state, true).unwrap(),
        SchedulesSummary {
            active: 0,
            paused: 2
        }
    );
    assert!(!schedule_pause_impl(&state, &a.id, false).unwrap().paused);
    let moved = schedule_update_impl(&state, &b.id, now + 7_200_000, "Check the deploy").unwrap();
    assert_eq!(
        (moved.next_due_ms, moved.title.as_str()),
        (Some(now + 7_200_000), "Check the deploy")
    );
    schedule_remove_impl(&state, &a.id).unwrap();
    assert_eq!(
        schedule_summary_impl(&state).unwrap(),
        SchedulesSummary {
            active: 0,
            paused: 1
        }
    );
    assert!(matches!(
        schedule_remove_impl(&state, &a.id),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn a_bad_reminder_is_refused_in_plain_words() {
    let (_dir, state) = state();
    let refused = |request: RemindRequest| match schedule_remind_impl(&state, &request) {
        Err(AppError::Validation { message, .. }) => message,
        other => format!("not refused: {other:?}"),
    };
    let ok = RemindRequest {
        at_ms: crate::commands::now_millis() + 60_000,
        title: "Look".into(),
        about: None,
        chat_turn: None,
    };
    assert_eq!(
        refused(RemindRequest {
            title: " ".into(),
            ..ok.clone()
        }),
        "A reminder needs a few words."
    );
    assert_eq!(
        refused(RemindRequest {
            at_ms: -5,
            ..ok.clone()
        }),
        "That time can't be used."
    );
    assert!(
        refused(RemindRequest {
            about: Some(WatchTarget::Any),
            ..ok
        })
        .contains("about")
    );
}
