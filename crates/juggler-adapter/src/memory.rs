//! D-51: a project's memory, the notes a tool keeps for itself about one
//! project (Claude Code's auto memory: an index, `MEMORY.md`, with a line
//! per note, and one Markdown file per note).
//!
//! An adapter that keeps such notes says so
//! ([`crate::HarnessAdapter::keeps_project_memory`]), says where they are,
//! and lists, reads and writes them through this module, which holds the
//! rules every tool shares:
//!
//! - Only plain note files directly in the notes folder are read: no
//!   links (the folder, the folders above it inside the account, or a
//!   note), no hidden files, no subfolders, no `..`, nothing outside it,
//!   and at most [`MAX_NOTES`] notes of [`MAX_NOTE_BYTES`] each.
//! - A copy adds only what the other account lacks, as new files only this
//!   user can read. A note the other account has with different words is a
//!   conflict: kept as it is, and said so. The index gets a line for each
//!   note it doesn't list yet, never one twice.
//! - The undo removes exactly what was added: the notes while they are
//!   still as they were made, and the index lines while they are there.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::AdapterError;
use crate::handover::sha256_hex;
use crate::inventory::{Applied, Undo};

/// The most notes read from one project's folder.
pub const MAX_NOTES: usize = 200;

/// The biggest note (or index) read, in bytes.
pub const MAX_NOTE_BYTES: u64 = 256 * 1024;

/// One note: its file name in the notes folder, and its text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Note {
    /// The file name (`project-codename.md`), a plain name.
    pub name: String,
    /// What it says.
    pub text: String,
}

/// One account's notes for one project.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProjectMemory {
    /// The notes folder.
    pub folder: PathBuf,
    /// The index file's name in it (`MEMORY.md`).
    pub index_name: String,
    /// The index's text, when there is one.
    pub index: Option<String>,
    /// The notes, by name, the index aside.
    pub notes: Vec<Note>,
}

/// What a copy of one account's notes to another would do.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MemoryPlan {
    /// The notes the other account lacks: copied.
    pub add: Vec<String>,
    /// The notes it has with different words: kept as they are.
    pub conflicts: Vec<String>,
    /// The notes it has, word for word.
    pub same: Vec<String>,
}

/// What a copy did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MemoryAdded {
    /// The record of it, with how to undo it.
    pub applied: Applied,
    /// The notes copied.
    pub added: Vec<String>,
    /// The notes kept as they were, because they differ.
    pub conflicts: Vec<String>,
}

/// Whether `name` is a note's file name: one plain Markdown file name (no
/// folder, no `..`, not hidden, no `\` or NUL).
#[must_use]
pub fn is_note_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.contains(['/', '\\', '\0'])
        && Path::new(name).components().count() == 1
        && name.to_ascii_lowercase().ends_with(".md")
}

/// The note a line of the index links to (`- [Codename](project-codename.md) — …`),
/// when it links to one.
#[must_use]
pub fn link_of(line: &str) -> Option<&str> {
    let at = line.find("](")?;
    let rest = &line[at + 2..];
    let name = rest[..rest.find(')')?].trim();
    let name = name.strip_prefix("./").unwrap_or(name);
    is_note_name(name).then_some(name)
}

/// What copying `source`'s notes to an account whose notes are `target`
/// would do.
#[must_use]
pub fn plan(source: &ProjectMemory, target: Option<&ProjectMemory>) -> MemoryPlan {
    let mut out = MemoryPlan::default();
    for note in &source.notes {
        let theirs = target.and_then(|target| target.notes.iter().find(|n| n.name == note.name));
        match theirs {
            None => out.add.push(note.name.clone()),
            Some(theirs) if theirs.text == note.text => out.same.push(note.name.clone()),
            Some(_) => out.conflicts.push(note.name.clone()),
        }
    }
    out
}

