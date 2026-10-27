//! AJ-E1520 (0.2): Kimi Code's subagents, for the Subagents tab, from its
//! documented session folder (read-only, D-28/D-37).
//!
//! A session's folder (`$KIMI_CODE_HOME/sessions/<workDirKey>/<id>/`,
//! `docs/en/guides/sessions.md`) holds `state.json` and one folder per
//! agent: `agents/main/` is the session's own, and `agents/<agentId>/`
//! each subagent's (`agent-0`, `agent-1`, … — `docs/en/configuration/
//! data-locations.md`), each with its `wire.jsonl` event journal.
//!
//! - `state.json`'s `agents` map names each agent: `type` (`sub`), and in
//!   `labels` its real parent (`parentAgentId`; the top-level
//!   `parentAgentId` is always `main`) and its profile (`profileName`) —
//!   `packages/agent-core-v2/src/session/sessionMetadata/sessionMetadata.ts`,
//!   `agentLifecycle/agentLifecycleService.ts`.
//! - The spawning agent's `wire.jsonl` holds the lifecycle records
//!   (`packages/agent-core-v2/docs/wire-manifest.d.ts`):
//!   `subagent.spawned {subagentId, subagentName, description,
//!   runInBackground, parentAgentId}`, `subagent.started`, and one of
//!   `subagent.completed` / `subagent.failed` / `subagent.cancelled`, each
//!   with its `time` (epoch ms).
//!
//! A subagent with a folder and no record is unknown (D-06). Each wire is
//! read as it grows, and only lines naming a `subagent.` record are
//! parsed. Kimi Code Desktop keeps its sessions in the same home, so its
//! sessions read the same way.
//!
//! Its conversation is `agents/<id>/wire.jsonl` too, an event journal the
//! session page's reader doesn't read (Kimi's own session conversations
//! aren't shown either), so it isn't offered yet.

use std::collections::{BTreeSet, HashMap};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentStatus, SubagentSupport,
};

use crate::transcript_tail::{TailLog, TranscriptTail};

/// What the Kimi Code adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "Kimi Code's documented session folder: state.json's agents map and the subagent.* records in each spawning agent's agents/<id>/wire.jsonl",
    conversations: SubagentConversations::NotRead(
        "Kimi Code keeps a subagent's conversation as an event journal (wire.jsonl) that AI Juggler doesn't read as a conversation yet.",
    ),
};

/// The session's own agent.
const MAIN: &str = "main";

/// The most bytes read from `state.json`.
const STATE_MAX: u64 = 16 * 1024 * 1024;

/// The most 16 MiB reads of journals one refresh takes.
const READS_PER_REFRESH: usize = 16;

/// The most subagents one session is read for.
const AGENTS_MAX: usize = 5_000;

/// A reader for the session whose `state.json` `scope` names.
#[must_use]
pub fn reader(scope: SubagentScope<'_>) -> Option<Box<dyn SubagentReader>> {
    if scope.path.file_name()? != crate::kimi_sessions::STATE_FILE {
        return None;
    }
    Some(Box::new(KimiSubagents::new(scope.path.parent()?)))
}

/// What one journal says about the subagents its agent started.
#[derive(Debug, Default)]
struct Records {
    spawned: HashMap<String, Spawn>,
}

#[derive(Debug, Default, Clone)]
struct Spawn {
    name: Option<String>,
    description: Option<String>,
    background: Option<bool>,
    parent: Option<String>,
    spawned_ms: Option<i64>,
    started: bool,
    end: Option<(SubagentStatus, Option<i64>)>,
}

impl TailLog for Records {
    fn feed(&mut self, bytes: &[u8]) {
        for line in bytes.split(|b| *b == b'\n') {
            // Only a lifecycle record is parsed.
            if !crate::subtasks::holds(line, b"subagent.") {
                continue;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
                continue;
            };
            let text = |key: &str| {
                value
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            };
            let (Some(kind), Some(id)) = (text("type"), text("subagentId")) else {
                continue;
            };
            let time = value.get("time").and_then(serde_json::Value::as_i64);
            let spawn = self.spawned.entry(id).or_default();
            match kind.as_str() {
                "subagent.spawned" => {
                    spawn.name = text("subagentName");
                    spawn.description = text("description");
                    spawn.background = value
                        .get("runInBackground")
                        .and_then(serde_json::Value::as_bool);
                    spawn.parent = text("parentAgentId").or_else(|| text("callerAgentId"));
                    spawn.spawned_ms = spawn.spawned_ms.or(time);
                }
                "subagent.started" => {
                    spawn.started = true;
                    spawn.spawned_ms = spawn.spawned_ms.or(time);
                }
                "subagent.completed" => {
                    spawn.end = spawn.end.or(Some((SubagentStatus::Done, time)));
                }
                "subagent.failed" => {
                    spawn.end = spawn.end.or(Some((SubagentStatus::Failed, time)));
                }
                "subagent.cancelled" => {
                    spawn.end = spawn.end.or(Some((SubagentStatus::Stopped, time)));
                }
                _ => {}
            }
        }
    }
}

