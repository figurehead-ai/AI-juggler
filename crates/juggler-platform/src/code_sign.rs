//! AJ-E3013 (security review M1): whether a program is signed by a given
//! Apple developer team, as the system's own `codesign` checks it.
//!
//! `codesign --verify --strict` with a requirement that the signature
//! chains to Apple's anchor and its leaf certificate's team
//! (`subject.OU`) is the one named: no `unsafe`, no Security framework
//! binding. The verdict is kept per file (its path, inode, size and
//! modification time), so a program is checked once, not on every call;
//! a replaced file is checked again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// What a verdict is kept by: the file as it is on disk.
type Key = (PathBuf, u64, u64, i128);

fn verdicts() -> &'static Mutex<HashMap<Key, bool>> {
    static VERDICTS: OnceLock<Mutex<HashMap<Key, bool>>> = OnceLock::new();
    VERDICTS.get_or_init(Mutex::default)
}

/// Whether `team` is a team identifier (ten capital letters or digits):
/// nothing else is ever put in a requirement.
#[must_use]
pub fn is_team_id(team: &str) -> bool {
    team.len() == 10
        && team
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// The file's identity for the cache: inode, size, modification time.
#[cfg(unix)]
fn key_of(path: &Path) -> Option<Key> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).ok()?;
    let mtime = i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec());
    Some((path.to_path_buf(), meta.ino(), meta.size(), mtime))
}

#[cfg(not(unix))]
fn key_of(_path: &Path) -> Option<Key> {
    None
}

/// Whether the program at `path` is validly signed by Apple developer
/// team `team` (`codesign --verify --strict`, Apple's anchor, the leaf's
/// team). `false` for anything else, a team id of another shape, or where
/// the system has no `codesign` (not macOS).
#[must_use]
pub fn signed_by_team(path: &Path, team: &str) -> bool {
    if !is_team_id(team) || !cfg!(target_os = "macos") {
        return false;
    }
    let Some(key) = key_of(path) else {
        return false;
    };
    if let Some(known) = verdicts()
        .lock()
        .ok()
        .and_then(|verdicts| verdicts.get(&key).copied())
    {
        return known;
    }
    let requirement = format!("anchor apple generic and certificate leaf[subject.OU] = \"{team}\"");
    let verdict = crate::process::helper("/usr/bin/codesign")
        .arg("--verify")
        .arg("--strict")
        .arg(format!("-R={requirement}"))
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    // Kept only while the file is the one checked.
    if key_of(path).as_ref() == Some(&key)
        && let Ok(mut verdicts) = verdicts().lock()
    {
        verdicts.insert(key, verdict);
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_team_id_is_ever_put_in_a_requirement() {
        assert!(is_team_id("Q6L2SF6YDW"));
        for bad in ["", "q6l2sf6ydw", "Q6L2SF6YD", "Q6L2SF6YDW\"", "A\" or 1"] {
            assert!(!is_team_id(bad), "{bad}");
            assert!(!signed_by_team(Path::new("/bin/ls"), bad));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn another_teams_program_or_none_isnt_signed_by_the_team() {
        // Apple's own tool: validly signed, but not by this team.
        assert!(!signed_by_team(Path::new("/bin/ls"), "Q6L2SF6YDW"));
        assert!(!signed_by_team(
            Path::new("/nonexistent/claude"),
            "Q6L2SF6YDW"
        ));
        // A program no one signed (a script).
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("claude");
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        assert!(!signed_by_team(&script, "Q6L2SF6YDW"));
        // The verdict is kept for the file as it was.
        assert_eq!(
            verdicts()
                .lock()
                .unwrap()
                .get(&key_of(&script).unwrap())
                .copied(),
            Some(false)
        );
    }
}
