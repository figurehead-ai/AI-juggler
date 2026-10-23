//! AJ-E1520: the subagents a session started (I-08), and what became of
//! each, read from its transcript.
//!
//! Claude Code starts one with its Task (now Agent) tool and answers that
//! call when the subagent is done; Codex starts one with `spawn_agent`,
//! whose output names the new agent, and says how it ended in later
//! `wait`/`close_agent` outputs. Each tool's parser reads its own lines
//! (`claude_transcript::subtask_line`, `codex_transcript::subtask_line`);
//! this module keeps the tool-neutral record, and reads a growing file a
//! piece at a time, so a tick only reads what was appended.

use std::collections::HashMap;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;

use crate::transcript::{TranscriptFormat, line_format};

/// The most characters of a subagent's task kept as its label.
const LABEL_CHARS: usize = 80;

/// The most bytes one [`SubtaskTail::read`] takes in, so a first read of
/// a long transcript is spread over a few ticks.
const READ_PER_CALL: u64 = 16 * 1024 * 1024;

/// Where a subagent stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtaskState {
    /// Started, and nothing in the transcript says it has ended.
    Running,
    /// Ended (finished, or closed by the session).
    Done,
    /// Ended with an error, or never started.
    Failed,
}

/// One subagent a session started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtask {
    /// The call that started it (stable within the transcript).
    pub id: String,
    /// What it was asked to do, on one line.
    pub label: String,
    /// The kind of agent, when the call named one (`Explore`, `worker`).
    pub kind: Option<String>,
    /// Where it stands.
    pub state: SubtaskState,
}

/// The subagents read so far from one transcript.
#[derive(Debug, Clone, Default)]
pub struct SubtaskLog {
    items: Vec<Subtask>,
    /// Starting call id → index into `items`.
    calls: HashMap<String, usize>,
    /// The agent's own id, once the tool named it → index into `items`.
    agents: HashMap<String, usize>,
    /// A call whose answer ends a named agent (Codex's `close_agent`).
    awaited: HashMap<String, String>,
    /// Decided by the first parsable line.
    format: Option<TranscriptFormat>,
}

impl SubtaskLog {
    /// Reads whole lines (`\n`-separated); a malformed line is skipped.
    pub fn feed(&mut self, bytes: &[u8]) {
        for line in bytes.split(|b| *b == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
                continue;
            };
            let format = *self.format.get_or_insert_with(|| line_format(&value));
            match format {
                TranscriptFormat::ClaudeCode => {
                    crate::claude_transcript::subtask_line(self, &value)
                }
                TranscriptFormat::CodexRollout => {
                    crate::codex_transcript::subtask_line(self, &value);
                }
            }
        }
    }

    /// Every subagent, in the order they were started.
    #[must_use]
    pub fn subtasks(&self) -> &[Subtask] {
        &self.items
    }

    /// How many are still running (the status row's count, I-08).
    #[must_use]
    pub fn running(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.state == SubtaskState::Running)
            .count()
    }

    /// A call started a subagent.
    pub(crate) fn started(&mut self, call: &str, task: &str, kind: Option<&str>) {
        if self.calls.contains_key(call) {
            return;
        }
        self.calls.insert(call.to_owned(), self.items.len());
        self.items.push(Subtask {
            id: call.to_owned(),
            label: one_line(task),
            kind: kind.filter(|kind| !kind.is_empty()).map(str::to_owned),
            state: SubtaskState::Running,
        });
    }

    /// The tool named the agent a call started.
    pub(crate) fn named(&mut self, call: &str, agent: &str) {
        if let Some(&index) = self.calls.get(call) {
            self.agents.insert(agent.to_owned(), index);
        }
    }

    /// A later call's answer will end this agent.
    pub(crate) fn awaits(&mut self, call: &str, agent: &str) {
        self.awaited.insert(call.to_owned(), agent.to_owned());
    }

    /// A call was answered: the subagent it started, or the one it was
    /// waiting on, now stands at `state`.
    pub(crate) fn answered(&mut self, call: &str, state: SubtaskState) {
        if let Some(&index) = self.calls.get(call) {
            self.set(index, state);
        } else if let Some(agent) = self.awaited.remove(call) {
            self.ended(&agent, state);
        }
    }

    /// A named agent now stands at `state`.
    pub(crate) fn ended(&mut self, agent: &str, state: SubtaskState) {
        if let Some(&index) = self.agents.get(agent) {
            self.set(index, state);
        }
    }

    fn set(&mut self, index: usize, state: SubtaskState) {
        if let Some(item) = self.items.get_mut(index) {
            item.state = state;
        }
    }
}

