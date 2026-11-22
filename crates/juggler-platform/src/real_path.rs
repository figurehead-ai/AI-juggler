//! A path's one real spelling (AJ-E2315), so that two spellings of one
//! folder decide the same way wherever AI Juggler asks "is this that
//! folder" or "is this inside it": a link, `.` and `..`, a trailing slash
//! and (where the volume ignores case, as APFS does by default) letter
//! case.
//!
//! - macOS and Linux: `realpath(3)` (through [`std::fs::canonicalize`]):
//!   links followed, `.` and `..` resolved as the system resolves them,
//!   and on macOS each part in the case it has on disk.
//! - Windows: the same call (`GetFinalPathNameByHandleW`).
//!
//! A part that doesn't exist (yet, or any more) can't be asked about: it
//! is kept as given after its deepest folder that does, with `.` and `..`
//! resolved by name. Comparing such paths is only as good as their
//! spelling; nothing here pretends otherwise.

use std::path::{Component, Path, PathBuf};

/// `path` in its one real spelling (see the module's notes).
#[must_use]
pub fn canonical(path: &Path) -> PathBuf {
    if let Ok(real) = std::fs::canonicalize(path) {
        return real;
    }
    let plain = lexical(path);
    let mut existing = plain.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(real) = std::fs::canonicalize(existing) {
            return rest.iter().rev().fold(real, |path, part| path.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            _ => return plain,
        }
    }
}

/// `path` with `.` and `..` resolved by name and no repeated or trailing
/// separator, without asking the system: for a path the system gave in its
/// real spelling already (a process's executable), or one that isn't
/// there.
#[must_use]
pub fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                // Above the root is the root; a relative path keeps it.
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(Component::ParentDir),
            },
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings_by_name_resolve_alike() {
        for spelling in [
            "/Users/you/P/work",
            "/Users/you/P/work/",
            "/Users/you/P/./work",
            "/Users/you/P/x/../work",
            "/Users/you//P/work",
            "/../Users/you/P/work",
        ] {
            assert_eq!(
                lexical(Path::new(spelling)),
                Path::new("/Users/you/P/work"),
                "{spelling}"
            );
        }
        assert_eq!(lexical(Path::new("../a/./b/..")), Path::new("../a"));
    }

    #[test]
    fn a_link_and_a_missing_part_resolve_to_the_real_folder() {
        let root = tempfile::tempdir().unwrap();
        let real = canonical(root.path());
        let work = real.join("P/work");
        std::fs::create_dir_all(&work).unwrap();
        assert_eq!(canonical(&work), work);
        assert_eq!(canonical(&real.join("P/work/")), work);
        assert_eq!(canonical(&real.join("P/x/../work")), work);
        // A part that isn't there: kept after the real folder above it.
        assert_eq!(
            canonical(&real.join("P/./gone/../work/later")),
            work.join("later")
        );
        #[cfg(unix)]
        {
            let link = real.join("link");
            std::os::unix::fs::symlink(&work, &link).unwrap();
            assert_eq!(canonical(&link), work);
            assert_eq!(canonical(&link.join("later")), work.join("later"));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn letter_case_resolves_as_the_disk_has_it_where_the_volume_ignores_case() {
        let root = tempfile::tempdir().unwrap();
        let real = canonical(root.path());
        let work = real.join("Work");
        std::fs::create_dir_all(&work).unwrap();
        let other_case = real.join("WORK");
        // Only on a volume that ignores case (APFS's default).
        if other_case.exists() {
            assert_eq!(canonical(&other_case), work);
        }
    }
}
