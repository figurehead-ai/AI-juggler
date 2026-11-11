//! The rc32 walk (P11 step 3): deleting a pool closes what of it waited
//! on the user. Needs you kept its Ask-first holds, its paused messages
//! ("Messages in «» are paused", with no name) and its held task change.

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

use juggler_agentbridge::wire::Refusal;
use juggler_app::features::messages::needs;
use juggler_app::features::pools;
use juggler_core::storage::TaskNeed;
use lab::Lab;

#[test]
fn a_deleted_pools_holds_and_notices_leave_needs_you() {
    let lab = Lab::new();
    let (_a, a) = lab.agent("s-a", "Alpha");
    let (b, _) = lab.agent("s-b", "Beta");
    pools::access::set_level_impl(&lab.state, &lab.pool, "read", &[]).unwrap();
    assert_eq!(
        lab.say(&a, &b, "Can I start?", None).refused,
        Some(Refusal::Ask)
    );
    {
        let storage = lab.state.storage.lock().unwrap();
        storage
            .messages()
            .pause_pool(&lab.pool, "two agents kept answering each other", "system")
            .unwrap();
        storage
            .tasks()
            .insert_need(&TaskNeed {
                id: "tneed-gone".into(),
                pool_id: lab.pool.clone(),
                task_id: None,
                kind: "ask".into(),
                action: "create_task".into(),
                actor_ref: "session:s-a".into(),
                actor_name: "Alpha".into(),
                change: None,
                summary: "Alpha wants to add a task".into(),
                created_at: 1,
            })
            .unwrap();
    }
    let before = needs::needs(&lab.state).unwrap();
    assert!(before.iter().any(|need| need.kind == "ask"), "{before:?}");
    assert!(
        before.iter().any(|need| need.kind == "pool_paused"),
        "{before:?}"
    );
    assert_eq!(
        juggler_app::features::tasks::needs::needs(&lab.state)
            .unwrap()
            .len(),
        1
    );

    pools::pool_delete_impl(&lab.state, &lab.pool).unwrap();

    let after = needs::needs(&lab.state).unwrap();
    assert!(after.is_empty(), "nothing of the deleted pool: {after:?}");
    assert!(
        juggler_app::features::tasks::needs::needs(&lab.state)
            .unwrap()
            .is_empty()
    );
    let storage = lab.state.storage.lock().unwrap();
    assert!(
        storage
            .messages()
            .open_asks(0, 50)
            .unwrap()
            .iter()
            .all(|ask| ask.pool_id != lab.pool),
        "its asks are closed, not only hidden"
    );
    assert_eq!(storage.messages().pool_pause(&lab.pool).unwrap(), None);
    assert!(storage.tasks().open_need("tneed-gone").unwrap().is_none());
}

/// No Needs you line ever names a pool with nothing between « and »: a
/// row whose pool isn't there is left out even when its hold is still
/// open (one written before this fix).
#[test]
fn a_hold_whose_pool_is_gone_never_shows_an_empty_name() {
    let lab = Lab::new();
    {
        let storage = lab.state.storage.lock().unwrap();
        storage
            .messages()
            .pause_pool(&lab.pool, "paused by you", "user")
            .unwrap();
        // The pool goes as an older build deleted it: its pause stays.
        storage.pools().delete(&lab.pool).unwrap();
    }
    let rows = needs::needs(&lab.state).unwrap();
    assert!(
        rows.iter()
            .all(|need| !need.title.contains("«»") && !need.pool_name.is_empty()),
        "{rows:?}"
    );
    assert!(rows.is_empty(), "{rows:?}");
}
