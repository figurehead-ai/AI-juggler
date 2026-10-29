//! D-44 (2026-10-27): the user sees and tracks the agents' tasks and has
//! four actions on them, each with its rules held by the backend, on fake
//! agents only:
//!
//! - **Approve** or **Send back** only a task in review: done, or in
//!   progress again with its holder kept and the user's note, which the
//!   holder gets as the user's message (in its inbox; at its next turn when
//!   it runs in AI Juggler) and reads in the task's history.
//! - **Free** only a task whose holder's sessions have all ended or
//!   crashed (as their own records say): no one holds it, and another
//!   agent can take it.
//! - **Cancel** only an open task.
//!
//! Each is in the task's history as the user's ("You" in the app, "the
//! user" to an agent).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

mod common;
#[path = "common/messages_lab.rs"]
#[allow(dead_code)]
mod lab;

use juggler_agentbridge::wire::{
    self, Args, Call, Response, TaskArgs, TaskCreateArgs, TaskUpdateArgs,
};
use juggler_app::features::messages;
use juggler_app::features::tasks::{oversight, views};
use juggler_core::model::{SessionId, SessionStatus};
use lab::{Lab, field, seed_task, wait_for};

fn task(lab: &Lab, title: &str) -> String {
    seed_task(
        &lab.state,
        &lab.pool,
        TaskCreateArgs {
            title: title.into(),
            ..TaskCreateArgs::default()
        },
    )
}

fn claim(lab: &Lab, key: &str, id: &str) {
    let won = lab.call(
        key,
        Call::TaskClaim,
        Args {
            task: Some(TaskArgs { task: id.into() }),
            ..Args::default()
        },
    );
    assert!(won.result.is_some(), "the claim is won");
}

fn mark(lab: &Lab, key: &str, id: &str, status: &str) -> Response {
    lab.call(
        key,
        Call::TaskUpdate,
        Args {
            task_change: Some(TaskUpdateArgs {
                task: id.into(),
                status: Some(status.into()),
                ..TaskUpdateArgs::default()
            }),
            ..Args::default()
        },
    )
}

/// The task as an agent reads it (`task_get`).
fn got(lab: &Lab, key: &str, id: &str) -> serde_json::Value {
    lab.call(
        key,
        Call::TaskGet,
        Args {
            task: Some(TaskArgs { task: id.into() }),
            ..Args::default()
        },
    )
    .result
    .unwrap()
}

/// The newest event of the task's history, in the app's words.
fn newest(lab: &Lab, id: &str) -> (String, String) {
    let event = views::detail(&lab.state, id, None)
        .unwrap()
        .history
        .remove(0);
    (event.who, event.what)
}

fn end_session(lab: &Lab, session: &SessionId, how: SessionStatus) {
    let storage = lab.state.storage.lock().unwrap();
    let mut record = storage.sessions().get(session).unwrap();
    record.status = how;
    storage.sessions().update(&record).unwrap();
}

#[test]
fn approve_only_a_task_in_review_and_it_is_logged_as_yours() {
    let lab = Lab::new();
    let (_b, builder) = lab.agent("s-b", "Builder");
    let id = task(&lab, "Shipping rates");
    claim(&lab, &builder, &id);
    let detail = views::detail(&lab.state, &id, None).unwrap();
    assert!(!detail.actions.review, "in progress: nothing to review");
    assert!(detail.actions.cancel);
    assert!(
        oversight::approve(&lab.state, &id).is_err(),
        "only a task in review is approved"
    );
    assert_eq!(
        views::detail(&lab.state, &id, None).unwrap().task.status,
        "in_progress"
    );
    assert!(mark(&lab, &builder, &id, "in_review").result.is_some());
    assert!(views::detail(&lab.state, &id, None).unwrap().actions.review);
    let done = oversight::approve(&lab.state, &id).unwrap();
    assert_eq!(done.task.status, "done");
    assert!(!done.actions.review && !done.actions.cancel && !done.actions.free);
    assert_eq!(
        newest(&lab, &id),
        ("You".to_owned(), "approved it: done".to_owned())
    );
    // The agent reads it as the user's, never as "You".
    let history = got(&lab, &builder, &id)["history"][0].clone();
    assert_eq!(history["who"], "the user");
    assert_eq!(history["what"], "approved");
    // Closed: nothing more.
    assert!(oversight::approve(&lab.state, &id).is_err());
    assert!(
        oversight::cancel(&lab.state, &id).is_err(),
        "done is closed"
    );
}

