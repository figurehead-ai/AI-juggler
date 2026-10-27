//! AJ-E1520 (0.2): Claude Code's subagents, for the Subagents tab, from
//! its own files (read-only, D-28/D-37).
//!
//! Claude Code (2.1.31x) keeps each subagent beside its session's
//! transcript: `<project>/<session-id>.jsonl` is the session, and
//! `<project>/<session-id>/subagents/` holds `agent-<id>.jsonl` (the
//! subagent's own conversation), `agent-<id>.meta.json` (`agentType`,
//! `description`, `toolUseId` — the Agent call that started it —,
//! `spawnDepth`, `parentAgentId` for a nested one, `requestShape`
//! `background`/`foreground`, `stoppedByUser`) and `agent-<id>.prefix.json`
//! (not read). The same layout serves the Claude app's Code tab, which runs
//! the same Claude Code on the same files.
//!
//! How each one stands comes from the transcript that started it (the
//! session's, or a nested one's parent subagent's), read by the shared
//! subagent log (`claude_transcript::subtask_line`): the Agent call's
//! answer (an error, `completed`, or `async_launched` for one in the
//! background) and, for a background one, the `<task-notification>` that
//! says how it ended. A subagent whose files have no meta and that no
//! transcript names is unknown, never a guess. Transcripts are read as
//! they grow (only what was appended), a meta once.

use std::collections::{BTreeSet, HashMap};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentStatus, SubagentSupport,
};

use crate::subtasks::{Subtask, SubtaskState, SubtaskTail};

/// What the Claude Code adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "Claude Code's files beside the session's transcript: subagents/agent-<id>.meta.json and .jsonl, and the session's own transcript for how each ended",
    conversations: SubagentConversations::Read,
};

/// The folder beside a session's transcript that holds its subagents.
pub const SUBAGENTS_DIR: &str = "subagents";

/// The most bytes read from one meta file (they hold a few fields).
const META_MAX: u64 = 256 * 1024;

/// The most bytes read from a subagent's transcript for its first time.
const HEAD_MAX: u64 = 64 * 1024;

/// The most subagents one session's folder is read for.
const AGENTS_MAX: usize = 5_000;

/// The most 16 MiB reads of transcripts one refresh takes (256 MiB).
const READS_PER_REFRESH: usize = 16;

/// A reader for the conversation `scope` names: a `.jsonl` transcript.
#[must_use]
pub fn reader(scope: SubagentScope<'_>) -> Option<Box<dyn SubagentReader>> {
    let path = scope.path;
    if path.extension().is_none_or(|ext| ext != "jsonl") {
        return None;
    }
    Some(Box::new(ClaudeSubagents::new(path)))
}

/// What a meta file says (every field optional: never assumed).
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    agent_type: Option<String>,
    description: Option<String>,
    tool_use_id: Option<String>,
    spawn_depth: Option<u32>,
    parent_agent_id: Option<String>,
    request_shape: Option<String>,
    stopped_by_user: Option<bool>,
}

/// A subagent's own transcript.
#[derive(Clone, Debug)]
struct AgentFile {
    path: PathBuf,
    /// Its first line's time, once it has one.
    first_ms: Option<i64>,
}

/// One session's subagents, read as its files grow.
#[derive(Debug)]
pub struct ClaudeSubagents {
    session: PathBuf,
    folder: PathBuf,
    /// Each transcript that starts subagents, as read so far: the
    /// session's (key `None`), and each subagent's that started one.
    tails: HashMap<Option<String>, SubtaskTail>,
    metas: HashMap<String, Meta>,
    files: HashMap<String, AgentFile>,
}

impl ClaudeSubagents {
    /// The subagents of the session whose transcript is `session`.
    #[must_use]
    pub fn new(session: &Path) -> Self {
        Self {
            session: session.to_path_buf(),
            folder: session.with_extension("").join(SUBAGENTS_DIR),
            tails: HashMap::new(),
            metas: HashMap::new(),
            files: HashMap::new(),
        }
    }

