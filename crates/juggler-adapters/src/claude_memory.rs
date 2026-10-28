//! D-51: Claude Code's auto memory, as a project memory
//! ([`juggler_adapter::memory`]).
//!
//! Claude Code keeps the notes it writes for itself about a project in the
//! account's folder, `projects/<project>/memory/`: `MEMORY.md`, an index
//! with a line per note, and one Markdown file per note. `<project>` is
//! the project's folder with every character that isn't a letter or a
//! digit made `-`, and the project is its repository: every worktree and
//! subfolder of one repository shares one memory
//! ([Claude Code: memory](https://code.claude.com/docs/en/memory)).

use std::path::{Path, PathBuf};

use juggler_adapter::error::AdapterError;
use juggler_adapter::memory::{self, MemoryAdded, ProjectMemory};

/// The index's file name.
pub const INDEX: &str = "MEMORY.md";

/// The names Claude Code may give a project's folder under `projects/`:
/// every character not a letter or digit made `-` (what it writes now),
/// then only the path separators (older folders).
#[must_use]
pub fn folder_names(project: &Path) -> Vec<String> {
    let root = juggler_core::storage::project_root(project);
    let text = root.to_string_lossy();
    let mut names = vec![
        text.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>(),
        juggler_adapter::handover::mangled_dir(&text),
    ];
    names.dedup();
    names
}

/// The notes folder for `project` in the account folder `state_dir`: the
/// one that is there, else where Claude Code would make it.
#[must_use]
pub fn memory_folder(state_dir: &Path, project: &Path) -> PathBuf {
    let projects = state_dir.join("projects");
    let names = folder_names(project);
    names
        .iter()
        .map(|name| projects.join(name).join("memory"))
        .find(|folder| folder.is_dir())
        .unwrap_or_else(|| projects.join(&names[0]).join("memory"))
}

/// The account's notes for `project`. Reads only.
#[must_use]
pub fn project_memory(state_dir: &Path, project: &Path) -> Option<ProjectMemory> {
    memory::read(state_dir, &memory_folder(state_dir, project), INDEX)
}

/// Copies `from`'s notes this account lacks for `project`.
///
/// # Errors
///
/// As [`memory::add`].
pub fn add(
    state_dir: &Path,
    project: &Path,
    from: &ProjectMemory,
) -> Result<MemoryAdded, AdapterError> {
    memory::add(state_dir, &memory_folder(state_dir, project), INDEX, from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_is_named_as_claude_code_names_it() {
        let names = folder_names(Path::new("/Users/you/Dev/my.app_x"));
        assert_eq!(
            names,
            vec!["-Users-you-Dev-my-app-x", "-Users-you-Dev-my.app_x"]
        );
        assert_eq!(folder_names(Path::new("/Users/you/Dev/shop")).len(), 1);
    }
}
