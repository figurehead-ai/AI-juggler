//! AJ-E3005: a session in two pools has one call limit, not one per pool —
//! two pools waking it can't double what it may do in a short time.

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

use juggler_agentbridge::wire::{Args, Call, Refusal, SendArgs};
use juggler_app::features::agent_bridge::calls::MAX_CALLS;
use juggler_app::features::messages::{delivery, needs, views};
use juggler_app::features::pools;
use juggler_core::storage::{MemberRef, PoolRole};
use lab::{Lab, field, wait_for};

#[test]
fn two_pools_share_one_sessions_call_limit() {
    let lab = Lab::new();
    let (session, key) = lab.agent("s-both", "Both");
    let second = pools::pool_create_impl(&lab.state, "Second", None)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.name == "Second")
        .unwrap()
        .id
        .clone();
    lab.state
        .storage
        .lock()
        .unwrap()
        .pools()
        .join(
            &second,
            &MemberRef::of_session(&session, None),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    let whoami = |pool: &str| {
        lab.call(
            &key,
            Call::Whoami,
            Args {
                pool: Some(pool.to_owned()),
                ..Args::default()
            },
        )
    };
    // Half the calls for one pool, half for the other: all within the one limit.
    for n in 0..MAX_CALLS {
        let response = whoami(if n % 2 == 0 {
            lab.pool.as_str()
        } else {
            second.as_str()
        });
        assert_eq!(response.refused, None, "call {n}: {response:?}");
    }
    // The next, for either pool, is over it.
    assert_eq!(whoami(&second).refused, Some(Refusal::Limit));
    assert_eq!(whoami(&lab.pool).refused, Some(Refusal::Limit));
}

/// P11 step 1 (the rc32 walk): a session in two pools has one message
/// rate, not one per pool. Its 10 a minute spent in one pool, the other
/// pool refuses too, and the words say the limit is the session's.
#[test]
fn two_pools_share_one_sessions_message_rate() {
    let lab = Lab::new();
    let (session, key) = lab.agent("s-both", "Both");
    let (here, _) = lab.agent("s-here", "Here");
    let (there, _) = lab.agent("s-there", "There");
    let second = pools::pool_create_impl(&lab.state, "Second", None)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.name == "Second")
        .unwrap()
        .id
        .clone();
    pools::pool_move_impl(&lab.state, there.as_str(), &second).unwrap();
    lab.state
        .storage
        .lock()
        .unwrap()
        .pools()
        .join(
            &second,
            &MemberRef::of_session(&session, None),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    let say = |pool: &str, to: &juggler_core::model::SessionId, body: &str| {
        lab.call(
            &key,
            Call::SendMessage,
            Args {
                pool: Some(pool.to_owned()),
                send: Some(SendArgs {
                    to: to.to_string(),
                    body: body.into(),
                    ..SendArgs::default()
                }),
                ..Args::default()
            },
        )
    };
    for n in 0..juggler_core::messaging::PER_MINUTE {
        let sent = say(&lab.pool, &here, &format!("update {n}"));
        assert_eq!(sent.refused, None, "message {n}: {sent:?}");
    }
    let over = say(&second, &there, "one more, in the other pool");
    assert_eq!(over.refused, Some(Refusal::Limit), "{over:?}");
    let words = over.reason.clone().unwrap_or_default();
    assert!(words.contains("in all its pools together"), "{words}");
    assert!(!words.contains('«'), "no one pool named: {words}");
}

