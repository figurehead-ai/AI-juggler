//! F-rc3g-11 (0.4.6 rc3g, the P11 walk), on fake agents only: an agent
//! wrote to "p11-a" in its pool and was told no one with that session_id
//! was in it; only p11-a's session id went through. A member is written to
//! by its name as `pool_roster` gives it (case and spacing aside); only a
//! name more than one member answers to is refused, naming each with the
//! session_id that names it alone. The lead's `task_assign` takes a
//! member the same way.

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

use juggler_agentbridge::wire::{Args, Call, Refusal, Response, SendArgs, TaskAssignArgs};
use juggler_app::features::{pools, tasks::needs};
use lab::{Lab, field};

fn write(lab: &Lab, key: &str, to: &str) -> Response {
    lab.send(
        key,
        SendArgs {
            to: to.into(),
            body: format!("lab ping to {to}"),
            ..SendArgs::default()
        },
    )
}

fn refused(response: &Response) -> (Option<Refusal>, String) {
    (
        response.refused,
        response.reason.clone().unwrap_or_default(),
    )
}

#[test]
fn a_member_is_written_to_by_its_name_as_the_roster_gives_it() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "p11-a");
    let (_both, both) = lab.agent("s-both", "p11-both");
    for to in ["p11-a", " P11-A "] {
        let sent = write(&lab, &both, to);
        assert!(sent.refused.is_none(), "{to}: {sent:?}");
        assert_eq!(field(&sent, "/to/0/name"), "p11-a", "{to}");
    }
    let inbox = lab.inbox(&a);
    assert_eq!(
        inbox.result.unwrap()["messages"].as_array().unwrap().len(),
        2,
        "both landed in p11-a's inbox"
    );
    // Its own name is itself; a name no one has says so.
    let (refusal, words) = refused(&write(&lab, &both, "p11-both"));
    assert_eq!(refusal, Some(Refusal::Bad));
    assert_eq!(words, "That's this session itself.");
    let (refusal, words) = refused(&write(&lab, &both, "p11-z"));
    assert_eq!(refusal, Some(Refusal::Deny));
    assert!(
        words.starts_with("No one with that session_id or name is in this pool"),
        "{words}"
    );
}

#[test]
fn a_name_two_members_share_is_refused_naming_each_and_the_rosters_own_name_picks_one() {
    let lab = Lab::new();
    lab.agent("s-t1", "twin");
    lab.agent("s-t2", "twin");
    let (_both, both) = lab.agent("s-both", "p11-both");
    let (refusal, words) = refused(&write(&lab, &both, "twin"));
    assert_eq!(refusal, Some(Refusal::Bad));
    assert!(
        words.starts_with("«twin» could be 2 members of this pool:"),
        "{words}"
    );
    assert!(
        words.contains("session_id s-t1") && words.contains("session_id s-t2"),
        "each named: {words}"
    );
    assert!(words.ends_with("Name the one this is for by its session_id."));
    // The roster tells them apart; its name for one is that one.
    let roster = lab.call(&both, Call::PoolRoster, Args::default());
    let members = roster.result.unwrap()["members"]
        .as_array()
        .unwrap()
        .clone();
    let second = members
        .iter()
        .find(|member| member["session_id"] == "s-t2")
        .unwrap()["name"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(second, "twin", "the roster tells the two apart");
    let sent = write(&lab, &both, &second);
    assert!(sent.refused.is_none(), "{second}: {sent:?}");
    assert_eq!(field(&sent, "/to/0/name"), second);
}

#[test]
fn the_lead_gives_a_task_to_a_member_by_its_name() {
    let lab = Lab::new();
    let (lead_id, lead) = lab.agent("s-lead", "p11-lead");
    let (_b, builder) = lab.agent("s-b", "p11-a");
    lab.agent("s-t1", "twin");
    lab.agent("s-t2", "twin");
    pools::pool_set_role_impl(&lab.state, lead_id.as_str(), "lead", None).unwrap();
    let made = lab.call(
        &builder,
        Call::TaskCreate,
        Args {
            new_task: Some(juggler_agentbridge::wire::TaskCreateArgs {
                title: "Build the API".into(),
                ..Default::default()
            }),
            ..Args::default()
        },
    );
    let task = field(&made, "/task/id");
    let assign = |to: &str| {
        lab.call(
            &lead,
            Call::TaskAssign,
            Args {
                assign: Some(TaskAssignArgs {
                    task: task.clone(),
                    to: to.into(),
                    note: None,
                }),
                ..Args::default()
            },
        )
    };
    let (refusal, words) = refused(&assign("twin"));
    assert_eq!(refusal, Some(Refusal::Bad));
    assert!(
        words.starts_with("«twin» could be 2 members of this pool:"),
        "{words}"
    );
    // By its name: held for the person's OK (the pool's level), as by its id.
    let held = assign("p11-a");
    assert_eq!(held.refused, Some(Refusal::Ask), "{held:?}");
    let asks = needs::needs(&lab.state).unwrap();
    assert_eq!(asks.len(), 1);
    assert!(asks[0].title.ends_with("to p11-a"), "{}", asks[0].title);
}
