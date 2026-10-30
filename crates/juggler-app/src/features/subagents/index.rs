//! AJ-E1520 (0.3): the Subagents tab's saved index — each session's
//! reader checkpoint (`SubagentReader::checkpoint`) in a file of its own
//! under AI Juggler's data folder (`subagents/<hash>.idx`), so a session
//! with hundreds of subagents and hundreds of megabytes of their
//! transcripts opens at once and reads only what grew.
//!
//! A cache, not a record: no database table, nothing that a newer or
//! older version can't throw away (a file it can't read is read again
//! from the tool's own files). Its words are redacted before they are
//! written (D-14), and a prompt, result or message only while last
//! messages are on (a file written the other way is dropped, and turning
//! them off drops every such file at once). One of the local caches
//! (ADR-0023): nothing is written while the cache is off or the disk is
//! nearly full, and the cache's manager keeps the newest [`KEEP`].
//! Written whole to a temporary file and moved into place, readable by
//! this user only.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::features::cache::{self, About, Gate, LocalCache, Trim, folder::FolderCache, stamp};
use crate::state::AppState;

/// The folder under AI Juggler's data folder.
pub const DIR: &str = "subagents";

/// The cache's key.
pub const CACHE: &str = "subagents";

/// The file's format: another is not read. (2: the cache's head line,
/// ADR-0023; 1 was this module's own.)
pub const FORMAT: u32 = 2;

/// How many sessions' indexes are kept (the newest): the manager's cap.
pub const KEEP: usize = 200;

/// The largest index read back.
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// The print of the settings an index is shaped by: whether last
/// messages are on (its prompts, results and messages are kept only
/// then).
#[must_use]
pub fn settings_print(messages: bool) -> u64 {
    stamp::settings_print(&[if messages { "messages" } else { "no messages" }])
}

fn stamp_of(path: &Path, messages: bool) -> stamp::Stamp<'_> {
    stamp::Stamp {
        cache: CACHE,
        format: FORMAT,
        settings: settings_print(messages),
        source: path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default(),
    }
}

/// The subagent lists, for the cache's registry.
#[must_use]
pub fn local_cache(state: &Arc<AppState>) -> Arc<dyn LocalCache> {
    Arc::new(FolderCache::new(
        About {
            id: CACHE,
            name: "Subagent lists",
            speeds_up: "A session's Subagents tab opens at once, even with hundreds of subagents.",
            slower: "Each session's Subagents tab reads its subagents' files again the first time it opens.",
            trim: Trim::Partly,
            own_switch: None,
            max_entries: Some(KEEP),
            shaped_by: &[crate::features::conversations::last_messages::SETTING_KEY],
        },
        state.data_dir.join(DIR),
        "idx",
        &[],
        cache::with_state(state, |state| {
            (
                FORMAT,
                settings_print(crate::features::conversations::last_messages::enabled(
                    state,
                )),
            )
        }),
    ))
}

/// The file for a session's index: a hash of its account, the tool's id
/// for it and its record's path (no name a person could read).
#[must_use]
pub fn path_for(dir: &Path, account: &str, id: &str, record: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in account
        .bytes()
        .chain([0])
        .chain(id.bytes())
        .chain([0])
        .chain(record.to_string_lossy().bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    dir.join(format!("{hash:016x}.idx"))
}

/// The checkpoint saved at `path`, when it was saved with last messages
/// as they are now (one saved otherwise, or unreadable, is dropped).
#[must_use]
pub fn load(gate: &Gate, path: &Path, messages: bool) -> Option<Vec<u8>> {
    stamp::read(gate, path, &stamp_of(path, messages), MAX_BYTES)
}

/// Saves `checkpoint` at `path` (whole, or not at all; nothing while the
/// cache is off or the disk nearly full). The cache's manager keeps the
/// newest [`KEEP`].
///
/// # Errors
///
/// The folder or file can't be written.
pub fn save(gate: &Gate, path: &Path, messages: bool, checkpoint: &[u8]) -> std::io::Result<bool> {
    // Whole or not at all, readable by this user only (where the system
    // has owner-only files: elsewhere nothing is saved, and the tab reads
    // the tool's files as before).
    stamp::write(gate, path, &stamp_of(path, messages), checkpoint)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_saved_index_loads_only_with_the_same_messages_setting_and_the_oldest_go() {
        let dir = tempfile::tempdir().unwrap();
        let gate = Gate::default();
        let path = path_for(dir.path(), "acc", "s1", Path::new("/p/s1.jsonl"));
        assert_ne!(
            path,
            path_for(dir.path(), "acc", "s2", Path::new("/p/s1.jsonl"))
        );
        assert!(!path.to_string_lossy().contains("s1"), "no readable name");
        assert!(save(&gate, &path, true, b"{\"x\":1}").unwrap());
        assert_eq!(load(&gate, &path, true).as_deref(), Some(&b"{\"x\":1}"[..]));
        assert_eq!(
            load(&gate, &path, false),
            None,
            "saved with messages: not used without"
        );
        assert!(!path.exists(), "and dropped at once");
        save(&gate, &path, true, b"{}").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "readable by this user only");
        }
        std::fs::write(&path, b"garbage").unwrap();
        assert_eq!(load(&gate, &path, true), None);
        assert!(!path.exists(), "a file that can't be read is dropped");
        // The manager keeps the newest (here three; 200 in the app).
        let manager = cache::Manager::new(
            dir.path().to_path_buf(),
            true,
            cache::Limit::Automatic,
            None,
        )
        .with_disk(|_| {
            Some(juggler_platform::room::DiskSpace {
                free: 500 << 30,
                total: 1000 << 30,
            })
        });
        manager.register(Arc::new(FolderCache::new(
            About {
                max_entries: Some(3),
                ..local_about()
            },
            dir.path().to_path_buf(),
            "idx",
            &[],
            Box::new(|| (FORMAT, settings_print(false))),
        )));
        for n in 0..5 {
            let other = path_for(dir.path(), "acc", &format!("o{n}"), Path::new("/p"));
            save(&gate, &other, false, b"{}").unwrap();
        }
        manager.pass(None);
        let left = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(left, 3);
    }

    fn local_about() -> About {
        About {
            id: CACHE,
            name: "Subagent lists",
            speeds_up: "",
            slower: "",
            trim: Trim::Partly,
            own_switch: None,
            max_entries: Some(KEEP),
            shaped_by: &[],
        }
    }
}
