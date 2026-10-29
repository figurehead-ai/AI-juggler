//! AJ-E1520 (0.2): Grok Build's subagents, for the Subagents tab, from
//! its session folders (read-only, D-28/D-37).
//!
//! A session's folder (`$GROK_HOME/sessions/<url-encoded cwd>/<id>/`,
//! user guide `17-sessions.md`) holds `subagents/<subagent_id>/meta.json`
//! for each subagent it started (`xai-grok-shell/src/agent/subagent/
//! mod.rs`): `subagent_type`, `description`, `status`
//! (`running`/`completed`/`failed`/`cancelled`; a `running` one its
//! process left behind is later rewritten to `cancelled`), `started_at`,
//! `completed_at`, and `child_session_id` — the subagent's own session, a
//! session folder of its own (under its own cwd's group), which holds the
//! subagents it started in turn.
//!
//! The status is the meta's own word (D-06: an unknown word is unknown).
//! A meta is read again only while it says `running`. Background or not
//! isn't in the meta, so it isn't said.
//!
//! The child's conversation is its own `updates.jsonl` (the ACP update
//! stream), which the session page's reader doesn't read (Grok's own
//! session conversations aren't shown either), so it isn't offered yet.

use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentStatus, SubagentSupport,
};

/// What the Grok Build adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "Grok Build's session folders: subagents/<id>/meta.json in the session's folder (and in each child session's, for nested ones)",
    conversations: SubagentConversations::NotRead(
        "Grok Build keeps a subagent's conversation as an ACP update stream (updates.jsonl) that AI Juggler doesn't read as a conversation yet.",
    ),
    stop_hint: None,
};

/// The folder of a session's subagents.
const SUBAGENTS_DIR: &str = "subagents";
/// One subagent's record.
const META_FILE: &str = "meta.json";
/// The most bytes read from one meta (a prompt is in it).
const META_MAX: u64 = 4 * 1024 * 1024;
/// The most subagents one session is read for.
const AGENTS_MAX: usize = 5_000;
/// How deep children of children are followed.
const DEPTH_MAX: u32 = 8;

/// A reader for the session whose `summary.json` `scope` names.
#[must_use]
pub fn reader(scope: SubagentScope<'_>) -> Option<Box<dyn SubagentReader>> {
    if scope.path.file_name()? != crate::grok_sessions::SUMMARY_FILE {
        return None;
    }
    Some(Box::new(GrokSubagents::new(scope.path.parent()?)))
}

/// What one meta says.
#[derive(Debug, Clone, Default)]
struct Meta {
    kind: Option<String>,
    task: Option<String>,
    status: SubagentStatus,
    started_ms: Option<i64>,
    ended_ms: Option<i64>,
    child: Option<String>,
}

/// One session's subagents, read from its folders.
#[derive(Debug)]
pub struct GrokSubagents {
    folder: PathBuf,
    /// Each subagent's meta and the session (folder) that started it.
    metas: HashMap<String, (Meta, Option<String>, u32)>,
    /// Each child session's folder, once found.
    children: HashMap<String, PathBuf>,
}

impl GrokSubagents {
    /// The subagents of the session in `folder`.
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_path_buf(),
            metas: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// `sessions/<group>/<id>/`: the folder of session `id` under any
    /// group (its cwd's), when there is one.
    fn session_folder(&self, id: &str) -> Option<PathBuf> {
        let root = self.folder.parent()?.parent()?;
        std::fs::read_dir(root)
            .ok()?
            .flatten()
            .map(|group| group.path().join(id))
            .find(|folder| folder.is_dir())
    }

    /// Reads the metas in `folder`'s `subagents/`, started by `parent`
    /// (`None`: the session) at depth `depth`.
    fn read_folder(&mut self, folder: &Path, parent: Option<&str>, depth: u32) {
        let Ok(entries) = std::fs::read_dir(folder.join(SUBAGENTS_DIR)) else {
            return;
        };
        for entry in entries.flatten() {
            if self.metas.len() >= AGENTS_MAX {
                return;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            // A final meta isn't read again.
            if self
                .metas
                .get(&id)
                .is_some_and(|(meta, _, _)| meta.status.is_final())
            {
                continue;
            }
            let meta = read_meta(&entry.path().join(META_FILE)).unwrap_or_default();
            self.metas
                .insert(id, (meta, parent.map(str::to_owned), depth));
        }
    }
}

impl SubagentReader for GrokSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        // The session's own record must be there.
        std::fs::metadata(self.folder.join(crate::grok_sessions::SUMMARY_FILE))?;
        let folder = self.folder.clone();
        self.read_folder(&folder, None, 1);
        // Each child session's own subagents, depth by depth.
        for depth in 1..DEPTH_MAX {
            let at_depth: Vec<(String, String)> = self
                .metas
                .iter()
                .filter(|(_, (_, _, d))| *d == depth)
                .filter_map(|(id, (meta, _, _))| Some((id.clone(), meta.child.clone()?)))
                .collect();
            if at_depth.is_empty() {
                break;
            }
            for (id, child) in at_depth {
                let found = self.children.get(&child).cloned().or_else(|| {
                    // Not there yet: looked for again next time.
                    let found = self.session_folder(&child)?;
                    self.children.insert(child.clone(), found.clone());
                    Some(found)
                });
                if let Some(folder) = found {
                    self.read_folder(&folder, Some(&id), depth + 1);
                }
            }
        }
        Ok(true)
    }

    fn subagents(&self) -> Vec<Subagent> {
        self.metas
            .iter()
            .map(|(id, (meta, parent, depth))| Subagent {
                id: id.clone(),
                kind: meta.kind.clone(),
                task: meta.task.clone(),
                status: meta.status,
                started_ms: meta.started_ms,
                ended_ms: meta.ended_ms.filter(|_| meta.status.is_final()),
                parent: parent.clone(),
                depth: *depth,
                background: None,
                transcript: None,
                details: juggler_adapter::subagents::SubagentDetails::default(),
            })
            .collect()
    }
}

