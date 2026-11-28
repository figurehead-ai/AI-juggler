//! G12 (AJ-E2027): watches' timed conditions are durable. A deadline
//! armed before a restart fires once, at its own time, after it; one
//! armed before a sleep is handed to the watches by the scheduler when
//! the Mac wakes and fires once, at its own time; the engine's state
//! comes back as a unit.

use std::collections::HashSet;
use std::sync::Arc;

use juggler_core::storage::memory::Writers;

use super::live_tests::{create, lab};
use super::*;
use crate::features::schedule::service::{context, take_due};
use crate::features::watches::durable;
use crate::features::watches::runtime::with_engine;
use crate::state::AppState;

fn reopen(dir: &std::path::Path) -> Arc<AppState> {
    let mut registry = juggler_adapter::AdapterRegistry::new();
    registry.register(Arc::new(juggler_adapters::fake::FakeHarnessAdapter::new(
        "/bin/true",
    )));
    Arc::new(AppState::init_with_registry(dir.to_path_buf(), registry).unwrap())
}

fn idle_watch(now: i64) -> Watch {
    let mut w = watch(
        "w-41",
        vec![session("s1")],
        vec![cond_p("idle_for", Param::Minutes(30))],
        Lifetime::Always,
    );
    w.created_ms = now - 20 * MIN;
    w
}

fn due_of(state: &AppState) -> Option<i64> {
    let storage = state.storage.lock().unwrap();
    storage
        .schedules()
        .get(&durable::row_id(&WatchId("w-41".into())))
        .unwrap()
        .and_then(|row| row.next_due_ms)
}

#[test]
fn the_engine_state_comes_back_as_a_unit() {
    let registry = Registry::shipped();
    let mut before = Engine::new(registry.clone());
    let w = idle_watch(T + 20 * MIN);
    before.add(w.clone(), T);
    let saved = before.saved(&w.id).unwrap();
    assert_eq!(before.wake_of(&w.id), Some(T + 30 * MIN));
    // A restart an hour later: added again, its idle count would start over.
    let mut after = Engine::new(registry);
    after.add(w.clone(), T + 60 * MIN);
    assert_eq!(
        after.wake_of(&w.id),
        Some(T + 90 * MIN),
        "the bug: armed from the restart"
    );
    assert!(after.restore(&w.id, &saved));
    assert_eq!(after.saved(&w.id).unwrap(), saved);
    let fired = after.tick(T + 60 * MIN);
    assert_eq!(fired.fires.len(), 1, "{fired:?}");
    assert_eq!(fired.fires[0].at_ms, T + 30 * MIN, "at its own time");
    assert!(after.tick(T + 61 * MIN).fires.is_empty(), "once");
    // A saved state that no longer fits the watch (edited since) is refused.
    let mut edited = w;
    edited.targets.push(session("s2"));
    after.add(edited.clone(), T + 62 * MIN);
    assert!(!after.restore(&edited.id, &saved));
}

#[test]
fn a_deadline_armed_before_a_restart_fires_once_at_its_time_after_it() {
    let dir = tempfile::tempdir().unwrap();
    let now = crate::commands::now_millis();
    let deadline = now + 10 * MIN;
    {
        let state = lab(dir.path());
        create(&state, &Writers::for_tests(), &idle_watch(now));
        with_engine(&state, now - 20 * MIN, |_| ()).unwrap();
        assert_eq!(due_of(&state), Some(deadline), "saved with its schedule");
    }
    // Restarted 15 minutes on.
    let state = reopen(dir.path());
    let later = now + 15 * MIN;
    let out = with_engine(&state, later, |engine| engine.tick(later)).unwrap();
    assert_eq!(out.fires.len(), 1, "{out:?}");
    assert_eq!(out.fires[0].at_ms, deadline);
    let again = with_engine(&state, later + MIN, |engine| engine.tick(later + MIN)).unwrap();
    assert!(again.fires.is_empty(), "once");
    assert_eq!(due_of(&state), None, "nothing armed now");
}

#[test]
fn a_deadline_armed_before_a_sleep_is_handed_over_on_wake_and_fires_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = lab(dir.path());
    let now = crate::commands::now_millis();
    let deadline = now + 10 * MIN;
    create(&state, &Writers::for_tests(), &idle_watch(now));
    with_engine(&state, now - 20 * MIN, |_| ()).unwrap();
    // Asleep two hours past it: the wake's recompute, then what is due.
    let woke = deadline + 120 * MIN;
    let registry = crate::features::schedule::handlers();
    let taken = {
        let storage = state.storage.lock().unwrap();
        let ctx = context(&storage);
        crate::features::schedule::service::recompute(&storage, &ctx, "wake", woke);
        take_due(
            &storage,
            &registry,
            &ctx,
            woke,
            Some(true),
            &HashSet::<String>::new(),
            2,
        )
    };
    let ours: Vec<_> = taken
        .iter()
        .filter(|d| d.schedule.row.handler == durable::HANDLER)
        .collect();
    assert_eq!(ours.len(), 1);
    assert!(ours[0].late);
    let twice = {
        let storage = state.storage.lock().unwrap();
        let ctx = context(&storage);
        take_due(
            &storage,
            &registry,
            &ctx,
            woke,
            Some(true),
            &HashSet::<String>::new(),
            2,
        )
    };
    assert!(
        twice
            .iter()
            .all(|d| d.schedule.row.handler != durable::HANDLER),
        "handed over once"
    );
    // The watches' thread, woken by it, ticks: one fire, at the deadline.
    let out = with_engine(&state, woke, |engine| engine.tick(woke)).unwrap();
    assert_eq!(out.fires.len(), 1, "{out:?}");
    assert_eq!(out.fires[0].at_ms, deadline);
    assert!(
        with_engine(&state, woke, |engine| engine.tick(woke))
            .unwrap()
            .fires
            .is_empty()
    );
}
