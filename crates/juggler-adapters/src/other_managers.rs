//! AJ-E1519 — what other session managers know that the tools' own files
//! don't: Claude Squad (`~/.claude-squad/`) and Vibe Kanban (its app data
//! folder).
//!
//! - **Which repository each of their worktrees belongs to**, so a worktree
//!   they've since removed still folds into its project rather than
//!   becoming a project of its own.
//! - **The config folders their profiles point Claude Code or Codex at**
//!   (`CLAUDE_CONFIG_DIR` / `CODEX_HOME`), offered as accounts.
//!
//! Read-only, and only those fields. The JSON files are read into types
//! that hold nothing else, so no other value (another variable in a
//! profile's `env`, a session's prompt) is ever kept; the database is
//! opened read-only and immutable, with its columns named, and a schema
//! it doesn't recognise reads as nothing. Vibe Kanban's sign-in and key
//! files (`credentials.json`, `relay_host_credentials.json`,
//! `server_ed25519_signing_key`, and `config.json`, which holds its GitHub
//! token) are never opened.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use juggler_adapter::environment::AliasHint;
use serde::Deserialize;

use crate::scan_signals::{ALIAS_VARIABLES, expand_value};

/// Claude Squad's name, as the UI says it.
pub const CLAUDE_SQUAD: &str = "Claude Squad";
/// Vibe Kanban's name, as the UI says it.
pub const VIBE_KANBAN: &str = "Vibe Kanban";

/// A settings file larger than this isn't one of theirs: it's not read.
const MAX_SETTINGS_BYTES: u64 = 8 * 1024 * 1024;

/// One worktree another manager made, and the repository it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedWorktree {
    /// The manager that made it ([`CLAUDE_SQUAD`] / [`VIBE_KANBAN`]).
    pub manager: &'static str,
    /// The worktree's folder (it may no longer exist).
    pub worktree: PathBuf,
    /// Its repository.
    pub repo: PathBuf,
}

/// Every worktree Claude Squad and Vibe Kanban know of under `home`,
/// whose repository is still on this Mac.
#[must_use]
pub fn worktrees(home: &Path) -> Vec<ManagedWorktree> {
    let mut out = claude_squad_worktrees(home);
    out.extend(vibe_kanban_worktrees(home));
    out
}

/// The `CLAUDE_CONFIG_DIR` / `CODEX_HOME` folders Claude Squad's and Vibe
/// Kanban's profiles set, when the folder exists; one hint per folder.
#[must_use]
pub fn aliases(home: &Path) -> Vec<AliasHint> {
    let mut out = claude_squad_aliases(home);
    out.extend(vibe_kanban_aliases(home));
    out.sort_by(|a, b| (&a.variable, &a.path).cmp(&(&b.variable, &b.path)));
    out.dedup_by(|a, b| a.variable == b.variable && a.path == b.path);
    out
}