    fn read_folder(&mut self) {
        let Ok(entries) = std::fs::read_dir(&self.folder) else {
            return;
        };
        for entry in entries.flatten().take(AGENTS_MAX * 3) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(rest) = name.strip_prefix("agent-") else {
                continue;
            };
            if let Some(id) = rest.strip_suffix(".meta.json") {
                if !self.metas.contains_key(id)
                    && let Some(meta) = read_meta(&entry.path())
                {
                    self.metas.insert(id.to_owned(), meta);
                }
            } else if let Some(id) = rest.strip_suffix(".jsonl") {
                let file = self
                    .files
                    .entry(id.to_owned())
                    .or_insert_with(|| AgentFile {
                        path: entry.path(),
                        first_ms: None,
                    });
                if file.first_ms.is_none() {
                    file.first_ms = first_timestamp(&file.path);
                }
            }
        }
    }
}

impl SubagentReader for ClaudeSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        let mut caught_up = self
            .tails
            .entry(None)
            .or_default()
            .read_some(&self.session, READS_PER_REFRESH)?;
        self.read_folder();
        // A subagent that started others: its own transcript says how they
        // stand.
        let spawners: BTreeSet<String> = self
            .metas
            .values()
            .filter_map(|meta| meta.parent_agent_id.clone())
            .collect();
        for spawner in spawners {
            let Some(file) = self.files.get(&spawner) else {
                continue;
            };
            let path = file.path.clone();
            // Gone or unreadable: its children stand as their own files say.
            caught_up &= self
                .tails
                .entry(Some(spawner))
                .or_default()
                .read_some(&path, READS_PER_REFRESH)
                .unwrap_or(true);
        }
        Ok(caught_up)
    }

    fn subagents(&self) -> Vec<Subagent> {
        // Every start the transcripts hold, by agent id and by call id.
        let mut by_agent: HashMap<&str, (&Subtask, Option<&str>)> = HashMap::new();
        let mut by_call: HashMap<&str, (&Subtask, Option<&str>)> = HashMap::new();
        for (spawner, tail) in &self.tails {
            for item in tail.log().subtasks() {
                let entry = (item, spawner.as_deref());
                by_call.insert(item.id.as_str(), entry);
                if let Some(agent) = item.agent.as_deref() {
                    by_agent.insert(agent, entry);
                }
            }
        }
        let mut ids: BTreeSet<&str> = self.files.keys().map(String::as_str).collect();
        ids.extend(self.metas.keys().map(String::as_str));
        let mut used_calls: BTreeSet<&str> = BTreeSet::new();
        let mut out: Vec<Subagent> = Vec::new();
        for id in ids.into_iter().take(AGENTS_MAX) {
            let meta = self.metas.get(id);
            let start = by_agent.get(id).copied().or_else(|| {
                meta.and_then(|meta| meta.tool_use_id.as_deref())
                    .and_then(|call| by_call.get(call).copied())
            });
            if let Some((item, _)) = start {
                used_calls.insert(item.id.as_str());
            }
            let file = self.files.get(id);
            out.push(described(id, meta, start, file));
        }
        // Starts with no files (yet, or a version that kept them in the
        // session's own transcript).
        for (call, (item, spawner)) in &by_call {
            if used_calls.contains(call)
                || item.agent.as_deref().is_some_and(|agent| {
                    self.files.contains_key(agent) || self.metas.contains_key(agent)
                })
            {
                continue;
            }
            let id = item.agent.clone().unwrap_or_else(|| item.id.clone());
            out.push(described(&id, None, Some((item, *spawner)), None));
        }
        crate::subtasks::fill_depths(&mut out);
        out
    }
}

