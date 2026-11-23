//! AJ-E2320: the shared, persistent hold marker (R1) and every start path
//! honouring it, a crashed move's too (R2). Fixtures only: a made-up Mac,
//! temp folders, a recorder in place of every start and quit.

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::moving::{MovePlan, MoveRequest, MoveSide, NotesChoice};
use juggler_handover::moving::{FORMAT, Manifest, VERSION};

use super::*;
use crate::features::places::fakes::{FakeDesktop, Recorder};
use crate::features::places::{self as places, MoveHold, QuitWait, RestartOf};
use crate::state::AppState;

fn lab() -> (tempfile::TempDir, Arc<AppState>, String) {
    let tmp = tempfile::tempdir().unwrap();
    let state =
        places::fakes::on_made_up_mac(AppState::init(tmp.path().join("data")).unwrap(), tmp.path());
    let (id, _, _) = places::remember_test_profile(&state, tmp.path(), "b-a", None);
    (tmp, state, id)
}

fn marker_path(state: &Arc<AppState>, id: &str) -> std::path::PathBuf {
    area_at(&state.data_dir).join("held").join(key(id))
}

/// Another AI Juggler's hold (its marker; this process's memory knows
/// nothing of it), owned by `owner`.
fn plant(state: &Arc<AppState>, id: &str, owner: ProcId, move_id: &str) {
    let path = marker_path(state, id);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let marker = Marker {
        move_id: move_id.into(),
        place: id.into(),
        owner_pid: owner.pid,
        owner_start_micros: u64::try_from(owner.start_micros).unwrap(),
    };
    std::fs::write(path, serde_json::to_vec(&marker).unwrap()).unwrap();
}

/// A live process that isn't this one: the test runner's parent.
fn another_live() -> ProcId {
    let pid = std::os::unix::process::parent_id();
    *signal::identify(&[pid]).get(&pid).expect("the parent runs")
}

/// A process gone: this pid, started at another time.
fn gone() -> ProcId {
    ProcId {
        pid: std::process::id(),
        start_micros: 1_000_000,
    }
}

