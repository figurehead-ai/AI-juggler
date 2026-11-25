//! AJ-E2321 (the review's A5.1, A5.2, A5.4, A6.4, B5.3): a move survives
//! a crash. At start, before keep alive's first tick, every move left part
//! way whose AI Juggler is gone is held for every AI Juggler on this Mac
//! (its marker, AJ-E2320), then settled in the background:
//!
//! - planned or backed up (nothing changed on disk): let go, its backups
//!   with it, no file touched;
//! - changed files while its hold kept every app away since: undone by the
//!   journal (Undo, without reopening the conversation);
//! - passed its checks before the apps reopened: checked once more and
//!   committed (Finish);
//! - anything else (a step the disk can't explain, one an older AI Juggler
//!   left with its apps free since): held, with a bell and its door on the
//!   conversation's page ("Finish or Undo"), never settled silently.
//!
//! AJ-E2321 R4 (0.5.1): a move stopped part way through its changes, or
//! after them and before its checks, under its hold, is finished forward
//! from where it stopped: the journal says which steps are done (the disk
//! settles an open one), each remaining step is applied only when its file
//! is still as the backup has it, then the checks before and after run as
//! a move's do. One whose remaining steps can't be proven safe is undone,
//! as before. The Moves door's Finish does the same for one a person
//! settles.
//!
//! 0.5.1 F92: a move settled here (let go, undone or finished) starts
//! again, through the one launcher (`places::start`, F65), every App it
//! had quit (its journal says which, written before the first quit); one
//! that won't start is told by the bell, with Start. Its row stays in the
//! Moves door with what happened (never swept as a refused attempt).
//!
//! Also here: Finish by the move's id (the Moves door). A committed move's
//! backups are never retired here: their retention is AJ-E2325's policy
//! alone (the coordinator's ruling, 2026-11-23), and a move left part way
//! keeps its own until the person settles it.

use std::fmt::Write as _;
use std::sync::Arc;

use juggler_handover::moving::{Phase, Recovery, Stopped, Transaction};

use super::{
    MoveSystem, Real, engine, lock, place_name, quit::quit_by, refused, start_checked, undo_as,
    writing_of_move,
};
use crate::error::AppResult;
use crate::features::places::hold::{self, Held};
use crate::state::AppState;

/// What startup recovery does with a move left part way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settle {
    /// Nothing changed on disk: let it go.
    LetGo,
    /// It changed files: Undo, by the journal.
    RollBack,
    /// It passed its checks: check once more and keep it.
    Finish,
    /// R4: it stopped in or after its changes, under its hold: finish it
    /// from where it stopped (else Undo).
    Forward,
    /// A person decides on its door, and why.
    Ask(String),
}

/// What recovery does with `txn` (`None`: it is settled already). `held`:
/// its hold marker kept every app away since it stopped.
#[must_use]
pub fn settle_of(txn: &Transaction, held: bool) -> Option<Settle> {
    match txn.recover() {
        Ok(Recovery::Settled(Phase::Committed | Phase::Undone)) => None,
        Ok(Recovery::Settled(Phase::Planned | Phase::BackedUp)) => Some(Settle::LetGo),
        Ok(Recovery::Settled(Phase::Verified)) => Some(Settle::Finish),
        Ok(Recovery::Settled(Phase::Applying | Phase::Applied) | Recovery::Applying { .. })
            if held =>
        {
            Some(Settle::Forward)
        }
        Ok(Recovery::Settled(_) | Recovery::Undoing) if held => Some(Settle::RollBack),
        // An app may have written since: Undo would put that aside, so the
        // person says.
        Ok(Recovery::Settled(_) | Recovery::Applying { .. } | Recovery::Undoing) => Some(
            Settle::Ask("it was left part way before AI Juggler held its Apps closed, so they may have run since".to_owned()),
        ),
        Ok(Recovery::Unclear(why)) => Some(Settle::Ask(why)),
        Err(error) => Some(Settle::Ask(error.to_string())),
    }
}

