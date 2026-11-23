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

#[test]
fn a_committed_moves_backups_retire_after_thirty_days_and_its_undo_goes() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let row = |lab: &Lab| {
        moves(&lab.state)
            .into_iter()
            .find(|m| m.move_id == done.move_id)
            .unwrap()
    };
    assert!(row(&lab).can_undo);
    let now = juggler_core::model::Timestamp::now().millis();
    assert_eq!(recover::retire(&lab.state, now), 0, "a new move keeps them");
    let later = now + (recover::KEEP_DAYS + 1) * 24 * 60 * 60 * 1000;
    assert_eq!(recover::retire(&lab.state, later), 1);
    assert!(backups(&lab).is_empty());
    let after = row(&lab);
    assert!(!after.can_undo && after.undo_why.is_some(), "{after:?}");
}
