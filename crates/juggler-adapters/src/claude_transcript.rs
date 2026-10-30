//! Claude Code transcript parsing (spec §4.1, M1-5).
//!
//! Claude Code writes one JSONL file per session under
//! `<config>/projects/<mangled-cwd>/`. The line shape is internal and
//! version-churning (research `docs/research/claude-code.md` §4), so this
//! parser is deliberately tolerant: lines parse into `serde_json::Value`
//! and known fields are extracted defensively. Unknown types are counted,
//! malformed lines are skipped and counted — parsing never panics and never
//! fails the whole file (`docs/plan/QUALITY.md` §1: degrade, never crash).

use std::collections::BTreeMap;

use juggler_core::model::TokenCounts;

/// Aggregate result of parsing one transcript file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedTranscript {
    /// Non-empty lines seen.
    pub lines_total: usize,
    /// Lines that were not valid JSON.
    pub lines_malformed: usize,
    /// Count per `type` field value (`"unknown"` when absent).
    pub kinds: BTreeMap<String, usize>,
    /// Count per `type/subtype` pair (e.g. `system/compact_boundary`)
    /// — the compaction scar lives in the subtype (AJ-E204 §H2.1).
    pub subtypes: BTreeMap<String, usize>,
    /// Extracted conversation messages (user + assistant, in file order).
    pub messages: Vec<ParsedMessage>,
    /// Summed token usage across assistant messages.
    pub usage: TokenCounts,
    /// AJ-E1541: the task list as the last call that set one left it.
    pub tasks: Option<Vec<crate::tool_summary::TaskItem>>,
    /// What the conversation carries besides its words: tool calls, their
    /// results and images (the measured switch fidelity counts them).
    pub content: ContentCounts,
}

/// What a conversation's main thread carries besides text, counted. A
/// subagent's own (sidechain) lines are left out: they are its work, not
/// the conversation's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct ContentCounts {
    /// Tool calls the model made.
    pub tool_calls: usize,
    /// Results that came back to it.
    pub tool_results: usize,
    /// Of those, results that were errors.
    pub tool_errors: usize,
    /// Images (and other attachments) in the conversation's messages.
    pub images: usize,
}

/// One conversation message extracted from a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMessage {
    /// Message uuid; absent in some versions/line shapes.
    pub uuid: Option<String>,
    /// Parent message uuid (conversation tree).
    pub parent_uuid: Option<String>,
    /// Owning session id, when present on the line.
    pub session_id: Option<String>,
    /// Conversational role.
    pub role: MessageRole,
    /// Concatenated text content (tool calls/results excluded).
    pub text: String,
    /// AJ-E1541: one line per tool call this message made, in order.
    pub tools: Vec<String>,
    /// Token usage carried by this line, if any.
    pub usage: Option<TokenCounts>,
    /// The model this assistant turn actually ran (verbatim vendor
    /// string; top-level `model` or `message.model`). `None` on lines
    /// that do not record one — the honest per-turn source (AJ-E412).
    pub model: Option<String>,
    /// True for sidechain (subagent) lines.
    pub sidechain: bool,
}

/// Conversational role of a parsed message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// Human/user turn.
    User,
    /// Model turn.
    Assistant,
    /// Anything else that still carries a message body.
    Other,
}

/// Parses transcript bytes. Never fails; damage is reported in the
/// aggregate counters.
#[must_use]
pub fn parse_transcript(bytes: &[u8]) -> ParsedTranscript {
    let mut parsed = ParsedTranscript::default();
    let mut board = TaskBoard::default();
    for line in bytes.split(|b| *b == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        parsed.lines_total += 1;
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
            parsed.lines_malformed += 1;
            continue;
        };
        let kind = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *parsed.kinds.entry(kind.clone()).or_default() += 1;
        if let Some(subtype) = value.get("subtype").and_then(serde_json::Value::as_str) {
            *parsed
                .subtypes
                .entry(format!("{kind}/{subtype}"))
                .or_default() += 1;
        }
        if (kind == "user" || kind == "assistant")
            && let Some(message) = extract_message(&value, &kind)
        {
            if !message.sidechain {
                count_content(
                    &mut parsed.content,
                    value.get("message").and_then(|m| m.get("content")),
                );
                if board.line(&value) {
                    parsed.tasks = Some(board.items());
                }
            }
            accumulate_usage(&mut parsed.usage, message.usage);
            for (name, input) in tool_uses(value.get("message").and_then(|m| m.get("content"))) {
                if let Some(tasks) = crate::tool_summary::task_list(name, input) {
                    parsed.tasks = Some(tasks);
                }
            }
            parsed.messages.push(message);
        }
    }
    parsed
}

