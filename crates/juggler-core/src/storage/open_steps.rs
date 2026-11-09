//! The first open after an update (0.4.1): its long steps told to a
//! watcher as they start, the room checked, and the room an upgrade freed
//! given back once (the tidy). Split from `storage` (its size budget).

use std::path::Path;

use rusqlite::Connection;

/// What opening the database is doing, for the app's "Updating AI
/// Juggler's data" window (0.4.1: an upgrade at the Dev database's size
/// took seconds before any window showed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenStep {
    /// The copy kept from before an upgrade.
    SafetyCopy,
    /// The upgrade itself.
    Updating,
    /// Giving back the room an upgrade freed (a VACUUM, once).
    Tidying,
}

/// Who watches an open: the steps as they start, the free room where the
/// database is (the core reads no disk statistics itself), and notes for
/// the debug log. Every method has a default that does nothing.
pub trait OpenWatch {
    /// `step` starts.
    fn step(&mut self, step: OpenStep) {
        let _ = step;
    }
    /// Free bytes on the volume holding `folder`; `None` when unknown.
    fn free_bytes(&self, folder: &Path) -> Option<u64> {
        let _ = folder;
        None
    }
    /// A plain note for the debug log (a tidy skipped, and why).
    fn note(&mut self, words: &str) {
        let _ = words;
    }
}

/// An open nobody watches.
pub(super) struct Unwatched;
impl OpenWatch for Unwatched {}

/// The free room a tidy needs, as a multiple of the database's size, in
/// tenths: with an upgrade due, the safety copy and the VACUUM (22 =
/// 2.2 ×); a tidy alone, the VACUUM's copy and its journal (12).
const TIDY_ROOM_WITH_COPY: u64 = 22;
const TIDY_ROOM_ALONE: u64 = 12;

/// The fewest free pages worth a tidy (8 MB at 4 KiB pages).
const TIDY_MIN_PAGES: i64 = 2_048;

/// Whether the volume holding `path` has room for a tidy: `None` when the
/// watcher can't say (the tidy then runs, and a failure is only noted).
pub(super) fn tidy_room(path: &Path, watch: &dyn OpenWatch, upgrading: bool) -> Option<bool> {
    let size = std::fs::metadata(path).ok()?.len();
    let free = watch.free_bytes(path.parent()?)?;
    let tenths = if upgrading {
        TIDY_ROOM_WITH_COPY
    } else {
        TIDY_ROOM_ALONE
    };
    Some(free.saturating_mul(10) >= size.saturating_mul(tenths))
}

/// Gives back the room a large delete or an upgrade freed, when it's
/// over a tenth of the file: a database that already gives room back
/// a little at a time does so now (quick); one that doesn't yet is
/// rebuilt once (a VACUUM), which also switches it to that, when
/// `room` allows. Never fails the open: anything in the way is noted
/// and the tidy waits for a later open.
pub(super) fn tidy(conn: &Connection, room: Option<bool>, watch: &mut dyn OpenWatch) {
    give_back(conn, room, watch);
    // The rc8 walk: after an upgrade and a VACUUM the WAL stayed at
    // 768 MB beside a 764 MB file. Folded in and emptied here, once per
    // open (quick when there's nothing in it).
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
}

/// [`tidy`]'s giving back.
fn give_back(conn: &Connection, room: Option<bool>, watch: &mut dyn OpenWatch) {
    let read = |pragma: &str| -> i64 {
        conn.query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
            .unwrap_or(0)
    };
    let (pages, free, mode) = (
        read("page_count"),
        read("freelist_count"),
        read("auto_vacuum"),
    );
    if free <= pages / 10 || free < TIDY_MIN_PAGES {
        return;
    }
    if mode == 2 {
        if let Err(error) = conn.execute_batch("PRAGMA incremental_vacuum") {
            watch.note(&format!("tidy: the freed room wasn't given back ({error})"));
        }
        return;
    }
    if room == Some(false) {
        watch.note("tidy: skipped, not enough free disk space for it; a later open tries again");
        return;
    }
    watch.step(OpenStep::Tidying);
    if let Err(error) = conn.execute_batch("PRAGMA auto_vacuum = INCREMENTAL; VACUUM;") {
        watch.note(&format!(
            "tidy: the rebuild didn't run ({error}); a later open tries again"
        ));
    }
}

/// The safety copy as the file itself (0.4.1: the first open after the
/// update spent its cold seconds reading and writing every page through
/// SQLite's backup): the WAL is checkpointed into the file first, so the
/// file alone is the whole database, then copied. On macOS the copy is a
/// clone where the volume is APFS (copy-on-write: near instant, cold or
/// warm) and a plain copy elsewhere; other systems copy as they can.
/// `false` (nothing written) when the WAL couldn't all be checkpointed:
/// the caller copies page by page instead.
pub(super) fn whole_file_copy(
    conn: &Connection,
    db: &Path,
    to: &Path,
) -> Result<bool, super::CopyFailure> {
    // (busy, frames in the WAL, frames checkpointed); `-1`s when the
    // database isn't in WAL mode (its file is then whole already).
    let (busy, log, done): (i64, i64, i64) =
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if busy != 0 || done < log {
        return Ok(false);
    }
    std::fs::copy(db, to)?;
    Ok(true)
}
