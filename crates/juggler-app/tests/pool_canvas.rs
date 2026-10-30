//! 0.3 (redesign §5): a pool's canvas, on fake agents only.
//!
//! - Edges: who wrote to whom, counted per direction, with the newest
//!   thread to open; nothing about members the canvas doesn't show.
//! - Asks: a message above the pool's level is an ask on the member that
//!   wrote it, and its edge says it waits on the answer.
//! - The stream: the pool's newest activity in plain words.
//! - Positions: kept under what the member stands for, refused for a
//!   session that isn't in the pool or a place off the canvas; Tidy up
//!   forgets them; the Pools screen's cards read them for open members.
//! - None of it changes a pool, a member, a role or a session.

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

use juggler_app::error::AppError;
use juggler_app::features::pools::{self, canvas};
use lab::Lab;

fn at(session: &str, x: f64, y: f64) -> pools::NodePositionInfo {
    pools::NodePositionInfo {
        session_id: session.to_owned(),
        x,
        y,
    }
}

fn refused(result: Result<(), AppError>) -> String {
    match result {
        Err(AppError::Refused(words)) => words,
        other => panic!("expected a refusal, got {}", other.is_ok()),
    }
}

#[test]
fn edges_count_who_wrote_to_whom_and_the_stream_says_it() {
    let lab = Lab::new();
    let (a_id, a) = lab.agent("s-a", "Alpha");
    let (b_id, _b) = lab.agent("s-b", "Beta");
    let (c_id, _c) = lab.agent("s-c", "Gamma");
    lab.say(&a, &b_id, "Totals are off by a cent", None);
    lab.say(&a, &b_id, "Round per line?", None);
    lab.say(&a, &c_id, "Copy for the button", None);

    let shown = vec![a_id.to_string(), b_id.to_string()];
    let info = canvas::canvas_impl(&lab.state, &lab.pool, shown).unwrap();
    assert_eq!(info.edges.len(), 1, "Gamma isn't shown: no edge to it");
    let edge = &info.edges[0];
    assert_eq!(
        (edge.from_session_id.as_str(), edge.to_session_id.as_str()),
        ("s-a", "s-b")
    );
    assert_eq!(edge.messages, 2);
    assert_eq!(edge.tasks, 0);
    assert!(
        edge.thread_id.is_some(),
        "the newest thread opens from the edge"
    );
    assert!(
        edge.pending.is_empty(),
        "a terminal session's inbox isn't Deliver's"
    );
    assert!(info.stream.len() >= 3);
    assert!(
        info.stream.iter().any(
            |line| line.words == "Alpha wrote to Beta: «Round per line?»"
                && line.session_id.as_deref() == Some("s-a")
                && line.other_session_id.as_deref() == Some("s-b")
        ),
        "the stream says it in words"
    );
    assert!(info.since_ms > 0);
}

#[test]
fn an_ask_sits_on_the_member_that_asked_and_its_edge_waits() {
    let lab = Lab::new();
    let (a_id, a) = lab.agent("s-a", "Alpha");
    let (b_id, _b) = lab.agent("s-b", "Beta");
    pools::access::set_level_impl(&lab.state, &lab.pool, "read", &[]).unwrap();
    lab.say(&a, &b_id, "Can I start?", None);

    let info = canvas::canvas_impl(
        &lab.state,
        &lab.pool,
        vec![a_id.to_string(), b_id.to_string()],
    )
    .unwrap();
    let ask = info.asks.first().expect("the held message's ask");
    assert_eq!(ask.session_id.as_deref(), Some("s-a"));
    assert_eq!(ask.source, "message");
    assert!(!ask.offer_always, "the first time: no rule offered");
    let edge = info.edges.first().expect("the held message's edge");
    assert_eq!(edge.pending.len(), 1);
    assert_eq!(edge.pending[0].kind, "asked");
    assert_eq!(edge.pending[0].ask_id.as_deref(), Some(ask.id.as_str()));

    // Not on the canvas: the ask is still there, on no member.
    let info = canvas::canvas_impl(&lab.state, &lab.pool, vec![b_id.to_string()]).unwrap();
    assert_eq!(info.asks.len(), 1);
    assert_eq!(info.asks[0].session_id, None);
}