/// Whether a crashed move's marker holds one of `txn`'s profiles.
fn crash_held(area: &std::path::Path, txn: &Transaction) -> bool {
    txn.manifest.places.iter().any(|place| {
        matches!(hold::held(area, place), Held::Crashed(marker) if marker.move_id == txn.manifest.id)
    })
}

/// The moves left part way that this AI Juggler settles, each held for
/// every AI Juggler from now on (runs before keep alive starts): none that
/// another AI Juggler is running (its lock), none held by another move.
pub(crate) fn claim(state: &Arc<AppState>) -> Vec<(String, Settle)> {
    let area = hold::area_at(&state.data_dir);
    let mut out = Vec::new();
    for txn in hold::all_moves(&state.data_dir) {
        let Some(settle) = settle_of(&txn, crash_held(&area, &txn)) else {
            continue;
        };
        if lock(state, &txn.manifest.plan.roots).is_err() {
            continue;
        }
        if let Err(why) = hold::take(&area, &txn.manifest.places, &txn.manifest.id) {
            tracing::info!(target: "juggler::move", "a move left part way waits: {why}");
            continue;
        }
        out.push((txn.manifest.id.clone(), settle));
    }
    out
}

/// Settles `claimed` (from [`claim`]); one that can't be settled stays
/// held and is told (the bell, and its door on the conversation's page).
pub(crate) fn settle(state: &Arc<AppState>, claimed: &[(String, Settle)], system: &dyn MoveSystem) {
    for (move_id, how) in claimed {
        let undo = || {
            undo_as(state, move_id, system, false)
                .map(|_| Settle::RollBack)
                .map_err(|error| error.to_string())
        };
        let done = match how {
            Settle::LetGo => let_go(state, move_id).map(|()| Settle::LetGo),
            Settle::RollBack => undo(),
            Settle::Finish => finish(state, move_id)
                .map(|_| Settle::Finish)
                .map_err(|e| e.to_string()),
            // R4: forward when every remaining step is safe, else back.
            Settle::Forward => finish(state, move_id)
                .map(|_| Settle::Forward)
                .or_else(|why| {
                    tracing::info!(target: "juggler::move", "a move left part way couldn't be finished, so it's undone: {why}");
                    undo()
                }),
            Settle::Ask(why) => Err(why.clone()),
        };
        match done {
            Ok(settled) => {
                tracing::info!(target: "juggler::move", "a move left part way was settled ({settled:?})");
                start_what_it_quit(state, move_id, &settled, system);
            }
            Err(why) => tell(state, move_id, &why),
        }
    }
}

/// 0.5.1 F92: the journal's event for what startup recovery did.
pub(crate) const RECOVERED: &str = "recovered";

/// What `how` did, as the journal names it.
fn settled_word(how: &Settle) -> &'static str {
    match how {
        Settle::LetGo => "let_go",
        Settle::RollBack => "rolled_back",
        Settle::Finish => "finished",
        Settle::Forward => "rolled_forward",
        Settle::Ask(_) => "asked",
    }
}

/// 0.5.1 F92: after `move_id` was settled (`how`), every App it had quit
/// and that isn't running now is started again, each on the folder the
/// move gives it (never one its sessions contradict); one that won't
/// start is told by the bell with Start. The journal says what happened
/// (the Moves door's row).
fn start_what_it_quit(state: &Arc<AppState>, move_id: &str, how: &Settle, system: &dyn MoveSystem) {
    let Ok(txn) = hold::open_move(&state.data_dir, move_id) else {
        return;
    };
    let mut started = Vec::new();
    let mut not_started = Vec::new();
    for id in quit_by(&txn) {
        if system.running(state, &id).is_some() {
            continue;
        }
        let place = crate::features::places::with_book(state, |book| book.places.get(&id).cloned())
            .ok()
            .flatten();
        let Some(place) = place else {
            // An App AI Juggler doesn't know any more: nothing to start.
            continue;
        };
        let name = place_name(state, &place);
        match start_checked(state, &id, &place, &txn.manifest.request, system) {
            Ok(()) => started.push(name),
            Err(why) => {
                tracing::warn!(target: "juggler::move", "an App a move had quit didn't start again: {why}");
                crate::features::notifications::sources::move_app_not_started(
                    state, &id, &name, &why,
                );
                not_started.push(name);
            }
        }
    }
    let detail = serde_json::json!({
        "settled": settled_word(how),
        "started": started,
        "not_started": not_started,
    });
    if let Err(error) = txn.note(RECOVERED, detail) {
        tracing::warn!(target: "juggler::move", "recovery's line wasn't written: {error}");
    }
}