/// Every subagent in a whole transcript.
#[must_use]
pub fn subtasks_of(bytes: &[u8]) -> SubtaskLog {
    let mut log = SubtaskLog::default();
    log.feed(bytes);
    log
}

/// Reads a transcript as it grows: each [`SubtaskTail::read`] takes in only
/// the bytes appended since the last one, and a line still being written
/// waits for the next read.
#[derive(Debug, Clone, Default)]
pub struct SubtaskTail {
    log: SubtaskLog,
    offset: u64,
    partial: Vec<u8>,
}

impl SubtaskTail {
    /// Reads what was appended to `path` (at most 16 MiB a call). A file
    /// that got shorter was replaced: it's read again from the start.
    ///
    /// # Errors
    ///
    /// The file can't be opened or read.
    pub fn read(&mut self, path: &Path) -> std::io::Result<()> {
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        if len < self.offset {
            *self = Self::default();
        }
        if len == self.offset {
            return Ok(());
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut chunk = Vec::new();
        file.take(READ_PER_CALL).read_to_end(&mut chunk)?;
        self.offset += chunk.len() as u64;
        self.partial.extend_from_slice(&chunk);
        if let Some(end) = self.partial.iter().rposition(|b| *b == b'\n') {
            let rest = self.partial.split_off(end + 1);
            let whole = std::mem::replace(&mut self.partial, rest);
            self.log.feed(&whole);
        }
        Ok(())
    }

    /// What has been read so far.
    #[must_use]
    pub fn log(&self) -> &SubtaskLog {
        &self.log
    }
}

/// The first line of a task, cut to [`LABEL_CHARS`].
fn one_line(text: &str) -> String {
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let mut cut: String = first.chars().take(LABEL_CHARS).collect();
    if first.chars().count() > LABEL_CHARS {
        cut.push('…');
    }
    cut
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{SubtaskState, SubtaskTail, subtasks_of};

    const CLAUDE_START: &str = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Task","input":{"description":"Find the parser","prompt":"Look for it","subagent_type":"Explore"}}]}}"#;
    const CLAUDE_DONE: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"found it"}]},"toolUseResult":{"status":"completed","agentId":"a1"}}"#;

    #[test]
    fn a_line_still_being_written_waits_for_the_next_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        let (head, tail) = CLAUDE_DONE.split_at(40);
        std::fs::write(&path, format!("{CLAUDE_START}\n{head}")).unwrap();
        let mut reader = SubtaskTail::default();
        reader.read(&path).unwrap();
        assert_eq!(
            reader.log().running(),
            1,
            "started, its answer half-written"
        );
        std::fs::write(&path, format!("{CLAUDE_START}\n{head}{tail}\n")).unwrap();
        reader.read(&path).unwrap();
        assert_eq!(reader.log().running(), 0);
        assert_eq!(reader.log().subtasks()[0].state, SubtaskState::Done);
    }

    #[test]
    fn a_replaced_file_is_read_again_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(&path, format!("{CLAUDE_START}\n{CLAUDE_DONE}\n")).unwrap();
        let mut reader = SubtaskTail::default();
        reader.read(&path).unwrap();
        std::fs::write(&path, format!("{CLAUDE_START}\n")).unwrap();
        reader.read(&path).unwrap();
        assert_eq!(reader.log().subtasks().len(), 1);
        assert_eq!(reader.log().running(), 1);
    }

    #[test]
    fn a_label_is_the_first_line_cut_short() {
        let long = "x".repeat(200);
        let line = format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t","name":"Agent","input":{{"prompt":"\n{long}\nmore"}}}}]}}}}"#
        );
        let log = subtasks_of(line.as_bytes());
        let label = &log.subtasks()[0].label;
        assert_eq!(label.chars().count(), 81, "{label}");
        assert!(label.ends_with('…'));
        assert_eq!(log.subtasks()[0].kind, None);
    }
}