/// The lines of `source`'s index to add to `target`'s: each line that
/// links to a note the target has (`on_target`) and that the target's
/// index doesn't list yet, once.
#[must_use]
pub fn index_lines_to_add(
    source: Option<&str>,
    target: Option<&str>,
    on_target: &BTreeSet<&str>,
) -> Vec<String> {
    let target = target.unwrap_or_default();
    let listed: BTreeSet<&str> = target.lines().filter_map(link_of).collect();
    let lines: BTreeSet<&str> = target.lines().map(str::trim).collect();
    let mut taken = BTreeSet::new();
    let mut out = Vec::new();
    for line in source.unwrap_or_default().lines() {
        let line = line.trim_end();
        let Some(name) = link_of(line) else { continue };
        if on_target.contains(name)
            && !listed.contains(name)
            && !lines.contains(line.trim())
            && taken.insert(name.to_owned())
        {
            out.push(line.to_owned());
        }
    }
    out
}

/// Whether every folder from `root` down to `folder` that is there is a
/// real folder, not a link, and `folder` is inside `root`.
fn confined(root: &Path, folder: &Path) -> bool {
    let Ok(rest) = folder.strip_prefix(root) else {
        return false;
    };
    let mut at = root.to_path_buf();
    for part in rest.components() {
        let std::path::Component::Normal(part) = part else {
            return false;
        };
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => return false,
            Ok(_) => {}
            Err(_) => return true,
        }
    }
    true
}

/// A plain file's text, when it is one (not a link), not too big, and
/// readable as text.
fn read_plain(path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.file_type().is_file() || meta.len() > MAX_NOTE_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// The notes in `folder`, a folder inside the account folder `root`.
/// `None` when there's no such folder, or it (or a folder above it inside
/// `root`) is a link. Reads only.
#[must_use]
pub fn read(root: &Path, folder: &Path, index_name: &str) -> Option<ProjectMemory> {
    if !confined(root, folder) || !folder.is_dir() {
        return None;
    }
    let mut notes: Vec<Note> = std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name == index_name || !is_note_name(&name) {
                return None;
            }
            let text = read_plain(&entry.path())?;
            Some(Note { name, text })
        })
        .collect();
    notes.sort_by(|a, b| a.name.cmp(&b.name));
    notes.truncate(MAX_NOTES);
    Some(ProjectMemory {
        folder: folder.to_path_buf(),
        index_name: index_name.to_owned(),
        index: read_plain(&folder.join(index_name)),
        notes,
    })
}

/// The folders from `root` down to `folder` that aren't there, made
/// closed to others; the ones made, top first.
fn make_folders(root: &Path, folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut made = Vec::new();
    let mut at = root.to_path_buf();
    for part in folder.strip_prefix(root).unwrap_or(folder).components() {
        at.push(part);
        if std::fs::symlink_metadata(&at).is_ok() {
            continue;
        }
        if let Err(error) = juggler_platform::private_fs::create_private_dir(&at) {
            for dir in made.iter().rev() {
                let _ = std::fs::remove_dir(dir);
            }
            return Err(error);
        }
        made.push(at.clone());
    }
    Ok(made)
}

fn refused(message: String) -> AdapterError {
    AdapterError::Unsupported(message)
}

/// What adding lines to an index did.
struct IndexAdded {
    /// The index, when lines went into it.
    path: Option<PathBuf>,
    /// Whether it was made for them.
    created: bool,
    /// The lines added.
    lines: Vec<String>,
    /// Why nothing was added, when it wasn't.
    note: Option<String>,
}

/// Adds `lines` to the end of the index at `index`, made private if it
/// isn't there; one that isn't a plain file (a link) is left as it is.
fn add_index_lines(
    index: &Path,
    index_name: &str,
    lines: Vec<String>,
) -> std::io::Result<IndexAdded> {
    let none = |note: Option<String>| IndexAdded {
        path: None,
        created: false,
        lines: Vec::new(),
        note,
    };
    if lines.is_empty() {
        return Ok(none(None));
    }
    let (now, created) = match std::fs::symlink_metadata(index) {
        Err(_) => (String::new(), true),
        Ok(meta) if meta.file_type().is_file() => (std::fs::read_to_string(index)?, false),
        Ok(_) => {
            return Ok(none(Some(format!(
                "{index_name} isn't a plain file, so it wasn't changed"
            ))));
        }
    };
    let gap = if now.is_empty() || now.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let text = format!("{now}{gap}{}\n", lines.join("\n"));
    juggler_platform::private_fs::write_private(index, text.as_bytes())?;
    Ok(IndexAdded {
        path: Some(index.to_path_buf()),
        created,
        lines,
        note: None,
    })
}