/// Claude Code's task tools (2.1.x), which keep the task list outside the
/// transcript: `TaskCreate` adds an item (its result names the item's id),
/// `TaskUpdate` changes one's status or subject, or deletes it. The list
/// is rebuilt from the calls, each call once (a message's lines repeat
/// it). The live lab of 2026-10-28: without this a session using them had
/// no task list in a handover, and its switch measured none.
#[derive(Debug, Default)]
struct TaskBoard {
    /// Each item with its id, once known.
    items: Vec<(Option<String>, crate::tool_summary::TaskItem)>,
    /// Calls already applied (by their `tool_use` id).
    seen: std::collections::BTreeSet<String>,
    /// A create call's `tool_use` id → its item.
    created: BTreeMap<String, usize>,
}

impl TaskBoard {
    /// Applies one line; whether the list changed.
    fn line(&mut self, value: &serde_json::Value) -> bool {
        let Some(serde_json::Value::Array(parts)) =
            value.get("message").and_then(|m| m.get("content"))
        else {
            return false;
        };
        let mut changed = false;
        for part in parts {
            match part.get("type").and_then(serde_json::Value::as_str) {
                Some("tool_use") => changed |= self.call(part),
                Some("tool_result") => {
                    let created = string_field(part, "tool_use_id")
                        .and_then(|call| self.created.get(&call).copied());
                    let id = value
                        .get("toolUseResult")
                        .and_then(|result| result.get("task"))
                        .and_then(|task| string_field(task, "id"));
                    if let (Some(index), Some(id)) = (created, id) {
                        self.items[index].0 = Some(id);
                    }
                }
                _ => {}
            }
        }
        changed
    }

    fn call(&mut self, part: &serde_json::Value) -> bool {
        let name = part.get("name").and_then(serde_json::Value::as_str);
        if !matches!(name, Some("TaskCreate" | "TaskUpdate")) {
            return false;
        }
        if let Some(call) = string_field(part, "id")
            && !self.seen.insert(call)
        {
            return false;
        }
        let input = part.get("input").unwrap_or(&serde_json::Value::Null);
        if name == Some("TaskCreate") {
            let Some(text) = string_field(input, "subject") else {
                return false;
            };
            if let Some(call) = string_field(part, "id") {
                self.created.insert(call, self.items.len());
            }
            self.items.push((
                None,
                crate::tool_summary::TaskItem {
                    text,
                    done: false,
                    in_progress: false,
                },
            ));
            return true;
        }
        let Some(id) = string_field(input, "taskId") else {
            return false;
        };
        // An item whose create result wasn't read is known by its place
        // (Claude Code numbers them from 1).
        let at = self
            .items
            .iter()
            .enumerate()
            .position(|(index, (known, _))| {
                known
                    .as_deref()
                    .map_or_else(|| (index + 1).to_string() == id, |known| known == id)
            });
        let Some(at) = at else {
            return false;
        };
        if let Some(subject) = string_field(input, "subject") {
            self.items[at].1.text = subject;
        }
        match string_field(input, "status").as_deref() {
            Some("deleted") => {
                self.items.remove(at);
                self.created.retain(|_, index| *index != at);
                for index in self.created.values_mut() {
                    if *index > at {
                        *index -= 1;
                    }
                }
            }
            Some(status) => {
                let item = &mut self.items[at].1;
                item.done = status == "completed";
                item.in_progress = status == "in_progress";
            }
            None => {}
        }
        true
    }