/// Move `move_id`'s journal in the shared area, at `phase`.
fn journal(state: &Arc<AppState>, move_id: &str, phase: Phase) {
    let side = |dir: &Path| MoveSide {
        tool_dir: dir.to_path_buf(),
        host_dir: None,
        version: None,
    };
    let manifest = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        id: move_id.into(),
        tool: "claude-code".into(),
        created_ms: 1,
        request: MoveRequest {
            conversation: "s-1".into(),
            source: side(Path::new("/Users/someone/a")),
            target: side(Path::new("/Users/someone/b")),
            notes: NotesChoice::KeepTarget,
        },
        plan: MovePlan::default(),
        phase,
        entries: Vec::new(),
        previous: None,
        problem: None,
        places: Vec::new(),
        accounts: Vec::new(),
        preserved: None,
    };
    let dir = journals(&area_at(&state.data_dir)).join(move_id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

/// R1: a test's data folder gets its own area, never the Mac's.
#[test]
fn a_tests_data_folder_never_shares_the_macs_move_area() {
    let (tmp, state, _) = lab();
    assert!(area_at(&state.data_dir).starts_with(tmp.path()));
    if let (Some(base), Ok(own)) = (dirs::data_dir(), crate::state::resolve_data_dir()) {
        assert_eq!(area_at(&own), base.join(SHARED).join("moves"));
    }
}

/// R1: the hold is a marker in the shared area naming the move and its
/// owner (pid, start), not only this process's memory.
#[test]
fn a_hold_is_a_shared_marker_naming_the_move_and_its_owner() {
    let (_tmp, state, id) = lab();
    let hold = MoveHold::take(&state, std::slice::from_ref(&id), "m-1").unwrap();
    let marker: Marker =
        serde_json::from_slice(&std::fs::read(marker_path(&state, &id)).unwrap()).unwrap();
    assert_eq!(marker.move_id, "m-1");
    assert_eq!(marker.place, id);
    assert_eq!(marker.owner_pid, std::process::id());
    assert!(marker.mine() && marker.owner_alive());
    assert!(any_held(&area_at(&state.data_dir)));
    // Never begun (no journal): it goes with the hold.
    drop(hold);
    assert!(!marker_path(&state, &id).exists());
    assert!(!any_held(&area_at(&state.data_dir)));
}

/// R1: a move left part way keeps its profiles held after its hold ends;
/// committed or undone, the marker goes.
#[test]
fn the_marker_goes_only_when_the_move_is_committed_or_undone() {
    let (_tmp, state, id) = lab();
    journal(&state, "m-1", Phase::Applying);
    drop(MoveHold::take(&state, std::slice::from_ref(&id), "m-1").unwrap());
    assert!(marker_path(&state, &id).exists(), "part way: still held");
    assert_eq!(
        places::why_held(&state, &id),
        Some(super::super::HELD_UNFINISHED)
    );
    // Another move is refused; the same move's Undo takes it over.
    assert!(MoveHold::take(&state, std::slice::from_ref(&id), "m-2").is_err());
    let undo = MoveHold::take(&state, std::slice::from_ref(&id), "m-1").unwrap();
    journal(&state, "m-1", Phase::Undone);
    drop(undo);
    assert!(!marker_path(&state, &id).exists(), "undone: let go");

    journal(&state, "m-3", Phase::Committed);
    drop(MoveHold::take(&state, std::slice::from_ref(&id), "m-3").unwrap());
    assert!(!marker_path(&state, &id).exists(), "committed: let go");
}

/// R1: the locks are in the shared area, by the folders' real spelling.
#[test]
fn the_folder_locks_are_in_the_shared_area() {
    let (tmp, state, _) = lab();
    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    let dir = locks(&area_at(&state.data_dir));
    let held = juggler_handover::moving::Lock::take_in(&dir, std::slice::from_ref(&root)).unwrap();
    assert!(
        juggler_handover::moving::Lock::take_in(&dir, &[root.join("../root")]).is_err(),
        "a second spelling is the same lock"
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    drop(held);
}

/// R2: another AI Juggler's move holds the profile: keep alive, Open,
/// Restart, a start and the automation watch here all leave it alone.
#[test]
fn another_ai_jugglers_hold_is_honoured_by_every_start_path() {
    let (_tmp, state, id) = lab();
    places::set_keep_alive(&state, &id, true).unwrap();
    plant(&state, &id, another_live(), "m-other");
    assert!(places::held_for_move(&state, &id));
    assert_eq!(places::why_held(&state, &id), Some(super::super::HELD));
    assert!(places::any_move(&state), "the automation watch waits");
    // Keep alive.
    let starter = Recorder::default();
    let start = 10_000_000;
    state.places.watch.lock().unwrap().settled_at(start);
    let mut now = start;
    while now < start + places::keep::WAIT_MS + 10_000 {
        now += 2_000;
        places::tick_with(&state, now, &starter, &|| false);
    }
    assert!(starter.starts.lock().unwrap().is_empty(), "keep alive");
    // Restart.
    let restarted = places::restart(
        &state,
        &id,
        &starter,
        true,
        RestartOf::Confirmed,
        QuitWait::default(),
        now,
    );
    assert!(restarted.unwrap_err().contains("moving"));
    // Open.
    let opened = places::open(
        &state,
        &id,
        None,
        Arc::new(FakeDesktop::default()),
        Arc::new(Recorder::default()),
        None,
    );
    assert!(opened.is_err(), "Open");
    // A start (Start in AI Juggler, a hand-off).
    let place = places::with_book(&state, |book| book.places.get(&id).cloned())
        .unwrap()
        .unwrap();
    let hosts = crate::features::conversations::places::HostRegistry::of(&state.registry);
    assert!(places::start_place(&state, &place, &hosts, &starter, true).is_err());
    assert!(starter.starts.lock().unwrap().is_empty() && starter.quits.lock().unwrap().is_empty());

    // Once it lets go, keep alive starts it (so the hold was the reason).
    std::fs::remove_file(marker_path(&state, &id)).unwrap();
    assert!(!places::any_move(&state));
    while now < start + 2 * places::keep::WAIT_MS + 20_000 {
        now += 2_000;
        places::tick_with(&state, now, &starter, &|| false);
    }
    assert!(!starter.starts.lock().unwrap().is_empty(), "free again");
}

/// R2: a hold whose owner is gone is a crashed move, not a free profile.
#[test]
fn a_crashed_moves_hold_keeps_the_profile_closed() {
    let (_tmp, state, id) = lab();
    plant(&state, &id, gone(), "m-crashed");
    assert!(matches!(
        held(&area_at(&state.data_dir), &id),
        Held::Crashed(_)
    ));
    assert_eq!(
        places::why_held(&state, &id),
        Some(super::super::HELD_UNFINISHED)
    );
    assert!(places::any_move(&state));
    // A new move is refused in words; the crashed move's own Undo may take it.
    let refused = MoveHold::take(&state, std::slice::from_ref(&id), "m-new").err();
    assert_eq!(refused.as_deref(), Some(UNFINISHED));
    drop(MoveHold::take(&state, std::slice::from_ref(&id), "m-crashed").unwrap());
}
