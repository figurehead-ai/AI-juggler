//! AJ-E1520 (0.3): what one Claude Code transcript says about the agent
//! that wrote it — a subagent's own `agent-<id>.jsonl`, or the session's
//! — read as it grows: the subagents it started (the shared subagent
//! log), its tokens (each API response counted once: Claude Code writes
//! one line per content block, each with the response's usage, keyed by
//! `message.id`, as the usage pipeline dedupes them), the model it ran
//! on, its tool calls by kind, the files it edited, the commands it ran,
//! the lines its edits changed (`toolUseResult.structuredPatch`, or a
//! new file's lines), its first prompt and its last activity.
//!
//! Most of a long transcript is tool output and request records
//! (`api-request-blob`): a line is parsed only when a cheap byte search
//! says it can matter (an assistant message, an edit's result, the first
//! prompt), and an assistant line is read into typed fields that skip
//! the rest without copying it.

use std::collections::{HashMap, VecDeque};

use juggler_adapter::subagents::{
    COMMAND_CHARS, COMMANDS_MAX, LinesChanged, PROMPT_CHARS, SubagentCommand, SubagentText,
    SubagentTokens, TextFilter, ToolCounts,
};
use serde::{Deserialize, Serialize};

use crate::subtasks::{SubtaskLog, cut, holds};
use crate::transcript_tail::TailLog;

/// The latest responses kept to count each one once (a response's lines
/// come together; a few more absorb an interleaving).
const RECENT: usize = 8;

/// Tool call ids kept to count each call once.
const RECENT_CALLS: usize = 16;

/// The most files one agent's record keeps by name.
const FILES_KEPT: usize = 500;

/// One transcript's agent, as read so far.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct OwnLog {
    /// The subagents it started.
    pub(crate) nested: SubtaskLog,
    /// Its tokens, each response once.
    pub(crate) tokens: SubagentTokens,
    /// The latest responses' usage, by id, to count a response once.
    recent: VecDeque<(String, SubagentTokens)>,
    /// The latest tool calls, by id.
    calls: VecDeque<String>,
    /// The model its latest response named.
    pub(crate) model: Option<String>,
    /// Its tool calls by kind.
    pub(crate) tools: ToolCounts,
    /// Lines its edits changed, once one was read.
    pub(crate) lines: Option<LinesChanged>,
    /// The files it edited or wrote, and how many times.
    pub(crate) files: HashMap<String, u32>,
    /// How many files it edited (beyond those kept by name too).
    pub(crate) files_total: u32,
    /// The commands it ran, the latest kept.
    pub(crate) commands: VecDeque<SubagentCommand>,
    /// Its first prompt (a subagent's task, whole).
    pub(crate) prompt: Option<String>,
    /// Its first and last line's time.
    pub(crate) first_ms: Option<i64>,
    pub(crate) last_ms: Option<i64>,
}

impl TailLog for OwnLog {
    fn feed(&mut self, bytes: &[u8]) {
        self.nested.feed(bytes);
        for line in bytes.split(|b| *b == b'\n') {
            if line.len() < 2 {
                continue;
            }
            // Request records: the bulk of a long transcript, never read.
            if holds(&line[..line.len().min(40)], br#""type":"api-request"#) {
                continue;
            }
            let at = timestamp_of(line);
            if let Some(at) = at {
                self.first_ms.get_or_insert(at);
                self.last_ms = Some(self.last_ms.map_or(at, |last| last.max(at)));
            }
            if holds(line, br#""role":"assistant""#) {
                if let Ok(parsed) = serde_json::from_slice::<AssistantLine>(line) {
                    self.assistant(parsed, at);
                }
            } else if holds(line, br#""structuredPatch""#) || holds(line, br#""type":"create""#) {
                self.patched(line);
            } else if self.prompt.is_none()
                && holds(line, br#""type":"user""#)
                && !holds(line, b"tool_result")
            {
                self.prompt = first_prompt(line);
            }
        }
    }
}

impl OwnLog {
    fn assistant(&mut self, line: AssistantLine, at: Option<i64>) {
        let (Some("assistant"), Some(message)) = (line.kind.as_deref(), line.message) else {
            return;
        };
        if let Some(model) = message.model.filter(|model| !model.starts_with('<')) {
            self.model = Some(model);
        }
        if let (Some(usage), Some(key)) = (message.usage.as_ref(), message.id.or(line.request_id)) {
            self.counted(&key, tokens_of(usage));
        }
        for part in message.content {
            if part.kind.as_deref() != Some("tool_use") {
                continue;
            }
            if let Some(id) = part.id {
                if self.calls.contains(&id) {
                    continue;
                }
                self.calls.push_back(id);
                if self.calls.len() > RECENT_CALLS {
                    self.calls.pop_front();
                }
            }
            self.called(part.name.as_deref().unwrap_or_default(), &part.input, at);
        }
    }

    /// One response's usage: a response seen before adds only what grew.
    fn counted(&mut self, key: &str, tokens: SubagentTokens) {
        if let Some((_, kept)) = self.recent.iter_mut().find(|(seen, _)| seen == key) {
            let grown = SubagentTokens {
                input: tokens.input.saturating_sub(kept.input),
                output: tokens.output.saturating_sub(kept.output),
                cache_read: tokens.cache_read.saturating_sub(kept.cache_read),
                cache_write: tokens.cache_write.saturating_sub(kept.cache_write),
            };
            kept.input = kept.input.max(tokens.input);
            kept.output = kept.output.max(tokens.output);
            kept.cache_read = kept.cache_read.max(tokens.cache_read);
            kept.cache_write = kept.cache_write.max(tokens.cache_write);
            self.tokens = self.tokens.plus(grown);
            return;
        }
        self.tokens = self.tokens.plus(tokens);
        self.recent.push_back((key.to_owned(), tokens));
        if self.recent.len() > RECENT {
            self.recent.pop_front();
        }
    }