    fn items(&self) -> Vec<crate::tool_summary::TaskItem> {
        self.items.iter().map(|(_, item)| item.clone()).collect()
    }
}

/// Counts one message's tool calls, tool results (and errors among them)
/// and images: Claude Code keeps each as a typed part of `content`.
fn count_content(counts: &mut ContentCounts, content: Option<&serde_json::Value>) {
    let Some(serde_json::Value::Array(parts)) = content else {
        return;
    };
    for part in parts {
        match part.get("type").and_then(serde_json::Value::as_str) {
            Some("tool_use") => counts.tool_calls += 1,
            Some("tool_result") => {
                counts.tool_results += 1;
                if part.get("is_error").and_then(serde_json::Value::as_bool) == Some(true) {
                    counts.tool_errors += 1;
                }
            }
            Some("image" | "document") => counts.images += 1,
            _ => {}
        }
    }
}

fn extract_message(value: &serde_json::Value, kind: &str) -> Option<ParsedMessage> {
    let message = value.get("message")?;
    let role = match message.get("role").and_then(serde_json::Value::as_str) {
        Some("user" | "human") => MessageRole::User,
        Some("assistant") => MessageRole::Assistant,
        _ if kind == "user" => MessageRole::User,
        _ if kind == "assistant" => MessageRole::Assistant,
        _ => MessageRole::Other,
    };
    let model = string_field(value, "model").or_else(|| string_field(message, "model"));
    Some(ParsedMessage {
        uuid: string_field(value, "uuid"),
        parent_uuid: string_field(value, "parentUuid"),
        session_id: string_field(value, "sessionId"),
        role,
        text: extract_text(message.get("content")),
        tools: tool_uses(message.get("content"))
            .into_iter()
            .map(|(name, input)| crate::tool_summary::summarize(name, input))
            .collect(),
        usage: extract_usage(message.get("usage")),
        model,
        sidechain: value
            .get("isSidechain")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// Content is either a plain string or an array of typed parts; text parts
/// are joined, everything else (`tool_use`/`tool_result`/…) contributes nothing
/// to the readable conversation.
fn extract_text(content: Option<&serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("text"))
            .filter_map(|part| {
                part.get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// AJ-E1520: a line's part in the session's subagents. The Task tool
/// (named Agent in later versions) starts one; the call's `tool_result`
/// ends it, failed when it's an error. A subagent started in the
/// background is answered at once (`toolUseResult.status`
/// `async_launched`): this transcript doesn't say when it ends, so it's
/// marked as in the background, not counted as running, until the
/// session hears how it ended: a `<task-notification>` naming the agent
/// (`<task-id>`) and its `<status>` (`completed`, `failed`, `killed`,
/// `stopped`), queued for the session (a `queue-operation` or a
/// `queued_command` attachment) and then given to it (a `user` line whose
/// `origin.kind` is `task-notification`). The first one heard counts.
pub(crate) fn subtask_line(log: &mut crate::subtasks::SubtaskLog, value: &serde_json::Value) {
    use crate::subtasks::SubtaskState;
    if let Some(text) = notification_text(value) {
        for told in notifications(text) {
            log.ended(&told.agent, told.state);
            log.told(told.call.as_deref(), Some(&told.agent), told.ending);
        }
        return;
    }
    // AJ-E1520 (0.3): the user's message a turn begins with.
    if let Some((text, key)) = user_prompt(value) {
        log.asked(&text, &key);
        return;
    }
    let content = value.get("message").and_then(|m| m.get("content"));
    let Some(serde_json::Value::Array(parts)) = content else {
        return;
    };
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("assistant") => {
            for part in parts {
                let is_agent = part.get("type").and_then(serde_json::Value::as_str)
                    == Some("tool_use")
                    && matches!(
                        part.get("name").and_then(serde_json::Value::as_str),
                        Some("Task" | "Agent")
                    );
                let (true, Some(id)) = (is_agent, string_field(part, "id")) else {
                    continue;
                };
                let input = part.get("input").unwrap_or(&serde_json::Value::Null);
                let task = string_field(input, "description")
                    .or_else(|| string_field(input, "prompt"))
                    .unwrap_or_default();
                log.started_with(
                    &id,
                    &task,
                    string_field(input, "subagent_type").as_deref(),
                    input.get("prompt").and_then(serde_json::Value::as_str),
                );
            }
        }
        Some("user") => {
            let results: Vec<&serde_json::Value> = parts
                .iter()
                .filter(|part| {
                    part.get("type").and_then(serde_json::Value::as_str) == Some("tool_result")
                })
                .collect();
            // `toolUseResult` describes the line's one result.
            let outcome = (results.len() == 1)
                .then(|| value.get("toolUseResult"))
                .flatten();
            for result in results {
                let Some(id) = string_field(result, "tool_use_id") else {
                    continue;
                };
                if let Some(agent) = outcome.and_then(|outcome| string_field(outcome, "agentId")) {
                    log.named(&id, &agent);
                }
                let failed = result
                    .get("is_error")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let background = outcome
                    .and_then(|outcome| string_field(outcome, "status"))
                    .is_some_and(|status| status == "async_launched");
                let state = if failed {
                    SubtaskState::Failed
                } else if background {
                    SubtaskState::Background
                } else {
                    SubtaskState::Done
                };
                if log.is_start(&id) {
                    log.told(Some(&id), None, answer(result, outcome, failed, background));
                }
                log.answered(&id, state);
            }
        }
        _ => {}
    }
}

/// AJ-E1520 (0.3): what an Agent call's answer says: its text (a
/// subagent in the foreground answers with its result; one in the
/// background with only its launch, so no result), why it failed, and
/// what Claude Code reported (`toolUseResult`: the model, its tokens,
/// tool calls by kind, lines changed, how long it ran).
fn answer(
    result: &serde_json::Value,
    outcome: Option<&serde_json::Value>,
    failed: bool,
    background: bool,
) -> crate::subtasks::Ending {
    use juggler_adapter::subagents::{LinesChanged, ToolCounts};
    let text = extract_text(result.get("content"));
    let number = |key: &str| {
        outcome
            .and_then(|o| o.get(key))
            .and_then(serde_json::Value::as_u64)
    };
    let tools = outcome.and_then(|o| o.get("toolStats")).map(|stats| {
        let count = |key: &str| {
            stats
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(0)
        };
        let reads = count("readCount");
        let searches = count("searchCount");
        let commands = count("bashCount");
        let edits = count("editFileCount");
        let other = count("otherToolCount");
        let lines = LinesChanged {
            added: stats
                .get("linesAdded")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            removed: stats
                .get("linesRemoved")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        };
        (
            ToolCounts {
                total: number("totalToolUseCount")
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(reads + searches + commands + edits + other),
                reads,
                searches,
                commands,
                edits,
                other,
            },
            lines,
        )
    });
    crate::subtasks::Ending {
        result: (!failed && !background && !text.trim().is_empty()).then(|| text.clone()),
        failure: failed.then_some(text),
        reported: crate::subtasks::Reported {
            model: outcome.and_then(|o| string_field(o, "resolvedModel")),
            tokens: number("totalTokens"),
            tool_uses: number("totalToolUseCount").and_then(|n| u32::try_from(n).ok()),
            duration_ms: number("totalDurationMs").and_then(|n| i64::try_from(n).ok()),
            tools: tools.map(|(tools, _)| tools),
            lines: tools.map(|(_, lines)| lines),
        },
    }
}

/// AJ-E1520 (0.3): the user's own message on a line, and the line's id:
/// a `user` line from a person (not a tool's result, a note Claude Code
/// adds, a notification or another agent's message), its text not a
/// command's tag.
fn user_prompt(value: &serde_json::Value) -> Option<(String, String)> {
    if value.get("type")?.as_str()? != "user"
        || value.get("isMeta").and_then(serde_json::Value::as_bool) == Some(true)
        || value
            .get("isCompactSummary")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        return None;
    }
    if let Some(kind) = value
        .get("origin")
        .and_then(|origin| origin.get("kind"))
        .and_then(serde_json::Value::as_str)
        && kind != "human"
    {
        return None;
    }
    let content = value.get("message")?.get("content")?;
    if let serde_json::Value::Array(parts) = content
        && parts
            .iter()
            .any(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("tool_result"))
    {
        return None;
    }
    let text = extract_text(Some(content));
    let trimmed = text.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return None;
    }
    let key = string_field(value, "uuid")
        .or_else(|| string_field(value, "timestamp"))
        .unwrap_or_default();
    Some((text, key))
}

/// Whether a raw line can matter to the subagent log: an Agent (or Task)
/// call, an answer to a call the log waits on, a task notification, or
/// (AJ-E1520, 0.3) a person's message.
pub(crate) fn subtask_line_matters(log: &crate::subtasks::SubtaskLog, line: &[u8]) -> bool {
    use crate::subtasks::{holds, values_of};
    if holds(line, b"<task-notification>") {
        return true;
    }
    if holds(line, b"tool_use") && (holds(line, b"\"Agent\"") || holds(line, b"\"Task\"")) {
        return true;
    }
    if holds(line, b"\"type\":\"user\"") && !holds(line, b"tool_result") {
        return true;
    }
    values_of(line, b"tool_use_id")
        .into_iter()
        .any(|id| std::str::from_utf8(id).is_ok_and(|id| log.wants(id)))
}

/// The words of a line that carries a task notification, from where
/// Claude Code puts one (never a tool's description or a tool's output
/// that only mentions the tag).
fn notification_text(value: &serde_json::Value) -> Option<&str> {
    const TAG: &str = "<task-notification>";
    let text = match value.get("type").and_then(serde_json::Value::as_str)? {
        "user" => {
            let origin = value.get("origin")?.get("kind")?.as_str()?;
            if origin != "task-notification" {
                return None;
            }
            value.get("message")?.get("content")?.as_str()?
        }
        "attachment" => {
            let attachment = value.get("attachment")?;
            if attachment.get("type")?.as_str()? != "queued_command" {
                return None;
            }
            attachment.get("prompt")?.as_str()?
        }
        "queue-operation" => {
            if value.get("operation")?.as_str()? != "enqueue" {
                return None;
            }
            value.get("content")?.as_str()?
        }
        _ => return None,
    };
    text.trim_start().starts_with(TAG).then_some(text)
}

/// AJ-E1520 (0.3): one task notification: the agent, the call that
/// started it, how it ended and what it said.
struct Told {
    agent: String,
    call: Option<String>,
    state: crate::subtasks::SubtaskState,
    ending: crate::subtasks::Ending,
}

/// Each notification's agent and how it ended (a status this reader
/// doesn't know is left out: never a guess), with its result, summary
/// and usage (`<usage>` holds `<subagent_tokens>`, `<tool_uses>` and
/// `<duration_ms>`).
fn notifications(text: &str) -> Vec<Told> {
    use crate::subtasks::SubtaskState;
    let tag = |block: &'_ str, name: &str| -> Option<String> {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let start = block.find(&open)? + open.len();
        let end = block[start..].find(&close)? + start;
        Some(block[start..end].trim().to_owned())
    };
    let usage =
        |block: &str, key: &str| -> Option<u64> { tag(&tag(block, "usage")?, key)?.parse().ok() };
    let mut out = Vec::new();
    for block in text.split("<task-notification>").skip(1) {
        let block = block.split("</task-notification>").next().unwrap_or(block);
        let (Some(agent), Some(status)) = (tag(block, "task-id"), tag(block, "status")) else {
            continue;
        };
        let state = match status.as_str() {
            "completed" => SubtaskState::Done,
            "failed" => SubtaskState::Failed,
            "killed" | "stopped" => SubtaskState::Stopped,
            _ => continue,
        };
        let summary = tag(block, "summary");
        let result = tag(block, "result");
        let failed = state == SubtaskState::Failed;
        out.push(Told {
            agent,
            call: tag(block, "tool-use-id"),
            state,
            ending: crate::subtasks::Ending {
                failure: failed
                    .then(|| result.clone().or_else(|| summary.clone()))
                    .flatten(),
                result: if failed { None } else { result.or(summary) },
                reported: crate::subtasks::Reported {
                    tokens: usage(block, "subagent_tokens"),
                    tool_uses: usage(block, "tool_uses").and_then(|n| u32::try_from(n).ok()),
                    duration_ms: usage(block, "duration_ms").and_then(|n| i64::try_from(n).ok()),
                    ..crate::subtasks::Reported::default()
                },
            },
        });
    }
    out
}

/// The `tool_use` parts of a message's content: (tool name, its input).
/// AJ-E1703: one line's tool calls, for what the session is working on
/// (the assistant's `tool_use` parts).
pub(crate) fn activity_line(log: &mut crate::activity::ActivityLog, value: &serde_json::Value) {
    if value.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
        return;
    }
    for (name, input) in tool_uses(value.get("message").and_then(|m| m.get("content"))) {
        log.call(name, input);
    }
}

