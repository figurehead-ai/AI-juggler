//! AJ-E2319: a move's refusals are typed. Each failure maps to its class
//! (transient, needs you, setup, bug), names what holds the files with its
//! door, carries the move's id, and offers Retry only where trying again
//! can help. Scripted processes over the move lab's real folders.

use super::*;
use crate::error::{MoveStopInfo, StopClass};

/// The window's reading of an error: its class, whether Retry shows, and
/// its typed stop.
fn read(error: &AppError) -> (StopClass, bool, MoveStopInfo) {
    let wire = error.wire();
    let stop = wire.stop.clone().expect("a move's stop is typed");
    (stop.class, wire.retryable, stop)
}

fn holder(lab: &Lab, side: &Option<(String, KnownPlace)>) -> MoveHolder {
    let (id, place) = side.as_ref().unwrap();
    MoveHolder {
        name: place_name(&lab.state, place),
        place_id: Some(id.clone()),
        ..MoveHolder::default()
    }
}

/// R4: the table, class by class: Retry only for a transient stop that
/// changed nothing (a move left part way has Undo instead).
#[test]
fn retry_shows_only_for_a_transient_stop_that_changed_nothing() {
    let table = [
        (StopClass::Transient, true, true),
        (StopClass::Transient, false, false),
        (StopClass::NeedsYou, true, false),
        (StopClass::NeedsYou, false, false),
        (StopClass::Setup, true, false),
        (StopClass::Bug, true, false),
        (StopClass::Bug, false, false),
    ];
    for (class, unchanged, retry) in table {
        let mut stop = MoveStop::new(class, "why", "detail");
        stop.unchanged = unchanged;
        stop.move_id = Some("m-1".to_owned());
        let wire = AppError::from(stop).wire();
        assert_eq!(wire.retryable, retry, "{class:?} unchanged={unchanged}");
        let typed = wire.stop.unwrap();
        assert_eq!((typed.class, typed.unchanged), (class, unchanged));
        assert_eq!(typed.move_id.as_deref(), Some("m-1"));
        assert_eq!(
            wire.code,
            if unchanged {
                "move_refused"
            } else {
                "move_stopped"
            }
        );
    }
}

/// R1: an untyped error reaching the window is typed: a refusal of the
/// class given, anything else a bug (never Retry).
#[test]
fn untyped_errors_are_typed_at_the_edge() {
    let cases = [
        (refused("busy"), StopClass::NeedsYou, StopClass::NeedsYou),
        (refused("no shape"), StopClass::Setup, StopClass::Setup),
        (
            AppError::Io {
                context: "move",
                message: "disk".to_owned(),
            },
            StopClass::NeedsYou,
            StopClass::Bug,
        ),
        (
            MoveStop::new(StopClass::Transient, "w", "d").into(),
            StopClass::Bug,
            StopClass::Transient,
        ),
    ];
    for (error, given, class) in cases {
        let said = error.to_string();
        assert_eq!(read(&typed(error, given)).0, class, "{said}");
    }
}

/// R2 transient: the files still changing name the profile writing them
/// (with its door), carry the move's id, offer Retry, and are tried once
/// more by themselves before the window sees them.
#[test]
fn files_still_changing_are_transient_named_and_tried_once_more() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = still_writing(&lab);
    let error = run_once_more(&lab.state, &lab.sides, &system).unwrap_err();
    let (class, retry, stop) = read(&error);
    assert_eq!((class, retry), (StopClass::Transient, true));
    assert_eq!(stop.holders, vec![holder(&lab, &lab.sides.source_place)]);
    let all = moves(&lab.state);
    assert_eq!(all.len(), 2, "tried once more: two moves, both undone");
    assert_eq!(stop.move_id.as_deref(), Some(all[0].move_id.as_str()));
    assert!(
        system
            .told
            .lock()
            .unwrap()
            .iter()
            .any(|t| t.words == "Trying once more…"),
        "said it tries again"
    );
    assert!(unchanged(&lab));
}

