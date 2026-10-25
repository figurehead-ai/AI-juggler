//! Codex rollout parsing (AJ-E1303, spec §4.2) into the same neutral
//! [`ParsedTranscript`] the app uses for every Provider.
//!
//! A rollout is `sessions/YYYY/MM/DD/rollout-<time>-<thread uuid>.jsonl`;
//! each line is `{timestamp, ordinal?, type, payload}` (codex-rs
//! `protocol/src/protocol.rs`, `rollout/src/policy.rs` at
//! `rust-v0.163.0`). The conversation is read from `response_item`
//! messages — the one carrier both history modes write: since 0.158 new
//! threads are `paginated` and no longer write `event_msg`
//! `user_message`/`agent_message`. Tokens come from `token_usage_record`
//! (response level, the same rows the usage pipeline counts), the model
//! from the latest `turn_context`, compaction from `compacted` lines.
//! Tolerant like Claude's parser: malformed lines are counted, never
//! fatal; unknown types are counted.

use juggler_core::model::TokenCounts;

use crate::claude_transcript::{MessageRole, ParsedMessage, ParsedTranscript};

/// User-role messages Codex injects itself (environment, instructions,
/// permissions) — context, not something the person typed. Left out of
/// the conversation.
const INJECTED_PREFIXES: &[&str] = &[
    "<environment_context>",
    "<user_instructions>",
    "<permissions instructions>",
    "<turn_aborted>",
    "# AGENTS.md instructions",
];

/// Whether a user-role message is one Codex itself injects (left out of
/// the conversation).
fn is_codex_context(body: &str) -> bool {
    let body = body.trim_start();
    INJECTED_PREFIXES.iter().any(|prefix| body.starts_with(prefix))
}

/// Whether a user-role message is one Codex, its app or a harness injects
/// rather than something the person typed, so never a title: Codex's own
/// context, or any tag-wrapped block (the app's `<external_…>`, …). The
/// conversation keeps the latter: a person can open with a tag too.
#[must_use]
pub fn is_injected(body: &str) -> bool {
    is_codex_context(body) || is_tagged(body.trim_start())
}

/// Whether `text` opens with a tag: `<`, an identifier-like name, then
/// `>` (or attributes and a `>` on the same line). `<3` or `<- see` is
/// not a tag.
fn is_tagged(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('<') else {
        return false;
    };
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    if !rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return false;
    }
    let name_end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')))
        .unwrap_or(rest.len());
    let after = &rest[name_end..];
    after.starts_with('>')
        || (after.starts_with([' ', '\t'])
            && after.lines().next().is_some_and(|line| line.contains('>')))
}

/// Parses rollout bytes. Never fails; damage is counted.
#[must_use]
pub fn parse_rollout(bytes: &[u8]) -> ParsedTranscript {
    let mut parser = RolloutParser::new(true);
    for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
        parser.line(index, line);
    }
    parser.finish()
}

/// A rollout parsed a line at a time, so a rollout of any size needs only
/// its current line in memory ([`crate::transcript::for_each_line`]).
/// [`parse_rollout`] is this over a whole file's bytes.
#[derive(Debug, Default)]
pub struct RolloutParser {
    parsed: ParsedTranscript,
    session_id: Option<String>,
    model: Option<String>,
    // One usage row per response: a forked or resumed rollout repeats
    // its parent's rows (the usage pipeline's dedupe key).
    counted: std::collections::BTreeSet<String>,
    // AJ-E1541: tool calls wait for what Codex says next, so a call and
    // the reply after it are one turn ("ran tests; added the test").
    pending_tools: Vec<String>,
    /// Keep the messages, or only the first thing the user asked (a
    /// conversation's summary needs no more).
    keep_messages: bool,
    first_ask: Option<String>,
}

impl RolloutParser {
    /// A parser; with `keep_messages` false it keeps only the first thing
    /// the user asked ([`Self::first_ask`]), not every message.
    #[must_use]
    pub fn new(keep_messages: bool) -> Self {
        Self {
            keep_messages,
            ..Self::default()
        }
    }

