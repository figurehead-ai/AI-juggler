//! AJ-E1542: copying a skill, a command or a rules file into another
//! account, as new files only, with a record to undo it.
//!
//! Nothing is ever replaced: a target that already has the file (or the
//! folder) is refused whole, before anything is written. Each created
//! file's hash goes into the undo record, so undoing removes only files
//! that are still exactly as they were made. Links are not followed, and a
//! folder bigger than [`MAX_FILES`] files or [`MAX_BYTES`] is refused.

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::sha256_hex;
use juggler_adapter::inventory::{Applied, Undo};

/// The most files a copied folder may hold.
pub const MAX_FILES: usize = 500;

/// The most bytes a copied folder may hold.
pub const MAX_BYTES: u64 = 50 * 1024 * 1024;

/// Copies one file to `to`, passing text files through `transform`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] when `to` exists; the read's or write's
/// error.
pub fn copy_file(
    from: &Path,
    to: &Path,
    transform: impl Fn(&str) -> String,
) -> Result<Applied, AdapterError> {
    if to.symlink_metadata().is_ok() {
        return Err(refused(to));
    }
    let bytes = std::fs::read(from)?;
    let bytes = transformed(bytes, &transform);
    let mut dirs = Vec::new();
    make_parents(to, &mut dirs)?;
    write_new(to, &bytes)?;
    Ok(Applied {
        summary: format!("Created {}", display(to)),
        undo: Undo::RemoveCreated {
            files: vec![(to.to_path_buf(), sha256_hex(&bytes))],
            dirs,
        },
    })
}

/// Copies the folder `from` to the new folder `to`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] when `to` exists or `from` is too big;
/// the read's or write's error (anything written is removed again).
pub fn copy_tree(
    from: &Path,
    to: &Path,
    transform: impl Fn(&str) -> String,
) -> Result<Applied, AdapterError> {
    if to.symlink_metadata().is_ok() {
        return Err(refused(to));
    }
    let mut files = Vec::new();
    juggler_adapter::handover::tree_files(from, "", 6, &mut files);
    if files.len() > MAX_FILES {
        return Err(AdapterError::Unsupported(format!(
            "{} holds more than {MAX_FILES} files",
            display(from)
        )));
    }
    let mut total: u64 = 0;
    let mut planned = Vec::new();
    for (name, path) in files {
        let bytes = std::fs::read(&path)?;
        total += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if total > MAX_BYTES {
            return Err(AdapterError::Unsupported(format!(
                "{} is too big to copy",
                display(from)
            )));
        }
        planned.push((
            to.join(name.trim_start_matches('/')),
            transformed(bytes, &transform),
        ));
    }
    let mut dirs = Vec::new();
    let mut created = Vec::new();
    for (path, bytes) in &planned {
        let wrote = make_parents(path, &mut dirs).and_then(|()| write_new(path, bytes));
        if let Err(error) = wrote {
            for (done, _) in &created {
                let _ = std::fs::remove_file(done);
            }
            for dir in dirs.iter().rev() {
                let _ = std::fs::remove_dir(dir);
            }
            return Err(error);
        }
        created.push((path.clone(), sha256_hex(bytes)));
    }
    Ok(Applied {
        summary: format!("Created {}", display(to)),
        undo: Undo::RemoveCreated {
            files: created,
            dirs,
        },
    })
}

/// Text through `transform`; anything that isn't UTF-8 as it is.
fn transformed(bytes: Vec<u8>, transform: &impl Fn(&str) -> String) -> Vec<u8> {
    match String::from_utf8(bytes) {
        Ok(text) => transform(&text).into_bytes(),
        Err(not_text) => not_text.into_bytes(),
    }
}

/// Makes `path`'s missing parent folders, recording the ones it made.
fn make_parents(path: &Path, made: &mut Vec<PathBuf>) -> Result<(), AdapterError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let mut missing = Vec::new();
    let mut at = parent;
    while at.symlink_metadata().is_err() {
        missing.push(at.to_path_buf());
        match at.parent() {
            Some(up) => at = up,
            None => break,
        }
    }
    for dir in missing.into_iter().rev() {
        std::fs::create_dir(&dir)?;
        made.push(dir);
    }
    Ok(())
}

/// Writes a new file: refuses one that appeared meanwhile.
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), AdapterError> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                refused(path)
            } else {
                AdapterError::from(error)
            }
        })?;
    file.write_all(bytes)?;
    Ok(())
}

fn refused(path: &Path) -> AdapterError {
    AdapterError::Unsupported(format!(
        "{} is already there, so nothing was replaced",
        display(path)
    ))
}

fn display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_copied_as_new_files_and_never_over_old_ones() {
        let work = tempfile::tempdir().unwrap();
        let from = work.path().join("skill");
        std::fs::create_dir_all(from.join("refs")).unwrap();
        std::fs::write(from.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        std::fs::write(from.join("refs/a.md"), "a").unwrap();
        let to = work.path().join("target/skills/x");
        let applied = copy_tree(&from, &to, str::to_owned).unwrap();
        assert_eq!(std::fs::read_to_string(to.join("refs/a.md")).unwrap(), "a");
        assert!(
            matches!(&applied.undo, Undo::RemoveCreated { files, dirs }
                if files.len() == 2 && dirs.contains(&work.path().join("target"))),
            "the undo removes the two files and the folders it made"
        );
        assert!(
            copy_tree(&from, &to, str::to_owned).is_err(),
            "never over an existing folder"
        );
        assert!(copy_file(&from.join("SKILL.md"), &to.join("SKILL.md"), str::to_owned).is_err());
    }
}