/// 0.5.1 F92: what startup recovery did with `txn`, in words for its row
/// in the Moves door; `None`: it wasn't recovered.
pub(crate) fn recovered_words(txn: &Transaction) -> Option<String> {
    let line = txn.notes(RECOVERED).pop()?;
    let names = |key: &str| -> Vec<String> {
        line[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| n.as_str().map(str::to_owned))
            .collect()
    };
    let mut words = match line["settled"].as_str() {
        Some("finished") => "AI Juggler stopped during this move, after its checks passed; it was finished when AI Juggler started again".to_owned(),
        Some("rolled_forward") => "AI Juggler stopped during this move; it was finished from where it stopped when AI Juggler started again, and checked".to_owned(),
        Some("rolled_back") => "AI Juggler stopped during this move; it was undone when AI Juggler started again, so nothing was moved".to_owned(),
        _ => "AI Juggler stopped during this move, before it changed anything, so nothing was moved".to_owned(),
    };
    let started = names("started");
    if !started.is_empty() {
        let _ = write!(
            words,
            ". {} {} started again",
            juggler_core::words::and_list(&started),
            if started.len() == 1 { "was" } else { "were" }
        );
    }
    let left = names("not_started");
    if !left.is_empty() {
        let one = left.len() == 1;
        let _ = write!(
            words,
            ". {} couldn't be started: start {} from the bell or Settings › Desktop apps",
            juggler_core::words::and_list(&left),
            if one { "it" } else { "them" }
        );
    }
    Some(words)
}

/// At start: claims what a crash left (before keep alive), then settles
/// it and sweeps undone moves' backups in the background.
pub fn at_startup(state: &Arc<AppState>) {
    let claimed = claim(state);
    let state = Arc::clone(state);
    let _ = std::thread::Builder::new()
        .name("move-recovery".into())
        .spawn(move || {
            startup_settle(&state, &claimed, &Real::quiet());
            // AJ-E2325: the backups' sweep once a day after the start's.
            loop {
                std::thread::sleep(std::time::Duration::from_hours(24));
                super::retention::sweep(&state, crate::commands::now_millis());
            }
        });
}

/// [`at_startup`]'s background half: what was claimed settled, then the
/// backups' policy ([`super::retention`]) and the backups of undone moves
/// let go (0.4.9: nothing is left to restore from them). Interrupted
/// moves keep theirs.
pub(crate) fn startup_settle(
    state: &Arc<AppState>,
    claimed: &[(String, Settle)],
    system: &dyn MoveSystem,
) {
    settle(state, claimed, system);
    // AJ-E2325: after recovery, the backups' policy.
    super::retention::sweep(state, crate::commands::now_millis());
    for folder in hold::journal_folders(&state.data_dir) {
        let went = juggler_handover::moving::sweep_undone(&folder);
        if went > 0 {
            tracing::info!(target: "juggler::move", "let go of {went} undone moves' backups");
        }
    }
}

/// A move that changed nothing, let go: undone without touching a file,
/// its backups and its hold gone.
fn let_go(state: &Arc<AppState>, move_id: &str) -> Result<(), String> {
    let mut txn = hold::open_move(&state.data_dir, move_id).map_err(|e| e.to_string())?;
    let _lock = lock(state, &txn.manifest.plan.roots).map_err(|e| e.to_string())?;
    txn.abandon().map_err(|e| e.to_string())?;
    hold::release(
        &hold::area_at(&state.data_dir),
        &txn.manifest.places,
        move_id,
    );
    Ok(())
}