    /// One line of the rollout, by its index in the file.
    pub fn line(&mut self, index: usize, line: &[u8]) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) {
            self.value(index, &value);
        } else {
            self.parsed.lines_total += 1;
            self.parsed.lines_malformed += 1;
        }
    }

    /// One line already parsed (a caller that reads the line for itself
    /// too parses it once).
    pub fn value(&mut self, index: usize, value: &serde_json::Value) {
        let parsed = &mut self.parsed;
        parsed.lines_total += 1;
        let kind = text(value, "type").unwrap_or_else(|| "unknown".to_owned());
        *parsed.kinds.entry(kind.clone()).or_default() += 1;
        let null = serde_json::Value::Null;
        let payload = value.get("payload").unwrap_or(&null);
        if let Some(subtype) = text(payload, "type") {
            *parsed
                .subtypes
                .entry(format!("{kind}/{subtype}"))
                .or_default() += 1;
        }
        match kind.as_str() {
            "session_meta" => self.session_id = text(payload, "id").or(self.session_id.take()),
            "turn_context" => self.model = text(payload, "model").or(self.model.take()),
            "token_usage_record" => {
                let key = ["thread_id", "turn_id", "response_id"]
                    .map(|field| text(payload, field).unwrap_or_default())
                    .join("/");
                if let Some(usage) = payload.get("usage")
                    && self.counted.insert(key)
                {
                    accumulate(&mut parsed.usage, usage);
                }
            }
            "response_item" if text(payload, "type").as_deref() == Some("message") => {
                let role = match text(payload, "role").as_deref() {
                    Some("user") => MessageRole::User,
                    Some("assistant") => MessageRole::Assistant,
                    _ => return, // developer/system instructions
                };
                let body = message_text(payload.get("content"));
                if body.trim().is_empty() || (role == MessageRole::User && is_codex_context(&body))
                {
                    return;
                }
                if role == MessageRole::User && self.first_ask.is_none() && !is_injected(&body) {
                    self.first_ask = Some(body.clone());
                }
                if !self.keep_messages {
                    return;
                }
                if role == MessageRole::User && !self.pending_tools.is_empty() {
                    flush_tools(
                        parsed,
                        &mut self.pending_tools,
                        self.session_id.as_deref(),
                        index,
                    );
                }
                parsed.messages.push(ParsedMessage {
                    // Rollouts are append-only: the line number is a
                    // stable identity when the item carries no id.
                    // Scoped by thread: line numbers repeat across
                    // rollouts, ids from two files must never collide.
                    uuid: text(payload, "id").or_else(|| {
                        Some(format!(
                            "{}:line:{index}",
                            self.session_id.as_deref().unwrap_or("rollout")
                        ))
                    }),
                    parent_uuid: None,
                    session_id: self.session_id.clone(),
                    role,
                    text: body,
                    tools: if role == MessageRole::Assistant {
                        std::mem::take(&mut self.pending_tools)
                    } else {
                        Vec::new()
                    },
                    usage: None,
                    model: (role == MessageRole::Assistant)
                        .then(|| self.model.clone())
                        .flatten(),
                    sidechain: false,
                });
            }
            // AJ-E1541: a tool call is its own item in a rollout: one line
            // saying what it did, and the plan it set.
            "response_item"
                if self.keep_messages
                    && matches!(
                        text(payload, "type").as_deref(),
                        Some("function_call" | "custom_tool_call")
                    ) =>
            {
                tool_call(parsed, &mut self.pending_tools, payload);
            }
            _ => {}
        }
    }

    /// The first thing the user asked (not a block Codex, its app or a
    /// harness injects: [`is_injected`]).
    #[must_use]
    pub fn first_ask(&self) -> Option<&str> {
        self.first_ask.as_deref()
    }

    /// The usage so far (each response once).
    #[must_use]
    pub fn usage(&self) -> TokenCounts {
        self.parsed.usage
    }

    /// The parse, once every line is in.
    #[must_use]
    pub fn finish(mut self) -> ParsedTranscript {
        flush_tools(
            &mut self.parsed,
            &mut self.pending_tools,
            self.session_id.as_deref(),
            usize::MAX,
        );
        self.parsed
    }
}