/// R2 needs you: an app that won't quit (asking something) is named with
/// its door; no Retry, and it isn't tried again by itself (each try
/// quits and reopens both apps).
#[test]
fn an_app_asking_something_needs_you_and_is_never_retried() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = stubborn_target(&lab);
    let error = run_once_more(&lab.state, &lab.sides, &system).unwrap_err();
    let (class, retry, stop) = read(&error);
    assert_eq!((class, retry), (StopClass::NeedsYou, false));
    assert_eq!(stop.holders, vec![holder(&lab, &lab.sides.target_place)]);
    assert!(error.wire().user_message.contains("asking something"));
    assert_eq!(
        system.quits.lock().unwrap().len(),
        2,
        "quit once, not twice"
    );
}

/// R2 needs you: a terminal on the conversation, refused before anything
/// quits; a session working, likewise.
#[test]
fn what_the_offer_refuses_needs_you() {
    clones_on_any_disk();
    let lab = lab();
    for system in [
        Scripted {
            elsewhere: 1,
            ..running_both(&lab)
        },
        Scripted {
            working: 2,
            ..running_both(&lab)
        },
    ] {
        let error = run_once_more(&lab.state, &lab.sides, &system).unwrap_err();
        let (class, retry, stop) = read(&error);
        assert_eq!((class, retry), (StopClass::NeedsYou, false), "{error}");
        assert!(stop.move_id.is_none(), "no move began");
        assert!(system.quits.lock().unwrap().is_empty());
    }
}

/// R2 setup: a disk that makes no clones (no Undo backups) refuses the
/// move before anything quits, on any disk this runs on, and says why.
#[test]
fn a_disk_without_clones_is_refused_as_setup() {
    juggler_platform::transfer::set_clones_on_this_thread(Some(false));
    let lab = lab();
    let system = running_both(&lab);
    let offer = offer_of(&lab.state, &lab.sides, &system);
    assert!(
        offer
            .blocks
            .iter()
            .any(|block| block.words.contains("clone backups")),
        "{:?}",
        offer.blocks
    );
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    let (class, retry, stop) = read(&error);
    assert_eq!((class, retry), (StopClass::Setup, false), "{error}");
    assert!(
        error.wire().user_message.contains("clone backups"),
        "{error}"
    );
    assert!(stop.move_id.is_none(), "no move began");
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
    assert!(unchanged(&lab));
}

/// R2 needs you: a profile started by hand mid-move is named with its
/// door, and the move names itself.
#[test]
fn a_profile_started_mid_move_needs_you_and_is_named() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let system = restarted_target(&lab);
    let error = run_once_more(&lab.state, &lab.sides, &system).unwrap_err();
    let (class, retry, stop) = read(&error);
    assert_eq!((class, retry), (StopClass::NeedsYou, false));
    assert_eq!(stop.holders, vec![holder(&lab, &lab.sides.target_place)]);
    assert_eq!(
        stop.move_id.as_deref(),
        Some(moves(&lab.state)[0].move_id.as_str())
    );
}

/// R2 transient: another move of these profiles is running.
#[test]
fn another_move_running_is_transient() {
    clones_on_any_disk();
    let lab = lab();
    let ids = place_ids(&lab);
    let _other = crate::features::places::MoveHold::take(&lab.state, &ids, "other").unwrap();
    let system = running_both(&lab);
    let error = run(&lab.state, &lab.sides, &system).unwrap_err();
    let (class, retry, _) = read(&typed(error, StopClass::NeedsYou));
    assert_eq!((class, retry), (StopClass::Transient, true));
    assert!(system.quits.lock().unwrap().is_empty());
}

/// R5: Undo says what of the profiles kept running when they quit, as
/// the move itself does.
#[test]
fn undo_keeps_what_kept_running() {
    let Some((lab, system, done)) = moved() else {
        return;
    };
    drop(system);
    let server = Scripted {
        kids: vec![kid(&lab, 711, "node", &["server.js"], true)],
        ..running_both(&lab)
    };
    let undone = undo_with(&lab.state, &done.move_id, &server).unwrap();
    assert!(
        undone.notes.iter().any(|n| n.starts_with("node that ")
            && n.ends_with("is still running; the move doesn't need it closed")),
        "{:?}",
        undone.notes
    );
}