    fn called(&mut self, name: &str, input: &Input, at: Option<i64>) {
        let tools = &mut self.tools;
        tools.total += 1;
        match name {
            "Read" | "NotebookRead" => tools.reads += 1,
            "Grep" | "Glob" | "LS" | "WebSearch" | "WebFetch" | "ToolSearch" => tools.searches += 1,
            "Bash" | "BashOutput" | "KillShell" | "KillBash" | "Monitor" => tools.commands += 1,
            "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => tools.edits += 1,
            _ => tools.other += 1,
        }
        if matches!(name, "Edit" | "MultiEdit" | "Write" | "NotebookEdit")
            && let Some(path) =
                text_of(input.file_path.as_ref()).or_else(|| text_of(input.notebook_path.as_ref()))
        {
            if let Some(count) = self.files.get_mut(path) {
                *count += 1;
            } else {
                self.files_total += 1;
                if self.files.len() < FILES_KEPT {
                    self.files.insert(path.to_owned(), 1);
                }
            }
        }
        if name == "Bash"
            && let Some(command) = text_of(input.command.as_ref())
        {
            self.commands.push_back(SubagentCommand {
                command: cut(command, COMMAND_CHARS),
                at_ms: at,
                background: input
                    .run_in_background
                    .as_ref()
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            });
            if self.commands.len() > COMMANDS_MAX {
                self.commands.pop_front();
            }
        }
    }

    /// An edit's result: the lines it added and removed.
    fn patched(&mut self, line: &[u8]) {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            return;
        };
        let Some(result) = value.get("toolUseResult") else {
            return;
        };
        let lines = self.lines.get_or_insert_with(LinesChanged::default);
        if result.get("type").and_then(serde_json::Value::as_str) == Some("create")
            && let Some(content) = result.get("content").and_then(serde_json::Value::as_str)
        {
            lines.added += content.lines().count() as u64;
            return;
        }
        let hunks = result
            .get("structuredPatch")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten();
        for hunk in hunks {
            let changed = hunk
                .get("lines")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str);
            for changed in changed {
                if changed.starts_with('+') {
                    lines.added += 1;
                } else if changed.starts_with('-') {
                    lines.removed += 1;
                }
            }
        }
    }

    /// The files it edited, most-edited first (then by name).
    pub(crate) fn files_ranked(&self, most: usize) -> Vec<String> {
        let mut files: Vec<(&String, &u32)> = self.files.iter().collect();
        files.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        files
            .into_iter()
            .take(most)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Passes every word it keeps through `text` (a save's redaction).
    pub(crate) fn filter_texts(&mut self, text: &TextFilter<'_>) {
        self.nested.filter_texts(text);
        self.prompt = self
            .prompt
            .take()
            .and_then(|prompt| text(SubagentText::Prompt, &prompt));
        self.files = std::mem::take(&mut self.files)
            .into_iter()
            .filter_map(|(path, count)| Some((text(SubagentText::File, &path)?, count)))
            .collect();
        self.commands = std::mem::take(&mut self.commands)
            .into_iter()
            .filter_map(|mut command| {
                command.command = text(SubagentText::Command, &command.command)?;
                Some(command)
            })
            .collect();
    }
}

/// An assistant line, only the fields read (the rest is skipped, not
/// copied).
#[derive(Deserialize)]
struct AssistantLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "parts")]
    content: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    #[serde(rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
    #[serde(default)]
    input: Input,
}

/// A tool call's input: only what's counted (any shape: a field of
/// another type is read as nothing).
#[derive(Default, Deserialize)]
struct Input {
    file_path: Option<serde_json::Value>,
    notebook_path: Option<serde_json::Value>,
    command: Option<serde_json::Value>,
    run_in_background: Option<serde_json::Value>,
}

/// A message's content parts; a plain string (or anything else) has
/// none.
fn parts<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<Part>, D::Error> {
    struct Parts;
    impl<'de> serde::de::Visitor<'de> for Parts {
        type Value = Vec<Part>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("content")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Vec<Part>, A::Error> {
            let mut out = Vec::new();
            while let Some(part) = seq.next_element::<Option<Part>>()? {
                out.extend(part);
            }
            Ok(out)
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Vec<Part>, E> {
            Ok(Vec::new())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Vec<Part>, E> {
            Ok(Vec::new())
        }
    }
    deserializer.deserialize_any(Parts)
}

fn text_of(value: Option<&serde_json::Value>) -> Option<&str> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// A usage block's tokens by kind.
fn tokens_of(usage: &serde_json::Value) -> SubagentTokens {
    let read = |key: &str| {
        usage
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
    };
    SubagentTokens {
        input: read("input_tokens"),
        output: read("output_tokens"),
        cache_read: read("cache_read_input_tokens"),
        cache_write: read("cache_creation_input_tokens"),
    }
}

/// A line's `"timestamp"` (a quoted key never appears unescaped inside a
/// JSON string, so the first is an object's own).
pub(crate) fn timestamp_of(line: &[u8]) -> Option<i64> {
    const KEY: &[u8] = br#""timestamp":""#;
    let at = memchr::memmem::find(line, KEY)? + KEY.len();
    let end = memchr::memchr(b'"', &line[at..])? + at;
    juggler_core::time::timestamp_ms(std::str::from_utf8(&line[at..end]).ok()?)
}

/// The text of a `user` line (a subagent's first: its prompt).
fn first_prompt(line: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    if value.get("type")?.as_str()? != "user" {
        return None;
    }
    let content = value.get("message")?.get("content")?;
    let text = match content {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let text = cut(&text, PROMPT_CHARS);
    (!text.is_empty()).then_some(text)
}
