//! ADR-0028: a Codex conversation's family, the subagent threads it
//! started and theirs, as Codex records them: each child is a rollout of
//! its own whose first line (`session_meta`) names its parent
//! (`parent_thread_id`, or `source.subagent.thread_spawn.parent_thread_id`).
//! The link lives inside the child's own file, so a family moves by
//! carrying the files together: ids kept, or forked all at once.
//!
//! Reading is bounded: one scan of the account's `sessions` folder, the
//! first line of each rollout (a compressed one is decoded only that far),
//! and a depth limit on the tree.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use crate::codex_transcript::thread_id_from_name;

/// How deep a family is followed (Codex nests far less).
const MAX_DEPTH: usize = 16;
/// How much of a rollout's first line is read for its parent.
const HEAD_MAX: u64 = 256 * 1024;

/// One member below the conversation: its rollout, its thread, its parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    /// Its rollout file (`.jsonl` or `.jsonl.zst`).
    pub path: PathBuf,
    /// Its thread id.
    pub thread: String,
    /// The thread that started it.
    pub parent: String,
}

/// The `sessions` folder a rollout lives under (its nearest ancestor of
/// that name), else `None`.
#[must_use]
pub fn sessions_root(rollout: &Path) -> Option<PathBuf> {
    rollout
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "sessions"))
        .map(Path::to_path_buf)
}

/// Every descendant of `thread` under `sessions`, parents before their
/// children, each once.
#[must_use]
pub fn descendants(sessions: &Path, thread: &str) -> Vec<Child> {
    let mut by_parent: HashMap<String, Vec<Child>> = HashMap::new();
    let mut files = Vec::new();
    collect(sessions, &mut files, 0);
    for path in files {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let Some(id) = thread_id_from_name(&name) else {
            continue;
        };
        if let Some(parent) = parent_of(&path) {
            by_parent.entry(parent.clone()).or_default().push(Child {
                path,
                thread: id,
                parent,
            });
        }
    }
    let mut out: Vec<Child> = Vec::new();
    let mut level = vec![thread.to_owned()];
    for _ in 0..MAX_DEPTH {
        let mut next = Vec::new();
        for parent in &level {
            let mut children = by_parent.remove(parent).unwrap_or_default();
            children.sort_by(|a, b| a.path.cmp(&b.path));
            for child in children {
                if child.thread != thread && !out.iter().any(|seen| seen.thread == child.thread) {
                    next.push(child.thread.clone());
                    out.push(child);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        level = next;
    }
    out
}

/// The thread that started the rollout at `path`, from its first line.
fn parent_of(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader: Box<dyn Read> = if crate::transcript::is_compressed(path) {
        Box::new(zstd::stream::Decoder::new(file).ok()?)
    } else {
        Box::new(file)
    };
    let mut line = String::new();
    BufReader::new(reader.take(HEAD_MAX))
        .read_line(&mut line)
        .ok()?;
    let value: serde_json::Value = serde_json::from_str(line.trim_end()).ok()?;
    if value.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    let payload = value.get("payload")?;
    let direct = payload
        .get("parent_thread_id")
        .and_then(serde_json::Value::as_str);
    let spawned = payload
        .pointer("/source/subagent/thread_spawn/parent_thread_id")
        .and_then(serde_json::Value::as_str);
    direct
        .or(spawned)
        .filter(|id| crate::codex_handover::is_thread_id(id))
        .map(str::to_owned)
}

/// Every `rollout-*` file under `dir`, bounded in depth.
fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect(&path, out, depth + 1);
        } else if entry.file_name().to_string_lossy().starts_with("rollout-") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{descendants, sessions_root};
    use std::path::Path;

    const PARENT: &str = "01a1a8ee-6142-7f00-80b0-03ff32c1e702";

    fn id(n: u32) -> String {
        format!("01a1a8ee-0000-7000-8000-{n:012}")
    }

    fn rollout(sessions: &Path, day: &str, thread: &str, parent: Option<&str>, spawned: bool) {
        let dir = sessions.join(day);
        std::fs::create_dir_all(&dir).expect("dir");
        let meta = match (parent, spawned) {
            (Some(p), true) => format!(
                r#"{{"type":"session_meta","payload":{{"id":"{thread}","source":{{"subagent":{{"thread_spawn":{{"parent_thread_id":"{p}"}}}}}}}}}}"#
            ),
            (Some(p), false) => format!(
                r#"{{"type":"session_meta","payload":{{"id":"{thread}","parent_thread_id":"{p}"}}}}"#
            ),
            (None, _) => format!(r#"{{"type":"session_meta","payload":{{"id":"{thread}"}}}}"#),
        };
        std::fs::write(
            dir.join(format!("rollout-2026-11-05T10-00-00-{thread}.jsonl")),
            format!("{meta}\n{{\"type\":\"event_msg\"}}\n"),
        )
        .expect("write");
    }

    #[test]
    fn finds_twenty_children_and_their_own_once_each_parents_first() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sessions = tmp.path().join("sessions");
        rollout(&sessions, "2026/11/05", PARENT, None, false);
        for n in 1..=20 {
            rollout(&sessions, "2026/11/05", &id(n), Some(PARENT), n % 2 == 0);
        }
        // Two levels: a grandchild on the next day.
        rollout(&sessions, "2026/11/06", &id(100), Some(&id(3)), true);
        // Not family: another conversation's child, and a stranger.
        rollout(&sessions, "2026/11/05", &id(200), Some(&id(999)), false);
        rollout(&sessions, "2026/11/05", &id(300), None, false);
        let family = descendants(&sessions, PARENT);
        assert_eq!(family.len(), 21, "{family:#?}");
        let at = |thread: &str| {
            family
                .iter()
                .position(|c| c.thread == thread)
                .expect(thread)
        };
        assert!(at(&id(3)) < at(&id(100)), "a parent before its child");
        assert_eq!(family[at(&id(100))].parent, id(3));
        assert!(
            family
                .iter()
                .all(|c| c.thread != id(200) && c.thread != id(300))
        );
    }

    #[test]
    fn a_lone_conversation_has_none() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sessions = tmp.path().join("sessions");
        rollout(&sessions, "2026/11/05", PARENT, None, false);
        assert!(descendants(&sessions, PARENT).is_empty());
    }

    #[test]
    fn the_sessions_root_is_found_from_a_rollout() {
        let path = Path::new("/h/.codex/sessions/2026/11/05/rollout-x.jsonl");
        assert_eq!(
            sessions_root(path).as_deref(),
            Some(Path::new("/h/.codex/sessions"))
        );
    }
}