/// AJ-E1541: a rollout's tool call (`function_call`, or a custom call
/// such as `apply_patch`): one line saying what it did, kept for the next
/// thing Codex says, and the plan it set, if it set one.
fn tool_call(
    parsed: &mut ParsedTranscript,
    pending: &mut Vec<String>,
    payload: &serde_json::Value,
) {
    let Some(name) = text(payload, "name") else {
        return;
    };
    // `function_call` carries JSON in a string; a custom call its raw input.
    let input = match text(payload, "arguments") {
        Some(arguments) => serde_json::from_str(&arguments).unwrap_or_default(),
        None => serde_json::json!({ "input": text(payload, "input").unwrap_or_default() }),
    };
    if let Some(tasks) = crate::tool_summary::task_list(&name, &input) {
        parsed.tasks = Some(tasks);
    }
    pending.push(crate::tool_summary::summarize(&name, &input));
}

/// Tool calls nothing was said after (the turn ended on them, or the user
/// spoke next): a turn of their own.
fn flush_tools(
    parsed: &mut ParsedTranscript,
    pending: &mut Vec<String>,
    session_id: Option<&str>,
    index: usize,
) {
    if pending.is_empty() {
        return;
    }
    parsed.messages.push(ParsedMessage {
        uuid: Some(format!("{}:tools:{index}", session_id.unwrap_or("rollout"))),
        parent_uuid: None,
        session_id: session_id.map(str::to_owned),
        role: MessageRole::Assistant,
        text: String::new(),
        tools: std::mem::take(pending),
        usage: None,
        model: None,
        sidechain: false,
    });
}

/// The thread id a rollout belongs to: `session_meta.payload.id` (the
/// id `codex resume` takes), else the UUID at the end of the file name.
#[must_use]
pub fn rollout_thread_id(bytes: &[u8], file_name: &str) -> Option<String> {
    let text_bytes = String::from_utf8_lossy(bytes);
    for line in text_bytes.lines().take(5) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
            && text(&value, "type").as_deref() == Some("session_meta")
            && let Some(id) = value.get("payload").and_then(|p| text(p, "id"))
        {
            return Some(id);
        }
    }
    thread_id_from_name(file_name)
}

/// `rollout-2026-10-20T08-00-00-<uuid>.jsonl` (or `…-<uuid>_<rollout>`
/// after a revert) → `<uuid>`.
#[must_use]
pub fn thread_id_from_name(file_name: &str) -> Option<String> {
    let stem = file_name
        .strip_suffix(".jsonl.zst")
        .or_else(|| file_name.strip_suffix(".jsonl"))?;
    let stem = stem.strip_prefix("rollout-")?;
    // The UUID is the last 36 characters before any `_rolloutId`.
    let base = stem.split('_').next()?;
    let id = base.get(base.len().checked_sub(36)?..)?;
    (id.len() == 36 && id.chars().filter(|c| *c == '-').count() == 4).then(|| id.to_owned())
}

/// The session's working directory from `session_meta` (the discovery
/// key for linking and transplant).
#[must_use]
pub fn rollout_cwd(bytes: &[u8]) -> Option<String> {
    let text_bytes = String::from_utf8_lossy(bytes);
    text_bytes.lines().take(5).find_map(|line| {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        (text(&value, "type").as_deref() == Some("session_meta"))
            .then(|| value.get("payload").and_then(|p| text(p, "cwd")))
            .flatten()
    })
}

