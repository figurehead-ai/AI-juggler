//! AJ-E2325: move backups clean themselves up, by one policy (the only
//! place that lets a move's backups go):
//!
//! - a refused attempt (undone before it changed a file, nothing kept
//!   aside) loses its whole folder at once;
//! - a committed move keeps its backups (its Undo) for the time the setting
//!   says ([`KEEP_KEY`]: 1, 7 or 30 days, or until the person deletes them;
//!   7 by default); backups that existed at the first sweep are kept at
//!   least [`DEFAULT_DAYS`] after they were made;
//! - an interrupted move's, or an undone one's work kept aside, never go by
//!   themselves;
//! - the sweep runs at start (after AJ-E2321's recovery) and once a day,
//!   never while a move runs (this AI Juggler's hold, any one's lock).

use std::sync::Arc;

use juggler_handover::moving::{Phase, Transaction};

use super::lock;
use crate::features::places::hold;
use crate::state::AppState;

/// The setting: `1`, `7`, `30` (days) or `forever`.
pub const KEEP_KEY: &str = "moves.keep_backups";
/// When the first sweep ran (epoch ms): backups made before it get the
/// default's days at least.
pub const FIRST_SWEEP_KEY: &str = "moves.first_sweep_ms";
/// The default, in days.
pub const DEFAULT_DAYS: i64 = 7;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// How long committed moves keep their backups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// This many days after the move.
    Days(i64),
    /// Until the person deletes them.
    UntilDeleted,
}

/// The setting's value read (anything else: the default).
#[must_use]
pub fn keep_of(value: Option<&str>) -> Keep {
    match value.map(str::trim) {
        Some("1") => Keep::Days(1),
        Some("30") => Keep::Days(30),
        Some("forever") => Keep::UntilDeleted,
        _ => Keep::Days(DEFAULT_DAYS),
    }
}

fn setting(state: &Arc<AppState>, key: &str) -> Option<String> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(key).ok().flatten())
}

/// The setting now.
#[must_use]
pub fn keep(state: &Arc<AppState>) -> Keep {
    keep_of(setting(state, KEEP_KEY).as_deref())
}

/// When the first sweep ran (now, the first time).
fn first_sweep(state: &Arc<AppState>, now_ms: i64) -> i64 {
    if let Some(at) = setting(state, FIRST_SWEEP_KEY).and_then(|v| v.trim().parse().ok()) {
        return at;
    }
    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .app_settings()
            .set(FIRST_SWEEP_KEY, &now_ms.to_string());
    }
    now_ms
}

/// Until when a committed move made at `created_ms` keeps its backups
/// (`None`: until the person deletes them).
#[must_use]
pub fn kept_until(created_ms: i64, keep: Keep, first_sweep_ms: i64) -> Option<i64> {
    let Keep::Days(days) = keep else {
        return None;
    };
    let days = if created_ms < first_sweep_ms {
        days.max(DEFAULT_DAYS)
    } else {
        days
    };
    Some(created_ms.saturating_add(days.saturating_mul(DAY_MS)))
}

/// [`kept_until`] for `txn` with the setting and the first sweep now.
#[must_use]
pub fn kept_until_of(state: &Arc<AppState>, txn: &Transaction) -> Option<i64> {
    let first = setting(state, FIRST_SWEEP_KEY)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(i64::MAX);
    kept_until(txn.manifest.created_ms, keep(state), first)
}

/// A refused attempt: undone before it changed a file, nothing kept
/// aside (its preserved folder, when Undo made one, holds no file).
fn refused_attempt(txn: &Transaction) -> bool {
    txn.manifest.phase == Phase::Undone
        && !txn.ever_applied()
        && !holds_a_file(&txn.dir().join("preserved"))
}

/// Whether `dir` holds anything but empty folders.
fn holds_a_file(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_type()
                .map_or(true, |kind| !kind.is_dir() || holds_a_file(&entry.path()))
        })
    })
}

/// What a sweep let go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swept {
    /// Refused attempts' folders.
    pub refused: usize,
    /// Committed moves' backups past their time.
    pub retired: usize,
}

/// Refused attempts' folders, at once (after a refusal, and at start).
pub fn drop_refused(state: &Arc<AppState>) -> usize {
    sweep_with(state, None).refused
}

/// The sweep at `now_ms`: refused attempts' folders, and committed moves'
/// backups past their time. Nothing while a move runs; a move another AI
/// Juggler is running (its lock) is passed by.
pub fn sweep(state: &Arc<AppState>, now_ms: i64) -> Swept {
    sweep_with(state, Some(now_ms))
}

fn sweep_with(state: &Arc<AppState>, now_ms: Option<i64>) -> Swept {
    let mut out = Swept::default();
    if crate::features::places::moving_here(state) {
        return out;
    }
    let policy = now_ms.map(|now| (now, keep(state), first_sweep(state, now)));
    for txn in hold::all_moves(&state.data_dir) {
        let Ok(_lock) = lock(state, &txn.manifest.plan.roots) else {
            continue;
        };
        if refused_attempt(&txn) {
            if std::fs::remove_dir_all(txn.dir()).is_ok() {
                out.refused += 1;
            }
            continue;
        }
        let Some((now, keep, first)) = policy else {
            continue;
        };
        if txn.manifest.phase == Phase::Committed
            && kept_until(txn.manifest.created_ms, keep, first).is_some_and(|until| now >= until)
            && hold::open_move(&state.data_dir, &txn.manifest.id)
                .and_then(|mut txn| txn.retire())
                .unwrap_or(false)
        {
            out.retired += 1;
        }
    }
    if out != Swept::default() {
        tracing::info!(target: "juggler::move", "move backups swept: {out:?}");
    }
    out
}

/// Delete backup (the Moves door, by the move's id): a committed move's
/// backups go now, and its Undo with them. Returns the words.
///
/// # Errors
///
/// No such move, one not committed (an interrupted move keeps its backup
/// until it is finished or undone), or another move holds its files.
pub fn delete_backup(state: &Arc<AppState>, move_id: &str) -> crate::error::AppResult<String> {
    if move_id.is_empty()
        || !move_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(super::refused("that isn't a move's id"));
    }
    let mut txn = hold::open_move(&state.data_dir, move_id).map_err(super::engine)?;
    if txn.manifest.phase != Phase::Committed {
        return Err(super::refused(
            "only a finished move's backup can be deleted; finish or undo this one first",
        ));
    }
    let _lock = lock(state, &txn.manifest.plan.roots)?;
    txn.retire().map_err(super::engine)?;
    Ok("Backup deleted: this move can no longer be undone.".to_owned())
}