/// A settings file's text, or `None` when it's missing, unreadable or
/// too large to be one.
fn settings_text(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_SETTINGS_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

// --- Claude Squad --------------------------------------------------------------------

/// `~/.claude-squad/state.json`: its sessions, of which only each
/// worktree's two folders are read.
#[derive(Deserialize)]
struct SquadState {
    #[serde(default)]
    instances: Vec<SquadInstance>,
}

#[derive(Deserialize)]
struct SquadInstance {
    #[serde(default)]
    worktree: SquadWorktree,
}

#[derive(Deserialize, Default)]
struct SquadWorktree {
    #[serde(default)]
    repo_path: String,
    #[serde(default)]
    worktree_path: String,
}

/// `~/.claude-squad/config.json`: only the programs it starts.
#[derive(Deserialize)]
struct SquadConfig {
    #[serde(default)]
    default_program: String,
    #[serde(default)]
    profiles: Vec<SquadProfile>,
}

#[derive(Deserialize)]
struct SquadProfile {
    #[serde(default)]
    program: String,
}

fn claude_squad_worktrees(home: &Path) -> Vec<ManagedWorktree> {
    let Some(text) = settings_text(&home.join(".claude-squad").join("state.json")) else {
        return Vec::new();
    };
    let Ok(state) = serde_json::from_str::<SquadState>(&text) else {
        return Vec::new(); // not the shape we know: nothing, never a guess
    };
    state
        .instances
        .into_iter()
        .filter_map(|instance| {
            managed(
                CLAUDE_SQUAD,
                Path::new(&instance.worktree.worktree_path),
                Path::new(&instance.worktree.repo_path),
            )
        })
        .collect()
}

fn claude_squad_aliases(home: &Path) -> Vec<AliasHint> {
    let file = home.join(".claude-squad").join("config.json");
    let Some(text) = settings_text(&file) else {
        return Vec::new();
    };
    let Ok(config) = serde_json::from_str::<SquadConfig>(&text) else {
        return Vec::new();
    };
    std::iter::once(config.default_program)
        .chain(config.profiles.into_iter().map(|profile| profile.program))
        .flat_map(|program| program_assignments(&program, home))
        .filter_map(|(variable, path)| hint(CLAUDE_SQUAD, &file, variable, path))
        .collect()
}

/// The leading `VAR=value` words of a program Claude Squad starts (it runs
/// through the shell, so `CLAUDE_CONFIG_DIR=~/work claude` sets it), for
/// the two known variables only. Nothing else is interpreted.
fn program_assignments(program: &str, home: &Path) -> Vec<(String, PathBuf)> {
    let mut words = program.split_whitespace().peekable();
    if words.peek() == Some(&"env") {
        words.next();
    }
    let mut out = Vec::new();
    for word in words {
        let Some((name, value)) = word.split_once('=') else {
            break; // the command itself: assignments only come first
        };
        if ALIAS_VARIABLES.contains(&name)
            && let Some(path) = expand_value(value, home)
        {
            out.push((name.to_owned(), path));
        }
    }
    out
}

// --- Vibe Kanban ---------------------------------------------------------------------

/// Vibe Kanban's data folder, on macOS, Linux and Windows.
fn vibe_kanban_dirs(home: &Path) -> [PathBuf; 3] {
    [
        home.join("Library/Application Support/ai.bloop.vibe-kanban"),
        home.join(".local/share/vibe-kanban"),
        home.join("AppData/Roaming/bloop/vibe-kanban/data"),
    ]
}

fn vibe_kanban_worktrees(home: &Path) -> Vec<ManagedWorktree> {
    let mut out = Vec::new();
    for dir in vibe_kanban_dirs(home) {
        // The newest database it keeps: `db.v2.sqlite`, else `db.sqlite`.
        let Some(db) = ["db.v2.sqlite", "db.sqlite"]
            .into_iter()
            .map(|name| dir.join(name))
            .find(|db| db.is_file())
        else {
            continue;
        };
        out.extend(vibe_kanban_rows(&db));
    }
    out
}

/// A workspace's worktree is `<container_ref>/<repo name>`, one per
/// repository it works on.
fn vibe_kanban_rows(db: &Path) -> Vec<ManagedWorktree> {
    let uri = format!("file:{}?mode=ro&immutable=1", uri_path(db));
    let Ok(connection) = rusqlite::Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return Vec::new();
    };
    // Folders only, the columns named: a schema drift reads as nothing.
    let rows: rusqlite::Result<Vec<(String, String, String)>> = connection
        .prepare(
            "SELECT w.container_ref, r.path, r.name FROM workspaces w \
             JOIN workspace_repos wr ON wr.workspace_id = w.id \
             JOIN repos r ON r.id = wr.repo_id \
             WHERE w.container_ref IS NOT NULL",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect()
        });
    rows.unwrap_or_default()
        .into_iter()
        .filter_map(|(container, repo, name)| {
            managed(
                VIBE_KANBAN,
                &Path::new(&container).join(name),
                Path::new(&repo),
            )
        })
        .collect()
}

