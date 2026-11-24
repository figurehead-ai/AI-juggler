//! AJ-E2324: the move's waits woken by events (no 1.5 s stillness floor,
//! no 250 ms polls of the apps' exits) and the open-file guard before the
//! backup and before anything moves. Scripted processes over the move
//! lab's real folders; the kernel's own waits are `juggler_platform::wait`'s
//! tests.

use super::*;

fn stop_of(error: &AppError) -> MoveStop {
    match error {
        AppError::Move(stop) => Some((**stop).clone()),
        _ => None,
    }
    .expect("a move's stop")
}

/// R1: the files hold still for one 300 ms window (woken by their
/// events), never the old 1.5 s floor.
#[test]
fn the_files_hold_still_for_one_short_window_not_a_floor() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = running_both(&lab);
    moved_with(&lab, &system);
    let settles = system.settles.lock().unwrap();
    assert!(settles.contains(&still::QUIET), "{settles:?}");
    assert!(
        settles
            .iter()
            .all(|for_| *for_ != Duration::from_millis(1500)),
        "{settles:?}"
    );
}

/// R1: the wait for the apps to quit is woken by their exits, each wait
/// on the processes still running (the real system's kernel wait).
#[test]
fn the_quit_waits_on_the_apps_exits() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = Scripted {
        slow: Some(Duration::from_millis(60)),
        ..running_both(&lab)
    };
    let mains: Vec<u32> = system
        .procs
        .lock()
        .unwrap()
        .values()
        .map(|p| p.pid)
        .collect();
    moved_with(&lab, &system);
    let waits = system.exit_waits.lock().unwrap();
    assert!(!waits.is_empty(), "woken by exits");
    assert!(
        waits
            .iter()
            .all(|pids| !pids.is_empty() && pids.iter().all(|pid| mains.contains(pid))),
        "{waits:?} of {mains:?}"
    );
}

/// R2: a program with the conversation's files open to write them (a
/// sync tool) stops the move before the backup, by name, as the
/// person's; nothing changed and no backup is kept.
#[test]
fn a_program_writing_the_files_stops_the_move_before_the_backup() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = running_both(&lab);
    *system.open_writers.lock().unwrap() = vec![vec![(512, "Dropbox".to_owned())]];
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    let stop = stop_of(&error);
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert_eq!(
        stop.words,
        "Dropbox (process 512) has this conversation's files open to write them, so nothing was moved."
    );
    assert_eq!(stop.holders.len(), 1);
    assert_eq!(stop.holders[0].name, "Dropbox");
    assert!(!system.stages().contains(&"backing_up".to_owned()));
    assert!(backups(&lab).is_empty());
    assert!(unchanged(&lab));
    assert_eq!(system.starts.lock().unwrap().len(), 2, "both started again");
}

/// R2: the guard looks again right before anything moves: a writer that
/// opened the files during the backup stops it there, and the backup is
/// let go.
#[test]
fn the_guard_looks_again_before_anything_moves() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = running_both(&lab);
    *system.open_writers.lock().unwrap() = vec![vec![], vec![(77, "Code".to_owned())]];
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    assert_eq!(*system.guard_looks.lock().unwrap(), 2);
    let stop = stop_of(&error);
    assert_eq!(stop.class, StopClass::NeedsYou);
    assert!(
        stop.words
            .starts_with("Code (process 77) has this conversation's files open"),
        "{}",
        stop.words
    );
    assert!(system.stages().contains(&"backing_up".to_owned()));
    assert!(!system.stages().contains(&"moving".to_owned()));
    assert!(unchanged(&lab));
    assert!(backups(&lab).is_empty());
}

/// R2: a move nobody else writes looks twice and goes on.
#[test]
fn a_move_nobody_else_writes_is_guarded_twice_and_goes_on() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = running_both(&lab);
    let done = moved_with(&lab, &system);
    assert_eq!(done.payload.state, "verified", "{}", done.payload.detail);
    assert_eq!(*system.guard_looks.lock().unwrap(), 2);
}
