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
            accumulate_usage(&mut parsed.usage, message.usage);
            parsed.messages.push(message);
        }
    }
    parsed
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
