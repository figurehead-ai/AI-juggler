//! One transcript line as the conversation feed reads it (ADR-0024): who
//! spoke and what their message holds, in order (words, thinking, tool
//! calls with the inputs a title needs), or which calls a line answers.
//! Parsed once per line, from the line itself: no position among other
//! lines is needed, so a line means the same read alone, in a window, or
//! as it is appended.
//!
//! Claude Code's lines and Codex's rollout lines are read here, each by
//! its own rules (the ones `claude_transcript` and `codex_transcript`
//! apply to a whole file, and what the Claude app draws:
//! `docs/research/native-conversation-display.md` §1); what makes a turn
//! is the feed's business ([`crate::conversation_feed`]).

use serde_json::Value;

use crate::conversation_feed::{CallInput, Speaker};

/// What one line says to the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Said {
    /// Nothing: a progress record, a line the tool marks as its own, an
    /// unknown kind.
    Nothing,
    /// The model the replies after it run on (a rollout's `turn_context`).
    Model(String),
    /// A message.
    Message(Message),
    /// Results of earlier calls (a Claude Code `tool_result` line).
    Results {
        /// Which calls, and how they ended.
        results: Vec<CallResult>,
        /// A subagent's line.
        sidechain: bool,
    },
}

/// A message: its parts in order, raw (the feed redacts them).
#[allow(clippy::struct_field_names)] // `message_id` is the transcript's own name
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    pub speaker: Speaker,
    pub parts: Vec<Part>,
    /// A subagent's line (Claude Code's `isSidechain`).
    pub sidechain: bool,
    /// The line's own id, when it has one (repeated lines share it).
    pub uuid: Option<String>,
    /// The message's id (several lines of one reply share it).
    pub message_id: Option<String>,
    /// The model it ran on, when the line says.
    pub model: Option<String>,
    /// When it was written (epoch ms), when the line says.
    pub at_ms: Option<i64>,
    /// AJ-E3106 Req.3: the tool marked it its turn's reply (Codex's
    /// `phase: "final_answer"`); its commentary folds with the work.
    pub final_answer: bool,
}

impl Message {
    /// Its words, the text parts joined (trimmed).
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A part of a message.
#[allow(clippy::large_enum_variant)] // one per part of one line, never stored
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Part {
    /// Words (trimmed, never empty).
    Text(String),
    /// The model thought: its words when the line keeps them, and
    /// whether its signature marks them as narration (drawn like words).
    Thought { text: String, narration: bool },
    /// A tool call.
    Call(Call),
}

/// One tool call.
#[allow(clippy::struct_field_names)] // `call_id` is the transcript's own name
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    /// What it did in a few words (`ran "pnpm test"`, `Edit a.ts`).
    pub line: String,
    /// The tool's name as the transcript gives it.
    pub tool: String,
    /// Its id, which its result names.
    pub call_id: Option<String>,
    /// The inputs a title needs.
    pub input: CallInput,
}

/// How a call ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallResult {
    pub call_id: String,
    pub failed: bool,
    /// What it did to a file (`create`, `update`), when it says.
    pub kind: Option<String>,
}