/// Copies `from`'s notes that the notes folder `folder` (inside the
/// account folder `root`) lacks, as new private files, and lists them in
/// its index `index_name`. A note it has with other words stays as it is
/// (a conflict). The result says how to undo exactly this.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] when the folder isn't confined to the
/// account (a link on the way), or there's nothing to copy; the write's
/// error, after taking back what was written.
pub fn add(
    root: &Path,
    folder: &Path,
    index_name: &str,
    from: &ProjectMemory,
) -> Result<MemoryAdded, AdapterError> {
    if !root.is_dir() || !confined(root, folder) || !is_note_name(index_name) {
        return Err(refused(format!(
            "{} isn't a plain folder of the account, so nothing was copied",
            folder.display()
        )));
    }
    let target = read(root, folder, index_name);
    let plan = plan(from, target.as_ref());
    if plan.add.is_empty() {
        return Err(refused("it already has these notes".to_owned()));
    }
    let dirs = make_folders(root, folder)?;
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut added = Vec::new();
    let mut conflicts = plan.conflicts.clone();
    let undo_writes = |files: &[(PathBuf, String)]| {
        for (path, _) in files {
            let _ = std::fs::remove_file(path);
        }
        for dir in dirs.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    };
    for name in &plan.add {
        let Some(note) = from.notes.iter().find(|note| &note.name == name) else {
            continue;
        };
        let path = folder.join(name);
        // Made since it was read: theirs now, kept as it is.
        if !is_note_name(name) || std::fs::symlink_metadata(&path).is_ok() {
            conflicts.push(name.clone());
            continue;
        }
        if let Err(error) = juggler_platform::private_fs::write_private(&path, note.text.as_bytes())
        {
            undo_writes(&files);
            return Err(error.into());
        }
        files.push((path, sha256_hex(note.text.as_bytes())));
        added.push(name.clone());
    }
    let on_target: BTreeSet<&str> = added.iter().chain(&plan.same).map(String::as_str).collect();
    let lines = index_lines_to_add(
        from.index.as_deref(),
        target.as_ref().and_then(|t| t.index.as_deref()),
        &on_target,
    );
    let index = match add_index_lines(&folder.join(index_name), index_name, lines) {
        Ok(index) => index,
        Err(error) => {
            undo_writes(&files);
            return Err(error.into());
        }
    };
    let mut notes = Vec::new();
    if !conflicts.is_empty() {
        notes.push(format!("kept as it is: {}", conflicts.join(", ")));
    }
    notes.extend(index.note);
    Ok(MemoryAdded {
        applied: Applied {
            summary: format!("Copied {} notes into {}", added.len(), folder.display()),
            undo: Undo::RemoveNotes {
                files,
                dirs,
                index: index.path,
                lines: index.lines,
                index_created: index.created,
            },
            note: (!notes.is_empty()).then(|| notes.join("; ")),
        },
        added,
        conflicts,
    })
}

