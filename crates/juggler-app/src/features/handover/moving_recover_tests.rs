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
    let ids = place_ids(lab);
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some(lab) = cloning_lab() else {
        return;
    };
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
    let Some((lab, system, first)) = moved() else {
        return;
    };
    let first_backups = || {
        backups(&lab)
            .into_iter()
            .filter(|path| path.to_string_lossy().contains(&first.move_id))
            .count()
    };
    let kept = first_backups();
    assert!(kept > 0, "a committed move keeps its backups for Undo");
    let back = moved_back(&lab);
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

// AJ-E2325: the backups' policy.
use crate::features::handover::moving::retention::{self, FIRST_SWEEP_KEY, KEEP_KEY, Keep, Swept};

const DAY: i64 = 24 * 60 * 60 * 1000;

fn set(lab: &Lab, key: &str, value: &str) {
    lab.state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(key, value)
        .unwrap();
}

fn move_dirs(lab: &Lab) -> usize {
    std::fs::read_dir(moves_dir(&lab.state)).map_or(0, |d| d.count())
}

#[test]
fn the_setting_reads_its_four_choices_seven_days_by_default() {
    assert_eq!(retention::keep_of(None), Keep::Days(7));
    assert_eq!(retention::keep_of(Some("1")), Keep::Days(1));
    assert_eq!(retention::keep_of(Some("30")), Keep::Days(30));
    assert_eq!(retention::keep_of(Some("forever")), Keep::UntilDeleted);
    assert_eq!(retention::keep_of(Some("junk")), Keep::Days(7));
    // R7: a backup older than the first sweep keeps the default's 7 days.
    assert_eq!(retention::kept_until(0, Keep::Days(1), 10), Some(7 * DAY));
    assert_eq!(retention::kept_until(20, Keep::Days(1), 10), Some(20 + DAY));
    assert_eq!(retention::kept_until(0, Keep::UntilDeleted, 10), None);
}

#[test]
fn a_refused_attempts_folder_goes_at_once_and_an_undone_moves_record_stays() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let id = crashed(&lab, Crash::BackedUp, false);
    let mut txn = hold::open_move(&lab.state.data_dir, &id).unwrap();
    txn.undo(&Stopped::checked(), 1).unwrap();
    assert_eq!(move_dirs(&lab), 1);
    assert_eq!(retention::drop_refused(&lab.state), 1);
    assert_eq!(move_dirs(&lab), 0, "nothing changed, nothing to undo");
    // One undone after it changed files keeps its record (and work kept aside).
    let Some((lab, system, done)) = moved() else {
        return;
    };
    undo_with(&lab.state, &done.move_id, &system).unwrap();
    assert_eq!(retention::drop_refused(&lab.state), 0);
    assert!(moves(&lab.state).iter().any(|m| m.move_id == done.move_id));
}

#[test]
fn a_committed_moves_backups_go_after_the_chosen_days_never_before() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    // The first sweep ran before the move: its own time counts.
    set(&lab, FIRST_SWEEP_KEY, &(created - 1).to_string());
    set(&lab, KEEP_KEY, "1");
    assert_eq!(
        retention::sweep(&lab.state, created + DAY / 2),
        Swept::default()
    );
    assert!(!backups(&lab).is_empty(), "kept for its day");
    assert_eq!(retention::sweep(&lab.state, created + DAY + 1).retired, 1);
    assert!(backups(&lab).is_empty());
}

#[test]
fn backups_made_before_the_first_sweep_keep_seven_days_and_until_deleted_keeps_them() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    set(&lab, KEEP_KEY, "1");
    // The first sweep, two days after the move (a 0.5.0 backup).
    assert_eq!(retention::sweep(&lab.state, created + 2 * DAY).retired, 0);
    assert_eq!(retention::sweep(&lab.state, created + 6 * DAY).retired, 0);
    set(&lab, KEEP_KEY, "forever");
    assert_eq!(retention::sweep(&lab.state, created + 400 * DAY).retired, 0);
    assert!(!backups(&lab).is_empty());
    set(&lab, KEEP_KEY, "7");
    assert_eq!(
        retention::sweep(&lab.state, created + 7 * DAY + 1).retired,
        1
    );
}