/// A Claude Code line.
pub(crate) fn claude(value: &Value) -> Said {
    let kind = string(value, "type");
    // A line the tool marks as its own (the caveats around a local
    // command, a peer's message) is not the person's.
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return Said::Nothing;
    }
    let sidechain = value.get("isSidechain").and_then(Value::as_bool) == Some(true);
    match kind.as_deref() {
        Some("user" | "assistant") => {}
        Some("attachment") => return queued_prompt(value, sidechain),
        _ => return Said::Nothing,
    }
    let Some(message) = value.get("message") else {
        return Said::Nothing;
    };
    let speaker = match message.get("role").and_then(Value::as_str) {
        Some("user" | "human") => Speaker::You,
        Some("assistant") => Speaker::Assistant,
        Some(_) => return Said::Nothing,
        None if kind.as_deref() == Some("user") => Speaker::You,
        None => Speaker::Assistant,
    };
    let mut parts: Vec<Part> = Vec::new();
    let mut results = Vec::new();
    match message.get("content") {
        Some(Value::String(words)) => push_words(&mut parts, words, speaker),
        Some(Value::Array(items)) => {
            for item in items {
                match item.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(words) = item.get("text").and_then(Value::as_str) {
                            push_words(&mut parts, words, speaker);
                        }
                    }
                    Some("thinking" | "redacted_thinking") if speaker == Speaker::Assistant => {
                        parts.push(Part::Thought {
                            text: string(item, "thinking")
                                .unwrap_or_default()
                                .trim()
                                .to_owned(),
                            narration: item
                                .get("signature")
                                .and_then(Value::as_str)
                                .is_some_and(is_narration),
                        });
                    }
                    Some("tool_use") if speaker == Speaker::Assistant => {
                        if let Some(name) = item.get("name").and_then(Value::as_str) {
                            parts.push(Part::Call(call(
                                name,
                                string(item, "id"),
                                item.get("input").unwrap_or(&Value::Null),
                            )));
                        }
                    }
                    Some("tool_result") => {
                        if let Some(call_id) = string(item, "tool_use_id") {
                            results.push(CallResult {
                                call_id,
                                failed: item.get("is_error").and_then(Value::as_bool) == Some(true),
                                kind: value
                                    .get("toolUseResult")
                                    .and_then(|result| string(result, "type")),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    if parts.is_empty() {
        return if results.is_empty() {
            Said::Nothing
        } else {
            Said::Results { results, sidechain }
        };
    }
    let uuid = string(value, "uuid");
    Said::Message(Message {
        speaker,
        parts,
        sidechain,
        message_id: string(message, "id").or_else(|| uuid.clone()),
        uuid,
        model: string(value, "model").or_else(|| string(message, "model")),
        at_ms: at_ms(value),
        final_answer: false,
    })
}

/// A message the person sent while the model worked: queued, then given
/// to it (`attachment.type` `queued_command`, from a person, as a
/// prompt). Task notifications are queued the same way but aren't the
/// person's.
fn queued_prompt(value: &Value, sidechain: bool) -> Said {
    let Some(attachment) = value.get("attachment") else {
        return Said::Nothing;
    };
    let human = attachment
        .get("origin")
        .and_then(|origin| string(origin, "kind"))
        .as_deref()
        == Some("human");
    if string(attachment, "type").as_deref() != Some("queued_command")
        || !human
        || string(attachment, "commandMode").as_deref() != Some("prompt")
    {
        return Said::Nothing;
    }
    let mut parts = Vec::new();
    match attachment.get("prompt") {
        Some(Value::String(words)) => push_words(&mut parts, words, Speaker::You),
        Some(Value::Array(items)) => {
            for item in items {
                if let Some(words) = item.get("text").and_then(Value::as_str) {
                    push_words(&mut parts, words, Speaker::You);
                }
            }
        }
        _ => {}
    }
    if parts.is_empty() {
        return Said::Nothing;
    }
    let uuid = string(value, "uuid");
    Said::Message(Message {
        speaker: Speaker::You,
        parts,
        sidechain,
        message_id: uuid.clone(),
        uuid,
        model: None,
        at_ms: at_ms(value),
        final_answer: false,
    })
}

/// Adds `words` (trimmed; a person's without the tool's
/// `<system-reminder>` blocks), joined to the text part before it.
fn push_words(parts: &mut Vec<Part>, words: &str, speaker: Speaker) {
    let words = if speaker == Speaker::You {
        without_reminders(words)
    } else {
        words.to_owned()
    };
    let words = words.trim();
    if words.is_empty() {
        return;
    }
    if let Some(Part::Text(last)) = parts.last_mut() {
        last.push('\n');
        last.push_str(words);
    } else {
        parts.push(Part::Text(words.to_owned()));
    }
}

/// `text` without the `<system-reminder>…</system-reminder>` blocks the
/// tool adds to the person's messages.
fn without_reminders(text: &str) -> String {
    const OPEN: &str = "<system-reminder>";
    const CLOSE: &str = "</system-reminder>";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find(CLOSE) {
            rest = &rest[start + end + CLOSE.len()..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

/// A Codex rollout line; `model` is the one its last `turn_context` set.
pub(crate) fn codex(value: &Value, model: Option<&str>) -> Said {
    let null = Value::Null;
    let payload = value.get("payload").unwrap_or(&null);
    match string(value, "type").as_deref() {
        Some("turn_context") => string(payload, "model").map_or(Said::Nothing, Said::Model),
        Some("response_item") => match string(payload, "type").as_deref() {
            Some("message") => codex_message(value, payload, model),
            Some("function_call" | "custom_tool_call") => {
                let Some((name, input)) = crate::codex_transcript::call_of(payload) else {
                    return Said::Nothing;
                };
                Said::Message(Message {
                    speaker: Speaker::Assistant,
                    parts: vec![Part::Call(call(&name, string(payload, "call_id"), &input))],
                    sidechain: false,
                    uuid: None,
                    message_id: string(payload, "id"),
                    model: model.map(str::to_owned),
                    at_ms: at_ms(value),
                    final_answer: false,
                })
            }
            // Its reasoning is not kept in words: a mark only.
            Some("reasoning") => Said::Message(Message {
                speaker: Speaker::Assistant,
                parts: vec![Part::Thought {
                    text: String::new(),
                    narration: false,
                }],
                sidechain: false,
                uuid: None,
                message_id: None,
                model: None,
                at_ms: at_ms(value),
                final_answer: false,
            }),
            _ => Said::Nothing,
        },
        _ => Said::Nothing,
    }
}

fn codex_message(value: &Value, payload: &Value, model: Option<&str>) -> Said {
    let speaker = match string(payload, "role").as_deref() {
        Some("user") => Speaker::You,
        Some("assistant") => Speaker::Assistant,
        // Developer and system instructions.
        _ => return Said::Nothing,
    };
    let body = crate::codex_transcript::message_text(payload.get("content"));
    // What Codex injects itself (its environment, instructions) is
    // context, not something the person typed.
    if body.trim().is_empty()
        || (speaker == Speaker::You && crate::codex_transcript::is_codex_context(&body))
    {
        return Said::Nothing;
    }
    Said::Message(Message {
        speaker,
        parts: vec![Part::Text(body.trim().to_owned())],
        sidechain: false,
        uuid: None,
        message_id: string(payload, "id"),
        model: (speaker == Speaker::Assistant)
            .then(|| model.map(str::to_owned))
            .flatten(),
        at_ms: at_ms(value),
        final_answer: speaker == Speaker::Assistant
            && string(payload, "phase").as_deref() == Some("final_answer"),
    })
}

fn call(name: &str, call_id: Option<String>, input: &Value) -> Call {
    let field = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|words| !words.is_empty())
            .map(|words| words.chars().take(INPUT_CHARS).collect::<String>())
    };
    Call {
        line: crate::tool_summary::summarize(name, input),
        tool: name.to_owned(),
        call_id,
        input: CallInput {
            description: field("description"),
            command: field("command").or_else(|| {
                // Codex's shell takes an argument vector.
                input.get("command").and_then(Value::as_array).map(|words| {
                    words
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(INPUT_CHARS)
                        .collect()
                })
            }),
            file_path: field("file_path").or_else(|| field("notebook_path")),
            path: field("path"),
            pattern: field("pattern"),
            query: field("query"),
            subagent_type: field("subagent_type"),
            action: input.get("action").and_then(identifier),
            batch: input
                .get("actions")
                .and_then(Value::as_array)
                .map(|actions| {
                    actions
                        .iter()
                        .take(BATCH_MAX)
                        .map(|step| {
                            let action = step
                                .get("input")
                                .and_then(|input| input.get("action"))
                                .or_else(|| step.get("action"))
                                .and_then(identifier)
                                .unwrap_or_default();
                            (
                                step.get("name").and_then(identifier).unwrap_or_default(),
                                action,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            url_without_name: field("url").is_some() && field("name").is_none(),
        },
    }
}

/// The most actions of a batch kept.
const BATCH_MAX: usize = 32;

/// A value that is a plain identifier (`left_click`, `read_page`): what
/// a title may name. Anything else (words, an address) is left out.
fn identifier(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|word| {
            (1..=40).contains(&word.len())
                && word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        .map(str::to_owned)
}

/// The most characters one call input keeps.
const INPUT_CHARS: usize = 300;

/// Whether a thinking block's signature marks it as narration: the
/// signature is a base64 protobuf whose field 2 → 1 → 8 reads
/// `narration` (the Claude app draws those like words).
fn is_narration(signature: &str) -> bool {
    let Some(bytes) = base64(signature) else {
        return false;
    };
    let mut found = false;
    for outer in fields(&bytes, 2) {
        for inner in fields(outer, 1) {
            found |= fields(inner, 8).any(|value| value == b"narration");
        }
    }
    found
}

/// The length-delimited values of field `number` in a protobuf message
/// (anything malformed ends the walk).
fn fields(bytes: &[u8], number: u64) -> impl Iterator<Item = &[u8]> {
    let mut at = 0;
    std::iter::from_fn(move || {
        while at < bytes.len() {
            let key = varint(bytes, &mut at)?;
            let (field, wire) = (key >> 3, key & 7);
            match wire {
                0 => {
                    varint(bytes, &mut at)?;
                }
                1 => at += 8,
                5 => at += 4,
                2 => {
                    let len = usize::try_from(varint(bytes, &mut at)?).ok()?;
                    let value = bytes.get(at..at.checked_add(len)?)?;
                    at += len;
                    if field == number {
                        return Some(value);
                    }
                }
                _ => return None,
            }
        }
        None
    })
}

fn varint(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*at)?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

/// Standard base64 (padding optional); `None` when it isn't.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

fn at_ms(value: &Value) -> Option<i64> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(juggler_core::time::timestamp_ms)
}

fn string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "conversation_lines_tests.rs"]
mod tests;
