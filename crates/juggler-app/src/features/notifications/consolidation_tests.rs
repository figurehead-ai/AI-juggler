//! ADR-0036 §1.7 through the real path: session statuses become due in
//! the engine, go through the batch, and reach the sink as one
//! notification; quiet hours release as one; nothing while AI Juggler is
//! in front. The clock is the test's ([`super::show_due_at`]).

use std::sync::{Arc, Mutex};

use juggler_core::model::SessionStatus;

use super::batch::{BATCH_MS, Opens};
use super::{Outgoing, prefs, session_status, show_due_at};
use crate::state::AppState;

const NAMES: [&str; 7] = ["api", "web", "docs", "infra", "ui", "db", "cli"];

/// A state with an account, an App and seven running sessions, each in a
/// folder named for it (`s1` is api … `s7` is cli); every kind on.
fn lab(dir: &std::path::Path) -> Arc<AppState> {
    let state = crate::test_support::bare_state(dir.join("data"));
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&crate::test_support::account(
                "acct-1",
                "claude-code",
                "Lab",
                dir.join("acct"),
            ))
            .unwrap();
        storage
            .profiles()
            .insert(&crate::test_support::launch_settings("p1", "Lab", "acct-1"))
            .unwrap();
        for (i, name) in NAMES.iter().enumerate() {
            storage
                .sessions()
                .insert(&crate::test_support::instance(
                    &format!("s{}", i + 1),
                    "p1",
                    SessionStatus::Running,
                    dir.join(name),
                ))
                .unwrap();
        }
    }
    // A wait is news at once here (AJ-E2904's threshold is the user's).
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("notify.wait_after_min", "0")
        .unwrap();
    prefs::notifications_set_impl(
        &state,
        prefs::SystemNotificationsInfo {
            waiting: true,
            stuck: true,
            limit: true,
            finished: true,
            watch: true,
            ..Default::default()
        },
    )
    .unwrap();
    state
}

fn sink(state: &AppState) -> Arc<Mutex<Vec<Outgoing>>> {
    let list = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&list);
    state.notifier.set_sink(Box::new(move |out: &Outgoing| {
        into.lock().unwrap().push(out.clone())
    }));
    list
}

#[test]
fn seven_finishes_in_a_minute_reach_the_system_as_one_notification() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let base = crate::commands::now_millis();
    state.notifier.local_hour_seen(Some(12), base);
    // Statuses first (no sink yet: due, not sent), then the path runs on
    // the test's clock.
    for i in 1..=7 {
        session_status(&state, &format!("s{i}"), "finished");
    }
    let list = sink(&state);
    let next = show_due_at(&state, base);
    assert!(
        list.lock().unwrap().is_empty(),
        "calm finishes wait for the batch"
    );
    assert_eq!(next, Some(base + BATCH_MS));
    assert_eq!(
        show_due_at(&state, base + BATCH_MS),
        None,
        "the burst is out"
    );
    let sent = list.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "one notification, not seven: {sent:?}");
    assert_eq!(sent[0].title, "api finished · 6 more");
    assert_eq!(sent[0].body, "web, docs, infra, ui and 2 more finished");
    assert_eq!(state.notifier.target_of(sent[0].id), Some(Opens::Feed));
}

#[test]
fn a_need_goes_at_once_and_quiet_hours_release_what_they_held_as_one() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let base = crate::commands::now_millis();
    let list = sink(&state);
    state.notifier.local_hour_seen(Some(12), base);
    session_status(&state, "s1", "waiting_user");
    show_due_at(&state, base);
    assert_eq!(list.lock().unwrap().len(), 1, "the first need at once");
    // Night: two more needs, held.
    state.notifier.local_hour_seen(Some(23), base);
    session_status(&state, "s2", "stuck");
    session_status(&state, "s3", "quota_blocked");
    show_due_at(&state, base + 2 * BATCH_MS);
    assert_eq!(list.lock().unwrap().len(), 1, "held through quiet hours");
    // Morning: one notification for everything held.
    state.notifier.local_hour_seen(Some(8), base);
    show_due_at(&state, base + 3 * BATCH_MS);
    let sent = list.lock().unwrap().clone();
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert_eq!(sent[1].title, "web looks stuck · 1 more");
    assert_eq!(sent[1].body, "docs hit its usage limit");
    assert_eq!(state.notifier.target_of(sent[1].id), Some(Opens::Centre));
}

#[test]
fn nothing_goes_out_while_ai_juggler_is_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let base = crate::commands::now_millis();
    let list = sink(&state);
    state.notifier.local_hour_seen(Some(12), base);
    state.notifier.set_in_front(true);
    session_status(&state, "s1", "waiting_user");
    session_status(&state, "s2", "finished");
    show_due_at(&state, base);
    show_due_at(&state, base + BATCH_MS);
    assert!(list.lock().unwrap().is_empty(), "the user sees it there");
    // Leaving the window doesn't tell it late.
    state.notifier.set_in_front(false);
    show_due_at(&state, base + 2 * BATCH_MS);
    assert!(list.lock().unwrap().is_empty());
}