/// What `state.json` says about one agent.
#[derive(Debug, Default, Clone)]
struct Named {
    parent: Option<String>,
    profile: Option<String>,
}

/// One session's subagents, read as its journals grow.
#[derive(Debug)]
pub struct KimiSubagents {
    folder: PathBuf,
    named: HashMap<String, Named>,
    folders: BTreeSet<String>,
    journals: HashMap<String, TranscriptTail<Records>>,
}

impl KimiSubagents {
    /// The subagents of the session in `folder`.
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_path_buf(),
            named: HashMap::new(),
            folders: BTreeSet::new(),
            journals: HashMap::new(),
        }
    }

    fn journal(&self, agent: &str) -> PathBuf {
        self.folder.join("agents").join(agent).join("wire.jsonl")
    }

    fn read_state(&mut self) -> std::io::Result<()> {
        let file = std::fs::File::open(self.folder.join(crate::kimi_sessions::STATE_FILE))?;
        let mut bytes = Vec::new();
        file.take(STATE_MAX).read_to_end(&mut bytes)?;
        let Ok(state) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return Ok(());
        };
        let Some(serde_json::Value::Object(agents)) = state.get("agents") else {
            return Ok(());
        };
        let text = |value: Option<&serde_json::Value>| {
            value
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        for (id, meta) in agents.iter().take(AGENTS_MAX) {
            if id == MAIN || text(meta.get("type")).as_deref() == Some("main") {
                continue;
            }
            let labels = meta.get("labels");
            self.named.insert(
                id.clone(),
                Named {
                    parent: text(labels.and_then(|l| l.get("parentAgentId")))
                        .or_else(|| text(meta.get("parentAgentId"))),
                    profile: text(labels.and_then(|l| l.get("profileName"))),
                },
            );
        }
        Ok(())
    }
}