/// Undoes [`add`]: takes the added lines out of the index (each while it's
/// there), and removes the notes it made (all of them only while each is
/// still as it was made), the index too when it made it and nothing else is
/// in it now, and the folders it made when empty.
///
/// # Errors
///
/// What stops it, in words: a note changed since (then nothing is
/// removed), an index that is a link now, or the write's error.
pub fn undo(
    files: &[(PathBuf, String)],
    dirs: &[PathBuf],
    index: Option<&Path>,
    lines: &[String],
    index_created: bool,
) -> Result<(), String> {
    let changed = files.iter().find(|(path, sha)| {
        std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
            || std::fs::read(path).is_ok_and(|bytes| sha256_hex(&bytes) != *sha)
    });
    if let Some((path, _)) = changed {
        return Err(format!(
            "{} has changed since it was copied, so it was left as it is",
            path.display()
        ));
    }
    if let Some(index) = index {
        match std::fs::symlink_metadata(index) {
            Err(_) => {}
            Ok(meta) if !meta.file_type().is_file() => {
                return Err(format!(
                    "{} has been replaced since, so it was left as it is",
                    index.display()
                ));
            }
            Ok(_) => {
                let now = std::fs::read_to_string(index).map_err(|e| e.to_string())?;
                let left = without_lines(&now, lines);
                if index_created && left.trim().is_empty() {
                    let _ = std::fs::remove_file(index);
                } else if left != now {
                    juggler_platform::private_fs::write_private(index, left.as_bytes())
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    for (path, _) in files {
        let _ = std::fs::remove_file(path);
    }
    for dir in dirs.iter().rev() {
        let _ = std::fs::remove_dir(dir);
    }
    Ok(())
}

/// `text` without each of `lines` (the first whole line equal to each).
fn without_lines(text: &str, lines: &[String]) -> String {
    let mut left: Vec<&str> = text.split_inclusive('\n').collect();
    for line in lines {
        if let Some(at) = left
            .iter()
            .position(|kept| kept.trim_end_matches(['\n', '\r']) == line)
        {
            left.remove(at);
        }
    }
    left.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(dir: &Path, index: Option<&str>, notes: &[(&str, &str)]) -> ProjectMemory {
        ProjectMemory {
            folder: dir.to_path_buf(),
            index_name: "MEMORY.md".into(),
            index: index.map(str::to_owned),
            notes: notes
                .iter()
                .map(|(name, text)| Note {
                    name: (*name).into(),
                    text: (*text).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn note_names_are_plain_markdown_files() {
        for good in ["a.md", "project-codename.md", "Notes.MD"] {
            assert!(is_note_name(good), "{good}");
        }
        for bad in [
            "",
            "..",
            ".hidden.md",
            "../x.md",
            "sub/x.md",
            "x.txt",
            "a\\b.md",
            "x\0.md",
        ] {
            assert!(!is_note_name(bad), "{bad:?}");
        }
        assert_eq!(
            link_of("- [Codename](project-codename.md) — the name"),
            Some("project-codename.md")
        );
        assert_eq!(link_of("- [Up](../secret.md)"), None);
        assert_eq!(link_of("- [Here](./a.md)"), Some("a.md"));
        assert_eq!(link_of("# Memory"), None);
    }

    #[test]
    fn a_plan_copies_what_is_missing_and_keeps_what_differs() {
        let dir = Path::new("/x");
        let from = memory(dir, None, &[("a.md", "A"), ("b.md", "B"), ("c.md", "C")]);
        let to = memory(dir, None, &[("b.md", "B"), ("c.md", "theirs")]);
        let plan = plan(&from, Some(&to));
        assert_eq!(plan.add, vec!["a.md"]);
        assert_eq!(plan.same, vec!["b.md"]);
        assert_eq!(plan.conflicts, vec!["c.md"]);
        assert_eq!(super::plan(&from, None).add.len(), 3);
    }

    #[test]
    fn index_lines_are_merged_without_repeats() {
        let source =
            "# Memory\n- [A](a.md) — one\n- [B](b.md) — two\n- [A again](a.md)\n- [C](c.md)\n";
        let target = "- [B, theirs](b.md) — listed already\n";
        let on: BTreeSet<&str> = ["a.md", "b.md"].into_iter().collect();
        assert_eq!(
            index_lines_to_add(Some(source), Some(target), &on),
            vec!["- [A](a.md) — one".to_owned()]
        );
        assert!(index_lines_to_add(Some(source), Some(source), &on).is_empty());
        assert_eq!(
            without_lines("x\n- [A](a.md) — one\ny\n", &["- [A](a.md) — one".into()]),
            "x\ny\n"
        );
    }
}