/// The bell for a move left part way: what happened, and its door.
fn tell(state: &Arc<AppState>, move_id: &str, why: &str) {
    tracing::warn!(target: "juggler::move", "a move left part way needs the user: {why}");
    let Ok(txn) = hold::open_move(&state.data_dir, move_id) else {
        return;
    };
    // AJ-E2321 R4: one stopped in or after its changes finishes too.
    let finishable = matches!(
        txn.recover(),
        Ok(Recovery::Applying { .. } | Recovery::Settled(Phase::Applied | Phase::Verified))
    );
    crate::features::notifications::sources::move_unfinished(
        state,
        move_id,
        &txn.manifest.request.conversation,
        finishable,
    );
}

/// What Finish says.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveFinishedInfo {
    /// In words.
    pub words: String,
}

/// R4: what an interrupted move has left to do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Left {
    /// Its steps from this one, then its checks.
    StepsFrom(usize),
    /// Every step done: its checks before the apps open.
    Checks,
    /// Its checks passed: only the check once more.
    Opening,
}

/// Finishes move `move_id` (the Moves door's Finish, or recovery): one
/// that stopped after its checks passed and before the apps reopened is
/// checked once more and committed; R4: one that stopped in or after its
/// changes is finished from where it stopped (each remaining step only
/// when its file is still as the backup has it), checked before and
/// after, and committed. Its hold goes. Nothing is quit or written but
/// the move's own steps and record.
///
/// # Errors
///
/// Not such a move, already finished, a step or check fails now (Undo is
/// its way back), or another move holds its files.
pub fn finish(state: &Arc<AppState>, move_id: &str) -> AppResult<MoveFinishedInfo> {
    let mut txn = super::open_by_id(state, move_id)?;
    let left = match (txn.manifest.phase, txn.recover()) {
        (Phase::Committed, _) => return Err(refused("this move is finished already")),
        (Phase::Verified, _) => Left::Opening,
        (_, Ok(Recovery::Applying { next })) => Left::StepsFrom(next),
        (_, Ok(Recovery::Settled(Phase::Applied))) => Left::Checks,
        _ => {
            return Err(refused(
                "only a move that stopped in or after its changes can be finished; Undo puts both accounts back",
            ));
        }
    };
    let (_lock, _hold, adapter) = super::take_move(state, &txn)?;
    let writing = writing_of_move(&adapter, &txn)?;
    let request = txn.manifest.request.clone();
    let plan = txn.manifest.plan.clone();
    if left != Left::Opening {
        if let Left::StepsFrom(from) = left {
            txn.apply(&Stopped::checked(), from).map_err(|error| {
                refused(format!(
                    "a step can't be finished ({error}); Undo puts both accounts back"
                ))
            })?;
        }
        let cold = txn
            .verify_cold(writing.mover.verify(&request, &plan))
            .map_err(engine)?;
        if !cold.passed() {
            return Err(refused(format!(
                "a check failed, so it wasn't finished ({}); Undo puts both accounts back",
                cold.failures().join("; ")
            )));
        }
    }
    let checks = writing.mover.verify(&request, &plan);
    let report = txn
        .verify_opened(&|line| writing.mover.appended(line), checks)
        .map_err(engine)?;
    if !report.passed() {
        return Err(refused(format!(
            "a check failed, so it wasn't finished ({}); Undo puts both accounts back",
            report.failures().join("; ")
        )));
    }
    Ok(MoveFinishedInfo {
        words: if left == Left::Opening {
            "Finished: the conversation stays where it moved, checked once more.".to_owned()
        } else {
            "Finished: the move went on from where it stopped, and the conversation is where it moved, checked.".to_owned()
        },
    })
}
