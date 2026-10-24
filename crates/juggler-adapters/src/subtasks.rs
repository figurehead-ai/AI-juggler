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

use std::collections::{HashMap, HashSet};

use crate::transcript::{TranscriptFormat, line_format};

/// The most characters of a subagent's task kept as its label.
const LABEL_CHARS: usize = 80;

/// Where a subagent stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtaskState {
    /// Started, and nothing in the transcript says it has ended.
    Running,
    /// Started in the background: this transcript doesn't say when it
    /// ends, so it isn't counted as running.
    Background,
    /// Ended (finished, or closed by the session).
    Done,
    /// Ended with an error, or never started.
    Failed,
}

impl SubtaskState {
    /// Done or failed: nothing later changes it.
    #[must_use]
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Failed)
    }
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
    /// A call whose answer says how named agents stand (Codex's `wait`).
    waits: HashSet<String>,
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
                    crate::claude_transcript::subtask_line(self, &value);
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

    /// A later call's answer says how named agents stand.
    pub(crate) fn polls(&mut self, call: &str) {
        self.waits.insert(call.to_owned());
    }

    /// Whether a call's answer matters here (it started, ends or reports on
    /// a subagent); any other call's output needn't be read.
    pub(crate) fn wants(&self, call: &str) -> bool {
        self.calls.contains_key(call)
            || self.awaited.contains_key(call)
            || self.waits.contains(call)
    }

    /// A call was answered: the subagent it started, or the one it was
    /// waiting on, now stands at `state`.
    pub(crate) fn answered(&mut self, call: &str, state: SubtaskState) {
        self.waits.remove(call);
        if let Some(&index) = self.calls.get(call) {
            self.set(index, state);
        } else if let Some(agent) = self.awaited.remove(call) {
            self.ended(&agent, state);
        }
    }

    /// A wait's answer arrived (what it said is applied by the caller).
    pub(crate) fn heard(&mut self, call: &str) {
        self.waits.remove(call);
    }

    /// A call's answer said nothing usable: a start that named no agent
    /// failed; a close or a wait changes nothing.
    pub(crate) fn not_started(&mut self, call: &str) {
        self.waits.remove(call);
        self.awaited.remove(call);
        if let Some(&index) = self.calls.get(call) {
            self.set(index, SubtaskState::Failed);
        }
    }

    /// A named agent now stands at `state`.
    pub(crate) fn ended(&mut self, agent: &str, state: SubtaskState) {
        if let Some(&index) = self.agents.get(agent) {
            self.set(index, state);
        }
    }

    /// Done and failed are final: a later close says nothing new.
    fn set(&mut self, index: usize, state: SubtaskState) {
        if let Some(item) = self.items.get_mut(index)
            && !item.state.is_final()
        {
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

/// Reads a transcript as it grows, into a [`SubtaskLog`] (only what was
/// appended since the last read).
pub type SubtaskTail = crate::transcript_tail::TranscriptTail<SubtaskLog>;

impl crate::transcript_tail::TailLog for SubtaskLog {
    fn feed(&mut self, bytes: &[u8]) {
        Self::feed(self, bytes);
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
    fn a_file_replaced_by_one_as_long_is_read_from_its_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(&path, format!("{CLAUDE_START}\n{CLAUDE_DONE}\n")).unwrap();
        let mut reader = SubtaskTail::default();
        reader.read(&path).unwrap();
        assert_eq!(reader.log().running(), 0);
        // Another file of the same length, moved into place: the start
        // only, padded.
        let other = dir.path().join("other.jsonl");
        let mut body = format!("{CLAUDE_START}\n");
        let pad = std::fs::metadata(&path).unwrap().len() as usize - body.len() - 1;
        body.push_str(&" ".repeat(pad));
        body.push('\n');
        std::fs::write(&other, body).unwrap();
        std::fs::rename(&other, &path).unwrap();
        reader.read(&path).unwrap();
        assert_eq!(reader.log().subtasks().len(), 1);
        assert_eq!(
            reader.log().running(),
            1,
            "read from the start, not the old offset"
        );
    }

    #[test]
    fn read_all_takes_a_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(&path, format!("{CLAUDE_START}\n{CLAUDE_DONE}\n")).unwrap();
        let mut reader = SubtaskTail::default();
        reader.read_all(&path).unwrap();
        assert_eq!(reader.log().subtasks()[0].state, SubtaskState::Done);
    }

    #[test]
    fn done_and_failed_are_final() {
        let mut log = super::SubtaskLog::default();
        log.started("c1", "one", None);
        log.named("c1", "a1");
        log.ended("a1", SubtaskState::Failed);
        log.ended("a1", SubtaskState::Done);
        log.ended("a1", SubtaskState::Running);
        assert_eq!(log.subtasks()[0].state, SubtaskState::Failed);
        // A close or a wait with nothing usable changes nothing.
        log.started("c2", "two", None);
        log.named("c2", "a2");
        log.awaits("x2", "a2");
        log.not_started("x2");
        assert_eq!(log.subtasks()[1].state, SubtaskState::Running);
        assert!(!log.wants("x2"));
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
