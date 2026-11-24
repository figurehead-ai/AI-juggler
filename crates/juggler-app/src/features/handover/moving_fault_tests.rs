//! AJ-E2323 R2 (the review's D2 and D3): a move cut short at every phase
//! and every step, then AI Juggler's start (AJ-E2321's recovery); two AI
//! Juggler data folders sharing one move area (AJ-E2320's hold); and the
//! app-level cases: a second AI Juggler's keep alive, something that
//! starts a profile again, an app asking at quit, the parallel quit and a
//! start with a held profile. A real app state, fixture folders and a
//! scripted system: nothing is quit, started or read from this Mac.

use juggler_adapter::moving::MoveStep;
use juggler_handover::moving::Recovery;

use super::*;
use crate::features::handover::moving::recover::{self, Settle};
use crate::features::places::{self as places, fakes::Recorder};

const ID: &str = "1700000000000-fault";

/// Where a crash stopped the move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum At {
    Planned,
    BackedUp,
    /// Step `n`'s intent written, the step not done.
    Intent(usize),
    /// Step `n` done, its done line never written.
    Acted(usize),
    Applied,
    Verified,
    /// Undo began (its phase written), nothing undone yet.
    Undoing,
}

fn both_ids(lab: &Lab) -> Vec<String> {
    [&lab.sides.source_place, &lab.sides.target_place]
        .into_iter()
        .flatten()
        .map(|(id, _)| id.clone())
        .collect()
}

/// Makes `step` stop when it runs (its first path briefly not as it was
/// backed up); returns how to put it back.
fn disturb(step: &MoveStep) -> Box<dyn FnOnce()> {
    let path = step.paths()[0].to_path_buf();
    if std::fs::symlink_metadata(&path).is_ok() {
        let aside = path.with_extension("aside-for-test");
        std::fs::rename(&path, &aside).unwrap();
        Box::new(move || std::fs::rename(&aside, &path).unwrap())
    } else {
        std::fs::write(&path, b"in the way").unwrap();
        Box::new(move || std::fs::remove_file(&path).unwrap())
    }
}