/// AJ-E3304 R2, named risky triple 4: Full × waking × the same session in
/// two pools. Both pools work at Full, so a message in either wakes the
/// one session they share (D-29); Stop all is per pool (the pools model's
/// edge case "Two pools at Full both wake one session"), so stopping one
/// pool's delivery leaves the other's waking it, and the stopped pool's
/// message waits in the inbox until its own delivery is back on.
#[test]
fn two_pools_at_full_both_wake_one_session_and_stop_all_holds_only_its_own() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Planner");
    let (_c, c) = lab.agent("s-c", "Client");
    let (builder, _b) = lab.driven("Builder");
    let second = pools::pool_create_impl(&lab.state, "Second", None)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.name == "Second")
        .unwrap()
        .id
        .clone();
    pools::pool_move_impl(&lab.state, "s-c", &second).unwrap();
    lab.state
        .storage
        .lock()
        .unwrap()
        .pools()
        .join(
            &second,
            &MemberRef::of_session(&builder, None),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    for pool in [&lab.pool, &second] {
        pools::access::set_level_impl(&lab.state, pool, "full", &[]).unwrap();
    }
    let state_of = |message: &str| -> Vec<String> {
        lab.state
            .storage
            .lock()
            .unwrap()
            .messages()
            .recipients(message)
            .unwrap()
            .into_iter()
            .map(|r| r.state.as_str().to_owned())
            .collect()
    };
    let pool_of = |message: &str| {
        lab.state
            .storage
            .lock()
            .unwrap()
            .messages()
            .message(message)
            .unwrap()
            .unwrap()
            .pool_id
    };
    let idle = || {
        wait_for("its turn to end", || {
            delivery::between_turns(&lab.state, &builder)
        });
    };
    // Each pool at Full wakes it.
    for (key, pool, body) in [(&a, &lab.pool, "From Team"), (&c, &second, "From Second")] {
        let sent = lab.say(key, &builder, body, None);
        assert_eq!(
            field(&sent, "/to/0/gets_it"),
            "at its next turn",
            "{sent:?}"
        );
        let message = field(&sent, "/message");
        assert_eq!(&pool_of(&message), pool, "sent in its own pool");
        wait_for("the delivery", || state_of(&message) == ["delivered"]);
        idle();
    }

    // Stop all in Second: its message waits; Team's still wakes it.
    pools::access::set_delivery_impl(&lab.state, &second, 5, true).unwrap();
    let held = lab.say(&c, &builder, "Second again", None);
    assert_eq!(
        field(&held, "/to/0/gets_it"),
        "in its inbox (delivery is stopped in this pool)"
    );
    let held = field(&held, "/message");
    let woken = lab.say(&a, &builder, "Team again", None);
    assert_eq!(field(&woken, "/to/0/gets_it"), "at its next turn");
    let woken = field(&woken, "/message");
    wait_for("Team's delivery", || state_of(&woken) == ["delivered"]);
    idle();
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(state_of(&held), ["unread"], "Second's stop holds its own");

    // Second's delivery back on: its next message wakes it again.
    pools::access::set_delivery_impl(&lab.state, &second, 5, false).unwrap();
    let back = lab.say(&c, &builder, "Second back", None);
    assert_eq!(field(&back, "/to/0/gets_it"), "at its next turn");
    let back = field(&back, "/message");
    wait_for("Second's delivery", || state_of(&back) == ["delivered"]);
}

