//! A Codex thread's title, in this order:
//!
//! 1. the name Codex keeps for the thread in `<CODEX_HOME>/session_index.jsonl`
//!    (one `{id, thread_name, updated_at}` line per naming, the latest
//!    wins; what `codex resume <name>` takes, docs/research/codex.md);
//! 2. the thread's goal, when the rollout records one
//!    (`event_msg` `thread_goal_updated`, `payload.goal.objective`);
//! 3. the first thing the user typed, never a block Codex or its app
//!    injects ([`crate::codex_transcript::is_injected`]);
//! 4. for a goal with no words of its own, "Codex goal" and the project.
//!
//! Each is cut to its first sentence, then to [`TITLE_MAX`] characters.
//! A name that is itself an injected block is no title.

use std::collections::HashMap;
use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use juggler_adapter::conversation::title_from;

use crate::codex_transcript::is_injected;

/// The longest title kept.
pub const TITLE_MAX: usize = 80;

/// The largest `session_index.jsonl` read (it holds a short line per
/// naming); a bigger one is not read.
const INDEX_MAX: u64 = 16 * 1024 * 1024;

/// A title from free text: its first line, cut after its first sentence,
/// at most [`TITLE_MAX`] characters; `None` for an injected block.
#[must_use]
pub fn title(text: &str) -> Option<String> {
    if is_injected(text) {
        return None;
    }
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let sentence = line
        .match_indices(['.', '?', '!'])
        .find(|(at, _)| line[at + 1..].starts_with(char::is_whitespace))
        .map_or(line, |(at, _)| &line[..=at]);
    title_from(sentence, TITLE_MAX)
}

/// What the rollout says, for its title.
#[derive(Debug, Default)]
pub struct Parts<'a> {
    /// The first thing the user typed (injected blocks already left out).
    pub first_ask: Option<&'a str>,
    /// Whether the thread pursues a goal.
    pub goal: bool,
    /// The goal's objective, when it has words.
    pub objective: Option<&'a str>,
    /// The thread's working folder.
    pub cwd: Option<&'a Path>,
}

/// The title from the rollout alone: its goal, else its first ask, else
/// "Codex goal" and the project for a goal with no words.
#[must_use]
pub fn from_rollout(parts: &Parts<'_>) -> Option<String> {
    parts
        .objective
        .and_then(title)
        .or_else(|| parts.first_ask.and_then(title))
        .or_else(|| {
            parts.goal.then(|| {
                parts.cwd.and_then(Path::file_name).map_or_else(
                    || "Codex goal".to_owned(),
                    |project| format!("Codex goal · {}", project.to_string_lossy()),
                )
            })
        })
}

/// The name Codex keeps for thread `id`, for the rollout at `rollout`
/// (`<home>/sessions/Y/M/D/rollout-….jsonl`): from its home's
/// `session_index.jsonl`, read again only when that file changed.
#[must_use]
pub fn indexed(rollout: &Path, id: &str) -> Option<String> {
    let sessions = rollout.ancestors().nth(4)?;
    if sessions.file_name()? != "sessions" {
        return None;
    }
    let names = index_of(&sessions.parent()?.join("session_index.jsonl"))?;
    names.get(id).and_then(|name| title(name))
}

/// One home's names, as last read.
struct Index {
    len: u64,
    modified: Option<SystemTime>,
    names: Arc<HashMap<String, String>>,
}

/// Every home's names read so far.
static INDEXES: LazyLock<Mutex<HashMap<PathBuf, Index>>> = LazyLock::new(Mutex::default);

/// `path`'s names by thread id (the latest line for each), cached by its
/// size and time.
fn index_of(path: &Path) -> Option<Arc<HashMap<String, String>>> {
    let meta = std::fs::metadata(path).ok()?;
    let (len, modified) = (meta.len(), meta.modified().ok());
    if len > INDEX_MAX {
        return None;
    }
    let mut indexes = INDEXES.lock().ok()?;
    if let Some(known) = indexes.get(path)
        && known.len == len
        && known.modified == modified
    {
        return Some(Arc::clone(&known.names));
    }
    let names = Arc::new(read_index(path));
    indexes.insert(
        path.to_path_buf(),
        Index {
            len,
            modified,
            names: Arc::clone(&names),
        },
    );
    Some(names)
}