impl SubagentReader for KimiSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        self.read_state()?;
        if let Ok(entries) = std::fs::read_dir(self.folder.join("agents")) {
            for entry in entries.flatten().take(AGENTS_MAX) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name != MAIN && entry.path().is_dir() {
                    self.folders.insert(name);
                }
            }
        }
        // The session's journal, and each subagent's that started others.
        let mut spawners: BTreeSet<String> = BTreeSet::from([MAIN.to_owned()]);
        spawners.extend(self.named.values().filter_map(|named| named.parent.clone()));
        let mut caught_up = true;
        for spawner in spawners {
            let path = self.journal(&spawner);
            let tail = self.journals.entry(spawner.clone()).or_default();
            match tail.read_some(&path, READS_PER_REFRESH) {
                Ok(done) => caught_up &= done,
                // The session's own journal not there yet: nothing started.
                Err(_) if spawner == MAIN => {}
                Err(_) => {}
            }
        }
        Ok(caught_up)
    }

    fn subagents(&self) -> Vec<Subagent> {
        let mut spawns: HashMap<&str, (&Spawn, &str)> = HashMap::new();
        for (spawner, tail) in &self.journals {
            for (id, spawn) in &tail.log().spawned {
                spawns.insert(id.as_str(), (spawn, spawner.as_str()));
            }
        }
        let mut ids: BTreeSet<&str> = self.folders.iter().map(String::as_str).collect();
        ids.extend(self.named.keys().map(String::as_str));
        ids.extend(spawns.keys().copied());
        let mut out: Vec<Subagent> = ids
            .into_iter()
            .take(AGENTS_MAX)
            .map(|id| {
                let named = self.named.get(id);
                let spawn = spawns.get(id).map(|(spawn, _)| *spawn);
                let status = match spawn {
                    Some(Spawn {
                        end: Some((end, _)),
                        ..
                    }) => *end,
                    Some(_) => SubagentStatus::Running,
                    None => SubagentStatus::Unknown,
                };
                let parent = named
                    .and_then(|named| named.parent.clone())
                    .or_else(|| spawn.and_then(|spawn| spawn.parent.clone()))
                    .or_else(|| spawns.get(id).map(|(_, spawner)| (*spawner).to_owned()))
                    .filter(|parent| parent != MAIN);
                Subagent {
                    id: id.to_owned(),
                    kind: spawn
                        .and_then(|spawn| spawn.name.clone())
                        .or_else(|| named.and_then(|named| named.profile.clone())),
                    task: spawn.and_then(|spawn| spawn.description.clone()),
                    status,
                    started_ms: spawn.and_then(|spawn| spawn.spawned_ms),
                    ended_ms: spawn.and_then(|spawn| spawn.end).and_then(|(_, at)| at),
                    parent,
                    depth: 0,
                    background: spawn.and_then(|spawn| spawn.background),
                    transcript: None,
                }
            })
            .collect();
        crate::subtasks::fill_depths(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn record(value: &serde_json::Value) -> String {
        value.to_string()
    }

    #[test]
    fn subagents_come_from_the_session_folder_and_the_spawners_journals() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("sessions/wd_x_123456789abc/s1");
        write(
            &folder.join("state.json"),
            &serde_json::json!({"title": "t", "agents": {
                "main": {"type": "main"},
                "agent-0": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "main", "profileName": "explore"}},
                "agent-1": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "main", "profileName": "coder"}},
                "agent-2": {"type": "sub", "parentAgentId": "main", "labels": {"parentAgentId": "agent-1", "profileName": "custom"}},
            }})
            .to_string(),
        );
        let main = [
            record(&serde_json::json!({"type": "metadata", "protocol_version": "1.5"})),
            record(
                &serde_json::json!({"type": "subagent.spawned", "subagentId": "agent-0", "subagentName": "explore",
                "description": "Find the parser", "runInBackground": false, "parentToolCallId": "c1", "time": 1_000}),
            ),
            record(
                &serde_json::json!({"type": "subagent.completed", "subagentId": "agent-0", "resultSummary": "ok", "time": 5_000}),
            ),
            record(
                &serde_json::json!({"type": "subagent.spawned", "subagentId": "agent-1", "subagentName": "coder",
                "description": "Fix it", "runInBackground": true, "time": 6_000}),
            ),
            record(
                &serde_json::json!({"type": "subagent.started", "subagentId": "agent-1", "time": 6_001}),
            ),
        ];
        write(
            &folder.join("agents/main/wire.jsonl"),
            &(main.join("\n") + "\n"),
        );
        let one = [
            record(
                &serde_json::json!({"type": "subagent.spawned", "subagentId": "agent-2", "subagentName": "custom",
                "description": "Deeper", "time": 7_000}),
            ),
            record(
                &serde_json::json!({"type": "subagent.failed", "subagentId": "agent-2", "error": "x", "time": 8_000}),
            ),
        ];
        write(
            &folder.join("agents/agent-1/wire.jsonl"),
            &(one.join("\n") + "\n"),
        );
        // A folder with no record of it: unknown.
        write(&folder.join("agents/agent-9/wire.jsonl"), "");

        let mut reader = KimiSubagents::new(&folder);
        assert!(reader.refresh().unwrap());
        let list = reader.subagents();
        let by = |id: &str| list.iter().find(|a| a.id == id).expect("listed");
        assert_eq!(list.len(), 4);
        assert_eq!(by("agent-0").status, SubagentStatus::Done);
        assert_eq!(by("agent-0").task.as_deref(), Some("Find the parser"));
        assert_eq!(by("agent-0").ended_ms, Some(5_000));
        assert_eq!(by("agent-0").background, Some(false));
        assert_eq!(by("agent-1").status, SubagentStatus::Running);
        assert_eq!(by("agent-1").background, Some(true));
        assert_eq!(by("agent-2").status, SubagentStatus::Failed);
        assert_eq!(by("agent-2").parent.as_deref(), Some("agent-1"));
        assert_eq!(by("agent-2").depth, 2);
        assert_eq!(by("agent-9").status, SubagentStatus::Unknown);
        assert!(list.iter().all(|a| a.transcript.is_none()));
    }

    #[test]
    fn only_a_state_file_names_a_kimi_session() {
        let path = Path::new("/x/sessions/k/s/state.json");
        assert!(reader(SubagentScope { path, id: "s" }).is_some());
        assert!(
            reader(SubagentScope {
                path: Path::new("/x/s.jsonl"),
                id: "s"
            })
            .is_none()
        );
    }
}