#[test]
fn positions_are_the_users_and_tidy_up_forgets_them() {
    let lab = Lab::new();
    let (a_id, _a) = lab.agent("s-a", "Alpha");
    let (b_id, _b) = lab.agent("s-b", "Beta");
    let before = pools::pools_list_impl(&lab.state).unwrap();

    canvas::place_impl(&lab.state, &lab.pool, &[at("s-a", 120.0, -40.5)]).unwrap();
    canvas::place_impl(&lab.state, &lab.pool, &[at("s-a", 160.0, -40.5)]).unwrap();
    let info = canvas::canvas_impl(&lab.state, &lab.pool, vec![a_id.to_string()]).unwrap();
    assert_eq!(info.positions.len(), 1, "one place a member");
    assert_eq!(info.positions[0].session_id, "s-a");
    assert_eq!((info.positions[0].x, info.positions[0].y), (160.0, -40.5));

    // The Pools screen's card reads it for an open member.
    let cards = canvas::previews_impl(&lab.state, vec![lab.pool.clone()]).unwrap();
    assert_eq!(cards[0].positions.len(), 1);
    assert_eq!(cards[0].moving, 0);

    // Refused: off the canvas, not a number, a session elsewhere, a pool gone.
    let far = refused(canvas::place_impl(
        &lab.state,
        &lab.pool,
        &[at("s-a", 1e9, 0.0)],
    ));
    assert!(far.contains("off the canvas"), "{far}");
    let nan = refused(canvas::place_impl(
        &lab.state,
        &lab.pool,
        &[at("s-a", f64::NAN, 0.0)],
    ));
    // One bad node and none is written.
    let mixed = [at("s-b", 5.0, 5.0), at("s-a", f64::INFINITY, 0.0)];
    refused(canvas::place_impl(&lab.state, &lab.pool, &mixed));
    let info = canvas::canvas_impl(&lab.state, &lab.pool, vec![b_id.to_string()]).unwrap();
    assert!(info.positions.is_empty(), "all or nothing");
    assert!(nan.contains("off the canvas"), "{nan}");
    let other = pools::pool_create_impl(&lab.state, "Other", None).unwrap();
    let other = other
        .pools
        .iter()
        .find(|p| p.name == "Other")
        .unwrap()
        .id
        .clone();
    let elsewhere = refused(canvas::place_impl(
        &lab.state,
        &other,
        &[at("s-a", 0.0, 0.0)],
    ));
    assert!(elsewhere.contains("isn't in this pool"), "{elsewhere}");
    let gone = refused(canvas::place_impl(
        &lab.state,
        "pool-gone",
        &[at("s-a", 0.0, 0.0)],
    ));
    assert!(gone.contains("isn't there"), "{gone}");

    canvas::tidy_impl(&lab.state, &lab.pool).unwrap();
    let info = canvas::canvas_impl(&lab.state, &lab.pool, vec![a_id.to_string()]).unwrap();
    assert!(info.positions.is_empty(), "Tidy up lays it out again");

    // Nothing about the pools changed.
    let after = pools::pools_list_impl(&lab.state).unwrap();
    assert_eq!(after.placements, before.placements);
}

#[test]
fn a_canvas_names_a_bounded_set_of_members() {
    let lab = Lab::new();
    let many: Vec<String> = (0..=canvas::MOST_NODES).map(|n| format!("s-{n}")).collect();
    match canvas::canvas_impl(&lab.state, &lab.pool, many) {
        Err(AppError::Refused(words)) => assert!(words.starts_with("At most"), "{words}"),
        other => panic!("expected a refusal, got {}", other.is_ok()),
    }
    let pools: Vec<String> = (0..=canvas::MOST_PREVIEWS)
        .map(|n| format!("p-{n}"))
        .collect();
    assert!(canvas::previews_impl(&lab.state, pools).is_err());
}
