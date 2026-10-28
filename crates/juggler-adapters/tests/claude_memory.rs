//! D-51: Claude Code keeps notes for each project (its auto memory), and an
//! adapter says so, lists, reads and writes them. A copy adds only what is
//! missing, as private files, never replaces a note that differs, merges
//! the index without repeating a line, stays inside the notes folder, and
//! its undo takes out exactly what it added. Temp folders only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use juggler_adapter::HarnessAdapter;
use juggler_adapter::inventory::Undo;
use juggler_adapter::memory::{self, Note, ProjectMemory};
use juggler_adapters::claude_memory::{INDEX, memory_folder};
use juggler_adapters::{ClaudeCodeAdapter, CodexAdapter};

const INDEX_A: &str = "- [Codename](project-codename.md) — what the project is called\n\
- [Tests](testing.md) — how to run them\n\
- [Style](style.md) — house style\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

/// Account A's notes for `project`: three, and its index.
fn account_a(home: &Path, project: &Path) -> std::path::PathBuf {
    let folder = memory_folder(home, project);
    write(&folder.join(INDEX), INDEX_A);
    write(
        &folder.join("project-codename.md"),
        "The codename is Bluebird.\n",
    );
    write(&folder.join("testing.md"), "Run just test.\n");
    write(&folder.join("style.md"), "Short sentences.\n");
    folder
}

fn undo_of(undo: &Undo) -> Result<(), String> {
    let Undo::RemoveNotes {
        files,
        dirs,
        index,
        lines,
        index_created,
    } = undo
    else {
        panic!("not a notes undo: {undo:?}");
    };
    memory::undo(files, dirs, index.as_deref(), lines, *index_created)
}

#[test]
fn claude_code_keeps_project_notes_and_codex_does_not() {
    let work = tempfile::tempdir().unwrap();
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let a = work.path().join("a");
    account_a(&a, &project);
    let claude = ClaudeCodeAdapter::with_bin("/bin/true");
    assert!(claude.keeps_project_memory());
    let read = claude.project_memory(&a, &project).unwrap();
    let names: Vec<&str> = read.notes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["project-codename.md", "style.md", "testing.md"]);
    assert_eq!(read.index.as_deref(), Some(INDEX_A));
    // Under the folder Claude Code names for the project.
    let slug = project
        .to_string_lossy()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    assert_eq!(read.folder, a.join("projects").join(slug).join("memory"));

    let codex = CodexAdapter::with_bin("/bin/true");
    assert!(!codex.keeps_project_memory());
    assert!(codex.project_memory(&a, &project).is_none());
    assert!(codex.add_project_memory(&a, &project, &read).is_err());
}

#[test]
fn a_copy_adds_what_is_missing_keeps_what_differs_and_undoes_exactly() {
    let work = tempfile::tempdir().unwrap();
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let (a, b) = (work.path().join("a"), work.path().join("b"));
    account_a(&a, &project);
    // B has the same testing note, its own codename note with other words,
    // and an index that lists the codename note.
    let b_folder = memory_folder(&b, &project);
    let b_index = "# B's notes\n- [Codename (B)](project-codename.md) — B's own\n";
    write(&b_folder.join(INDEX), b_index);
    write(
        &b_folder.join("project-codename.md"),
        "The codename is Kestrel.\n",
    );
    write(&b_folder.join("testing.md"), "Run just test.\n");

    let claude = ClaudeCodeAdapter::with_bin("/bin/true");
    let from = claude.project_memory(&a, &project).unwrap();
    let to = claude.project_memory(&b, &project).unwrap();
    let plan = memory::plan(&from, Some(&to));
    assert_eq!(plan.add, ["style.md"]);
    assert_eq!(plan.conflicts, ["project-codename.md"]);
    assert_eq!(plan.same, ["testing.md"]);

    let added = claude.add_project_memory(&b, &project, &from).unwrap();
    assert_eq!(added.added, ["style.md"]);
    assert_eq!(added.conflicts, ["project-codename.md"]);
    // The note that differs is kept, word for word.
    assert_eq!(
        std::fs::read_to_string(b_folder.join("project-codename.md")).unwrap(),
        "The codename is Kestrel.\n"
    );
    assert_eq!(
        std::fs::read_to_string(b_folder.join("style.md")).unwrap(),
        "Short sentences.\n"
    );
    assert_eq!(mode(&b_folder.join("style.md")), 0o600, "written private");
    // The index lists each note once: B's own line for the codename stays,
    // and the lines for the notes it lacked a line for are added.
    let index = std::fs::read_to_string(b_folder.join(INDEX)).unwrap();
    assert_eq!(
        index,
        format!(
            "{b_index}- [Tests](testing.md) — how to run them\n- [Style](style.md) — house style\n"
        )
    );
    assert_eq!(index.matches("project-codename.md").count(), 1);

    // Again: nothing left to copy, and nothing written twice.
    assert!(claude.add_project_memory(&b, &project, &from).is_err());
    assert_eq!(
        std::fs::read_to_string(b_folder.join(INDEX)).unwrap(),
        index
    );

    // Claude adds a note of its own meanwhile; the undo takes out only
    // what the copy added.
    let own_line = "- [Later](later.md) — B's, after the copy\n";
    std::fs::write(b_folder.join(INDEX), format!("{index}{own_line}")).unwrap();
    undo_of(&added.applied.undo).unwrap();
    assert!(!b_folder.join("style.md").exists());
    assert_eq!(
        std::fs::read_to_string(b_folder.join(INDEX)).unwrap(),
        format!("{b_index}{own_line}")
    );
    assert!(b_folder.join("testing.md").is_file(), "B's own note stays");
    assert!(b_folder.join("project-codename.md").is_file());
}