/// AJ-E1520: a rollout line's part in the session's subagents
/// (codex-rs `core/src/tools/handlers/multi_agents/`). `spawn_agent` starts
/// one, and its output names it (`{"agent_id", "nickname"}`); a
/// `wait`/`wait_agent` output says how each named agent stands
/// (`{"status": {id: status}}`), a `close_agent` output that the agent it
/// targeted is shut down, and a `sub_agent_activity` event that one
/// completed. The `collab_*` events never reach a rollout (`rollout/src/
/// policy.rs` keeps them out).
pub(crate) fn subtask_line(log: &mut crate::subtasks::SubtaskLog, value: &serde_json::Value) {
    use crate::subtasks::SubtaskState;
    let null = serde_json::Value::Null;
    let payload = value.get("payload").unwrap_or(&null);
    let item = text(payload, "type");
    match (text(value, "type").as_deref(), item.as_deref()) {
        (Some("response_item"), Some("function_call")) => {
            let (Some(name), Some(call)) = (text(payload, "name"), text(payload, "call_id")) else {
                return;
            };
            if !matches!(
                name.as_str(),
                "spawn_agent" | "close_agent" | "wait" | "wait_agent"
            ) {
                return;
            }
            let arguments: serde_json::Value = text(payload, "arguments")
                .and_then(|arguments| serde_json::from_str(&arguments).ok())
                .unwrap_or_default();
            match name.as_str() {
                "spawn_agent" => {
                    let task = text(&arguments, "message")
                        .unwrap_or_else(|| message_text(arguments.get("items")));
                    log.started(&call, &task, text(&arguments, "agent_type").as_deref());
                }
                "close_agent" => {
                    if let Some(target) =
                        text(&arguments, "target").or_else(|| text(&arguments, "id"))
                    {
                        log.awaits(&call, &target);
                    }
                }
                _ => log.polls(&call),
            }
        }
        (Some("response_item"), Some("function_call_output")) => {
            // Other calls' outputs (a shell's, often large) aren't read.
            let Some(call) = text(payload, "call_id").filter(|call| log.wants(call)) else {
                return;
            };
            let output: serde_json::Value =
                serde_json::from_str(&message_text(payload.get("output"))).unwrap_or_default();
            if let Some(agent) = text(&output, "agent_id") {
                log.named(&call, &agent);
            } else if let Some(previous) = output.get("previous_status") {
                // Closed: as it last stood when it had ended, else done.
                log.answered(&call, agent_state(previous).unwrap_or(SubtaskState::Done));
            } else if let Some(serde_json::Value::Object(statuses)) = output.get("status") {
                log.heard(&call);
                for (agent, status) in statuses {
                    if let Some(state) = agent_state(status) {
                        log.ended(agent, state);
                    }
                }
            } else {
                // Nothing usable: a spawn that named no agent never started.
                log.not_started(&call);
            }
        }
        (Some("event_msg"), Some("sub_agent_activity"))
            if text(payload, "kind").as_deref() == Some("completed") =>
        {
            if let Some(agent) = text(payload, "agent_thread_id") {
                log.ended(&agent, SubtaskState::Done);
            }
        }
        _ => {}
    }
}

/// An agent's status (`"running"`, `{"completed": …}`, `{"errored": …}`,
/// `"shutdown"`, `"not_found"`) as an ending, or `None` while it runs.
fn agent_state(status: &serde_json::Value) -> Option<crate::subtasks::SubtaskState> {
    use crate::subtasks::SubtaskState;
    let name = match status {
        serde_json::Value::String(name) => name.as_str(),
        serde_json::Value::Object(map) => map.keys().next()?.as_str(),
        _ => return None,
    };
    match name {
        "completed" | "shutdown" | "not_found" => Some(SubtaskState::Done),
        "errored" => Some(SubtaskState::Failed),
        _ => None,
    }
}