/// `profiles.json`: each tool's variants, of which only the two known
/// variables in each one's `env` are read.
#[derive(Deserialize)]
struct KanbanProfiles {
    #[serde(default)]
    executors: HashMap<String, HashMap<String, HashMap<String, KanbanCommand>>>,
}

#[derive(Deserialize)]
struct KanbanCommand {
    #[serde(default)]
    env: Option<KanbanEnv>,
}

#[derive(Deserialize)]
struct KanbanEnv {
    #[serde(rename = "CLAUDE_CONFIG_DIR")]
    claude: Option<String>,
    #[serde(rename = "CODEX_HOME")]
    codex: Option<String>,
}

fn vibe_kanban_aliases(home: &Path) -> Vec<AliasHint> {
    let mut out = Vec::new();
    for dir in vibe_kanban_dirs(home) {
        let file = dir.join("profiles.json");
        let Some(text) = settings_text(&file) else {
            continue;
        };
        let Ok(profiles) = serde_json::from_str::<KanbanProfiles>(&text) else {
            continue;
        };
        for env in profiles
            .executors
            .into_values()
            .flat_map(HashMap::into_values)
            .flat_map(HashMap::into_values)
            .filter_map(|command| command.env)
        {
            for (variable, value) in [
                (ALIAS_VARIABLES[0], env.claude),
                (ALIAS_VARIABLES[1], env.codex),
            ] {
                if let Some(path) = value.and_then(|value| expand_value(&value, home)) {
                    out.extend(hint(VIBE_KANBAN, &file, variable.to_owned(), path));
                }
            }
        }
    }
    out
}

/// A path as a SQLite `file:` URI's path: `%`, `?` and `#` escaped, so a
/// folder named with them still opens the file it names.
fn uri_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('%', "%25")
        .replace('?', "%3f")
        .replace('#', "%23")
}

// --- shared --------------------------------------------------------------------------

/// A worktree whose repository is a repository on this Mac, its folders
/// absolute; `None` otherwise.
fn managed(manager: &'static str, worktree: &Path, repo: &Path) -> Option<ManagedWorktree> {
    if !worktree.is_absolute() || !repo.is_absolute() || worktree == repo {
        return None;
    }
    let repo = std::fs::canonicalize(repo)
        .ok()
        .filter(|repo| repo.join(".git").exists())?;
    let worktree = std::fs::canonicalize(worktree).unwrap_or_else(|_| worktree.to_path_buf());
    Some(ManagedWorktree {
        manager,
        worktree,
        repo,
    })
}

/// A hint for a folder that exists: one that doesn't isn't a setup to
/// offer, and it isn't the shell's to count.
fn hint(manager: &str, file: &Path, variable: String, path: PathBuf) -> Option<AliasHint> {
    path.is_dir().then(|| AliasHint {
        variable,
        path,
        source_file: file.to_string_lossy().into_owned(),
        line: 0,
        target_exists: true,
        set_by: Some(manager.to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_sets_only_its_leading_known_variables() {
        let home = Path::new("/Users/dev");
        assert_eq!(
            program_assignments("CLAUDE_CONFIG_DIR=~/work claude --model opus", home),
            vec![("CLAUDE_CONFIG_DIR".to_owned(), home.join("work"))]
        );
        assert_eq!(
            program_assignments("env FOO=1 CODEX_HOME='/opt/codex b' codex", home),
            vec![("CODEX_HOME".to_owned(), PathBuf::from("'/opt/codex"))],
            "a quoted path with a space reads wrong and won't exist: never offered"
        );
        assert!(program_assignments("claude CLAUDE_CONFIG_DIR=/x", home).is_empty());
        assert!(program_assignments("claude", home).is_empty());
    }
}