/// The journal without its last `n` lines (a crash before they were
/// written).
fn cut_journal(txn: &Transaction, n: usize) {
    let path = txn.dir().join("journal.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    std::fs::write(&path, format!("{}\n", lines[..lines.len() - n].join("\n"))).unwrap();
}

fn set_phase_on_disk(txn: &Transaction, from: &str, to: &str) {
    let path = txn.dir().join("manifest.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let from = format!("\"phase\": \"{from}\"");
    assert!(text.contains(&from), "{text}");
    std::fs::write(&path, text.replace(&from, &format!("\"phase\": \"{to}\""))).unwrap();
}

/// The lab's move stopped by a crash `at`, its profiles held by markers
/// whose AI Juggler is gone.
fn crashed_at(lab: &Lab, at: At) {
    let plan = plan_of(&lab.sides).unwrap();
    let steps = plan.steps.clone();
    let mut txn = Transaction::begin(
        &moves_dir(&lab.state),
        ID,
        "claude-code",
        lab.sides.request.clone(),
        plan.clone(),
        None,
        // Now: a finished move's backups keep their days (AJ-E2325).
        crate::commands::now_millis(),
    )
    .unwrap();
    txn.remember_places(both_ids(lab), lab.sides.accounts.clone())
        .unwrap();
    let stopped = Stopped::checked();
    if at != At::Planned {
        txn.back_up(&stopped).unwrap();
    }
    let last = steps.len() - 1;
    match at {
        At::Planned | At::BackedUp => {}
        At::Intent(n) => {
            let put_back = disturb(&steps[n]);
            assert!(txn.apply(&stopped, 0).is_err());
            put_back();
        }
        At::Acted(n) if n < last => {
            let put_back = disturb(&steps[n + 1]);
            assert!(txn.apply(&stopped, 0).is_err());
            put_back();
            // Step n's done line and step n+1's intent never written.
            cut_journal(&txn, 2);
        }
        At::Acted(_) => {
            txn.apply(&stopped, 0).unwrap();
            // The last step's done line and the phase after never written.
            set_phase_on_disk(&txn, "applied", "applying");
            cut_journal(&txn, 2);
        }
        At::Applied | At::Undoing => txn.apply(&stopped, 0).unwrap(),
        At::Verified => {
            txn.apply(&stopped, 0).unwrap();
            let mover = lab.sides.adapter.in_place_move().unwrap();
            let cold = txn
                .verify_cold(mover.verify(&lab.sides.request, &plan))
                .unwrap();
            assert!(cold.passed(), "{:?}", cold.failures());
        }
    }
    if at == At::Undoing {
        set_phase_on_disk(&txn, "applied", "undoing");
    }
    // What the journal and the disk say together, read back.
    let read = Transaction::open(txn.dir()).unwrap().recover().unwrap();
    let expected = match at {
        At::Planned => Recovery::Settled(Phase::Planned),
        At::BackedUp => Recovery::Settled(Phase::BackedUp),
        At::Intent(n) => Recovery::Applying { next: n },
        At::Acted(n) => Recovery::Applying { next: n + 1 },
        At::Applied => Recovery::Settled(Phase::Applied),
        At::Verified => Recovery::Settled(Phase::Verified),
        At::Undoing => Recovery::Undoing,
    };
    assert_eq!(read, expected, "{at:?}");
    let area = hold::area_at(&lab.state.data_dir);
    std::fs::create_dir_all(area.join("held")).unwrap();
    for place in both_ids(lab) {
        let marker = hold::Marker {
            move_id: ID.into(),
            place: place.clone(),
            owner_pid: 999_999,
            owner_start_micros: 1,
        };
        std::fs::write(
            area.join("held").join(hold::key(&place)),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
    }
}

/// The move's phase; `None`: its folder went (a move that changed
/// nothing, let go: AJ-E2325's refused attempt).
fn phase_of(lab: &Lab) -> Option<Phase> {
    hold::open_move(&lab.state.data_dir, ID)
        .ok()
        .map(|txn| txn.manifest.phase)
}

/// The move's backups left on disk.
fn backups_left(lab: &Lab) -> usize {
    hold::move_folder(&lab.state.data_dir, ID)
        .and_then(|dir| std::fs::read_dir(dir.join("backups")).ok())
        .map_or(0, Iterator::count)
}

/// D2: a crash after each phase and after each step's intent (and each
/// step's act), then AI Juggler's start: nothing changed → let go; files
/// changed under its hold → undone by the journal; past its checks →
/// finished. Every hold goes, nothing is opened, the backups as AJ-E2321
/// and AJ-E2325 say (an undone move's go, a finished move's stay).
#[test]
fn a_crash_after_each_phase_and_each_step_is_settled_at_start() {
    if !clones() {
        return;
    }
    let steps = plan_of(&lab().sides).unwrap().steps.len();
    let mut cases = vec![At::Planned, At::BackedUp];
    for n in 0..steps {
        cases.extend([At::Intent(n), At::Acted(n)]);
    }
    cases.extend([At::Applied, At::Verified, At::Undoing]);
    for at in cases {
        let lab = lab();
        crashed_at(&lab, at);
        let expected = match at {
            At::Planned | At::BackedUp => Settle::LetGo,
            At::Verified => Settle::Finish,
            _ => Settle::RollBack,
        };
        let claimed = recover::claim(&lab.state);
        assert_eq!(claimed, [(ID.to_owned(), expected.clone())], "{at:?}");
        let system = Scripted::default();
        recover::startup_settle(&lab.state, &claimed, &system);
        if expected == Settle::Finish {
            assert_eq!(
                phase_of(&lab),
                Some(Phase::Committed),
                "{at:?}: rolled forward"
            );
            assert!(lab.b_folder.join("local_main.json").is_file());
            assert!(backups_left(&lab) > 0, "{at:?}: kept for Undo");
        } else if expected == Settle::LetGo {
            let phase = phase_of(&lab);
            assert!(
                matches!(phase, None | Some(Phase::Undone)),
                "{at:?}: {phase:?}"
            );
            assert!(unchanged(&lab), "{at:?}: no file touched");
            assert_eq!(backups_left(&lab), 0, "{at:?}: its backups let go");
        } else {
            assert_eq!(
                phase_of(&lab),
                Some(Phase::Undone),
                "{at:?}: its record stays"
            );
            assert!(unchanged(&lab), "{at:?}: back where it was");
            assert_eq!(backups_left(&lab), 0, "{at:?}: nothing left to restore");
        }
        assert!(
            !hold::any_held(&hold::area_at(&lab.state.data_dir)),
            "{at:?}: keep alive watches both again"
        );
        assert!(
            system.sent.lock().unwrap().is_empty(),
            "{at:?}: opens nothing"
        );
        assert!(
            system.quits.lock().unwrap().is_empty(),
            "{at:?}: quits nothing"
        );
    }
    // A committed move is nobody's at start.
    let lab = lab();
    run(&lab.state, &lab.sides, &running_both(&lab)).unwrap();
    assert!(recover::claim(&lab.state).is_empty());
}

/// A second AI Juggler data folder whose move area is `lab`'s (as every AI
/// Juggler on a Mac shares one), on a made-up Mac.
fn second_juggler(lab: &Lab) -> (tempfile::TempDir, Arc<AppState>) {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    std::fs::create_dir_all(&data).unwrap();
    let area = hold::area_at(&lab.state.data_dir);
    std::fs::create_dir_all(&area).unwrap();
    std::os::unix::fs::symlink(&area, data.join("shared-moves")).unwrap();
    let state = places::fakes::on_made_up_mac(AppState::init(data).unwrap(), tmp.path());
    assert_eq!(
        std::fs::canonicalize(hold::area_at(&state.data_dir)).unwrap(),
        std::fs::canonicalize(&area).unwrap()
    );
    (tmp, state)
}

/// Ticks `state`'s keep alive for `ms` from `from`; returns the clock.
fn keep_alive_for(state: &Arc<AppState>, from: i64, ms: i64, starter: &Recorder) -> i64 {
    let mut now = from;
    while now < from + ms {
        now += 2_000;
        places::tick_with(state, now, starter, &|| false);
    }
    now
}

/// D2: two AI Juggler data folders sharing one move area: the second's
/// move of the same files (back again) blocks Undo of the first's.
#[test]
fn a_later_move_by_another_data_folder_blocks_undo_of_the_earlier_one() {
    if !clones() {
        return;
    }
    let lab = lab();
    let (_tmp, other) = second_juggler(&lab);
    let first = run(&lab.state, &lab.sides, &running_both(&lab)).unwrap();
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
    run(&other, &back, &running_both(&lab)).unwrap();
    assert!(
        hold::all_moves(&lab.state.data_dir).len() >= 2,
        "the first data folder sees the second's move"
    );
    let error = undo_with(&lab.state, &first.move_id, &running_both(&lab))
        .unwrap_err()
        .to_string();
    assert!(error.contains("later move"), "{error}");
    assert!(unchanged(&lab), "the second move put it back; Undo left it");
}

/// D3: keep alive in a second AI Juggler (another data folder, the same
/// move area) wants the source back while a move holds it: it starts
/// nothing until the hold goes, by this AI Juggler's move or another
/// one's.
#[test]
fn a_second_ai_jugglers_keep_alive_starts_nothing_a_move_holds() {
    let lab = lab();
    let (_tmp, other) = second_juggler(&lab);
    let root = lab.a_code.parent().unwrap().parent().unwrap().to_path_buf();
    let (id, _, _) = places::remember_test_profile(&other, &root, "b-a", None);
    assert_eq!(id, source_id(&lab), "the same profile in both");
    places::set_keep_alive(&other, &id, true).unwrap();
    let starter = Recorder::default();
    let start = 10_000_000;
    other.places.watch.lock().unwrap().settled_at(start);
    {
        // This AI Juggler's move, before its first quit.
        let _hold = places::MoveHold::take(&lab.state, &both_ids(&lab), "m-here").unwrap();
        assert!(places::held_for_move(&other, &id));
        keep_alive_for(&other, start, places::keep::WAIT_MS + 10_000, &starter);
        assert!(starter.starts.lock().unwrap().is_empty(), "held: no start");
    }
    assert!(
        !places::held_for_move(&other, &id),
        "a move never begun lets go"
    );
    let now = keep_alive_for(
        &other,
        start + places::keep::WAIT_MS + 10_000,
        places::keep::WAIT_MS + 10_000,
        &starter,
    );
    assert!(
        !starter.starts.lock().unwrap().is_empty(),
        "free again: the hold was the reason"
    );
    // Another running AI Juggler's move (its marker, a live process that
    // isn't this one).
    let starter = Recorder::default();
    let parent = std::os::unix::process::parent_id();
    let owner = *crate::features::ownership::signal::identify(&[parent])
        .get(&parent)
        .expect("the parent runs");
    let area = hold::area_at(&lab.state.data_dir);
    std::fs::create_dir_all(area.join("held")).unwrap();
    let marker = hold::Marker {
        move_id: "m-theirs".into(),
        place: id.clone(),
        owner_pid: owner.pid,
        owner_start_micros: u64::try_from(owner.start_micros).unwrap(),
    };
    std::fs::write(
        area.join("held").join(hold::key(&id)),
        serde_json::to_vec(&marker).unwrap(),
    )
    .unwrap();
    keep_alive_for(&other, now, 2 * places::keep::WAIT_MS + 20_000, &starter);
    assert!(
        starter.starts.lock().unwrap().is_empty(),
        "theirs holds too"
    );
}

fn stop_of(error: AppError) -> crate::error::MoveStop {
    match error {
        AppError::Move(stop) => *stop,
        other => panic!("not a move's stop: {other}"),
    }
}

/// D3: something starts the profile again as soon as it quits (a launch
/// agent, the person from the Dock): the person's to clear (named, its
/// door), never tried again by itself, nothing moved.
#[test]
fn a_profile_something_restarts_is_the_persons_with_no_retry() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = Scripted {
        restarted_by_user: vec![source_id(&lab)],
        ..running_both(&lab)
    };
    let stop = stop_of(run_once_more(&lab.state, &lab.sides, &system).unwrap_err());
    assert_eq!(stop.class, StopClass::NeedsYou, "{}", stop.words);
    assert!(!stop.retryable());
    assert!(!stop.holders.is_empty(), "it names the profile");
    assert_eq!(
        system.quits.lock().unwrap().len(),
        2,
        "one try: {:?}",
        system.quits
    );
    assert!(unchanged(&lab));
}

/// D3: an app asking something at quit (it never ends): the person's,
/// no Retry, the other profile started again, nothing moved.
#[test]
fn an_app_asking_at_quit_is_the_persons_and_the_other_profile_starts_again() {
    if !clones() {
        return;
    }
    let lab = lab();
    let target = lab.sides.target_place.as_ref().unwrap().0.clone();
    let system = Scripted {
        stubborn: vec![target.clone()],
        ..running_both(&lab)
    };
    let stop = stop_of(run_once_more(&lab.state, &lab.sides, &system).unwrap_err());
    assert_eq!(stop.class, StopClass::NeedsYou, "{}", stop.words);
    assert!(!stop.retryable());
    assert!(stop.unchanged);
    assert_eq!(system.quits.lock().unwrap().len(), 2, "one try");
    let starts = system.starts.lock().unwrap().clone();
    assert!(starts.contains(&source_id(&lab)), "{starts:?}");
    assert!(
        !starts.contains(&target),
        "the one still open isn't started"
    );
    assert!(unchanged(&lab));
}

/// D3: both profiles take a while to quit: they are asked together, so
/// the stop takes one wait, not two (the review's 3 s each → ≤ 3.5 s,
/// here 1 s each → under 1.9 s).
#[test]
fn both_profiles_quit_in_parallel() {
    if !clones() {
        return;
    }
    let lab = lab();
    let system = Scripted {
        slow: Some(Duration::from_secs(1)),
        wait: Some(Duration::from_secs(5)),
        ..running_both(&lab)
    };
    let began = std::time::Instant::now();
    run(&lab.state, &lab.sides, &system).unwrap();
    let took = began.elapsed();
    assert_eq!(
        *system.running_at_quit.lock().unwrap(),
        [2, 2],
        "both asked while both ran"
    );
    assert!(took >= Duration::from_secs(1), "{took:?}");
    assert!(took < Duration::from_millis(1_900), "{took:?}");
}

/// D3: AI Juggler starts with a profile a crashed move holds and its
/// journal interrupted: keep alive's ticks start nothing until recovery
/// has settled it, and the Moves door lists it meanwhile.
#[test]
fn a_start_with_a_held_profile_waits_for_recovery_before_keep_alive() {
    if !clones() {
        return;
    }
    let lab = lab();
    let root = lab.a_code.parent().unwrap().parent().unwrap().to_path_buf();
    let _ = lab.state.places.made_up.set(places::MadeUpMac {
        table: crate::features::conversations::procs::some(&[std::process::id()]),
        home: root.join("home"),
        installs: Some(Vec::new()),
    });
    let id = source_id(&lab);
    places::set_keep_alive(&lab.state, &id, true).unwrap();
    let steps = plan_of(&lab.sides).unwrap().steps.len();
    crashed_at(&lab, At::Acted(steps - 1));
    // The start: claimed before keep alive's first tick.
    let claimed = recover::claim(&lab.state);
    assert_eq!(claimed, [(ID.to_owned(), Settle::RollBack)]);
    let starter = Recorder::default();
    let start = 10_000_000;
    lab.state.places.watch.lock().unwrap().settled_at(start);
    let now = keep_alive_for(&lab.state, start, places::keep::WAIT_MS + 10_000, &starter);
    assert!(starter.starts.lock().unwrap().is_empty(), "held: no start");
    let door = moves(&lab.state);
    assert!(door.iter().any(|m| m.move_id == ID), "{door:?}");
    // Recovery done: keep alive has it again.
    recover::startup_settle(&lab.state, &claimed, &Scripted::default());
    assert!(unchanged(&lab));
    keep_alive_for(&lab.state, now, places::keep::WAIT_MS + 10_000, &starter);
    assert!(
        !starter.starts.lock().unwrap().is_empty(),
        "free again after recovery"
    );
}
