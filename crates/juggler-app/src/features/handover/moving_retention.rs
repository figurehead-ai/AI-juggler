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
//!   never while a move runs (this AI Juggler's hold, any one's lock);
//! - a nearly full disk (under the cache's low-disk line) or a move refused
//!   for room brings an offer ([`offer_if_pressed`]) to clear backups older
//!   than a day, naming what it frees; only the person's click clears them
//!   ([`clear_old`]), never the offer.

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
/// Juggler is running (its lock) is passed by. Then the low-disk offer
/// ([`offer_if_pressed`]).
pub fn sweep(state: &Arc<AppState>, now_ms: i64) -> Swept {
    let out = sweep_with(state, Some(now_ms));
    // What the policy keeps still fills a nearly full disk: offer, never
    // clear.
    offer_if_pressed(state, now_ms);
    out
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

/// What deleting `txn`'s backups gives back (`None`: they can't be read).
#[must_use]
pub fn backup_size(txn: &Transaction) -> Option<juggler_platform::room::Reclaim> {
    juggler_platform::room::reclaimable(&txn.dir().join("backups")).ok()
}

/// When a move was last refused for room (epoch ms).
pub const ROOM_SHORT_KEY: &str = "moves.room_short_ms";

/// AJ-E2325: the backups kept for Undo, together: the Moves door's total,
/// the header's "N to clean up", and the low-disk offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct MoveBackupsInfo {
    /// Finished moves whose backup is kept.
    pub count: u32,
    /// What deleting them all gives back, in bytes (at most, unless
    /// `exact`).
    #[ts(type = "number")]
    pub bytes: u64,
    /// `bytes` is exactly what deleting frees (no clones share blocks).
    pub exact: bool,
    /// Of them, made more than a day ago: what the offer clears.
    pub old_count: u32,
    /// What clearing those gives back, in bytes (at most, unless `exact`).
    #[ts(type = "number")]
    pub old_bytes: u64,
    /// The disk is nearly full, or a move was refused for room in the
    /// last day: the offer to clear the older ones shows.
    pub low_room: bool,
}

/// Whether the disk AI Juggler's data is on is under the low-disk line
/// (the local cache's), or a move was refused for room within a day.
fn low_room(state: &Arc<AppState>, now_ms: i64) -> bool {
    let disk_low = juggler_platform::room::disk_space(&state.data_dir).is_ok_and(|disk| {
        let free = juggler_platform::room::free_space(&state.data_dir).unwrap_or(disk.free);
        free < crate::features::cache::budget::low_disk_line(disk)
    });
    let refused = setting(state, ROOM_SHORT_KEY)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .is_some_and(|at| now_ms.saturating_sub(at) < DAY_MS);
    disk_low || refused
}

/// The backups kept for Undo at `now_ms`, with what deleting them frees.
#[must_use]
pub fn backups(state: &Arc<AppState>, now_ms: i64) -> MoveBackupsInfo {
    let mut out = MoveBackupsInfo {
        exact: true,
        ..MoveBackupsInfo::default()
    };
    for txn in hold::all_moves(&state.data_dir) {
        if !super::door::committed_backup(&txn) {
            continue;
        }
        let size = backup_size(&txn).unwrap_or_default();
        out.count += 1;
        out.bytes = out.bytes.saturating_add(size.bytes);
        out.exact &= size.exact;
        if now_ms.saturating_sub(txn.manifest.created_ms) >= DAY_MS {
            out.old_count += 1;
            out.old_bytes = out.old_bytes.saturating_add(size.bytes);
        }
    }
    out.low_room = low_room(state, now_ms);
    out
}

/// "7.8 GB" said as what it frees: "up to 7.8 GB" unless exact.
#[must_use]
pub fn frees_words(bytes: u64, exact: bool) -> String {
    let size = juggler_supervisor::machine::memory_words(bytes);
    if exact { size } else { format!("up to {size}") }
}

/// A move was refused for room now: the offer may show (at once, and for
/// a day).
pub fn note_room_short(state: &Arc<AppState>, now_ms: i64) {
    if let Ok(storage) = state.storage.lock() {
        let _ = storage
            .app_settings()
            .set(ROOM_SHORT_KEY, &now_ms.to_string());
    }
    offer_if_pressed(state, now_ms);
}

/// The low-disk offer: when room is short and backups older than a day
/// are kept, one notice a day saying what clearing them frees, opening
/// Settings › Desktop apps › Moves. Nothing is deleted here; never while a
/// move runs. Returns whether an offer was made.
pub fn offer_if_pressed(state: &Arc<AppState>, now_ms: i64) -> bool {
    if crate::features::places::moving_here(state) {
        return false;
    }
    let info = backups(state, now_ms);
    if !info.low_room || info.old_count == 0 {
        return false;
    }
    let target = crate::features::notifications::centre::NoticeTarget {
        page: Some("moves".to_owned()),
        ..Default::default()
    };
    let notice = juggler_core::storage::NewNotice {
        key: format!("move-backups:low-room:{}", now_ms / DAY_MS),
        kind: "move-backups".to_owned(),
        title: format!(
            "The disk is nearly full: move backups older than a day free {}",
            frees_words(info.old_bytes, info.exact)
        ),
        detail: "Nothing is deleted until you choose. Clearing them means those moves can no longer be undone.".to_owned(),
        data: crate::features::notifications::centre::data_of(
            &target,
            &crate::features::notifications::centre::NoticeFacts::default(),
        ),
        action: Some("move-backups".to_owned()),
        ..Default::default()
    };
    state.storage.lock().is_ok_and(|storage| {
        crate::features::notifications::centre::record(&storage, &notice, now_ms)
            .is_ok_and(|made| made.is_some())
    })
}

/// The offer accepted (Clear backups older than a day, after its sheet):
/// every finished move made more than a day before `now_ms` loses its
/// backup, and its Undo. Never while a move runs; an interrupted move's
/// are never touched; a move another AI Juggler holds is passed by.
/// Returns the words.
///
/// # Errors
///
/// A move is running now.
pub fn clear_old(state: &Arc<AppState>, now_ms: i64) -> crate::error::AppResult<String> {
    if crate::features::places::moving_here(state) {
        return Err(super::refused(
            "a move is running; clear backups once it has finished",
        ));
    }
    let (mut cleared, mut freed, mut exact) = (0_u32, 0_u64, true);
    for txn in hold::all_moves(&state.data_dir) {
        if !super::door::committed_backup(&txn)
            || now_ms.saturating_sub(txn.manifest.created_ms) < DAY_MS
        {
            continue;
        }
        let Ok(_lock) = lock(state, &txn.manifest.plan.roots) else {
            continue;
        };
        let size = backup_size(&txn).unwrap_or_default();
        if hold::open_move(&state.data_dir, &txn.manifest.id)
            .and_then(|mut txn| txn.retire())
            .unwrap_or(false)
        {
            cleared += 1;
            freed = freed.saturating_add(size.bytes);
            exact &= size.exact;
        }
    }
    tracing::info!(target: "juggler::move", "cleared {cleared} move backups older than a day");
    Ok(match cleared {
        0 => "No backups older than a day to clear.".to_owned(),
        1 => format!(
            "Cleared 1 backup ({}); that move can no longer be undone.",
            frees_words(freed, exact)
        ),
        n => format!(
            "Cleared {n} backups ({}); those moves can no longer be undone.",
            frees_words(freed, exact)
        ),
    })
}