/// 0.4.1: the hop limit across two pools. One driven session is in both;
/// two agent chains, one a pool, bounce messages through it. Each pool's
/// hop limit (Team 2, Second 3) pauses its own thread only, the other
/// pool's thread goes on, and Stop all in one pool holds only its own.
#[test]
fn two_pools_chains_through_one_session_each_pause_at_their_own_hop_limit() {
    let lab = Lab::new();
    let (planner, a) = lab.agent("s-a", "Planner");
    let (client, c) = lab.agent("s-c", "Client");
    let (builder, b) = lab.driven("Builder");
    let second = pools::pool_create_impl(&lab.state, "Second", None)
        .unwrap()
        .pools
        .iter()
        .find(|pool| pool.name == "Second")
        .unwrap()
        .id
        .clone();
    pools::pool_move_impl(&lab.state, "s-c", &second).unwrap();
    lab.state
        .storage
        .lock()
        .unwrap()
        .pools()
        .join(
            &second,
            &MemberRef::of_session(&builder, None),
            &[],
            PoolRole::Member,
            None,
        )
        .unwrap();
    for (pool, hops) in [(&lab.pool, 2), (&second, 3)] {
        pools::access::set_level_impl(&lab.state, pool, "full", &[]).unwrap();
        pools::access::set_delivery_impl(&lab.state, pool, hops, false).unwrap();
    }
    let state_of = |message: &str| -> Vec<String> {
        lab.state
            .storage
            .lock()
            .unwrap()
            .messages()
            .recipients(message)
            .unwrap()
            .into_iter()
            .map(|r| r.state.as_str().to_owned())
            .collect()
    };
    let thread_state = |thread: &str| {
        views::thread(&lab.state, thread, None)
            .unwrap()
            .thread
            .state
    };
    let idle = || {
        wait_for("its turn to end", || {
            delivery::between_turns(&lab.state, &builder)
        });
    };
    // The builder answers in the pool it names (it's in two).
    let builder_says =
        |pool: &str, to: &juggler_core::model::SessionId, body: &str, thread: &str| {
            let sent = lab.call(
                &b,
                Call::SendMessage,
                Args {
                    pool: Some(pool.to_owned()),
                    send: Some(SendArgs {
                        to: to.to_string(),
                        body: body.into(),
                        thread: Some(thread.to_owned()),
                        ..SendArgs::default()
                    }),
                    ..Args::default()
                },
            );
            assert!(sent.refused.is_none(), "{sent:?}");
            sent
        };
    let delivered = |sent: &juggler_agentbridge::wire::Response| {
        assert_eq!(field(sent, "/to/0/gets_it"), "at its next turn", "{sent:?}");
        let message = field(sent, "/message");
        wait_for("the delivery", || state_of(&message) == ["delivered"]);
        idle();
    };

    // Both chains start, interleaved through the one session.
    let team_1 = lab.say(&a, &builder, "Team one", None);
    let team = field(&team_1, "/thread");
    delivered(&team_1);
    let second_1 = lab.say(&c, &builder, "Second one", None);
    let other = field(&second_1, "/thread");
    delivered(&second_1);
    builder_says(&lab.pool, &planner, "Team two", &team);
    builder_says(&second, &client, "Second two", &other);

    // Team's third hop is past its limit (2): Team's thread pauses alone.
    let team_3 = lab.say(&a, &builder, "Team three", Some(&team));
    assert_eq!(
        field(&team_3, "/to/0/gets_it"),
        "when the user says (the chain reached the hop limit)"
    );
    assert_eq!(field(&team_3, "/hop_limit"), "2");
    assert_eq!(thread_state(&team), "paused");
    assert_eq!(
        thread_state(&other),
        "open",
        "the other pool's thread goes on"
    );
    let refused = lab.say(&a, &builder, "Team four", Some(&team));
    assert_eq!(refused.refused, Some(Refusal::Limit), "{refused:?}");

    // Second's goes on to its own limit (3), through the same session.
    let second_3 = lab.say(&c, &builder, "Second three", Some(&other));
    assert_eq!(field(&second_3, "/hop"), "3");
    delivered(&second_3);
    builder_says(&second, &client, "Second four", &other);
    let second_5 = lab.say(&c, &builder, "Second five", Some(&other));
    assert_eq!(
        field(&second_5, "/to/0/gets_it"),
        "when the user says (the chain reached the hop limit)"
    );
    assert_eq!(field(&second_5, "/hop_limit"), "3");
    assert_eq!(thread_state(&other), "paused");
    let asks: Vec<_> = needs::needs(&lab.state)
        .unwrap()
        .into_iter()
        .filter(|need| need.kind == "deliver")
        .map(|need| (need.pool_id, need.thread_id.unwrap()))
        .collect();
    assert_eq!(asks.len(), 2, "one ask a paused thread: {asks:?}");
    assert!(asks.contains(&(lab.pool.clone(), team.clone())));
    assert!(asks.contains(&(second.clone(), other.clone())));

    // Stop all in Second: Second's new chain waits; Team's still wakes it.
    pools::access::set_delivery_impl(&lab.state, &second, 3, true).unwrap();
    let held = lab.say(&c, &builder, "Second, stopped", None);
    assert_eq!(
        field(&held, "/to/0/gets_it"),
        "in its inbox (delivery is stopped in this pool)"
    );
    let held = field(&held, "/message");
    let woken = lab.say(&a, &builder, "Team, new chain", None);
    delivered(&woken);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(state_of(&held), ["unread"], "Second's stop holds its own");
    assert_eq!(state_of(&field(&second_5, "/message")), ["unread"]);
}