#[test]
fn the_sweep_never_runs_during_a_move_nor_touches_an_interrupted_one() {
    let Some(lab) = cloning_lab() else {
        return;
    };
    let id = crashed(&lab, Crash::MidApply, true);
    let later = juggler_core::model::Timestamp::now().millis() + 400 * DAY;
    assert_eq!(retention::sweep(&lab.state, later), Swept::default());
    assert!(
        !backups(&lab).is_empty(),
        "an interrupted move keeps its own"
    );
    assert_eq!(phase(&lab, &id), Phase::Applying);
    // A move of this AI Juggler running: nothing at all.
    let Some((lab, _system, _done)) = moved() else {
        return;
    };
    let ids = vec![source_id(&lab)];
    let _running = crate::features::places::MoveHold::take(&lab.state, &ids, "running").unwrap();
    assert_eq!(retention::sweep(&lab.state, later), Swept::default());
    assert!(!backups(&lab).is_empty());
}

#[test]
fn delete_backup_by_its_id_ends_undo_and_never_takes_an_interrupted_ones() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let row = |lab: &Lab, id: &str| {
        moves(&lab.state)
            .into_iter()
            .find(|m| m.move_id == id)
            .unwrap()
    };
    let before = row(&lab, &done.move_id);
    assert!(before.has_backup && before.can_undo, "{before:?}");
    assert!(
        before.kept_until_ms.is_some(),
        "kept until a date by default"
    );
    let said = retention::delete_backup(&lab.state, &done.move_id).unwrap();
    assert!(said.contains("can no longer be undone"), "{said}");
    let after = row(&lab, &done.move_id);
    assert!(
        !after.has_backup && !after.can_undo && after.undo_why.is_some(),
        "{after:?}"
    );
    let other = super::lab();
    let id = crashed(&other, Crash::Verified, true);
    assert!(retention::delete_backup(&other.state, &id).is_err());
    assert!(retention::delete_backup(&other.state, "../x").is_err());
}

// AJ-E2325 R3–R6: sizes, the total, the low-disk offer and its clear.

fn created_of(lab: &Lab, id: &str) -> i64 {
    hold::open_move(&lab.state.data_dir, id)
        .unwrap()
        .manifest
        .created_ms
}

fn offers(lab: &Lab) -> Vec<juggler_core::storage::Notice> {
    let storage = lab.state.storage.lock().unwrap();
    let all = storage.notices().earlier(None, 50).unwrap();
    all.into_iter()
        .filter(|n| n.kind == "move-backups")
        .collect()
}

#[test]
fn each_backup_and_the_total_say_what_deleting_frees() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    let row = moves(&lab.state)
        .into_iter()
        .find(|m| m.move_id == done.move_id)
        .unwrap();
    let bytes = row.backup_bytes.expect("a kept backup has its size");
    // D-146: exact, also where its backups are clones (APFS measures what
    // they alone hold: an untouched clone frees nothing).
    assert!(row.backup_exact, "{row:?}");
    let total = retention::backups(&lab.state, created + DAY / 2);
    assert_eq!((total.count, total.bytes, total.exact), (1, bytes, true));
    assert_eq!(
        (total.old_count, total.old_bytes),
        (0, 0),
        "not a day old yet"
    );
    let later = retention::backups(&lab.state, created + DAY);
    assert_eq!((later.old_count, later.old_bytes), (1, bytes));
    assert_eq!(retention::frees_words(2_500_000_000, true), "2.3 GB");
    assert_eq!(retention::frees_words(2_500_000_000, false), "up to 2.3 GB");
    // Deleted: no size, out of the total.
    retention::delete_backup(&lab.state, &done.move_id).unwrap();
    let row = moves(&lab.state)
        .into_iter()
        .find(|m| m.move_id == done.move_id)
        .unwrap();
    assert_eq!(row.backup_bytes, None);
    assert_eq!(retention::backups(&lab.state, created + DAY).count, 0);
}

