//! AJ-E1520 (0.2): Claude Code's subagents, for the Subagents tab, from
//! its own files (read-only, D-28/D-37).
//!
//! Claude Code (2.1.31x) keeps each subagent beside its session's
//! transcript: `<project>/<session-id>.jsonl` is the session, and
//! `<project>/<session-id>/subagents/` holds `agent-<id>.jsonl` (the
//! subagent's own conversation), `agent-<id>.meta.json` (`agentType`,
//! `description`, `toolUseId` — the Agent call that started it —,
//! `spawnDepth`, `parentAgentId` for a nested one, `requestShape`
//! `background`/`foreground`, `stoppedByUser`, `worktreePath` and
//! `worktreeBranch`) and `agent-<id>.prefix.json` (not read). The same
//! layout serves the Claude app's Code tab, which runs the same Claude
//! Code on the same files.
//!
//! How each one stands comes from the transcript that started it (the
//! session's, or a nested one's parent subagent's), read by the shared
//! subagent log (`claude_transcript::subtask_line`): the Agent call (its
//! prompt, and the user's message that turn began with), its answer (an
//! error, `completed` with the result and what Claude Code reported, or
//! `async_launched` for one in the background) and, for a background
//! one, the `<task-notification>` that says how it ended. A subagent
//! whose files have no meta and that no transcript names is unknown,
//! never a guess.
//!
//! AJ-E1520 (0.3): each subagent's own transcript says what it used and
//! did (`own.rs`: tokens, model, tools, files, commands), read as it
//! grows within the tab's byte budget ([`SubagentReader::refresh_details`]);
//! the rail's count never reads it. What was read is saved by the app as
//! a checkpoint (`saved.rs`) so a long session's tab opens at once.
//! Transcripts are read as they grow (only what was appended), a meta
//! again only when it changed.

mod own;
mod saved;

use std::collections::{BTreeSet, HashMap};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use juggler_adapter::subagents::{
    FILES_MAX, Subagent, SubagentConversations, SubagentDetails, SubagentReader, SubagentScope,
    SubagentStatus, SubagentSupport, SubagentTokens, TextFilter, TokensCounted,
};
use serde::{Deserialize, Serialize};

use crate::subtasks::{Subtask, SubtaskState};
use crate::transcript_tail::TranscriptTail;
use own::OwnLog;

/// What the Claude Code adapter answers.
pub const SUPPORT: SubagentSupport = SubagentSupport::Supported {
    route: "Claude Code's files beside the session's transcript: subagents/agent-<id>.meta.json and .jsonl, and the session's own transcript for how each ended",
    conversations: SubagentConversations::Read,
    stop_hint: Some("In Claude Code: /tasks, select it, press x."),
};

/// The folder beside a session's transcript that holds its subagents.
pub const SUBAGENTS_DIR: &str = "subagents";

/// The most bytes read from one meta file (they hold a few fields).
const META_MAX: u64 = 256 * 1024;

/// The most bytes read from a subagent's transcript for its first time.
const HEAD_MAX: u64 = 64 * 1024;

/// The most subagents one session's folder is read for.
const AGENTS_MAX: usize = 5_000;

/// The most bytes of the transcripts that start subagents one refresh
/// reads (256 MiB).
const REFRESH_BYTES: u64 = 256 * 1024 * 1024;

/// The session's own transcript in [`Read::tails`].
const SESSION: &str = "";

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
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    agent_type: Option<String>,
    description: Option<String>,
    tool_use_id: Option<String>,
    spawn_depth: Option<u32>,
    parent_agent_id: Option<String>,
    request_shape: Option<String>,
    stopped_by_user: Option<bool>,
    worktree_path: Option<String>,
    worktree_branch: Option<String>,
}

/// A meta as read, with the file's size and time then (read again when
/// they change).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct MetaSeen {
    meta: Meta,
    len: u64,
    modified_ms: i64,
}

/// A subagent's own transcript.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentFile {
    path: PathBuf,
    /// Its first line's time, once it has one.
    first_ms: Option<i64>,
}

/// Everything read so far (what a checkpoint saves).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Read {
    /// Each transcript read: the session's ([`SESSION`]) and each
    /// subagent's, by its id.
    tails: HashMap<String, TranscriptTail<OwnLog>>,
    metas: HashMap<String, MetaSeen>,
    files: HashMap<String, AgentFile>,
}

/// One session's subagents, read as its files grow.
#[derive(Debug)]
pub struct ClaudeSubagents {
    session: PathBuf,
    folder: PathBuf,
    read: Read,
}