/// One subagent from what its meta, its start and its file say.
fn described(
    id: &str,
    meta: Option<&Meta>,
    start: Option<(&Subtask, Option<&str>)>,
    file: Option<&AgentFile>,
) -> Subagent {
    let item = start.map(|(item, _)| item);
    let mut status = match item.map(|item| item.state) {
        Some(SubtaskState::Running | SubtaskState::Background) => SubagentStatus::Running,
        Some(SubtaskState::Done) => SubagentStatus::Done,
        Some(SubtaskState::Failed) => SubagentStatus::Failed,
        Some(SubtaskState::Stopped) => SubagentStatus::Stopped,
        // Nothing links its files to a start: how it stands is unknown.
        None => SubagentStatus::Unknown,
    };
    if !status.is_final() && meta.and_then(|meta| meta.stopped_by_user) == Some(true) {
        status = SubagentStatus::Stopped;
    }
    let text = |value: Option<&String>| value.filter(|v| !v.trim().is_empty()).cloned();
    let background = meta
        .and_then(|meta| meta.request_shape.as_deref())
        .map(|shape| shape == "background")
        .or_else(|| {
            item.and_then(|item| {
                if item.background {
                    Some(true)
                } else if item.state.is_final() {
                    Some(false)
                } else {
                    None
                }
            })
        });
    Subagent {
        id: id.to_owned(),
        kind: text(meta.and_then(|meta| meta.agent_type.as_ref()))
            .or_else(|| item.and_then(|item| item.kind.clone())),
        task: text(meta.and_then(|meta| meta.description.as_ref()))
            .or_else(|| item.map(|item| item.task.clone()).filter(|t| !t.is_empty())),
        status,
        started_ms: item
            .and_then(|item| item.started_ms)
            .or_else(|| file.and_then(|file| file.first_ms)),
        ended_ms: if status.is_final() {
            item.and_then(|item| item.ended_ms)
        } else {
            None
        },
        parent: meta
            .and_then(|meta| meta.parent_agent_id.clone())
            .or_else(|| start.and_then(|(_, spawner)| spawner.map(str::to_owned))),
        depth: meta.and_then(|meta| meta.spawn_depth).unwrap_or(0),
        background,
        transcript: file.map(|file| file.path.clone()),
    }
}

