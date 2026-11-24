//! 0.5.1 F98: Quit… closes a running App through the one quit (Restart's
//! and Move's), starts nothing, and keep alive never starts it again until
//! its next Open; a quit that wasn't sent leaves keep alive as it was.

use std::sync::Arc;

use super::super::fakes::{FakeDesktop, Recorder};
use super::super::tests::{quick, running_as_me, state_with_places};
use super::super::{keep, open, set_keep_alive, tick_with, with_book};
use super::{QUIT_HERE, quit};

fn paused(state: &Arc<crate::state::AppState>, id: &str) -> Option<String> {
    with_book(state, |book| book.places[id].paused.clone()).unwrap()
}

#[test]
fn quit_closes_a_running_app_once_and_starts_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    let (me, _) = running_as_me(&state, &work);
    let starter = Recorder::default();
    let said = quit(&state, &work, &starter, quick()).unwrap();
    assert!(said.starts_with("Quit "), "{said}");
    assert_eq!(*starter.quits.lock().unwrap(), [me], "asked to quit once");
    assert!(
        starter.starts.lock().unwrap().is_empty(),
        "never started again"
    );
    // Keep alive was off: nothing to pause.
    assert_eq!(paused(&state, &work), None);
}

/// The finding: keep alive brought a quit App straight back. Quit here
/// pauses it for that App, so it stays closed, until its next Open.
#[test]
fn keep_alive_never_starts_an_app_quit_here_until_its_next_open() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    running_as_me(&state, &work);
    let starter = Recorder::default();
    quit(&state, &work, &starter, quick()).unwrap();
    assert_eq!(paused(&state, &work).as_deref(), Some(QUIT_HERE));
    // It's gone now: no process of its own anywhere.
    state.places.watch.lock().unwrap().forget_pid(&work);
    let mut now = start;
    while now < start + keep::WAIT_MS + 20_000 {
        now += 2_000;
        tick_with(&state, now, &starter, &|| false);
    }
    assert!(
        starter.starts.lock().unwrap().is_empty(),
        "keep alive left it closed"
    );
    // Its next Open (here it finds this test's own process, so it brings
    // that forward rather than starting) ends the pause.
    open(
        &state,
        &work,
        None,
        Arc::new(FakeDesktop::opening_links()),
        Arc::new(Recorder::default()),
        None,
    )
    .unwrap();
    assert_eq!(paused(&state, &work), None, "keep alive on again");
}

/// The process changed between the look and the signal: nothing is
/// signalled, and keep alive is as it was.
#[test]
fn a_quit_that_wasnt_sent_leaves_keep_alive_as_it_was() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    running_as_me(&state, &work);
    let starter = Recorder {
        reused_after_look: true,
        ..Recorder::default()
    };
    let error = quit(&state, &work, &starter, quick()).unwrap_err();
    assert!(error.contains("isn't the process"), "{error}");
    assert!(starter.quits.lock().unwrap().is_empty(), "never signalled");
    assert_eq!(paused(&state, &work), None);
}

/// It was asked but still runs (it may be asking something of its own):
/// said so, nothing started, and it stays paused in case it closes later.
#[test]
fn one_that_doesnt_quit_in_time_is_said_and_stays_paused() {
    let root = tempfile::tempdir().unwrap();
    let (state, work, _) = state_with_places(root.path());
    set_keep_alive(&state, &work, true).unwrap();
    running_as_me(&state, &work);
    let starter = Recorder {
        stays: true,
        ..Recorder::default()
    };
    let error = quit(&state, &work, &starter, quick()).unwrap_err();
    assert!(error.contains("didn't quit"), "{error}");
    assert_eq!(starter.quits.lock().unwrap().len(), 1);
    assert!(starter.starts.lock().unwrap().is_empty());
    assert_eq!(paused(&state, &work).as_deref(), Some(QUIT_HERE));
}

#[test]
fn an_app_it_doesnt_remember_is_never_quit() {
    let root = tempfile::tempdir().unwrap();
    let (state, _, _) = state_with_places(root.path());
    let starter = Recorder::default();
    let error = quit(&state, "app:nope:", &starter, quick()).unwrap_err();
    assert!(error.contains("doesn't remember"), "{error}");
    assert!(starter.quits.lock().unwrap().is_empty());
}