fn text(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Text parts of a message's `content` (`input_text` / `output_text` /
/// `text`); images and anything else contribute nothing.
fn message_text(content: Option<&serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter(|part| {
                matches!(
                    part.get("type").and_then(serde_json::Value::as_str),
                    Some("input_text" | "output_text" | "text")
                )
            })
            .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Adds one response's usage in the same sense as Claude's: `input` is
/// the FRESH prompt (`OpenAI`'s `input_tokens` includes the cached part,
/// which is the re-sent history), `cache_read` the cached part. Summing
/// raw `input_tokens` would grow with the square of the conversation.
fn accumulate(total: &mut TokenCounts, usage: &serde_json::Value) {
    let read = |key: &str| {
        usage
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
    };
    let cached = read("cached_input_tokens");
    total.input += read("input_tokens").saturating_sub(cached);
    total.output += read("output_tokens");
    total.cache_read += cached;
    total.cache_write += read("cache_write_input_tokens");
    total.reasoning += read("reasoning_output_tokens");
}

/// The whole-file parser this module's [`RolloutParser`] replaced, kept
/// as it was so the tests can hold the new one to it (0.1: transcripts
/// read in bounded memory must say exactly what they said).
#[cfg(test)]
pub(crate) mod reference {
    use super::*;

    pub fn parse_rollout(bytes: &[u8]) -> ParsedTranscript {
        let mut parsed = ParsedTranscript::default();
        let mut session_id: Option<String> = None;
        let mut model: Option<String> = None;
        // One usage row per response: a forked or resumed rollout repeats
        // its parent's rows (the usage pipeline's dedupe key).
        let mut counted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        // AJ-E1541: tool calls wait for what Codex says next, so a call and
        // the reply after it are one turn ("ran tests; added the test").
        let mut pending_tools: Vec<String> = Vec::new();
        for (index, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            parsed.lines_total += 1;
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
                parsed.lines_malformed += 1;
                continue;
            };
            let kind = text(&value, "type").unwrap_or_else(|| "unknown".to_owned());
            *parsed.kinds.entry(kind.clone()).or_default() += 1;
            let payload = value.get("payload").cloned().unwrap_or_default();
            if let Some(subtype) = text(&payload, "type") {
                *parsed
                    .subtypes
                    .entry(format!("{kind}/{subtype}"))
                    .or_default() += 1;
            }
            match kind.as_str() {
                "session_meta" => session_id = text(&payload, "id").or(session_id),
                "turn_context" => model = text(&payload, "model").or(model),
                "token_usage_record" => {
                    let key = ["thread_id", "turn_id", "response_id"]
                        .map(|field| text(&payload, field).unwrap_or_default())
                        .join("/");
                    if let Some(usage) = payload.get("usage")
                        && counted.insert(key)
                    {
                        accumulate(&mut parsed.usage, usage);
                    }
                }
                "response_item" if text(&payload, "type").as_deref() == Some("message") => {
                    let role = match text(&payload, "role").as_deref() {
                        Some("user") => MessageRole::User,
                        Some("assistant") => MessageRole::Assistant,
                        _ => continue, // developer/system instructions
                    };
                    let body = message_text(payload.get("content"));
                    if body.trim().is_empty()
                        || (role == MessageRole::User && is_codex_context(&body))
                    {
                        continue;
                    }
                    if role == MessageRole::User && !pending_tools.is_empty() {
                        flush_tools(
                            &mut parsed,
                            &mut pending_tools,
                            session_id.as_deref(),
                            index,
                        );
                    }
                    parsed.messages.push(ParsedMessage {
                        // Rollouts are append-only: the line number is a
                        // stable identity when the item carries no id.
                        // Scoped by thread: line numbers repeat across
                        // rollouts, ids from two files must never collide.
                        uuid: text(&payload, "id").or_else(|| {
                            Some(format!(
                                "{}:line:{index}",
                                session_id.as_deref().unwrap_or("rollout")
                            ))
                        }),
                        parent_uuid: None,
                        session_id: session_id.clone(),
                        role,
                        text: body,
                        tools: if role == MessageRole::Assistant {
                            std::mem::take(&mut pending_tools)
                        } else {
                            Vec::new()
                        },
                        usage: None,
                        model: (role == MessageRole::Assistant)
                            .then(|| model.clone())
                            .flatten(),
                        sidechain: false,
                    });
                }
                // AJ-E1541: a tool call is its own item in a rollout: one line
                // saying what it did, and the plan it set.
                "response_item"
                    if matches!(
                        text(&payload, "type").as_deref(),
                        Some("function_call" | "custom_tool_call")
                    ) =>
                {
                    tool_call(&mut parsed, &mut pending_tools, &payload);
                }
                _ => {}
            }
        }
        flush_tools(
            &mut parsed,
            &mut pending_tools,
            session_id.as_deref(),
            usize::MAX,
        );
        parsed
    }
}
