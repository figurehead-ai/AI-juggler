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
use juggler_app::features::pools;
use juggler_core::storage::{MemberRef, PoolRole};
use lab::Lab;

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