impl ClaudeSubagents {
    /// The subagents of the session whose transcript is `session`.
    #[must_use]
    pub fn new(session: &Path) -> Self {
        Self {
            session: session.to_path_buf(),
            folder: session.with_extension("").join(SUBAGENTS_DIR),
            read: Read::default(),
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
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                let (len, modified_ms) = (metadata.len(), modified_ms(&metadata));
                let fresh = self
                    .read
                    .metas
                    .get(id)
                    .is_some_and(|seen| seen.len == len && seen.modified_ms == modified_ms);
                if !fresh && let Some(meta) = read_meta(&entry.path()) {
                    self.read.metas.insert(
                        id.to_owned(),
                        MetaSeen {
                            meta,
                            len,
                            modified_ms,
                        },
                    );
                }
            } else if let Some(id) = rest.strip_suffix(".jsonl") {
                let file = self
                    .read
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

    /// Reads what `id`'s transcript gained, within `budget` bytes: true
    /// when caught up, and the bytes read. Gone or unreadable: caught up
    /// (it stands as its files say).
    fn read_agent(&mut self, id: &str, budget: u64) -> (bool, u64) {
        let Some(path) = self.read.files.get(id).map(|file| file.path.clone()) else {
            return (true, 0);
        };
        let tail = self.read.tails.entry(id.to_owned()).or_default();
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() == tail.offset()) {
            return (true, 0);
        }
        if budget == 0 {
            return (false, 0);
        }
        tail.read_within(&path, budget).unwrap_or((true, 0))
    }

    /// Every subagent; with `details`, what each one's records add.
    fn listed(&self, details: bool) -> Vec<Subagent> {
        // Every start the transcripts hold, by agent id and by call id.
        let mut by_agent: HashMap<&str, (&Subtask, Option<&str>)> = HashMap::new();
        let mut by_call: HashMap<&str, (&Subtask, Option<&str>)> = HashMap::new();
        for (spawner, tail) in &self.read.tails {
            let spawner = (spawner != SESSION).then_some(spawner.as_str());
            for item in tail.log().nested.subtasks() {
                let entry = (item, spawner);
                by_call.insert(item.id.as_str(), entry);
                if let Some(agent) = item.agent.as_deref() {
                    by_agent.insert(agent, entry);
                }
            }
        }
        let mut ids: BTreeSet<&str> = self.read.files.keys().map(String::as_str).collect();
        ids.extend(self.read.metas.keys().map(String::as_str));
        let mut used_calls: BTreeSet<&str> = BTreeSet::new();
        let mut out: Vec<Subagent> = Vec::new();
        for id in ids.into_iter().take(AGENTS_MAX) {
            let meta = self.read.metas.get(id).map(|seen| &seen.meta);
            let start = by_agent.get(id).copied().or_else(|| {
                meta.and_then(|meta| meta.tool_use_id.as_deref())
                    .and_then(|call| by_call.get(call).copied())
            });
            if let Some((item, _)) = start {
                used_calls.insert(item.id.as_str());
            }
            let file = self.read.files.get(id);
            let own = details
                .then(|| self.read.tails.get(id).map(TranscriptTail::log))
                .flatten();
            let mut agent = described(id, meta, start, file);
            if details {
                agent.details = details_of(meta, start.map(|(item, _)| item), own);
            }
            out.push(agent);
        }
        // Starts with no files (yet, or a version that kept them in the
        // session's own transcript).
        for (call, (item, spawner)) in &by_call {
            if used_calls.contains(call)
                || item.agent.as_deref().is_some_and(|agent| {
                    self.read.files.contains_key(agent) || self.read.metas.contains_key(agent)
                })
            {
                continue;
            }
            let id = item.agent.clone().unwrap_or_else(|| item.id.clone());
            let mut agent = described(&id, None, Some((item, *spawner)), None);
            if details {
                agent.details = details_of(None, Some(item), None);
            }
            out.push(agent);
        }
        crate::subtasks::fill_depths(&mut out);
        out
    }
}

impl SubagentReader for ClaudeSubagents {
    fn refresh(&mut self) -> std::io::Result<bool> {
        let (mut caught_up, _) = self
            .read
            .tails
            .entry(SESSION.to_owned())
            .or_default()
            .read_within(&self.session, REFRESH_BYTES)?;
        self.read_folder();
        // A subagent that started others: its own transcript says how they
        // stand.
        let spawners: BTreeSet<String> = self
            .read
            .metas
            .values()
            .filter_map(|seen| seen.meta.parent_agent_id.clone())
            .collect();
        for spawner in spawners {
            caught_up &= self.read_agent(&spawner, REFRESH_BYTES).0;
        }
        Ok(caught_up)
    }

