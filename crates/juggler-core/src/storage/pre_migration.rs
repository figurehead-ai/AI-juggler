//! The copy of the database kept before an upgrade (`<file>.before-v<N>`),
//! and the newest [`KEPT_PRE_MIGRATION_COPIES`] of them kept beside it.
//! Moved from `storage` whole, unchanged (its size budget).

use std::path::Path;

use rusqlite::Connection;

use super::{open_hardened, open_steps, owner_only};
use crate::error::CoreError;

/// How many pre-migration copies (`<file>.before-v<N>`) are kept beside a
/// database: the newest by schema.
pub const KEPT_PRE_MIGRATION_COPIES: usize = 2;

/// The pre-migration copies kept beside the database at `db`, newest
/// schema first: `(schema, path)`.
#[must_use]
pub fn pre_migration_copies(db: &Path) -> Vec<(i64, std::path::PathBuf)> {
    let (Some(dir), Some(name)) = (db.parent(), db.file_name()) else {
        return Vec::new();
    };
    let prefix = format!("{}.before-v", name.to_string_lossy());
    let mut copies: Vec<(i64, std::path::PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file = entry.file_name().to_string_lossy().into_owned();
            let schema = file.strip_prefix(&prefix)?.parse::<i64>().ok()?;
            Some((schema, entry.path()))
        })
        .collect();
    copies.sort_by_key(|copy| std::cmp::Reverse(copy.0));
    copies
}

/// Keeps a consistent copy (SQLite's online backup API, so what is still
/// only in the WAL is in it too) of the database about to be migrated, as
/// `<file>.before-v<current>`: written to a temporary name, flushed to disk
/// with its folder, owner-only, then renamed — a half-written copy never
/// carries the name. Then prunes to the newest
/// [`KEPT_PRE_MIGRATION_COPIES`]. On failure the temporary file is removed
/// and nothing is migrated.
pub(super) fn keep_pre_migration_copy(
    conn: &Connection,
    db: &Path,
    current: i64,
) -> Result<(), CoreError> {
    let name = db
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target = db.with_file_name(format!("{name}.before-v{current}"));
    let partial = db.with_file_name(format!("{name}.before-v{current}.partial"));
    let _ = std::fs::remove_file(&partial);
    let written = (|| -> Result<(), CopyFailure> {
        // The file itself once the WAL is in it (a clone on APFS: near
        // instant even cold); else SQLite's online backup, page by page.
        if !open_steps::whole_file_copy(conn, db, &partial)? {
            let mut copy = open_hardened(&partial)?;
            let backup = rusqlite::backup::Backup::new(conn, &mut copy)?;
            backup.run_to_completion(256, std::time::Duration::ZERO, None)?;
        }
        super::memory::backup::scrub_copy(&partial)?; // ADR-0037 §1.5: no memory in a copy
        owner_only(&partial);
        // A writable handle: Windows flushes (FlushFileBuffers) only
        // through one, and a read-only flush fails there.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&partial)?
            .sync_all()?;
        std::fs::rename(&partial, &target)?;
        // The rename survives a power cut once the folder is flushed. Only
        // Unix opens a folder as a file; Windows has no such flush.
        #[cfg(unix)]
        if let Some(dir) = db.parent()
            && let Ok(folder) = std::fs::File::open(dir)
        {
            let _ = folder.sync_all();
        }
        Ok(())
    })();
    if let Err(failure) = written {
        let _ = std::fs::remove_file(&partial);
        let hint = if failure.disk_full {
            "the disk is full — free some space, then open AI Juggler again"
        } else {
            "nothing was changed"
        };
        return Err(CoreError::Io(format!(
            "couldn't keep a copy of the data before upgrading it ({}); {hint}{}",
            failure.message,
            if failure.disk_full {
                "; nothing was changed"
            } else {
                ""
            }
        )));
    }
    for (_, stale) in pre_migration_copies(db)
        .into_iter()
        .skip(KEPT_PRE_MIGRATION_COPIES)
    {
        let _ = std::fs::remove_file(stale);
    }
    Ok(())
}

/// Why a pre-migration copy couldn't be written.
pub(super) struct CopyFailure {
    message: String,
    disk_full: bool,
}

impl From<rusqlite::Error> for CopyFailure {
    fn from(error: rusqlite::Error) -> Self {
        let disk_full = matches!(
            &error,
            rusqlite::Error::SqliteFailure(failure, _) if failure.code == rusqlite::ErrorCode::DiskFull
        );
        Self {
            message: error.to_string(),
            disk_full,
        }
    }
}

impl From<std::io::Error> for CopyFailure {
    fn from(error: std::io::Error) -> Self {
        Self {
            disk_full: error.kind() == std::io::ErrorKind::StorageFull,
            message: error.to_string(),
        }
    }
}