/// Reads `session_index.jsonl` field by field: `id` and `thread_name`
/// only; a line of another shape is skipped.
fn read_index(path: &Path) -> HashMap<String, String> {
    let mut names = HashMap::new();
    let Ok(file) = std::fs::File::open(path) else {
        return names;
    };
    for line in std::io::BufReader::new(file).lines() {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let field = |key: &str| value.get(key).and_then(serde_json::Value::as_str);
        if let (Some(id), Some(name)) = (field("id"), field("thread_name")) {
            names.insert(id.to_owned(), name.to_owned());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_the_first_sentence_of_what_the_user_typed() {
        assert_eq!(
            title("Fix the cart total. Then run the tests.\nMore").as_deref(),
            Some("Fix the cart total.")
        );
        assert_eq!(title("  \n  v1.2 is out, ship it").as_deref(), Some("v1.2 is out, ship it"));
        assert_eq!(title("Why? Because").as_deref(), Some("Why?"));
        let long = "word ".repeat(40);
        assert_eq!(title(&long).unwrap().chars().count(), TITLE_MAX + 1);
    }

    #[test]
    fn an_injected_block_is_never_a_title() {
        for injected in [
            "<external_codex_apps_operation>{\"kind\":\"x\"}</external_codex_apps_operation>",
            "<environment_context>\n  <cwd>/w</cwd>\n</environment_context>",
            "<user_instructions>be brief</user_instructions>",
            "<permissions instructions>…</permissions instructions>",
            "<external_codex_apps_op id=\"1\">",
            "# AGENTS.md instructions for /w",
        ] {
            assert_eq!(title(injected), None, "{injected}");
        }
        // Not a tag: the person's own words.
        assert_eq!(title("<3 this fix").as_deref(), Some("<3 this fix"));
        assert_eq!(title("<- use that one").as_deref(), Some("<- use that one"));
    }

    #[test]
    fn a_goal_titles_its_thread_before_its_first_ask() {
        let cwd = Path::new("/Users/you/Dev/shop");
        let parts = Parts {
            first_ask: Some("Look at the build"),
            goal: true,
            objective: Some("Ship the checkout rewrite. Keep tests green."),
            cwd: Some(cwd),
        };
        assert_eq!(
            from_rollout(&parts).as_deref(),
            Some("Ship the checkout rewrite.")
        );
        let wordless = Parts {
            first_ask: None,
            objective: Some("<external_codex_apps_goal>"),
            ..parts
        };
        assert_eq!(
            from_rollout(&wordless).as_deref(),
            Some("Codex goal · shop")
        );
        assert_eq!(from_rollout(&Parts::default()), None);
    }

    #[test]
    fn codex_s_own_name_for_a_thread_is_read_from_its_index() {
        let home = tempfile::tempdir().unwrap();
        let day = home.path().join("sessions/2026/10/25");
        std::fs::create_dir_all(&day).unwrap();
        let id = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b";
        let rollout = day.join(format!("rollout-2026-10-25T08-00-00-{id}.jsonl"));
        let index = home.path().join("session_index.jsonl");
        std::fs::write(
            &index,
            format!(
                "{{\"id\":\"{id}\",\"thread_name\":\"Old name\",\"updated_at\":\"2026-10-25T08:00:00Z\"}}\n\
                 not json\n\
                 {{\"id\":\"{id}\",\"thread_name\":\"Checkout rewrite\",\"updated_at\":\"2026-10-25T09:00:00Z\"}}\n\
                 {{\"id\":\"other\",\"thread_name\":\"<external_codex_apps_x>\"}}\n"
            ),
        )
        .unwrap();
        assert_eq!(indexed(&rollout, id).as_deref(), Some("Checkout rewrite"));
        assert_eq!(indexed(&rollout, "other"), None, "an injected name is none");
        assert_eq!(indexed(&rollout, "missing"), None);
        // Not under `sessions/`: no index to read.
        assert_eq!(indexed(&home.path().join("rollout.jsonl"), id), None);
    }
}