#[test]
fn a_copy_to_an_account_without_notes_makes_private_folders_and_undo_removes_them() {
    let work = tempfile::tempdir().unwrap();
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let (a, b) = (work.path().join("a"), work.path().join("b"));
    account_a(&a, &project);
    std::fs::create_dir_all(&b).unwrap();
    let claude = ClaudeCodeAdapter::with_bin("/bin/true");
    let from = claude.project_memory(&a, &project).unwrap();
    assert!(claude.project_memory(&b, &project).is_none());

    let added = claude.add_project_memory(&b, &project, &from).unwrap();
    assert_eq!(added.added.len(), 3);
    let folder = memory_folder(&b, &project);
    assert_eq!(
        std::fs::read_to_string(folder.join(INDEX)).unwrap(),
        INDEX_A
    );
    for dir in [
        b.join("projects"),
        folder.parent().unwrap().to_path_buf(),
        folder.clone(),
    ] {
        assert_eq!(mode(&dir), 0o700, "{}", dir.display());
    }
    assert_eq!(mode(&folder.join(INDEX)), 0o600);

    undo_of(&added.applied.undo).unwrap();
    assert!(
        !b.join("projects").exists(),
        "every folder the copy made is gone"
    );
}

#[test]
fn a_note_changed_since_the_copy_keeps_the_undo_from_removing_anything() {
    let work = tempfile::tempdir().unwrap();
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let (a, b) = (work.path().join("a"), work.path().join("b"));
    account_a(&a, &project);
    std::fs::create_dir_all(&b).unwrap();
    let claude = ClaudeCodeAdapter::with_bin("/bin/true");
    let from = claude.project_memory(&a, &project).unwrap();
    let added = claude.add_project_memory(&b, &project, &from).unwrap();
    let folder = memory_folder(&b, &project);
    std::fs::write(folder.join("style.md"), "Short sentences. Edited on B.\n").unwrap();
    let why = undo_of(&added.applied.undo).unwrap_err();
    assert!(why.contains("style.md"), "{why}");
    assert!(folder.join("testing.md").is_file(), "nothing was removed");
    assert_eq!(
        std::fs::read_to_string(folder.join(INDEX)).unwrap(),
        INDEX_A
    );
}

#[test]
fn notes_stay_inside_the_notes_folder() {
    let work = tempfile::tempdir().unwrap();
    let project = work.path().join("shop");
    std::fs::create_dir_all(&project).unwrap();
    let (a, b) = (work.path().join("a"), work.path().join("b"));
    let folder = account_a(&a, &project);
    std::fs::create_dir_all(&b).unwrap();
    let outside = work.path().join("outside");
    write(&outside.join("secret.md"), "not a note\n");
    // A link among the notes, a hidden file and a subfolder are not read.
    std::os::unix::fs::symlink(outside.join("secret.md"), folder.join("linked.md")).unwrap();
    write(&folder.join(".hidden.md"), "hidden\n");
    write(&folder.join("sub/deeper.md"), "deeper\n");
    let claude = ClaudeCodeAdapter::with_bin("/bin/true");
    let from = claude.project_memory(&a, &project).unwrap();
    assert_eq!(from.notes.len(), 3, "{:?}", from.notes);

    // A notes folder that is a link isn't read, or written through.
    let c = work.path().join("c");
    let c_folder = memory_folder(&c, &project);
    std::fs::create_dir_all(c_folder.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&outside, &c_folder).unwrap();
    assert!(claude.project_memory(&c, &project).is_none());
    assert!(claude.add_project_memory(&c, &project, &from).is_err());
    assert!(
        !outside.join("style.md").exists(),
        "nothing written through the link"
    );

    // A note named to climb out (only a crafted memory could carry one) is
    // never written.
    let crafted = ProjectMemory {
        folder: folder.clone(),
        index_name: INDEX.to_owned(),
        index: Some("- [Up](../../escape.md)\n".to_owned()),
        notes: vec![
            Note {
                name: "../../escape.md".to_owned(),
                text: "out\n".to_owned(),
            },
            Note {
                name: "ok.md".to_owned(),
                text: "in\n".to_owned(),
            },
        ],
    };
    let added = claude.add_project_memory(&b, &project, &crafted).unwrap();
    assert_eq!(added.added, ["ok.md"]);
    let b_folder = memory_folder(&b, &project);
    assert!(!b_folder.join("../../escape.md").exists());
    assert!(
        !b_folder.join(INDEX).exists(),
        "no line for a note that can't be"
    );
}