/// A time in a meta: RFC 3339, or epoch seconds or ms.
fn time_of(value: Option<&serde_json::Value>) -> Option<i64> {
    match value? {
        serde_json::Value::String(text) => juggler_core::time::timestamp_ms(text),
        serde_json::Value::Number(number) => {
            let n = number.as_i64()?;
            Some(if n < 10_000_000_000 { n * 1000 } else { n })
        }
        _ => None,
    }
}

fn read_meta(path: &Path) -> Option<Meta> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(META_MAX).read_to_end(&mut bytes).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let text = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
    };
    Some(Meta {
        kind: text("subagent_type"),
        task: text("description"),
        status: match text("status").as_deref() {
            Some("running") => SubagentStatus::Running,
            Some("completed") => SubagentStatus::Done,
            Some("failed") => SubagentStatus::Failed,
            Some("cancelled") => SubagentStatus::Stopped,
            _ => SubagentStatus::Unknown,
        },
        started_ms: time_of(value.get("started_at")),
        ended_ms: time_of(value.get("completed_at")),
        child: text("child_session_id"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn meta(folder: &Path, id: &str, value: &serde_json::Value) {
        write(
            &folder.join("subagents").join(id).join("meta.json"),
            &value.to_string(),
        );
    }

    #[test]
    fn subagents_come_from_their_metas_and_nested_ones_from_the_child_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sessions");
        let session = root.join("%2Fwork%2Fproject/s1");
        write(&session.join("summary.json"), "{}");
        meta(
            &session,
            "a1",
            &serde_json::json!({"subagent_id": "a1", "child_session_id": "a1",
            "subagent_type": "explore", "description": "Find the parser", "status": "completed",
            "started_at": "2026-10-27T10:00:00Z", "completed_at": "2026-10-27T10:01:00Z"}),
        );
        meta(
            &session,
            "a2",
            &serde_json::json!({"subagent_id": "a2", "child_session_id": "a2",
            "subagent_type": "general", "description": "Fix it", "status": "running",
            "started_at": "2026-10-27T10:02:00Z"}),
        );
        meta(
            &session,
            "a3",
            &serde_json::json!({"subagent_id": "a3", "status": "cancelled"}),
        );
        meta(
            &session,
            "a4",
            &serde_json::json!({"subagent_id": "a4", "status": "paused"}),
        );
        // a2's own session, in another cwd's group, started one more.
        let child = root.join("%2Fwork%2Fworktree/a2");
        write(&child.join("summary.json"), "{}");
        meta(
            &child,
            "b1",
            &serde_json::json!({"subagent_id": "b1", "status": "failed",
            "started_at": "2026-10-27T10:03:00Z", "completed_at": "2026-10-27T10:04:00Z"}),
        );

        let mut reader = GrokSubagents::new(&session);
        assert!(reader.refresh().unwrap());
        let list = reader.subagents();
        let by = |id: &str| list.iter().find(|a| a.id == id).expect("listed");
        assert_eq!(list.len(), 5);
        assert_eq!(by("a1").status, SubagentStatus::Done);
        assert_eq!(by("a1").kind.as_deref(), Some("explore"));
        assert_eq!(
            by("a1").ended_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:01:00Z")
        );
        assert_eq!(by("a2").status, SubagentStatus::Running);
        assert_eq!(by("a3").status, SubagentStatus::Stopped);
        assert_eq!(
            by("a4").status,
            SubagentStatus::Unknown,
            "a word it doesn't know"
        );
        assert_eq!(by("b1").status, SubagentStatus::Failed);
        assert_eq!(by("b1").parent.as_deref(), Some("a2"));
        assert_eq!(by("b1").depth, 2);
        // Ended: read again, it says so.
        meta(
            &session,
            "a2",
            &serde_json::json!({"status": "completed", "completed_at": "2026-10-27T10:09:00Z"}),
        );
        reader.refresh().unwrap();
        let list = reader.subagents();
        assert_eq!(
            list.iter().find(|a| a.id == "a2").unwrap().status,
            SubagentStatus::Done
        );
    }
}
