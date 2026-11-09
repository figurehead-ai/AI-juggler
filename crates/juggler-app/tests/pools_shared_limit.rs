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

use juggler_agentbridge::wire::{Args, Call, Refusal};
use juggler_app::features::agent_bridge::calls::MAX_CALLS;
use juggler_app::features::messages::delivery;
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
