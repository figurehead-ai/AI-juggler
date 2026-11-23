//! AJ-E2321: a move a crash left part way, over a real app state, real
//! fixture folders and a scripted system: settled at start before keep
//! alive, or held with its door (Finish or Undo by its id).

use super::*;
use crate::features::handover::moving::recover::{self, Settle};

/// Where a crash stopped the move.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Crash {
    /// Backed up, nothing applied.
    BackedUp,
    /// Mid-apply: the last step happened, its done line never written.
    MidApply,
    /// After the cold checks, before the apps opened again.
    Verified,
}

/// A move of the lab's conversation stopped by a crash at `at`; `held`:
/// its markers name an owner that is gone (else none: an older AI Juggler
/// left it). Returns its id.
fn crashed(lab: &Lab, at: Crash, held: bool) -> String {
    let plan = plan_of(&lab.sides).unwrap();
    let ids: Vec<String> = [&lab.sides.source_place, &lab.sides.target_place]
        .into_iter()
        .flatten()
        .map(|(id, _)| id.clone())
        .collect();
    let id = "1700000000000-crash".to_owned();
    let mut txn = Transaction::begin(
        &moves_dir(&lab.state),
        &id,
        "claude-code",
        lab.sides.request.clone(),
        plan.clone(),
        None,
        1_700_000_000_000,
    )
    .unwrap();
    txn.remember_places(ids.clone(), lab.sides.accounts.clone())
        .unwrap();
    txn.back_up(&Stopped::checked()).unwrap();
    if at != Crash::BackedUp {
        txn.apply(&Stopped::checked(), 0).unwrap();
    }
    if at == Crash::Verified {
        let mover = lab.sides.adapter.in_place_move().unwrap();
        let cold = txn
            .verify_cold(mover.verify(&lab.sides.request, &plan))
            .unwrap();
        assert!(cold.passed(), "{:?}", cold.failures());
    }
    if at == Crash::MidApply {
        // The crash: the manifest still says applying, the last step's
        // done line (and the phase after) never written.
        let manifest = txn.dir().join("manifest.json");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(
            &manifest,
            text.replace("\"phase\": \"applied\"", "\"phase\": \"applying\""),
        )
        .unwrap();
        let journal = txn.dir().join("journal.jsonl");
        let text = std::fs::read_to_string(&journal).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        std::fs::write(
            &journal,
            format!("{}\n", lines[..lines.len() - 2].join("\n")),
        )
        .unwrap();
    }
    if held {
        let area = hold::area_at(&lab.state.data_dir);
        std::fs::create_dir_all(area.join("held")).unwrap();
        for place in &ids {
            let marker = hold::Marker {
                move_id: id.clone(),
                place: place.clone(),
                owner_pid: 999_999,
                owner_start_micros: 1,
            };
            std::fs::write(
                area.join("held").join(hold::key(place)),
                serde_json::to_vec(&marker).unwrap(),
            )
            .unwrap();
        }
    }
    id
}

fn phase(lab: &Lab, id: &str) -> Phase {
    hold::open_move(&lab.state.data_dir, id)
        .unwrap()
        .manifest
        .phase
}

fn held_any(lab: &Lab) -> bool {
    [&lab.sides.source_place, &lab.sides.target_place]
        .into_iter()
        .flatten()
        .any(|(id, _)| crate::features::places::held_for_move(&lab.state, id))
}

#[test]
fn a_move_a_crash_left_mid_apply_is_undone_at_start_before_keep_alive() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::MidApply, true);
    assert!(!unchanged(&lab), "the crash left it half moved");
    assert!(
        held_any(&lab),
        "keep alive leaves both closed until recovery"
    );
    let claimed = recover::claim(&lab.state);
    assert_eq!(claimed, [(id.clone(), Settle::RollBack)]);
    let system = Scripted::default();
    recover::settle(&lab.state, &claimed, &system);
    assert_eq!(phase(&lab, &id), Phase::Undone);
    assert!(unchanged(&lab), "back where it was");
    assert!(!held_any(&lab), "keep alive watches both again");
    assert!(
        system.sent.lock().unwrap().is_empty(),
        "recovery opens nothing"
    );
}

#[test]
fn a_move_a_crash_left_after_its_checks_is_finished_at_start() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::Verified, true);
    let claimed = recover::claim(&lab.state);
    assert_eq!(claimed, [(id.clone(), Settle::Finish)]);
    recover::settle(&lab.state, &claimed, &Scripted::default());
    assert_eq!(phase(&lab, &id), Phase::Committed, "rolled forward");
    assert!(lab.b_folder.join("local_main.json").is_file());
    assert!(!held_any(&lab));
}

