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
//! messages are on (a file written the other way is not used). The
//! newest [`KEEP`] files stay; older ones go. Written whole to a
//! temporary file and moved into place, readable by this user only.

use std::path::{Path, PathBuf};

/// The folder under AI Juggler's data folder.
pub const DIR: &str = "subagents";

/// How many sessions' indexes are kept (the newest).
pub const KEEP: usize = 200;

/// The file's format: another is not read.
const VERSION: u32 = 1;

/// The first line of a file.
#[derive(serde::Serialize, serde::Deserialize)]
struct Head {
    v: u32,
    messages: bool,
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
/// as they are now.
#[must_use]
pub fn load(path: &Path, messages: bool) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let end = memchr_newline(&bytes)?;
    let head: Head = serde_json::from_slice(&bytes[..end]).ok()?;
    (head.v == VERSION && head.messages == messages).then(|| bytes[end + 1..].to_vec())
}

/// Saves `checkpoint` at `path` (whole, or not at all), then lets the
/// oldest indexes beyond [`KEEP`] go.
///
/// # Errors
///
/// The folder or file can't be written.
pub fn save(path: &Path, messages: bool, checkpoint: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let head = serde_json::to_vec(&Head {
        v: VERSION,
        messages,
    })
    .map_err(std::io::Error::other)?;
    let mut bytes = Vec::with_capacity(head.len() + 1 + checkpoint.len());
    bytes.extend_from_slice(&head);
    bytes.push(b'\n');
    bytes.extend_from_slice(checkpoint);
    // Whole or not at all, readable by this user only (where the system
    // has owner-only files: elsewhere nothing is saved, and the tab reads
    // the tool's files as before).
    juggler_platform::private_fs::write_private(path, &bytes)?;
    prune(dir, KEEP);
    Ok(())
}

/// Lets all but the newest `keep` indexes in `dir` go.
pub fn prune(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "idx"))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort_by_key(|file| std::cmp::Reverse(file.0));
    for (_, path) in files.into_iter().skip(keep) {
        let _ = std::fs::remove_file(path);
    }
}

fn memchr_newline(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|b| *b == b'\n')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_saved_index_loads_only_with_the_same_messages_setting_and_the_oldest_go() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_for(dir.path(), "acc", "s1", Path::new("/p/s1.jsonl"));
        assert_ne!(
            path,
            path_for(dir.path(), "acc", "s2", Path::new("/p/s1.jsonl"))
        );
        assert!(!path.to_string_lossy().contains("s1"), "no readable name");
        save(&path, true, b"{\"x\":1}").unwrap();
        assert_eq!(load(&path, true).as_deref(), Some(&b"{\"x\":1}"[..]));
        assert_eq!(
            load(&path, false),
            None,
            "saved with messages: not used without"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "readable by this user only");
        }
        std::fs::write(&path, b"garbage").unwrap();
        assert_eq!(load(&path, true), None);
        for n in 0..5 {
            let other = path_for(dir.path(), "acc", &format!("o{n}"), Path::new("/p"));
            save(&other, false, b"{}").unwrap();
        }
        prune(dir.path(), 3);
        let left = std::fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(left, 3);
    }
}
