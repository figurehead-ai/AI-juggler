//! D-119 item 6 (2026-11-16), on fake agents only: a pool-wide message
//! goes to the members open now, an ended member gets no copy; a direct
//! message to an ended member still lands; a resumed member's first read
//! has what the whole pool was told while it was gone (the newest 50,
//! "N earlier" said once), never twice.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    reason = "a test"
)]
#![cfg(unix)]

mod common;
#[path = "common/messages_lab.rs"]
#[allow(dead_code, reason = "the lab's helpers this test doesn't use")]
mod lab;

use juggler_agentbridge::wire::SendArgs;
use juggler_app::features::messages::{catch_up, dto::ComposeInput, views};
use juggler_app::features::pools;
use juggler_core::model::{SessionId, SessionStatus};
use lab::{Lab, field};

/// Ends `session` as its tool's exit would (its status journaled).
fn end(lab: &Lab, session: &SessionId) {
    let storage = lab.state.storage.lock().unwrap();
    let mut record = storage.sessions().get(session).unwrap();
    record.status = SessionStatus::Finished;
    storage.sessions().update(&record).unwrap();
}

/// Who holds a copy of `message`, by ref.
fn copies(lab: &Lab, message: &str) -> Vec<String> {
    let mut refs: Vec<String> = lab
        .state
        .storage
        .lock()
        .unwrap()
        .messages()
        .recipients(message)
        .unwrap()
        .into_iter()
        .map(|r| r.recipient_ref)
        .collect();
    refs.sort();
    refs
}

/// The person's message to everyone (their Everyone).
fn to_everyone(lab: &Lab, body: &str) -> String {
    let page = views::compose(
        &lab.state,
        ComposeInput {
            pool_id: lab.pool.clone(),
            to: "pool".into(),
            thread_id: None,
            kind: Some("note".into()),
            subject: Some(body.into()),
            body: body.into(),
            deliver: false,
        },
    )
    .unwrap();
    page.messages[0].id.clone()
}

#[test]
fn a_pool_wide_message_goes_to_open_members_only_and_a_direct_one_to_an_ended_member_too() {
    let lab = Lab::new();
    let (_planner, planner) = lab.agent("s-planner", "Planner");
    let (_builder, _) = lab.agent("s-builder", "Builder");
    let (old1, _) = lab.agent("s-old1", "Old one");
    let (old2, _) = lab.agent("s-old2", "Old two");
    end(&lab, &old1);
    end(&lab, &old2);

    // An agent's pool-wide message: one copy, the open member's.
    let all = lab.send(
        &planner,
        SendArgs {
            to: "pool".into(),
            body: "The schema changed.".into(),
            ..SendArgs::default()
        },
    );
    assert!(all.refused.is_none(), "{all:?}");
    let message = field(&all, "/message");
    assert_eq!(copies(&lab, &message), ["session:s-builder"]);
    assert_eq!(field(&all, "/to/0/name"), "Builder");
    assert_eq!(
        all.result.as_ref().unwrap()["to"].as_array().unwrap().len(),
        1
    );

    // The person's Everyone: the open members only.
    let everyone = to_everyone(&lab, "Stand-up in five.");
    assert_eq!(
        copies(&lab, &everyone),
        ["session:s-builder", "session:s-planner"]
    );

    // A message to an ended member by name still lands in its inbox.
    let direct = lab.say(&planner, &old1, "When you're back: the schema.", None);
    assert!(direct.refused.is_none(), "{direct:?}");
    assert_eq!(
        copies(&lab, &field(&direct, "/message")),
        ["session:s-old1"]
    );
    assert_eq!(field(&direct, "/to/0/gets_it"), "in its inbox");

    // No one else open: kept in the pool's log, no copy, said so.
    end(&lab, &SessionId::new("s-builder"));
    let alone = lab.send(
        &planner,
        SendArgs {
            to: "pool".into(),
            body: "Anyone?".into(),
            ..SendArgs::default()
        },
    );
    assert!(alone.refused.is_none(), "{alone:?}");
    assert!(copies(&lab, &field(&alone, "/message")).is_empty());
    assert!(
        field(&alone, "/to/0/gets_it").contains("no one else in the pool is open now"),
        "{alone:?}"
    );
    let _ = old2;
}

#[test]
fn a_resumed_member_reads_the_newest_50_it_missed_says_n_earlier_once_and_never_twice() {
    let lab = Lab::new();
    let (_planner, planner) = lab.agent("s-planner", "Planner");
    let (reviewer, _) = lab.agent("s-rev", "Reviewer");
    // Before it ended: its copy is in its inbox already.
    let before = lab.send(
        &planner,
        SendArgs {
            to: "pool".into(),
            body: "Before you go.".into(),
            ..SendArgs::default()
        },
    );
    let before = field(&before, "/message");
    assert_eq!(copies(&lab, &before), ["session:s-rev"]);
    end(&lab, &reviewer);
    // Its end and what follows, a clock tick apart.
    std::thread::sleep(std::time::Duration::from_millis(5));

    // While it's gone: an agent's and 53 of the person's.
    let agents = lab.send(
        &planner,
        SendArgs {
            to: "pool".into(),
            body: "While you were out.".into(),
            ..SendArgs::default()
        },
    );
    let agents = field(&agents, "/message");
    assert!(
        copies(&lab, &agents).is_empty(),
        "the planner is its sender"
    );
    let mut sent = vec![agents];
    for n in 0..53 {
        sent.push(to_everyone(&lab, &format!("Note {n}")));
    }

    // Resume: a new session on the same conversation, in its pools.
    let (resumed, key) = lab.agent("s-rev2", "Reviewer");
    pools::switching::carry_resumed(&lab.state, &reviewer, &resumed).unwrap();
    let copied = catch_up::after_resume(&lab.state, &reviewer, &resumed).unwrap();
    assert_eq!(copied, 50, "the newest 50 of 54");
    let newest = sent.last().unwrap();
    assert_eq!(
        copies(&lab, newest),
        ["session:s-planner", "session:s-rev2"]
    );
    assert!(
        !copies(&lab, &sent[3]).contains(&"session:s-rev2".to_owned()),
        "the 4 earliest stay in the log"
    );
    assert_eq!(copies(&lab, &before), ["session:s-rev"], "no second copy");

    // Its first read: newest first, and the 4 earlier ones said.
    let first = lab.inbox(&key);
    let result = first.result.as_ref().unwrap();
    assert_eq!(result["messages"][0]["id"], newest.as_str());
    let caught = &result["caught_up"][0];
    assert_eq!(caught["copied"], 50);
    assert_eq!(caught["earlier"], 4);
    let says = caught["says"].as_str().unwrap();
    assert!(says.contains("4 earlier"), "{says}");
    // After it, as before: nothing said again, nothing copied again.
    let second = lab.inbox(&key);
    assert!(second.result.as_ref().unwrap()["caught_up"].is_null());
    assert_eq!(
        catch_up::after_resume(&lab.state, &reviewer, &resumed).unwrap(),
        0
    );
    let row_count = lab
        .state
        .storage
        .lock()
        .unwrap()
        .messages()
        .recipients(newest)
        .unwrap()
        .len();
    assert_eq!(row_count, 2, "one copy each");

    // Open again: a new pool-wide message reaches it as any member.
    let after = to_everyone(&lab, "Welcome back.");
    assert!(copies(&lab, &after).contains(&"session:s-rev2".to_owned()));
}