#[test]
fn a_backed_up_move_a_crash_left_is_let_go_without_quitting_anything() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::BackedUp, true);
    let system = running_both(&lab);
    let claimed = recover::claim(&lab.state);
    assert_eq!(claimed, [(id.clone(), Settle::LetGo)]);
    recover::settle(&lab.state, &claimed, &system);
    assert_eq!(phase(&lab, &id), Phase::Undone);
    assert!(system.quits.lock().unwrap().is_empty(), "nothing quit");
    assert!(unchanged(&lab));
    assert!(!held_any(&lab));
    assert!(backups(&lab).is_empty(), "its backups let go");
}

#[test]
fn a_move_an_older_ai_juggler_left_part_way_is_held_for_the_persons_finish_or_undo() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::MidApply, false);
    assert!(!held_any(&lab), "no marker: an older AI Juggler's");
    let claimed = recover::claim(&lab.state);
    assert!(matches!(&claimed[..], [(one, Settle::Ask(_))] if *one == id));
    recover::settle(&lab.state, &claimed, &Scripted::default());
    assert!(held_any(&lab), "held now, before keep alive's first tick");
    assert_eq!(phase(&lab, &id), Phase::Applying, "never settled silently");
    // Its door: by its id, Undo (Finish isn't offered: it stopped mid-apply).
    let door = moves(&lab.state);
    let row = door.iter().find(|m| m.move_id == id).unwrap();
    assert_eq!(row.state, "interrupted");
    assert!(row.can_undo && !row.can_finish, "{row:?}");
    assert!(row.timeline.iter().any(|line| line.starts_with("intent")));
    assert!(recover::finish(&lab.state, &id).is_err());
    undo_with(&lab.state, &id, &Scripted::default()).unwrap();
    assert!(unchanged(&lab));
    assert!(!held_any(&lab));
}

#[test]
fn a_move_another_ai_juggler_is_running_is_left_alone() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::MidApply, true);
    let roots = hold::open_move(&lab.state.data_dir, &id)
        .unwrap()
        .manifest
        .plan
        .roots;
    let _theirs = lock(&lab.state, &roots).unwrap();
    assert!(recover::claim(&lab.state).is_empty());
    assert_eq!(phase(&lab, &id), Phase::Applying);
}

#[test]
fn the_door_finishes_an_interrupted_move_by_its_id() {
    if !clones() {
        return;
    }
    let lab = lab();
    let id = crashed(&lab, Crash::Verified, true);
    let row = moves(&lab.state)
        .into_iter()
        .find(|m| m.move_id == id)
        .unwrap();
    assert!(row.can_finish && row.can_undo, "{row:?}");
    assert_eq!(row.title.as_deref(), Some("T"));
    assert!(recover::finish(&lab.state, "not/an id").is_err());
    let said = recover::finish(&lab.state, &id).unwrap();
    assert!(said.words.starts_with("Finished"), "{said:?}");
    assert_eq!(phase(&lab, &id), Phase::Committed);
    assert!(!held_any(&lab));
    assert!(
        recover::finish(&lab.state, &id).is_err(),
        "finished already"
    );
}

/// The coordinator's ruling (2026-11-23): retention is AJ-E2325's policy
/// alone. A second move of the same family, and the start's recovery and
/// sweep after it, leave the first move's backups (its Undo) in place.
#[test]
fn a_second_move_of_the_same_family_leaves_the_first_moves_backups() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = running_both(&lab);
    let first = run(&lab.state, &lab.sides, &system).unwrap();
    let first_backups = || {
        backups(&lab)
            .into_iter()
            .filter(|path| path.to_string_lossy().contains(&first.move_id))
            .count()
    };
    let kept = first_backups();
    assert!(kept > 0, "a committed move keeps its backups for Undo");
    let back = Sides {
        request: MoveRequest {
            source: lab.sides.request.target.clone(),
            target: lab.sides.request.source.clone(),
            ..lab.sides.request.clone()
        },
        source_place: lab.sides.target_place.clone(),
        target_place: lab.sides.source_place.clone(),
        adapter: lab.sides.adapter.clone(),
        tool: lab.sides.tool.clone(),
        source_label: "B".into(),
        target_label: "A".into(),
        source_live: false,
        accounts: lab.sides.accounts.clone(),
        stop: None,
    };
    run(&lab.state, &back, &system).unwrap();
    let claimed = recover::claim(&lab.state);
    assert!(claimed.is_empty(), "both moves are settled");
    recover::startup_settle(&lab.state, &claimed, &system);
    assert_eq!(first_backups(), kept, "the first move's backups stay");
    let row = moves(&lab.state)
        .into_iter()
        .find(|m| m.move_id == first.move_id)
        .unwrap();
    assert!(
        row.undo_why
            .as_deref()
            .is_some_and(|why| why.contains("moved again")),
        "{row:?}"
    );
}
