//! AJ-E1542: copying a skill, a command or a rules file into another
//! account, as new files only, with a record to undo it.
//!
//! Nothing is ever replaced: a target that already has the file (or the
//! folder) is refused whole, before anything is written. Each created
//! file's hash goes into the undo record, so undoing removes only files
//! that are still exactly as they were made. Links are not followed, and a
//! folder bigger than [`MAX_FILES`] files or [`MAX_BYTES`] is refused.
//! Hidden files (`.env`, `.git/`) stay behind, as do links and folders
//! deeper than [`MAX_DEPTH`]; the result names what was left out. Each
//! copy keeps its file's permissions (a skill's script stays runnable).

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::handover::sha256_hex;
use juggler_adapter::inventory::{Applied, McpServer, McpTransport, Undo, Value, secret_variable};

/// The most files a copied folder may hold.
pub const MAX_FILES: usize = 500;

/// The most bytes a copied folder may hold.
pub const MAX_BYTES: u64 = 50 * 1024 * 1024;

/// How deep into a copied folder files are taken.
pub const MAX_DEPTH: usize = 6;

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
    write_new(to, &bytes, from)?;
    Ok(Applied {
        summary: format!("Created {}", display(to)),
        undo: Undo::RemoveCreated {
            files: vec![(to.to_path_buf(), sha256_hex(&bytes))],
            dirs,
        },
        note: None,
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
    juggler_adapter::handover::tree_files(from, "", MAX_DEPTH, &mut files);
    // Review of #132 (M6): a hidden file (`.env`, `.git/`) is not part of
    // a skill, and may hold a secret: it stays behind, named.
    let (hidden, files): (Vec<_>, Vec<_>) = files
        .into_iter()
        .partition(|(name, _)| name.split('/').any(|part| part.starts_with('.')));
    // Named once each, down to the hidden part (`.git/`, not its files).
    let mut left_out: Vec<String> = Vec::new();
    for (name, _) in &hidden {
        let parts: Vec<&str> = name.trim_start_matches('/').split('/').collect();
        let at = parts
            .iter()
            .position(|part| part.starts_with('.'))
            .unwrap_or(0);
        let shown = parts[..=at].join("/") + if at + 1 < parts.len() { "/" } else { "" };
        if !left_out.contains(&shown) {
            left_out.push(shown);
        }
    }
    left_out.extend(unreachable_entries(from, MAX_DEPTH));
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
            path,
        ));
    }
    let mut dirs = Vec::new();
    let mut created = Vec::new();
    for (path, bytes, source) in &planned {
        let wrote = make_parents(path, &mut dirs).and_then(|()| write_new(path, bytes, source));
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
        note: (!left_out.is_empty()).then(|| format!("left out: {}", left_out.join(", "))),
    })
}

/// What a copy of `from` doesn't take besides hidden files: links (never
/// followed) and anything deeper than `depth` folders, by name.
fn unreachable_entries(from: &Path, depth: usize) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue; // named with the hidden files
            }
            let shown = format!("{prefix}{name}");
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                out.push(format!("{shown} (a link)"));
            } else if kind.is_dir() {
                if depth == 0 {
                    out.push(format!("{shown}/ (too deep)"));
                } else {
                    walk(&entry.path(), &format!("{shown}/"), depth - 1, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(from, "", depth, &mut out);
    out
}

/// The user's home folder (shared by every tool's inventory: Codex reads
/// the shared `~/.agents/skills` there).
pub(crate) fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `dir`'s entries (folders when `dirs`, else files), by name, sorted,
/// hidden ones and `.trash` left out.
pub(crate) fn children(dir: &Path, dirs: bool) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| if dirs { kind.is_dir() } else { kind.is_file() })
        })
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .filter(|(name, _)| !name.starts_with('.'))
        .collect();
    out.sort();
    out
}

/// The variables a server's dropped secrets become.
pub(crate) fn secret_names(name: &str, server: &McpServer) -> Vec<String> {
    let pairs = match &server.transport {
        McpTransport::Stdio { env, .. } => env.as_slice(),
        McpTransport::Http { headers, .. } => headers.as_slice(),
        McpTransport::Other { .. } => &[],
    };
    pairs
        .iter()
        .filter(|(_, value)| *value == Value::Secret)
        .map(|(key, _)| secret_variable(name, key))
        .collect()
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

/// Writes a new file with `source`'s permissions: refuses one that
/// appeared meanwhile.
fn write_new(path: &Path, bytes: &[u8], source: &Path) -> Result<(), AdapterError> {
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
    file.set_permissions(std::fs::metadata(source)?.permissions())?;
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

    // Unix modes and links (a Windows link needs a privilege).
    #[cfg(unix)]
    #[test]
    fn hidden_files_links_and_deep_folders_stay_behind_named_and_scripts_stay_runnable() {
        // Review of #132 (M6).
        use std::os::unix::fs::PermissionsExt;
        let work = tempfile::tempdir().unwrap();
        let from = work.path().join("skill");
        std::fs::create_dir_all(from.join(".git/objects")).unwrap();
        std::fs::write(from.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        std::fs::write(from.join(".env"), "TOKEN=sk-secret").unwrap();
        std::fs::write(from.join(".git/objects/a"), "x").unwrap();
        std::fs::write(from.join("run.sh"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(from.join("run.sh"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        std::os::unix::fs::symlink("/etc/hosts", from.join("hosts")).unwrap();
        let deep = (0..=MAX_DEPTH).fold(from.clone(), |at, n| at.join(format!("d{n}")));
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("far.md"), "far").unwrap();

        let to = work.path().join("target/skills/x");
        let applied = copy_tree(&from, &to, str::to_owned).unwrap();
        assert!(to.join("SKILL.md").is_file());
        assert!(!to.join(".env").exists(), "a hidden file stays behind");
        assert!(!to.join(".git").exists());
        assert!(!to.join("hosts").exists(), "a link is never followed");
        let note = applied.note.unwrap();
        assert!(note.contains(".env") && note.contains(".git/"), "{note}");
        assert!(
            note.contains("hosts (a link)") && note.contains("(too deep)"),
            "{note}"
        );
        assert!(
            !note.contains("objects"),
            "named down to the hidden part: {note}"
        );
        let mode = std::fs::metadata(to.join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755, "the script stays runnable");
    }
}