#[test]
fn a_sent_back_task_keeps_its_holder_and_the_note_reaches_it() {
    let lab = Lab::new();
    let (b_id, builder) = lab.agent("s-b", "Builder");
    let id = task(&lab, "Shipping rates");
    claim(&lab, &builder, &id);
    assert!(
        oversight::send_back(&lab.state, &id, "Use the carrier").is_err(),
        "only a task in review is sent back"
    );
    assert!(mark(&lab, &builder, &id, "in_review").result.is_some());
    assert!(
        oversight::send_back(&lab.state, &id, "  \n ").is_err(),
        "a note is needed"
    );
    assert!(
        oversight::send_back(&lab.state, &id, &"a".repeat(501)).is_err(),
        "a short note"
    );
    let note = "Rates must come from the carrier's API, not the table";
    let back = oversight::send_back(&lab.state, &id, note).unwrap();
    assert_eq!(back.task.status, "in_progress");
    assert_eq!(back.task.holder.as_deref(), Some("Builder"));
    assert_eq!(back.task.holder_session_id.as_deref(), Some(b_id.as_str()));
    assert_eq!(
        newest(&lab, &id),
        ("You".to_owned(), format!("sent it back: {note}"))
    );
    // In its history for the agent…
    let history = got(&lab, &builder, &id)["history"][0].clone();
    assert_eq!(history["who"], "the user");
    assert_eq!(history["what"], "sent_back");
    assert_eq!(history["detail"], note);
    // …and in its inbox, as the user's message (it runs in a terminal:
    // never typed into).
    let inbox = lab.inbox(&builder);
    let message = inbox.result.as_ref().unwrap()["messages"][0].clone();
    assert_eq!(message["from"], "You");
    let subject = message["subject"].as_str().unwrap_or_default();
    assert!(
        subject.starts_with(&format!("{id} sent back")),
        "its subject names the task"
    );
    let read = lab.call(
        &builder,
        Call::ReadInbox,
        Args {
            inbox: Some(wire::InboxArgs {
                message: message["id"].as_str().map(str::to_owned),
                ..wire::InboxArgs::default()
            }),
            ..Args::default()
        },
    );
    let body = serde_json::to_string(&read.result.unwrap()).unwrap();
    assert!(body.contains(note), "the note is the message");
    // The task's page shows the thread that names it.
    let threads = views::detail(&lab.state, &id, None).unwrap().threads;
    assert_eq!(threads.len(), 1);
    // In progress again: the holder can say where it is, as before.
    assert_eq!(
        field(&mark(&lab, &builder, &id, "in_review"), "/task/status"),
        "in_review"
    );
}

#[test]
fn a_holder_run_in_ai_juggler_gets_the_note_at_its_next_turn() {
    let lab = Lab::new();
    let (session, builder) = lab.driven("Builder");
    let id = task(&lab, "Shipping rates");
    claim(&lab, &builder, &id);
    assert!(mark(&lab, &builder, &id, "in_review").result.is_some());
    let note = "Add the carrier's timeout to the rates call";
    let back = oversight::send_back(&lab.state, &id, note).unwrap();
    // Not waiting on a Deliver click: the user's Send back is the click.
    let thread = back.threads[0].thread_id.clone();
    let delivered = || {
        let page = messages::views::thread(&lab.state, &thread, None).unwrap();
        let message = &page.messages[0];
        assert!(message.body.contains(note), "the note is the message");
        message.recipients.iter().all(|r| r.state == "delivered")
    };
    wait_for("the note delivered to the holder's session", delivered);
    wait_for("the delivered turn", || {
        lab.lines(&session)
            .iter()
            .any(|line| line == "Started by a message from You")
    });
}

#[test]
fn free_only_a_task_whose_holders_session_ended_and_another_agent_takes_it() {
    let lab = Lab::new();
    let (b_id, builder) = lab.agent("s-b", "Builder");
    let (_r, reviewer) = lab.agent("s-r", "Reviewer");
    let id = task(&lab, "Flaky test in cart.spec");
    assert!(oversight::free(&lab.state, &id).is_err(), "no one holds it");
    claim(&lab, &builder, &id);
    assert!(!views::detail(&lab.state, &id, None).unwrap().actions.free);
    assert!(
        oversight::free(&lab.state, &id).is_err(),
        "its holder's session still runs"
    );
    // Its session crashed (as its record says): now it may be freed.
    end_session(&lab, &b_id, SessionStatus::Failed);
    let detail = views::detail(&lab.state, &id, None).unwrap();
    assert!(detail.actions.free);
    assert_eq!(detail.task.holder_session_id, None);
    let freed = oversight::free(&lab.state, &id).unwrap();
    assert_eq!(freed.task.status, "ready");
    assert_eq!(freed.task.holder, None);
    assert!(!freed.actions.free);
    assert_eq!(
        newest(&lab, &id),
        (
            "You".to_owned(),
            "freed it from Builder, whose session had ended".to_owned()
        )
    );
    // Another agent takes it.
    claim(&lab, &reviewer, &id);
    assert_eq!(
        views::detail(&lab.state, &id, None)
            .unwrap()
            .task
            .holder
            .as_deref(),
        Some("Reviewer")
    );
    // A session that finished normally counts as ended too.
    let (s_id, second) = lab.agent("s-s", "Second");
    let other = task(&lab, "Docs");
    claim(&lab, &second, &other);
    end_session(&lab, &s_id, SessionStatus::Finished);
    assert!(oversight::free(&lab.state, &other).is_ok());
}

#[test]
fn cancel_only_an_open_task() {
    let lab = Lab::new();
    let (_b, builder) = lab.agent("s-b", "Builder");
    let ready = task(&lab, "Refunds in cents");
    let cancelled = oversight::cancel(&lab.state, &ready).unwrap();
    assert_eq!(cancelled.task.status, "cancelled");
    assert!(!cancelled.actions.cancel);
    assert_eq!(
        newest(&lab, &ready),
        ("You".to_owned(), "cancelled it".to_owned())
    );
    assert!(
        oversight::cancel(&lab.state, &ready).is_err(),
        "closed already"
    );
    assert!(oversight::send_back(&lab.state, &ready, "x").is_err());
    // A task someone is working on can be cancelled too; the agent sees it.
    let held = task(&lab, "Old checkout copy");
    claim(&lab, &builder, &held);
    oversight::cancel(&lab.state, &held).unwrap();
    assert_eq!(got(&lab, &builder, &held)["status"], "cancelled");
    assert!(
        oversight::cancel(&lab.state, "aj-ffff").is_err(),
        "not there"
    );
}