#[test]
fn a_nearly_full_disk_offers_once_a_day_naming_what_it_frees_and_clears_nothing() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    // F12: backups that free something (an untouched clone frees nothing).
    diverge(&lab);
    // Room enough: no offer.
    juggler_platform::room::set_free_space_on_this_thread(Some(|_| u64::MAX / 2));
    assert!(!retention::backups(&lab.state, created + 2 * DAY).low_room);
    assert!(!retention::offer_if_pressed(&lab.state, created + 2 * DAY));
    // Nearly full: the sweep offers (its keep time not up, nothing goes).
    juggler_platform::room::set_free_space_on_this_thread(Some(|_| 1024));
    assert!(retention::backups(&lab.state, created + 2 * DAY).low_room);
    assert_eq!(
        retention::sweep(&lab.state, created + 2 * DAY),
        Swept::default()
    );
    let made = offers(&lab);
    assert_eq!(made.len(), 1, "one offer");
    assert!(
        made[0].title.contains("older than a day free ") && !made[0].title.contains("up to"),
        "exact (D-146): {}",
        made[0].title
    );
    assert!(made[0].data.contains("\"moves\""), "{}", made[0].data);
    assert!(!backups(&lab).is_empty(), "an offer, never a clean-up");
    // Once a day.
    assert!(!retention::offer_if_pressed(
        &lab.state,
        created + 2 * DAY + 1
    ));
    // Never during a move.
    let ids = vec![source_id(&lab)];
    let running = crate::features::places::MoveHold::take(&lab.state, &ids, "running").unwrap();
    assert!(!retention::offer_if_pressed(&lab.state, created + 3 * DAY));
    drop(running);
    // A move refused for room offers too, with room on the disk again.
    juggler_platform::room::set_free_space_on_this_thread(Some(|_| u64::MAX / 2));
    retention::note_room_short(&lab.state, created + 4 * DAY);
    assert_eq!(offers(&lab).len(), 2);
    assert!(
        !retention::backups(&lab.state, created + 6 * DAY).low_room,
        "for a day"
    );
    juggler_platform::room::set_free_space_on_this_thread(None);
}

#[test]
fn clearing_old_backups_takes_only_finished_moves_a_day_old_never_during_a_move() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    let said = retention::clear_old(&lab.state, created + DAY / 2).unwrap();
    assert!(said.starts_with("No backups"), "{said}");
    assert!(!backups(&lab).is_empty());
    let ids = vec![source_id(&lab)];
    let running = crate::features::places::MoveHold::take(&lab.state, &ids, "running").unwrap();
    assert!(retention::clear_old(&lab.state, created + 2 * DAY).is_err());
    assert!(!backups(&lab).is_empty());
    drop(running);
    let said = retention::clear_old(&lab.state, created + 2 * DAY).unwrap();
    assert!(
        said.starts_with("Cleared 1 backup (") && !said.contains("up to"),
        "exact (D-146): {said}"
    );
    assert!(said.contains("can no longer be undone"), "{said}");
    assert!(backups(&lab).is_empty());
    // An interrupted move's backups are never cleared.
    let other = super::lab();
    let id = crashed(&other, Crash::MidApply, true);
    let said = retention::clear_old(&other.state, 1_700_000_000_000 + 400 * DAY).unwrap();
    assert!(said.starts_with("No backups"), "{said}");
    assert!(!backups(&other).is_empty());
    assert_eq!(phase(&other, &id), Phase::Applying);
}

/// The moved history written anew (new blocks): its backup, a clone,
/// no longer shares them, so deleting it frees something.
fn diverge(lab: &Lab) {
    let history = lab.b_code.join("projects").join(SLUG).join("main.jsonl");
    std::fs::write(&history, "{\"type\":\"user\"}\n".repeat(4096)).unwrap();
}

/// F12 (D-127): day-old backups that free nothing (untouched clones,
/// measured exactly under D-146) bring no offer, however full the disk;
/// once clearing them frees something, the offer shows.
#[test]
fn backups_that_free_nothing_bring_no_low_disk_offer() {
    let Some((lab, _system, done)) = moved() else {
        return;
    };
    let created = created_of(&lab, &done.move_id);
    juggler_platform::room::set_free_space_on_this_thread(Some(|_| 1024));
    let info = retention::backups(&lab.state, created + 2 * DAY);
    assert!(info.low_room && info.old_count == 1, "{info:?}");
    if info.old_bytes == 0 {
        assert!(!retention::offer_if_pressed(&lab.state, created + 2 * DAY));
        assert!(offers(&lab).is_empty(), "an offer that frees nothing");
    }
    diverge(&lab);
    assert!(retention::backups(&lab.state, created + 2 * DAY).old_bytes > 0);
    assert!(retention::offer_if_pressed(&lab.state, created + 2 * DAY));
    assert_eq!(offers(&lab).len(), 1);
    juggler_platform::room::set_free_space_on_this_thread(None);
}
