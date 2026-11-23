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
//! Rolling an interrupted apply forward comes later (the release notes say
//! so): Undo is its way back now.
//!
//! Also here: Finish by the move's id (the Moves door). A committed move's
//! backups are never retired here: their retention is AJ-E2325's policy
//! alone (the coordinator's ruling, 2026-11-23), and a move left part way
//! keeps its own until the person settles it.

use std::sync::Arc;

use juggler_handover::moving::{Phase, Recovery, Transaction};

use super::{MoveSystem, Real, engine, lock, refused, undo_as, writing_of_move};
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
        Ok(Recovery::Settled(_) | Recovery::Applying { .. } | Recovery::Undoing) if held => {
            Some(Settle::RollBack)
        }
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
        let done = match how {
            Settle::LetGo => let_go(state, move_id),
            Settle::RollBack => undo_as(state, move_id, system, false)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            Settle::Finish => finish(state, move_id)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Settle::Ask(why) => Err(why.clone()),
        };
        match done {
            Ok(()) => {
                tracing::info!(target: "juggler::move", "a move left part way was settled ({how:?})");
            }
            Err(why) => tell(state, move_id, &why),
        }
    }
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
    let finishable = txn.manifest.phase == Phase::Verified;
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

/// Finishes move `move_id` (the Moves door's Finish, or recovery): one
/// that stopped after its checks passed and before the apps reopened is
/// checked once more and committed; its hold goes. Nothing is quit or
/// written but the move's own record.
///
/// # Errors
///
/// Not such a move, already finished, a check fails now (Undo is its way
/// back), or another move holds its files.
pub fn finish(state: &Arc<AppState>, move_id: &str) -> AppResult<MoveFinishedInfo> {
    if move_id.is_empty()
        || !move_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(refused("that isn't a move's id"));
    }
    let mut txn = hold::open_move(&state.data_dir, move_id).map_err(engine)?;
    match txn.manifest.phase {
        Phase::Verified => {}
        Phase::Committed => return Err(refused("this move is finished already")),
        _ => {
            return Err(refused(
                "only a move that passed its checks can be finished; Undo puts both accounts back",
            ));
        }
    }
    let _lock = lock(state, &txn.manifest.plan.roots)?;
    let _hold =
        crate::features::places::MoveHold::take(state, &txn.manifest.places, &txn.manifest.id)
            .map_err(refused)?;
    let adapter = state
        .registry
        .get(&juggler_core::model::HarnessId::new(
            txn.manifest.tool.as_str(),
        ))
        .ok_or_else(|| refused("this tool's conversations travel as a copy"))?;
    let writing = writing_of_move(&adapter, &txn)?;
    let checks = writing
        .mover
        .verify(&txn.manifest.request, &txn.manifest.plan);
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
        words: "Finished: the conversation stays where it moved, checked once more.".to_owned(),
    })
}