fn tool_uses(content: Option<&serde_json::Value>) -> Vec<(&str, &serde_json::Value)> {
    let Some(serde_json::Value::Array(parts)) = content else {
        return Vec::new();
    };
    parts
        .iter()
        .filter(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("tool_use"))
        .filter_map(|part| {
            Some((
                part.get("name")?.as_str()?,
                part.get("input").unwrap_or(&serde_json::Value::Null),
            ))
        })
        .collect()
}

fn extract_usage(usage: Option<&serde_json::Value>) -> Option<TokenCounts> {
    let usage = usage?;
    let read = |key: &str| {
        usage
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
    };
    Some(TokenCounts {
        input: read("input_tokens"),
        output: read("output_tokens"),
        cache_read: read("cache_read_input_tokens"),
        cache_write: read("cache_creation_input_tokens"),
        reasoning: 0,
    })
}

fn accumulate_usage(total: &mut TokenCounts, usage: Option<TokenCounts>) {
    if let Some(usage) = usage {
        total.input += usage.input;
        total.output += usage.output;
        total.cache_read += usage.cache_read;
        total.cache_write += usage.cache_write;
        total.reasoning += usage.reasoning;
    }
}

/// Removes messages whose `uuid` repeats (resumed sessions re-emit the
/// history — the ccusage dedupe pattern). First occurrence wins; messages
/// without a uuid are always kept.
///
/// Returns the deduped list in original order.
#[must_use]
pub fn dedupe_messages(messages: &[ParsedMessage]) -> Vec<ParsedMessage> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for message in messages {
        let keep = match &message.uuid {
            Some(uuid) => seen.insert(uuid.clone()),
            None => true,
        };
        if keep {
            out.push(message.clone());
        }
    }
    out
}

impl ParsedTranscript {
    /// Whether the transcript contains a `compact_boundary` line
    /// (AJ-E203): post-compact live context ≠ the raw file, so the
    /// fidelity engine caps such sources at F2.
    #[must_use]
    pub fn has_compact_boundary(&self) -> bool {
        // The verified line shape is `type:system,
        // subtype:compact_boundary` (session-malformed.jsonl:6); a
        // bare `compact_boundary` type is tolerated defensively.
        self.subtypes
            .iter()
            .any(|(key, count)| key.ends_with("/compact_boundary") && *count > 0)
            || self
                .kinds
                .get("compact_boundary")
                .is_some_and(|count| *count > 0)
            // Codex rollouts mark a compaction with a `compacted` line
            // (AJ-E1303) — the same cap applies.
            || self.kinds.get("compacted").is_some_and(|count| *count > 0)
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::parse_transcript;

    #[test]
    fn empty_input_is_valid() {
        let parsed = parse_transcript(b"");
        assert_eq!(parsed.lines_total, 0);
        assert!(parsed.messages.is_empty());
    }
}