fn read_meta(path: &Path) -> Option<Meta> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(META_MAX).read_to_end(&mut bytes).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// The first `"timestamp"` in a transcript's head (a quoted key never
/// appears unescaped inside a JSON string, so the first is a line's own).
fn first_timestamp(path: &Path) -> Option<i64> {
    const KEY: &[u8] = br#""timestamp":""#;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(HEAD_MAX).read_to_end(&mut bytes).ok()?;
    let at = bytes.windows(KEY.len()).position(|w| w == KEY)? + KEY.len();
    let end = bytes[at..].iter().position(|b| *b == b'"')? + at;
    juggler_core::time::timestamp_ms(std::str::from_utf8(&bytes[at..end]).ok()?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn start(call: &str, kind: &str, task: &str, at: &str) -> String {
        serde_json::json!({
            "type": "assistant", "timestamp": at,
            "message": {"role": "assistant", "content": [{"type": "tool_use", "id": call, "name": "Agent",
                "input": {"description": task, "prompt": "do it", "subagent_type": kind}}]},
        })
        .to_string()
    }

    fn launched(call: &str, agent: &str, status: &str, at: &str) -> String {
        serde_json::json!({
            "type": "user", "timestamp": at,
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": call, "content": "ok"}]},
            "toolUseResult": {"status": status, "agentId": agent},
        })
        .to_string()
    }

    fn notified(agent: &str, status: &str, at: &str) -> String {
        serde_json::json!({
            "type": "user", "timestamp": at, "origin": {"kind": "task-notification"},
            "message": {"role": "user", "content": format!(
                "<task-notification>\n<task-id>{agent}</task-id>\n<status>{status}</status>\n<summary>s</summary>\n</task-notification>")},
        })
        .to_string()
    }

    fn meta(dir: &Path, agent: &str, value: &serde_json::Value) {
        write(
            &dir.join(format!("agent-{agent}.meta.json")),
            &value.to_string(),
        );
    }

    fn own(dir: &Path, agent: &str, lines: &[String]) {
        let first = serde_json::json!({"type": "user", "isSidechain": true, "timestamp": "2026-10-27T10:00:05Z",
            "message": {"role": "user", "content": "the task"}})
        .to_string();
        let mut all = vec![first];
        all.extend_from_slice(lines);
        write(
            &dir.join(format!("agent-{agent}.jsonl")),
            &(all.join("\n") + "\n"),
        );
    }

    fn by_id<'a>(list: &'a [Subagent], id: &str) -> &'a Subagent {
        list.iter()
            .find(|agent| agent.id == id)
            .expect("a subagent the fixture holds is listed")
    }

    #[test]
    fn subagents_are_read_from_their_files_and_the_session_says_how_each_ended() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("proj/s1.jsonl");
        let folder = dir.path().join("proj/s1/subagents");
        write(
            &session,
            &([
                start("t1", "Explore", "Find the parser", "2026-10-27T10:00:00Z"),
                launched("t1", "a1", "completed", "2026-10-27T10:01:00Z"),
                start("t2", "worker", "Fix the test", "2026-10-27T10:02:00Z"),
                launched("t2", "a2", "async_launched", "2026-10-27T10:02:01Z"),
                start("t3", "worker", "Break it", "2026-10-27T10:03:00Z"),
                launched("t3", "a3", "async_launched", "2026-10-27T10:03:01Z"),
                start("t4", "worker", "Still going", "2026-10-27T10:04:00Z"),
                launched("t4", "a4", "async_launched", "2026-10-27T10:04:01Z"),
                notified("a2", "completed", "2026-10-27T10:09:00Z"),
                notified("a3", "failed", "2026-10-27T10:10:00Z"),
            ]
            .join("\n")
                + "\n"),
        );
        for (agent, call, shape) in [
            ("a1", "t1", "foreground"),
            ("a2", "t2", "background"),
            ("a3", "t3", "background"),
            ("a4", "t4", "background"),
        ] {
            meta(
                &folder,
                agent,
                &serde_json::json!({"agentType": "frontend-worker", "description": format!("task of {agent}"),
                    "toolUseId": call, "spawnDepth": 1, "requestShape": shape, "requestNonInteractive": true}),
            );
            own(&folder, agent, &[]);
        }
        // A nested one, started by a4; how it stands is in a4's transcript.
        meta(
            &folder,
            "n1",
            &serde_json::json!({"agentType": "Explore", "description": "Look deeper", "toolUseId": "t9",
                "spawnDepth": 2, "parentAgentId": "a4", "requestShape": "foreground"}),
        );
        own(&folder, "n1", &[]);
        own(
            &folder,
            "a4",
            &[
                start("t9", "Explore", "Look deeper", "2026-10-27T10:05:00Z"),
                launched("t9", "n1", "completed", "2026-10-27T10:06:00Z"),
            ],
        );
        // Files with no meta that nothing names: unknown.
        own(&folder, "orphan", &[]);

        let mut reader = ClaudeSubagents::new(&session);
        reader.refresh().unwrap();
        let list = reader.subagents();
        assert_eq!(list.len(), 6);
        let a1 = by_id(&list, "a1");
        assert_eq!(a1.status, SubagentStatus::Done);
        assert_eq!(a1.kind.as_deref(), Some("frontend-worker"));
        assert_eq!(a1.task.as_deref(), Some("task of a1"));
        assert_eq!(a1.background, Some(false));
        assert_eq!(a1.depth, 1);
        assert_eq!(a1.parent, None);
        assert_eq!(
            a1.started_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:00:00Z")
        );
        assert_eq!(
            a1.ended_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:01:00Z")
        );
        assert!(a1.transcript.as_ref().unwrap().ends_with("agent-a1.jsonl"));
        let a2 = by_id(&list, "a2");
        assert_eq!(a2.status, SubagentStatus::Done);
        assert_eq!(a2.background, Some(true));
        assert_eq!(
            a2.ended_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:09:00Z")
        );
        assert_eq!(by_id(&list, "a3").status, SubagentStatus::Failed);
        let a4 = by_id(&list, "a4");
        assert_eq!(a4.status, SubagentStatus::Running);
        assert_eq!(a4.ended_ms, None);
        let n1 = by_id(&list, "n1");
        assert_eq!(n1.status, SubagentStatus::Done);
        assert_eq!(n1.parent.as_deref(), Some("a4"));
        assert_eq!(n1.depth, 2);
        let orphan = by_id(&list, "orphan");
        assert_eq!(orphan.status, SubagentStatus::Unknown);
        assert_eq!(orphan.kind, None);
        assert_eq!(orphan.task, None);
        assert_eq!(
            orphan.started_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:00:05Z")
        );
    }

    #[test]
    fn a_later_notification_is_read_on_the_next_refresh_and_stopped_counts_as_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s.jsonl");
        let folder = dir.path().join("s/subagents");
        let head = [
            start("t1", "worker", "One", "2026-10-27T10:00:00Z"),
            launched("t1", "a1", "async_launched", "2026-10-27T10:00:01Z"),
        ]
        .join("\n")
            + "\n";
        write(&session, &head);
        meta(
            &folder,
            "a1",
            &serde_json::json!({"toolUseId": "t1", "requestShape": "background"}),
        );
        let mut reader = ClaudeSubagents::new(&session);
        reader.refresh().unwrap();
        assert_eq!(reader.subagents()[0].status, SubagentStatus::Running);
        // Queued for the session first: that is when it ended.
        let queued = serde_json::json!({"type": "queue-operation", "operation": "enqueue", "timestamp": "2026-10-27T10:05:00Z",
            "content": "<task-notification><task-id>a1</task-id><status>killed</status></task-notification>"})
        .to_string();
        write(&session, &format!("{head}{queued}\n"));
        reader.refresh().unwrap();
        let one = &reader.subagents()[0];
        assert_eq!(one.status, SubagentStatus::Stopped);
        assert_eq!(
            one.ended_ms,
            juggler_core::time::timestamp_ms("2026-10-27T10:05:00Z")
        );
        assert_eq!(one.transcript, None, "no file of its own yet");
    }

    #[test]
    fn a_tool_description_that_mentions_the_tag_ends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s.jsonl");
        let snapshot = serde_json::json!({"type": "attachment", "attachment": {"type": "prompt_snapshot",
            "tools": [{"description": "<task-notification><task-id>a1</task-id><status>completed</status></task-notification>"}]}})
        .to_string();
        let pasted = serde_json::json!({"type": "user", "message": {"role": "user",
            "content": "<task-notification><task-id>a1</task-id><status>completed</status></task-notification>"}})
        .to_string();
        write(
            &session,
            &[
                start("t1", "worker", "One", "2026-10-27T10:00:00Z"),
                launched("t1", "a1", "async_launched", "2026-10-27T10:00:01Z"),
                snapshot,
                pasted,
            ]
            .join("\n"),
        );
        let mut reader = ClaudeSubagents::new(&session);
        reader.refresh().unwrap();
        let list = reader.subagents();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, SubagentStatus::Running);
        assert_eq!(list[0].id, "a1");
        assert_eq!(list[0].background, Some(true));
    }

    #[test]
    fn a_session_with_none_lists_none_and_a_missing_transcript_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("s.jsonl");
        write(
            &session,
            "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n",
        );
        let mut reader = ClaudeSubagents::new(&session);
        reader.refresh().unwrap();
        assert!(reader.subagents().is_empty());
        let mut gone = ClaudeSubagents::new(&dir.path().join("gone.jsonl"));
        assert!(gone.refresh().is_err());
        assert!(reader_of(&dir.path().join("x.json")).is_none());
    }

    fn reader_of(path: &Path) -> Option<Box<dyn SubagentReader>> {
        reader(SubagentScope { path, id: "x" })
    }
}