    fn subagents(&self) -> Vec<Subagent> {
        self.listed(true)
    }

    fn running(&self) -> u64 {
        self.listed(false)
            .iter()
            .filter(|agent| agent.status == SubagentStatus::Running)
            .count() as u64
    }

    fn refresh_details(&mut self, budget: u64) -> std::io::Result<bool> {
        // The latest started first: what runs now is read first.
        let mut ids: Vec<(Option<i64>, String)> = self
            .read
            .files
            .iter()
            .map(|(id, file)| (file.first_ms, id.clone()))
            .collect();
        ids.sort_by(|a, b| b.cmp(a));
        let mut left = budget;
        let mut caught_up = true;
        for (_, id) in ids {
            if left == 0 {
                caught_up &= self.read_agent(&id, 0).0;
                continue;
            }
            let (done, used) = self.read_agent(&id, left);
            left = left.saturating_sub(used);
            caught_up &= done;
        }
        Ok(caught_up)
    }

    fn session_tokens(&self) -> Option<SubagentTokens> {
        self.read
            .tails
            .get(SESSION)
            .map(|tail| tail.log().tokens)
            .filter(|tokens| tokens.total() > 0)
    }

    fn tokens_counted(&self) -> TokensCounted {
        TokensCounted::InSession
    }

    fn checkpoint(&self, text: &TextFilter<'_>) -> Option<Vec<u8>> {
        saved::save(&self.session, &self.read, text)
    }

    fn resume(&mut self, bytes: &[u8]) -> bool {
        match saved::load(&self.session, bytes) {
            Some(read) => {
                self.read = read;
                true
            }
            None => false,
        }
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
        details: SubagentDetails::default(),
    }
}

/// What a subagent's meta, its start (and end) and its own transcript
/// say beyond where it stands. What Claude Code reported at the end
/// (exact) wins over what its transcript was counted to say.
fn details_of(
    meta: Option<&Meta>,
    item: Option<&Subtask>,
    own: Option<&OwnLog>,
) -> SubagentDetails {
    let reported = item.map(|item| &item.reported);
    let tokens = own
        .map(|own| own.tokens)
        .filter(|tokens| tokens.total() > 0);
    let edited = own.is_some_and(|own| own.tools.edits > 0);
    SubagentDetails {
        prompt: item
            .and_then(|item| item.prompt.clone())
            .or_else(|| own.and_then(|own| own.prompt.clone())),
        result: item.and_then(|item| item.result.clone()),
        failure: item.and_then(|item| item.failure.clone()),
        model: own
            .and_then(|own| own.model.clone())
            .or_else(|| reported.and_then(|reported| reported.model.clone())),
        tokens,
        tools: reported
            .and_then(|reported| reported.tools)
            .or_else(|| own.map(|own| own.tools).filter(|tools| tools.total > 0)),
        lines: reported
            .and_then(|reported| reported.lines)
            .or_else(|| own.and_then(|own| own.lines).filter(|_| edited)),
        files: own
            .map(|own| own.files_ranked(FILES_MAX))
            .unwrap_or_default(),
        files_total: own.map_or(0, |own| own.files_total),
        commands: own
            .map(|own| own.commands.iter().cloned().collect())
            .unwrap_or_default(),
        asked: item.and_then(|item| item.asked.clone()),
        stopped_by_user: meta.and_then(|meta| meta.stopped_by_user),
        worktree: meta.and_then(|meta| meta.worktree_path.clone()),
        branch: meta.and_then(|meta| meta.worktree_branch.clone()),
        last_activity_ms: own.and_then(|own| own.last_ms),
        duration_ms: reported.and_then(|reported| reported.duration_ms),
    }
}

fn modified_ms(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|at| i64::try_from(at.as_millis()).ok())
        .unwrap_or_default()
}

fn read_meta(path: &Path) -> Option<Meta> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(META_MAX).read_to_end(&mut bytes).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// The first `"timestamp"` in a transcript's head.
fn first_timestamp(path: &Path) -> Option<i64> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(HEAD_MAX).read_to_end(&mut bytes).ok()?;
    own::timestamp_of(&bytes)
}

#[cfg(test)]
mod tests;
